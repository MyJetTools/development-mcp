use std::hash::Hasher;
use std::sync::Arc;

use ahash::AHashMap;

use crate::app::AppContext;
use crate::mcp::{all_doc_entries, scripts::load_resource_by_http};
use crate::rag::{chunk_markdown, DocIndex, IndexedChunk};

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
