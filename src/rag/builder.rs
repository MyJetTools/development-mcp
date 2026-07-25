use std::hash::Hasher;
use std::sync::Arc;

use ahash::AHashMap;

use crate::app::AppContext;
use crate::mcp::{all_doc_entries, scripts::load_resource_by_http};
use crate::rag::{chunk_markdown, DocIndex, IndexedChunk};

/// How often the documents are re-fetched and checked for changes. Cheap now
/// that a poll without changes costs 29 HTTP requests and no CPU at all.
const POLL_INTERVAL_SECS: u64 = 5 * 60;

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
async fn fetch_all_docs() -> Vec<FetchedDoc> {
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
fn has_changes(known: &AHashMap<&'static str, u64>, fetched: &[FetchedDoc]) -> bool {
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
pub async fn build_index(
    app: &Arc<AppContext>,
    docs: &[FetchedDoc],
) -> Result<DocIndex, String> {
    let mut chunks: Vec<IndexedChunk> = Vec::new();
    let mut documents_indexed = 0usize;

    for doc in docs {
        let raw_chunks = chunk_markdown(&doc.content);

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

    let vectors = app.embedder.embed_passages(passages).await?;

    if vectors.len() != chunks.len() {
        return Err(format!(
            "Embedder returned {} vectors for {} chunks",
            vectors.len(),
            chunks.len()
        ));
    }

    Ok(DocIndex::new(chunks, vectors, documents_indexed))
}

/// Polls the documents every few minutes and rebuilds the index only when one
/// of them actually changed.
///
/// The documents live in other GitHub repositories, so nothing notifies this
/// service when they are edited - polling is the only way to notice. Hashing
/// what came back keeps the common case (nothing changed) free of CPU work:
/// the guides move a couple of times a week, so almost every poll is a no-op.
///
/// The service starts serving immediately with an empty index: `search_docs`
/// reports that it is still warming up and `get_doc` keeps working regardless,
/// so a cold start never leaves the server useless.
pub fn spawn_index_builder(app: Arc<AppContext>) {
    tokio::spawn(async move {
        if let Err(err) = app.embedder.init().await {
            my_logger::LOGGER.write_fatal_error(
                "rag::spawn_index_builder",
                format!("Embedding model failed to load: {}", err),
                None.into(),
            );
            return;
        }

        // Hashes of the documents the current index was built from. Owned by
        // this task alone, so it needs no lock.
        let mut indexed_hashes: AHashMap<&'static str, u64> = AHashMap::new();

        loop {
            let docs = fetch_all_docs().await;

            if docs.is_empty() {
                my_logger::LOGGER.write_error(
                    "rag::spawn_index_builder",
                    "No documents could be fetched - keeping the previous index".to_string(),
                    None.into(),
                );

                tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
                continue;
            }

            // A missing index means the previous build failed (or this is the
            // first pass), so rebuild even when the hashes look unchanged.
            let index_missing = app.get_index().is_none();

            if !index_missing && !has_changes(&indexed_hashes, &docs) {
                tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
                continue;
            }

            match build_index(&app, &docs).await {
                Ok(index) => {
                    my_logger::LOGGER.write_info(
                        "rag::spawn_index_builder",
                        format!(
                            "Index rebuilt: {} chunks from {} documents",
                            index.chunks_amount(),
                            index.documents_indexed
                        ),
                        None.into(),
                    );

                    indexed_hashes = docs.iter().map(|it| (it.filename, it.hash)).collect();

                    app.index.store(Some(Arc::new(index)));
                }
                Err(err) => {
                    my_logger::LOGGER.write_error(
                        "rag::spawn_index_builder",
                        format!("Index rebuild failed: {}", err),
                        None.into(),
                    );
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
        }
    });
}
