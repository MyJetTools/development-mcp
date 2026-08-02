use rust_extensions::date_time::DateTimeAsMicroseconds;

use crate::rag::{Bm25Index, RuntimeSettings};

/// Which half of the index answers a query. Selectable per request so the three
/// can be compared against the same corpus without a redeploy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    /// Embeddings only. Good at paraphrase, bad at exact identifiers.
    Dense,
    /// BM25 only. Good at exact identifiers, and says nothing when the corpus
    /// genuinely does not contain the words.
    Bm25,
    /// Both, fused with reciprocal rank fusion.
    Hybrid,
}

impl SearchMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "dense" | "vector" | "embedding" => Some(Self::Dense),
            "bm25" | "lexical" | "keyword" => Some(Self::Bm25),
            "hybrid" | "both" | "rrf" => Some(Self::Hybrid),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dense => "dense",
            Self::Bm25 => "bm25",
            Self::Hybrid => "hybrid",
        }
    }
}

/// A single indexed piece of a document, together with everything needed to
/// tell the caller where it came from.
pub struct IndexedChunk {
    pub filename: String,
    pub doc_name: String,
    pub heading_path: String,
    pub text: String,
}

pub struct SearchHit {
    pub filename: String,
    pub doc_name: String,
    pub heading_path: String,
    pub text: String,
    pub score: f32,
}

/// The whole search index. Built in the background, published through
/// `ArcSwapOption` and never mutated in place - readers just clone the `Arc`.
pub struct DocIndex {
    chunks: Vec<IndexedChunk>,
    /// Parallel to `chunks`. Normalized, so a dot product is the cosine.
    vectors: Vec<Vec<f32>>,
    bm25: Bm25Index,
    pub built_at: DateTimeAsMicroseconds,
    pub documents_indexed: usize,
}

impl DocIndex {
    pub fn new(
        chunks: Vec<IndexedChunk>,
        vectors: Vec<Vec<f32>>,
        bm25: Bm25Index,
        documents_indexed: usize,
    ) -> Self {
        Self {
            chunks,
            vectors,
            bm25,
            built_at: DateTimeAsMicroseconds::now(),
            documents_indexed,
            }
    }

    pub fn chunks_amount(&self) -> usize {
        self.chunks.len()
    }

    pub fn search(
        &self,
        mode: SearchMode,
        query_vector: &[f32],
        query_text: &str,
        top_k: usize,
        settings: &RuntimeSettings,
    ) -> Vec<SearchHit> {
        if top_k == 0 {
            return Vec::new();
        }

        let scored = match mode {
            SearchMode::Dense => self.search_dense(query_vector, top_k, settings),
            SearchMode::Bm25 => self.search_bm25(query_text, top_k, settings),
            SearchMode::Hybrid => self.search_hybrid(query_vector, query_text, top_k, settings),
        };

        scored
            .into_iter()
            .map(|(index, score)| {
                let chunk = &self.chunks[index];

                SearchHit {
                    filename: chunk.filename.clone(),
                    doc_name: chunk.doc_name.clone(),
                    heading_path: chunk.heading_path.clone(),
                    text: chunk.text.clone(),
                    score,
                }
            })
            .collect()
    }

    fn dense_ranking(&self, query_vector: &[f32]) -> Vec<(usize, f32)> {
        let query_vector = normalize(query_vector);

        let mut scored: Vec<(usize, f32)> = self
            .vectors
            .iter()
            .enumerate()
            .map(|(index, vector)| (index, dot(&query_vector, vector)))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored
    }

    fn search_dense(
        &self,
        query_vector: &[f32],
        top_k: usize,
        settings: &RuntimeSettings,
    ) -> Vec<(usize, f32)> {
        let mut scored = self.dense_ranking(query_vector);
        scored.retain(|(_, score)| *score >= settings.min_score);
        scored.truncate(top_k);
        scored
    }

    fn search_bm25(
        &self,
        query_text: &str,
        top_k: usize,
        settings: &RuntimeSettings,
    ) -> Vec<(usize, f32)> {
        let mut scored = self.bm25.search(query_text, settings.bm25_k1, settings.bm25_b);
        scored.retain(|(_, score)| *score >= settings.min_bm25_score);
        scored.truncate(top_k);
        scored
    }

    /// Reciprocal rank fusion: each chunk scores `1 / (RRF_K + rank)` in each
    /// ranking it appears in, and the two are summed.
    ///
    /// Fusing ranks rather than scores is the point - a cosine of 0.84 and a
    /// BM25 score of 11.3 are not comparable numbers, but "third place" and
    /// "third place" are.
    fn search_hybrid(
        &self,
        query_vector: &[f32],
        query_text: &str,
        top_k: usize,
        settings: &RuntimeSettings,
    ) -> Vec<(usize, f32)> {
        let mut fused: ahash::AHashMap<usize, f32> = ahash::AHashMap::new();

        for (rank, (index, _)) in self
            .dense_ranking(query_vector)
            .into_iter()
            .take(settings.hybrid_candidates)
            .enumerate()
        {
            *fused.entry(index).or_insert(0.0) += 1.0 / (settings.rrf_k + rank as f32 + 1.0);
        }

        for (rank, (index, _)) in self
            .bm25
            .search(query_text, settings.bm25_k1, settings.bm25_b)
            .into_iter()
            .take(settings.hybrid_candidates)
            .enumerate()
        {
            *fused.entry(index).or_insert(0.0) += 1.0 / (settings.rrf_k + rank as f32 + 1.0);
        }

        let mut scored: Vec<(usize, f32)> = fused.into_iter().collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        scored
    }
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() {
        return 0.0;
    }

    let mut result = 0.0;

    for i in 0..left.len() {
        result += left[i] * right[i];
    }

    result
}

/// fastembed already returns normalized vectors, but a query vector coming from
/// a different code path should not be trusted to be - normalizing twice is
/// cheap and makes the dot product an honest cosine either way.
fn normalize(vector: &[f32]) -> Vec<f32> {
    let mut length = 0.0;

    for value in vector {
        length += value * value;
    }

    let length = length.sqrt();

    if length == 0.0 {
        return vector.to_vec();
    }

    vector.iter().map(|it| it / length).collect()
}
