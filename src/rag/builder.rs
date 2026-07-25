use std::hash::Hasher;
use std::sync::Arc;

use ahash::AHashMap;

use crate::app::AppContext;
use crate::mcp::{all_doc_entries, scripts::load_resource_by_http};
use crate::rag::{chunk_markdown, Bm25Index, DocIndex, IndexedChunk};

/// A document as fetched from GitHub, with the hash used to decide whether
/// anything actually changed since the last poll.
pub struct FetchedDoc {
    pub filename: &'static str,
    pub name: &'static str,
    pub content: String,
    pub hash: u64,
}

fn hash_content(content: &str) -> u64 {
    let mut hasher = ahash::AHasher::default();
    hasher.write(content.as_bytes());
    hasher.finish()
}

pub async fn fetch_doc(url: &str) -> Result<String, String> {
    let result = load_resource_by_http("resource://rag-index", "text/markdown", url).await?;

    let content = result
        .contents
        .into_iter()
        .next()
        .ok_or_else(|| format!("{} returned no content", url))?;

    content
        .text
        .ok_or_else(|| format!("{} returned no text content", url))
}

/// Fetches every document in the catalog. A document that fails to download is
/// skipped rather than failing the whole poll - one unreachable README must not
/// take the index down with it.
pub async fn fetch_all_docs() -> Vec<FetchedDoc> {
    let entries = all_doc_entries();

    let mut result = Vec::with_capacity(entries.len());

    for entry in entries.iter() {
        match fetch_doc(entry.url).await {
            Ok(content) => {
                let hash = hash_content(&content);

                result.push(FetchedDoc {
                    filename: entry.filename,
                    name: entry.name,
                    content,
                    hash,
                });
            }
            Err(err) => {
                my_logger::LOGGER.write_error(
                    "rag::fetch_all_docs",
                    format!("Skipping {}: {}", entry.filename, err),
                    None.into(),
                );
            }
        }
    }

    result
}

/// True when anything at all differs from what the index was built from - a
/// changed document, a new one, or one that disappeared.
pub fn has_changes(known: &AHashMap<&'static str, u64>, fetched: &[FetchedDoc]) -> bool {
    if known.len() != fetched.len() {
        return true;
    }

    for doc in fetched {
        match known.get(doc.filename) {
            Some(hash) if *hash == doc.hash => {}
            _ => return true,
        }
    }

    false
}

/// Chunks and embeds the fetched documents. This is the expensive half - it is
/// only reached when `has_changes` says something moved.
pub async fn build_index(app: &Arc<AppContext>, docs: &[FetchedDoc]) -> Result<DocIndex, String> {
    let settings = app.get_settings();

    let mut chunks: Vec<IndexedChunk> = Vec::new();
    let mut documents_indexed = 0usize;

    for doc in docs {
        let raw_chunks = chunk_markdown(
            &doc.content,
            settings.max_chunk_chars,
            settings.min_chunk_chars,
        );

        if raw_chunks.is_empty() {
            continue;
        }

        documents_indexed += 1;

        for raw in raw_chunks {
            chunks.push(IndexedChunk {
                filename: doc.filename.to_string(),
                doc_name: doc.name.to_string(),
                heading_path: raw.heading_path,
                text: raw.text,
            });
        }
    }

    if chunks.is_empty() {
        return Err("No documents could be chunked - index would be empty".to_string());
    }

    // The heading breadcrumb is prepended to the embedded text so that a chunk
    // buried under "Creating a Tool Call > Step 2" still carries that context
    // into the vector - the body alone often does not mention it.
    let passages: Vec<String> = chunks
        .iter()
        .map(|it| {
            if it.heading_path.is_empty() {
                format!("{}\n{}", it.doc_name, it.text)
            } else {
                format!("{} > {}\n{}", it.doc_name, it.heading_path, it.text)
            }
        })
        .collect();

    let vectors = app.embedder.embed_passages(passages.clone()).await?;

    if vectors.len() != chunks.len() {
        return Err(format!(
            "Embedder returned {} vectors for {} chunks",
            vectors.len(),
            chunks.len()
        ));
    }

    // BM25 is built from exactly the same passage strings the vectors were
    // built from, so the two rankings address identical chunks by index.
    let bm25 = Bm25Index::build(&passages);

    Ok(DocIndex::new(chunks, vectors, bm25, documents_indexed))
}

/// One poll cycle: fetch, hash, and hand a rebuild to the events loop if it is
/// warranted. Shared by the timer and by the `rebuild_index` tool so both take
/// exactly the same path.
///
/// `force` skips the hash comparison - that is what makes a manual rebuild
/// useful after a chunking setting changed, since the documents themselves did
/// not move and the hashes would say there is nothing to do.
pub async fn poll_and_maybe_rebuild(app: &Arc<AppContext>, force: bool) -> PollOutcome {
    if app.index_is_rebuilding() {
        return PollOutcome::AlreadyRebuilding;
    }

    let docs = fetch_all_docs().await;

    if docs.is_empty() {
        my_logger::LOGGER.write_error(
            "rag::poll_and_maybe_rebuild",
            "No documents could be fetched - keeping the previous index".to_string(),
            None.into(),
        );

        return PollOutcome::NothingFetched;
    }

    let documents = docs.len();

    // A missing index means the previous rebuild failed (or this is the first
    // pass), so rebuild even when the hashes look unchanged.
    let index_missing = app.get_index().is_none();

    let changed = {
        let indexed_hashes = app.indexed_hashes.lock();
        has_changes(&indexed_hashes, &docs)
    };

    if !force && !index_missing && !changed {
        return PollOutcome::NoChanges { documents };
    }

    if !app.try_begin_index_rebuild() {
        return PollOutcome::AlreadyRebuilding;
    }

    app.rebuild_index_events_loop.send(docs);

    PollOutcome::RebuildStarted { documents }
}

pub enum PollOutcome {
    RebuildStarted { documents: usize },
    NoChanges { documents: usize },
    AlreadyRebuilding,
    NothingFetched,
}

impl PollOutcome {
    pub fn describe(&self) -> String {
        match self {
            Self::RebuildStarted { documents } => format!(
                "Rebuild started for {} documents. It runs in the background - \
                 call search_docs in a moment and check the build timestamp in the status.",
                documents
            ),
            Self::NoChanges { documents } => format!(
                "No rebuild needed: all {} documents are unchanged since the current index \
                 was built. Pass force=true to rebuild anyway.",
                documents
            ),
            Self::AlreadyRebuilding => {
                "A rebuild is already running - nothing was started.".to_string()
            }
            Self::NothingFetched => {
                "No documents could be fetched; the previous index was kept.".to_string()
            }
        }
    }
}
