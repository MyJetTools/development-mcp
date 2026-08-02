use std::sync::Arc;

use rust_extensions::events_loop::EventsLoopTick;

use crate::app::AppContext;
use crate::rag::{build_index, FetchedDoc};

/// Does the expensive half of the work: chunk, embed, publish.
///
/// It lives in an events loop rather than in the poll timer so that a rebuild
/// running for minutes never holds up the next poll, and so that rebuilds are
/// serialized against each other by construction.
pub struct RebuildIndexEventLoop {
    app: Arc<AppContext>,
}

impl RebuildIndexEventLoop {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl EventsLoopTick<Vec<FetchedDoc>> for RebuildIndexEventLoop {
    async fn started(&self) {
        my_logger::LOGGER.write_info(
            "RebuildIndexEventLoop",
            "Docs index rebuild loop started".to_string(),
            None.into(),
        );
    }

    async fn tick(&self, docs: Vec<FetchedDoc>) {
        // Idempotent - only the first call actually loads the model. Doing it
        // here rather than at startup keeps the ~450MB download off the boot
        // path, so the server answers immediately with an empty index.
        if let Err(err) = self.app.embedder.init().await {
            my_logger::LOGGER.write_fatal_error(
                "RebuildIndexEventLoop",
                format!("Embedding model failed to load: {}", err),
                None.into(),
            );

            self.app.index_rebuild_finished();
            return;
        }

        match build_index(&self.app, &docs).await {
            Ok(index) => {
                my_logger::LOGGER.write_info(
                    "RebuildIndexEventLoop",
                    format!(
                        "Index rebuilt: {} chunks from {} documents",
                        index.chunks_amount(),
                        index.documents_indexed
                    ),
                    None.into(),
                );

                let hashes = docs.iter().map(|it| (it.filename, it.hash)).collect();

                self.app.index.store(Some(Arc::new(index)));

                *self.app.indexed_hashes.lock() = hashes;
            }
            Err(err) => {
                // Hashes are left untouched on purpose - the next poll sees the
                // documents as still unindexed and tries again.
                my_logger::LOGGER.write_error(
                    "RebuildIndexEventLoop",
                    format!("Index rebuild failed: {}", err),
                    None.into(),
                );
            }
        }

        self.app.index_rebuild_finished();
    }

    async fn finished(&self) {
        my_logger::LOGGER.write_info(
            "RebuildIndexEventLoop",
            "Docs index rebuild loop stopped".to_string(),
            None.into(),
        );
    }
}
