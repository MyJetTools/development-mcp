use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ahash::AHashMap;
use arc_swap::ArcSwapOption;
use parking_lot::Mutex;
use rust_extensions::events_loop::EventsLoop;
use rust_extensions::AppStates;

use crate::rag::{DocIndex, Embedder, FetchedDoc};

pub const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &'static str = env!("CARGO_PKG_NAME");

/// A full rebuild embeds every chunk of every document. That is minutes of CPU,
/// not seconds, so the events loop must not treat it as a stuck iteration.
const REBUILD_ITERATION_TIMEOUT_SECS: u64 = 20 * 60;

pub struct AppContext {
    pub app_states: Arc<AppStates>,
    pub embedder: Arc<Embedder>,

    /// Read on every `search_docs` call, written once per rebuild - textbook
    /// ArcSwap territory, so readers never take a lock.
    pub index: ArcSwapOption<DocIndex>,

    /// Raised when a rebuild is handed to the events loop and lowered when it
    /// finishes. Keeps the poll timer from queueing a second rebuild behind one
    /// that is still running.
    index_rebuild_in_progress: AtomicBool,

    /// Hashes of the documents the current index was built from. Updated only
    /// after a rebuild succeeds, so a failed one is retried rather than latched
    /// as up to date.
    pub indexed_hashes: Mutex<AHashMap<&'static str, u64>>,

    pub rebuild_index_events_loop: EventsLoop<Vec<FetchedDoc>>,
}

impl AppContext {
    pub async fn new() -> Self {
        AppContext {
            app_states: Arc::new(AppStates::create_initialized()),
            embedder: Arc::new(Embedder::new()),
            index: ArcSwapOption::empty(),
            index_rebuild_in_progress: AtomicBool::new(false),
            indexed_hashes: Mutex::new(AHashMap::new()),
            rebuild_index_events_loop: EventsLoop::new("RebuildDocsIndex").set_iteration_timeout(
                Duration::from_secs(REBUILD_ITERATION_TIMEOUT_SECS),
            ),
        }
    }

    pub fn get_index(&self) -> Option<Arc<DocIndex>> {
        self.index.load_full()
    }

    pub fn index_is_rebuilding(&self) -> bool {
        self.index_rebuild_in_progress.load(Ordering::Relaxed)
    }

    /// Raises the flag and reports whether this caller is the one that raised
    /// it. Returns false when a rebuild is already under way.
    pub fn try_begin_index_rebuild(&self) -> bool {
        self.index_rebuild_in_progress
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
    }

    pub fn index_rebuild_finished(&self) {
        self.index_rebuild_in_progress.store(false, Ordering::SeqCst);
    }
}
