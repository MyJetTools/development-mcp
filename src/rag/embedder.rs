use std::sync::Arc;

use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use parking_lot::Mutex;

use crate::rag::{
    DEFAULT_MODEL_CACHE_DIR, EMBED_BATCH_SIZE, MODEL_CACHE_DIR_ENV, PASSAGE_PREFIX, QUERY_PREFIX,
};

/// The embedding models worth comparing here.
///
/// The choice is not "bigger is better" but "which language do the queries
/// arrive in". A multilingual model spends most of its weights on a 250K-token
/// vocabulary; an English-only one of the same architecture is a third of the
/// size and usually sharper on English, because none of that capacity is spent
/// on scripts this corpus never uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingModelChoice {
    /// multilingual-e5-small - ~450MB. Handles Russian queries against English
    /// documents.
    MultilingualE5Small,
    /// bge-small-en-v1.5 - ~130MB. English only: a Russian query against it
    /// lands nowhere near the right passage.
    BgeSmallEn,
    /// bge-base-en-v1.5 - ~440MB. English only, stronger than small.
    BgeBaseEn,
}

impl EmbeddingModelChoice {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().replace('_', "-").as_str() {
            "e5" | "e5-small" | "multilingual-e5-small" | "multilingual" => {
                Some(Self::MultilingualE5Small)
            }
            "bge-small" | "bge-small-en" | "english" | "en" => Some(Self::BgeSmallEn),
            "bge-base" | "bge-base-en" => Some(Self::BgeBaseEn),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MultilingualE5Small => "multilingual-e5-small",
            Self::BgeSmallEn => "bge-small-en",
            Self::BgeBaseEn => "bge-base-en",
        }
    }

    /// Only the E5 family is trained with these prefixes; prepending them to a
    /// BGE model would just be noise inside the text.
    fn uses_e5_prefixes(&self) -> bool {
        matches!(self, Self::MultilingualE5Small)
    }

    fn to_fastembed(self) -> EmbeddingModel {
        match self {
            Self::MultilingualE5Small => EmbeddingModel::MultilingualE5Small,
            Self::BgeSmallEn => EmbeddingModel::BGESmallENV15,
            Self::BgeBaseEn => EmbeddingModel::BGEBaseENV15,
        }
    }
}

/// Wraps the fastembed model.
///
/// fastembed is synchronous, CPU-bound and needs `&mut self`, so every call
/// goes through `spawn_blocking` - otherwise indexing would park a tokio worker
/// for seconds at a time.
pub struct Embedder {
    inner: Arc<Mutex<Option<(EmbeddingModelChoice, TextEmbedding)>>>,
}

impl Embedder {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
        }
    }

    pub fn loaded_model(&self) -> Option<EmbeddingModelChoice> {
        self.inner.lock().as_ref().map(|(choice, _)| *choice)
    }

    /// Loads - or swaps - the model. Slow on a cold cache: call it from the
    /// rebuild loop, never from a tool call.
    pub async fn init(&self, choice: EmbeddingModelChoice) -> Result<(), String> {
        if self.loaded_model() == Some(choice) {
            return Ok(());
        }

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let cache_dir = std::env::var(MODEL_CACHE_DIR_ENV)
                .unwrap_or_else(|_| DEFAULT_MODEL_CACHE_DIR.to_string());

            let options = TextInitOptions::new(choice.to_fastembed())
                .with_cache_dir(std::path::PathBuf::from(cache_dir));

            let model = TextEmbedding::try_new(options)
                .map_err(|err| format!("Failed to init {}: {:?}", choice.as_str(), err))?;

            *inner.lock() = Some((choice, model));

            Ok(())
        })
        .await;

        match result {
            Ok(result) => result,
            Err(err) => Err(format!("Embedder init task panicked: {:?}", err)),
        }
    }

    pub async fn embed_passages(
        &self,
        choice: EmbeddingModelChoice,
        passages: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, String> {
        let prepared = if choice.uses_e5_prefixes() {
            passages
                .into_iter()
                .map(|it| format!("{}{}", PASSAGE_PREFIX, it))
                .collect()
        } else {
            passages
        };

        self.embed(choice, prepared).await
    }

    pub async fn embed_query(
        &self,
        choice: EmbeddingModelChoice,
        query: &str,
    ) -> Result<Vec<f32>, String> {
        let prepared = if choice.uses_e5_prefixes() {
            format!("{}{}", QUERY_PREFIX, query)
        } else {
            query.to_string()
        };

        let mut result = self.embed(choice, vec![prepared]).await?;

        if result.is_empty() {
            return Err("Embedding model returned no vector for the query".to_string());
        }

        Ok(result.remove(0))
    }

    async fn embed(
        &self,
        choice: EmbeddingModelChoice,
        documents: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, String> {
        self.init(choice).await?;

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut guard = inner.lock();

            let Some((loaded, model)) = guard.as_mut() else {
                return Err("Embedding model is not initialized yet".to_string());
            };

            if *loaded != choice {
                return Err(format!(
                    "Embedding model changed under the query: wanted {}, loaded {}",
                    choice.as_str(),
                    loaded.as_str()
                ));
            }

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

/// True when the text contains Cyrillic.
///
/// Used to refuse a Russian query against an English-only stack: the failure is
/// otherwise silent, and a confident answer built from the wrong passages is
/// worse than an error that says what to do.
pub fn contains_cyrillic(text: &str) -> bool {
    text.chars().any(|ch| matches!(ch, '\u{0400}'..='\u{04FF}'))
}
