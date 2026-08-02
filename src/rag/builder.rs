use std::sync::Arc;

use crate::app::AppContext;
use crate::mcp::{all_doc_entries, scripts::load_resource_by_http};
use crate::rag::{chunk_markdown, DocIndex, IndexedChunk};

/// How often the index is rebuilt. The documents live in GitHub repositories
/// that change a few times a week at most, so this is deliberately lazy.
const REBUILD_INTERVAL_SECS: u64 = 15 * 60;

/// Fetches every document from the catalog, chunks it and embeds the chunks.
///
/// The catalog (`all_doc_entries`) stays the single source of truth: adding a
/// document there is enough for it to appear in the index on the next rebuild.
pub async fn build_index(app: &Arc<AppContext>) -> Result<DocIndex, String> {
    let entries = all_doc_entries();

    let mut chunks: Vec<IndexedChunk> = Vec::new();
    let mut documents_indexed = 0usize;

    for entry in entries.iter() {
        let content = match fetch_doc(entry.url).await {
            Ok(content) => content,
            Err(err) => {
                // One unreachable README must not sink the whole index.
                my_logger::LOGGER.write_error(
                    "rag::build_index",
                    format!("Skipping {}: {}", entry.filename, err),
                    None.into(),
                );
                continue;
            }
        };

        let raw_chunks = chunk_markdown(&content);

        if raw_chunks.is_empty() {
            continue;
        }

        documents_indexed += 1;

        for raw in raw_chunks {
            chunks.push(IndexedChunk {
                filename: entry.filename.to_string(),
                doc_name: entry.name.to_string(),
                heading_path: raw.heading_path,
                text: raw.text,
            });
        }
    }

    if chunks.is_empty() {
        return Err("No documents could be fetched - index would be empty".to_string());
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

/// Builds the index in the background and republishes it on a timer.
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

        loop {
            match build_index(&app).await {
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

            tokio::time::sleep(std::time::Duration::from_secs(REBUILD_INTERVAL_SECS)).await;
        }
    });
}
