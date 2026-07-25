use std::sync::Arc;

use rust_extensions::MyTimerTick;

use crate::app::AppContext;
use crate::rag::poll_and_maybe_rebuild;

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
        poll_and_maybe_rebuild(&self.app, false).await;
    }
}
