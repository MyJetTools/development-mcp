use std::sync::Arc;

use fastembed::{TextEmbedding, TextInitOptions};
use parking_lot::Mutex;

use crate::rag::{
    DEFAULT_MODEL_CACHE_DIR, EMBEDDING_MODEL, EMBED_BATCH_SIZE, MODEL_CACHE_DIR_ENV,
    PASSAGE_PREFIX, QUERY_PREFIX,
};

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
            let cache_dir = std::env::var(MODEL_CACHE_DIR_ENV)
                .unwrap_or_else(|_| DEFAULT_MODEL_CACHE_DIR.to_string());

            let options = TextInitOptions::new(EMBEDDING_MODEL)
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
