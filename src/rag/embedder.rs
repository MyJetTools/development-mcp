use std::sync::Arc;

use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use parking_lot::Mutex;

/// Multilingual on purpose: the guides are written in English, but questions
/// arrive in Russian just as often. A monolingual English model (the fastembed
/// default) silently misses those - the query and the passage end up in
/// unrelated regions of the vector space.
const MODEL: EmbeddingModel = EmbeddingModel::MultilingualE5Small;

/// E5-family models are trained with these prefixes and lose noticeable
/// accuracy without them.
const PASSAGE_PREFIX: &str = "passage: ";
const QUERY_PREFIX: &str = "query: ";

/// Kept small on purpose. The default (256) builds huge intermediate tensors
/// and spikes RSS by hundreds of MB during a rebuild; 32 costs a few extra
/// seconds once every 15 minutes, which nobody will ever notice.
const EMBED_BATCH_SIZE: usize = 32;

/// Where model weights are cached. Must point at a mounted volume in Docker,
/// otherwise the ~450MB download repeats on every container restart.
const CACHE_DIR_ENV: &str = "FASTEMBED_CACHE_PATH";
const DEFAULT_CACHE_DIR: &str = "/app/model-cache";

/// Wraps the fastembed model.
///
/// fastembed is synchronous, CPU-bound and needs `&mut self`, so every call
/// goes through `spawn_blocking` - otherwise indexing 29 documents would park
/// a tokio worker for seconds.
pub struct Embedder {
    inner: Arc<Mutex<Option<TextEmbedding>>>,
}

impl Embedder {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.inner.lock().is_some()
    }

    /// Downloads (first run only) and loads the model. Slow - call it from the
    /// background task, never from a tool call.
    pub async fn init(&self) -> Result<(), String> {
        if self.is_ready() {
            return Ok(());
        }

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let cache_dir = std::env::var(CACHE_DIR_ENV)
                .unwrap_or_else(|_| DEFAULT_CACHE_DIR.to_string());

            let options = TextInitOptions::new(MODEL)
                .with_cache_dir(std::path::PathBuf::from(cache_dir));

            let model = TextEmbedding::try_new(options)
                .map_err(|err| format!("Failed to init embedding model: {:?}", err))?;

            let mut guard = inner.lock();
            *guard = Some(model);

            Ok(())
        })
        .await;

        match result {
            Ok(result) => result,
            Err(err) => Err(format!("Embedder init task panicked: {:?}", err)),
        }
    }

    pub async fn embed_passages(&self, passages: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
        let prefixed = passages
            .into_iter()
            .map(|it| format!("{}{}", PASSAGE_PREFIX, it))
            .collect();

        self.embed(prefixed).await
    }

    pub async fn embed_query(&self, query: &str) -> Result<Vec<f32>, String> {
        let prefixed = format!("{}{}", QUERY_PREFIX, query);

        let mut result = self.embed(vec![prefixed]).await?;

        if result.is_empty() {
            return Err("Embedding model returned no vector for the query".to_string());
        }

        Ok(result.remove(0))
    }

    async fn embed(&self, documents: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut guard = inner.lock();

            let Some(model) = guard.as_mut() else {
                return Err("Embedding model is not initialized yet".to_string());
            };

            model
                .embed(documents, Some(EMBED_BATCH_SIZE))
                .map_err(|err| format!("Failed to generate embeddings: {:?}", err))
        })
        .await;

        match result {
            Ok(result) => result,
            Err(err) => Err(format!("Embedding task panicked: {:?}", err)),
        }
    }
}
