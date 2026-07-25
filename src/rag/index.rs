use rust_extensions::date_time::DateTimeAsMicroseconds;

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
    pub built_at: DateTimeAsMicroseconds,
    pub documents_indexed: usize,
}

impl DocIndex {
    pub fn new(
        chunks: Vec<IndexedChunk>,
        vectors: Vec<Vec<f32>>,
        documents_indexed: usize,
    ) -> Self {
        Self {
            chunks,
            vectors,
            built_at: DateTimeAsMicroseconds::now(),
            documents_indexed,
        }
    }

    pub fn chunks_amount(&self) -> usize {
        self.chunks.len()
    }

    pub fn search(&self, query_vector: &[f32], top_k: usize, min_score: f32) -> Vec<SearchHit> {
        if top_k == 0 {
            return Vec::new();
        }

        let query_vector = normalize(query_vector);

        let mut scored: Vec<(usize, f32)> = Vec::with_capacity(self.vectors.len());

        for (index, vector) in self.vectors.iter().enumerate() {
            let score = dot(&query_vector, vector);

            if score < min_score {
                continue;
            }

            scored.push((index, score));
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

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
