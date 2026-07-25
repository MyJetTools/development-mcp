//! Every knob that affects retrieval quality, in one place.
//!
//! Measured on 2026-07-25 against 568 chunks from 29 documents, six probe
//! questions asked in Russian. Numbers here are the observations that should
//! inform the next change - not a claim that the current values are right.


use crate::rag::{EmbeddingModelChoice, RerankerModelChoice, SearchMode};

// ---------------------------------------------------------------------------
// Chunking
// ---------------------------------------------------------------------------

/// Sections longer than this are split further, by paragraph.
///
/// Suspected too large: the ArcSwap section of performance-considerations.md
/// never surfaces for a query that literally describes it, most likely because
/// one embedding has to carry the whole of a long section.
pub const MAX_CHUNK_CHARS: usize = 2400;

/// Chunks shorter than this are merged into the following one.
pub const MIN_CHUNK_CHARS: usize = 220;

// ---------------------------------------------------------------------------
// Embedding
// ---------------------------------------------------------------------------

/// Which embedding model the index is built with. Multilingual is the safe
/// default because it survives a query in any language; the English-only models
/// are a third of the size and sharper on English, but only if every query
/// really does arrive in English - which is what REQUIRE_ENGLISH_QUERY enforces.
pub const EMBEDDING_MODEL: EmbeddingModelChoice = EmbeddingModelChoice::MultilingualE5Small;

/// Refuse a query containing Cyrillic instead of answering it badly.
///
/// The instruction in the tool description is an agreement, not a guarantee.
/// With an English-only model a Russian query does not fail loudly - it returns
/// confident nonsense. This turns that into an error that says what to do.
pub const REQUIRE_ENGLISH_QUERY: bool = false;

/// E5-family models are trained with these prefixes and lose noticeable
/// accuracy without them.
pub const PASSAGE_PREFIX: &str = "passage: ";
pub const QUERY_PREFIX: &str = "query: ";

/// Kept small on purpose. The fastembed default (256) builds huge intermediate
/// tensors and spikes RSS by hundreds of MB during a rebuild.
pub const EMBED_BATCH_SIZE: usize = 32;

/// Where model weights are cached. Must point at a mounted volume in Docker,
/// otherwise the download repeats on every container restart.
pub const MODEL_CACHE_DIR_ENV: &str = "FASTEMBED_CACHE_PATH";
pub const DEFAULT_MODEL_CACHE_DIR: &str = "/app/model-cache";

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// Cosine below which a chunk is treated as noise.
///
/// E5 compresses cosines into a narrow high band, so this is far less
/// forgiving to tune than it looks. Measured top-hit scores:
///
/// | query                        | top hit |
/// |------------------------------|---------|
/// | Cargo.toml dependency        | 0.861   |
/// | websocket reconnect timeouts | 0.856   |
/// | PartitionKey / RowKey        | 0.846   |
/// | which HTTP client            | 0.842   |
/// | read-mostly state            | 0.836   |
/// | Kafka consumer group (absent)| 0.811   |
///
/// Nothing ever scores below ~0.79, so 0.72 never rejects anything: a question
/// the corpus does not cover still comes back with five confident-looking
/// fragments. Somewhere around 0.82-0.83 separates the one absent topic from
/// the five present ones - but that is six samples, so treat it as a starting
/// point and move it against real logs.
pub const MIN_SCORE: f32 = 0.72;

pub const DEFAULT_TOP_K: i32 = 6;
pub const MAX_TOP_K: i32 = 15;

/// Which ranking answers a query when the caller does not say.
///
/// Left on dense so that changing this file does not silently change what the
/// baseline measurement was taken against - pass `mode` explicitly to compare.
pub const DEFAULT_SEARCH_MODE: SearchMode = SearchMode::Dense;

/// Standard BM25 constants. `k1` controls how fast term frequency saturates,
/// `b` how hard long chunks are penalised.
pub const BM25_K1: f32 = 1.2;
pub const BM25_B: f32 = 0.75;

/// BM25 scores are unbounded and corpus-dependent, so this is not comparable to
/// MIN_SCORE. It only needs to be above zero: a chunk sharing no term with the
/// query is never scored at all, which is what makes "not in the guides" come
/// out right without any tuning.
pub const MIN_BM25_SCORE: f32 = 0.0;

/// How deep into each ranking reciprocal rank fusion looks.
pub const HYBRID_CANDIDATES: usize = 30;

/// The damping constant in `1 / (k + rank)`. 60 is the value from the original
/// RRF paper and is what most implementations use.
pub const RRF_K: f32 = 60.0;

// ---------------------------------------------------------------------------
// Index refresh
// ---------------------------------------------------------------------------

/// How often the documents are re-fetched and hashed. A poll that finds no
/// change costs 29 HTTP requests and no CPU, so this can be short.
pub const POLL_INTERVAL_SECS: u64 = 5 * 60;

/// The poll itself only fetches and hashes - generous, but it is 29 sequential
/// HTTP requests, so not as generous as it looks.
pub const POLL_ITERATION_TIMEOUT_SECS: u64 = 180;

/// A full rebuild embeds every chunk of every document: minutes of CPU, not
/// seconds. The events loop must not treat that as a stuck iteration.
pub const REBUILD_ITERATION_TIMEOUT_SECS: u64 = 20 * 60;

// ---------------------------------------------------------------------------
// Reranking
// ---------------------------------------------------------------------------

/// Off by default: the weights are ~1.1GB on top of the embedding model and
/// they load lazily, so nothing is paid until reranking is switched on.
pub const RERANK_ENABLED: bool = false;

pub const RERANK_MODEL: RerankerModelChoice = RerankerModelChoice::JinaV2Multilingual;

/// How many candidates the first stage hands to the cross-encoder. Reranking
/// cost is linear in this, so it trades latency for the chance to recover a
/// document the first stage ranked badly.
pub const RERANK_CANDIDATES: usize = 30;

/// Cross-encoder scores are calibrated, unlike the dense cosines - a genuinely
/// irrelevant chunk lands near or below zero. That is what makes an honest
/// "not covered" possible at all, which no MIN_SCORE could achieve.
pub const MIN_RERANK_SCORE: f32 = 0.0;

pub const RERANK_BATCH_SIZE: usize = 8;
