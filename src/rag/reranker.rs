use std::sync::Arc;

use fastembed::{RerankInitOptions, RerankerModel, TextRerank};
use parking_lot::Mutex;

use crate::rag::{DEFAULT_MODEL_CACHE_DIR, MODEL_CACHE_DIR_ENV, RERANK_BATCH_SIZE};

/// Cross-encoder reranking.
///
/// The dense index scores a query and a chunk as two independent vectors, which
/// is why its cosines pile up in a narrow band and a question the corpus does
/// not cover still scores 0.84. A cross-encoder runs the pair through one
/// encoder together, so it can actually answer "does this text answer this
/// question" rather than "are these two texts about similar things".
///
/// Loaded lazily and only when reranking is switched on: the weights are ~1.1GB
/// on top of the embedding model, and there is no reason to pay that on a cold
/// start when nobody asked for it.
pub struct Reranker {
    inner: Arc<Mutex<Option<(RerankerModelChoice, TextRerank)>>>,
}

/// The reranker models worth having here. Both multilingual on purpose - the
/// questions arrive in Russian, the guides are in English, and an English-only
/// reranker would repeat the mistake the first embedding model made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RerankerModelChoice {
    /// jina-reranker-v2-base-multilingual - ~1.1GB, one onnx file.
    JinaV2Multilingual,
    /// bge-reranker-v2-m3 - stronger, but pulls a multi-GB weights sidecar.
    BgeV2M3,
}

impl RerankerModelChoice {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().replace('_', "-").as_str() {
            "jina" | "jina-v2" | "jina-v2-multilingual" => Some(Self::JinaV2Multilingual),
            "bge" | "bge-v2-m3" | "bge-reranker-v2-m3" => Some(Self::BgeV2M3),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::JinaV2Multilingual => "jina-v2-multilingual",
            Self::BgeV2M3 => "bge-v2-m3",
        }
    }

    fn to_fastembed(self) -> RerankerModel {
        match self {
            Self::JinaV2Multilingual => RerankerModel::JINARerankerV2BaseMultiligual,
            Self::BgeV2M3 => RerankerModel::BGERerankerV2M3,
        }
    }
}

impl Reranker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
        }
    }

    pub fn loaded_model(&self) -> Option<RerankerModelChoice> {
        self.inner.lock().as_ref().map(|(choice, _)| *choice)
    }

    /// Scores each document against the query and returns `(original index,
    /// score)` best first.
    ///
    /// Loads - or swaps - the model when the requested one is not the one in
    /// memory, so the model can be changed at runtime like any other setting.
    /// Synchronous and CPU-bound, hence `spawn_blocking`.
    pub async fn rerank(
        &self,
        choice: RerankerModelChoice,
        query: String,
        documents: Vec<String>,
    ) -> Result<Vec<(usize, f32)>, String> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut guard = inner.lock();

            let needs_load = match guard.as_ref() {
                Some((loaded, _)) => *loaded != choice,
                None => true,
            };

            if needs_load {
                let cache_dir = std::env::var(MODEL_CACHE_DIR_ENV)
                    .unwrap_or_else(|_| DEFAULT_MODEL_CACHE_DIR.to_string());

                let options = RerankInitOptions::new(choice.to_fastembed())
                    .with_cache_dir(std::path::PathBuf::from(cache_dir));

                let model = TextRerank::try_new(options)
                    .map_err(|err| format!("Failed to load reranker {}: {:?}", choice.as_str(), err))?;

                *guard = Some((choice, model));
            }

            let Some((_, model)) = guard.as_mut() else {
                return Err("Reranker is not initialized".to_string());
            };

            // The generic is pinned by the query type, so the documents have
            // to be borrowed as &str too.
            let refs: Vec<&str> = documents.iter().map(|it| it.as_str()).collect();

            let scored = model
                .rerank(query.as_str(), &refs, false, Some(RERANK_BATCH_SIZE))
                .map_err(|err| format!("Reranking failed: {:?}", err))?;

            let mut result: Vec<(usize, f32)> =
                scored.into_iter().map(|it| (it.index, it.score)).collect();

            result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            Ok(result)
        })
        .await;

        match result {
            Ok(result) => result,
            Err(err) => Err(format!("Reranking task panicked: {:?}", err)),
        }
    }
}
