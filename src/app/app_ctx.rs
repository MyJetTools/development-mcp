use std::sync::Arc;

use arc_swap::ArcSwapOption;
use rust_extensions::AppStates;

use crate::rag::{DocIndex, Embedder};

pub const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &'static str = env!("CARGO_PKG_NAME");

pub struct AppContext {
    pub app_states: Arc<AppStates>,
    pub embedder: Arc<Embedder>,
    /// Read on every `search_docs` call, written once per rebuild - textbook
    /// ArcSwap territory, so readers never take a lock.
    pub index: ArcSwapOption<DocIndex>,
}

impl AppContext {
    pub async fn new() -> Self {
        AppContext {
            app_states: Arc::new(AppStates::create_initialized()),
            embedder: Arc::new(Embedder::new()),
            index: ArcSwapOption::empty(),
        }
    }

    pub fn get_index(&self) -> Option<Arc<DocIndex>> {
        self.index.load_full()
    }
}
