//! Every knob that affects retrieval quality, in one place.
//!
//! Measured on 2026-07-25 against 568 chunks from 29 documents, six probe
//! questions asked in Russian. Numbers here are the observations that should
//! inform the next change - not a claim that the current values are right.

use fastembed::EmbeddingModel;

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

/// Multilingual on purpose: the guides are in English, the questions often are
/// not. Costs ~450MB resident - a monolingual model is ~130MB but misses every
/// Russian query.
pub const EMBEDDING_MODEL: EmbeddingModel = EmbeddingModel::MultilingualE5Small;

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
