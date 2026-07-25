use std::sync::Arc;

use rust_extensions::MyTimerTick;

use crate::app::AppContext;
use crate::rag::{fetch_all_docs, has_changes};

/// Does the cheap half of the work: fetch the documents, hash them, and decide
/// whether anything moved. The actual rebuild is handed to the events loop, so
/// this tick stays short even when a rebuild takes minutes.
pub struct PollDocsTimer {
    app: Arc<AppContext>,
}

impl PollDocsTimer {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl MyTimerTick for PollDocsTimer {
    async fn tick(&self) {
        // A rebuild is already running - fetching again would only produce a
        // second copy of work we are about to redo anyway.
        if self.app.index_is_rebuilding() {
            return;
        }

        let docs = fetch_all_docs().await;

        if docs.is_empty() {
            my_logger::LOGGER.write_error(
                "PollDocsTimer",
                "No documents could be fetched - keeping the previous index".to_string(),
                None.into(),
            );
            return;
        }

        // A missing index means the previous rebuild failed (or this is the
        // first pass), so rebuild even when the hashes look unchanged.
        let index_missing = self.app.get_index().is_none();

        let changed = {
            let indexed_hashes = self.app.indexed_hashes.lock();
            index_missing || has_changes(&indexed_hashes, &docs)
        };

        if !changed {
            return;
        }

        if !self.app.try_begin_index_rebuild() {
            return;
        }

        self.app.rebuild_index_events_loop.send(docs);
    }
}
