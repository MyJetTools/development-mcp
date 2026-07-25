use crate::rag::{
    EmbeddingModelChoice, RerankerModelChoice, SearchMode, BM25_B, BM25_K1, DEFAULT_SEARCH_MODE,
    DEFAULT_TOP_K, EMBEDDING_MODEL, REQUIRE_ENGLISH_QUERY,
    HYBRID_CANDIDATES, MAX_CHUNK_CHARS, MAX_TOP_K, MIN_BM25_SCORE, MIN_CHUNK_CHARS,
    MIN_RERANK_SCORE, MIN_SCORE, RERANK_CANDIDATES, RERANK_ENABLED, RERANK_MODEL, RRF_K,
};

/// The knobs that can be changed without a redeploy.
///
/// Held behind an `ArcSwap`: read on every search, written only when somebody
/// deliberately tunes it. Immutable once published - a change clones, edits the
/// clone and swaps it in.
///
/// The embedding model is deliberately NOT here. Changing it means unloading
/// ~450MB and downloading another, which is a restart, not a setting.
#[derive(Clone)]
pub struct RuntimeSettings {
    // --- applied on every query, no reindex needed ---
    pub search_mode: SearchMode,
    pub require_english_query: bool,
    pub min_score: f32,
    pub min_bm25_score: f32,
    pub default_top_k: i32,
    pub max_top_k: i32,
    pub rrf_k: f32,
    pub hybrid_candidates: usize,
    pub bm25_k1: f32,
    pub bm25_b: f32,

    /// Second stage: re-score the first stage's candidates with a cross-encoder.
    pub rerank_enabled: bool,
    pub rerank_model: RerankerModelChoice,
    pub rerank_candidates: usize,
    pub min_rerank_score: f32,

    // --- only take effect on the next rebuild ---
    pub max_chunk_chars: usize,
    pub min_chunk_chars: usize,
    pub embedding_model: EmbeddingModelChoice,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            search_mode: DEFAULT_SEARCH_MODE,
            require_english_query: REQUIRE_ENGLISH_QUERY,
            min_score: MIN_SCORE,
            min_bm25_score: MIN_BM25_SCORE,
            default_top_k: DEFAULT_TOP_K,
            max_top_k: MAX_TOP_K,
            rrf_k: RRF_K,
            hybrid_candidates: HYBRID_CANDIDATES,
            bm25_k1: BM25_K1,
            bm25_b: BM25_B,
            rerank_enabled: RERANK_ENABLED,
            rerank_model: RERANK_MODEL,
            rerank_candidates: RERANK_CANDIDATES,
            min_rerank_score: MIN_RERANK_SCORE,
            max_chunk_chars: MAX_CHUNK_CHARS,
            min_chunk_chars: MIN_CHUNK_CHARS,
            embedding_model: EMBEDDING_MODEL,
        }
    }
}

impl RuntimeSettings {
    /// True when the two settings would produce a different index, so the
    /// caller knows a rebuild is needed before the change means anything.
    ///
    /// Only the chunking pair qualifies: `bm25_k1` / `bm25_b` are applied while
    /// scoring, not while building, so they take effect on the very next query.
    pub fn needs_reindex_against(&self, other: &Self) -> bool {
        self.max_chunk_chars != other.max_chunk_chars
            || self.min_chunk_chars != other.min_chunk_chars
            || self.embedding_model != other.embedding_model
    }
}
