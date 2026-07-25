use std::sync::Arc;

use mcp_server_middleware::*;
use serde::{Deserialize, Serialize};

use crate::app::AppContext;
use crate::rag::{poll_and_maybe_rebuild, RuntimeSettings, SearchMode};

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct EmptySettingsInput {}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct SearchSettingsView {
    #[property(description = "Ranking used when a search does not name one: dense, bm25 or hybrid")]
    pub search_mode: String,

    #[property(description = "Cosine below which a dense hit is discarded")]
    pub min_score: f32,

    #[property(description = "BM25 score below which a lexical hit is discarded")]
    pub min_bm25_score: f32,

    #[property(description = "Hits returned when the caller does not say")]
    pub default_top_k: i32,

    #[property(description = "Upper bound on requested hits")]
    pub max_top_k: i32,

    #[property(description = "Damping constant k in the reciprocal rank fusion term 1/(k+rank)")]
    pub rrf_k: f32,

    #[property(description = "How deep into each ranking hybrid fusion looks")]
    pub hybrid_candidates: i32,

    #[property(description = "BM25 term-frequency saturation constant")]
    pub bm25_k1: f32,

    #[property(description = "BM25 length-normalisation constant")]
    pub bm25_b: f32,

    #[property(description = "Chunking: sections longer than this are split further. Needs a rebuild to take effect")]
    pub max_chunk_chars: i32,

    #[property(description = "Chunking: chunks shorter than this are merged forward. Needs a rebuild to take effect")]
    pub min_chunk_chars: i32,
}

impl SearchSettingsView {
    fn from(settings: &RuntimeSettings) -> Self {
        Self {
            search_mode: settings.search_mode.as_str().to_string(),
            min_score: settings.min_score,
            min_bm25_score: settings.min_bm25_score,
            default_top_k: settings.default_top_k,
            max_top_k: settings.max_top_k,
            rrf_k: settings.rrf_k,
            hybrid_candidates: settings.hybrid_candidates as i32,
            bm25_k1: settings.bm25_k1,
            bm25_b: settings.bm25_b,
            max_chunk_chars: settings.max_chunk_chars as i32,
            min_chunk_chars: settings.min_chunk_chars as i32,
        }
    }
}

// ---------------------------------------------------------------------------
// get_search_settings
// ---------------------------------------------------------------------------

pub struct GetSearchSettingsHandler {
    app: Arc<AppContext>,
}

impl GetSearchSettingsHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for GetSearchSettingsHandler {
    const FUNC_NAME: &'static str = "get_search_settings";
    const DESCRIPTION: &'static str =
        "Read the current retrieval tuning of the docs index: ranking mode, score thresholds, \
         top_k defaults, BM25 and fusion constants, and chunk sizes. Use it before changing \
         anything so you know what you are changing from.";
}

#[async_trait::async_trait]
impl McpToolCall<EmptySettingsInput, SearchSettingsView> for GetSearchSettingsHandler {
    async fn execute_tool_call(
        &self,
        _model: EmptySettingsInput,
    ) -> Result<SearchSettingsView, String> {
        Ok(SearchSettingsView::from(&self.app.get_settings()))
    }
}

// ---------------------------------------------------------------------------
// update_search_settings
// ---------------------------------------------------------------------------

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct UpdateSearchSettingsInput {
    #[property(description = "Default ranking: dense, bm25 or hybrid")]
    pub search_mode: Option<String>,

    #[property(description = "Cosine below which a dense hit is discarded. Nothing in this corpus scores below ~0.79, so values under that reject nothing")]
    pub min_score: Option<f32>,

    #[property(description = "BM25 score below which a lexical hit is discarded")]
    pub min_bm25_score: Option<f32>,

    #[property(description = "Hits returned when the caller does not say")]
    pub default_top_k: Option<i32>,

    #[property(description = "Upper bound on requested hits")]
    pub max_top_k: Option<i32>,

    #[property(description = "Damping constant k in 1/(k+rank). 60 is the usual value")]
    pub rrf_k: Option<f32>,

    #[property(description = "How deep into each ranking hybrid fusion looks")]
    pub hybrid_candidates: Option<i32>,

    #[property(description = "BM25 term-frequency saturation constant, typically 1.2 to 2.0")]
    pub bm25_k1: Option<f32>,

    #[property(description = "BM25 length normalisation, 0 to 1. 0.75 is the usual value")]
    pub bm25_b: Option<f32>,

    #[property(description = "Sections longer than this are split further. Requires a rebuild")]
    pub max_chunk_chars: Option<i32>,

    #[property(description = "Chunks shorter than this are merged forward. Requires a rebuild")]
    pub min_chunk_chars: Option<i32>,

    #[property(description = "Reset every setting to its compiled-in default before applying the rest")]
    pub reset_to_defaults: Option<bool>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct UpdateSearchSettingsResponse {
    #[property(description = "The settings now in force")]
    pub settings: SearchSettingsView,

    #[property(description = "True when the change only takes effect after the index is rebuilt")]
    pub rebuild_required: bool,

    #[property(description = "What happened, and what to do next")]
    pub status: String,
}

pub struct UpdateSearchSettingsHandler {
    app: Arc<AppContext>,
}

impl UpdateSearchSettingsHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for UpdateSearchSettingsHandler {
    const FUNC_NAME: &'static str = "update_search_settings";
    const DESCRIPTION: &'static str =
        "Change the retrieval tuning of the docs index at runtime - no redeploy. Every field is \
         optional; omitted fields keep their current value. Search-side settings (mode, \
         thresholds, top_k, BM25 and fusion constants) apply to the very next query. Chunk sizes \
         only change anything after the index is rebuilt, which the response will tell you.";
}

#[async_trait::async_trait]
impl McpToolCall<UpdateSearchSettingsInput, UpdateSearchSettingsResponse>
    for UpdateSearchSettingsHandler
{
    async fn execute_tool_call(
        &self,
        model: UpdateSearchSettingsInput,
    ) -> Result<UpdateSearchSettingsResponse, String> {
        let previous = self.app.get_settings();

        let mut next = if model.reset_to_defaults.unwrap_or(false) {
            RuntimeSettings::default()
        } else {
            previous.as_ref().clone()
        };

        if let Some(value) = model.search_mode.as_deref() {
            next.search_mode = SearchMode::parse(value)
                .ok_or_else(|| format!("Unknown mode '{}'. Use dense, bm25 or hybrid.", value))?;
        }

        if let Some(value) = model.min_score {
            next.min_score = value;
        }

        if let Some(value) = model.min_bm25_score {
            next.min_bm25_score = value;
        }

        if let Some(value) = model.default_top_k {
            if value < 1 {
                return Err("default_top_k must be at least 1".to_string());
            }
            next.default_top_k = value;
        }

        if let Some(value) = model.max_top_k {
            if value < 1 {
                return Err("max_top_k must be at least 1".to_string());
            }
            next.max_top_k = value;
        }

        if let Some(value) = model.rrf_k {
            if value < 0.0 {
                return Err("rrf_k must not be negative".to_string());
            }
            next.rrf_k = value;
        }

        if let Some(value) = model.hybrid_candidates {
            if value < 1 {
                return Err("hybrid_candidates must be at least 1".to_string());
            }
            next.hybrid_candidates = value as usize;
        }

        if let Some(value) = model.bm25_k1 {
            if value <= 0.0 {
                return Err("bm25_k1 must be positive".to_string());
            }
            next.bm25_k1 = value;
        }

        if let Some(value) = model.bm25_b {
            if !(0.0..=1.0).contains(&value) {
                return Err("bm25_b must be between 0 and 1".to_string());
            }
            next.bm25_b = value;
        }

        if let Some(value) = model.max_chunk_chars {
            if value < 200 {
                return Err("max_chunk_chars below 200 would shred the documents".to_string());
            }
            next.max_chunk_chars = value as usize;
        }

        if let Some(value) = model.min_chunk_chars {
            if value < 0 {
                return Err("min_chunk_chars must not be negative".to_string());
            }
            next.min_chunk_chars = value as usize;
        }

        if next.min_chunk_chars >= next.max_chunk_chars {
            return Err("min_chunk_chars must be smaller than max_chunk_chars".to_string());
        }

        let rebuild_required = next.needs_reindex_against(&previous);

        let status = if rebuild_required {
            "Applied. Chunk sizes changed, so the current index still reflects the old ones - \
             call rebuild_index with force=true to reindex."
                .to_string()
        } else {
            "Applied to the next query. No rebuild needed.".to_string()
        };

        let view = SearchSettingsView::from(&next);

        self.app.set_settings(next);

        Ok(UpdateSearchSettingsResponse {
            settings: view,
            rebuild_required,
            status,
        })
    }
}

// ---------------------------------------------------------------------------
// rebuild_index
// ---------------------------------------------------------------------------

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct RebuildIndexInput {
    #[property(
        description = "Rebuild even when no document changed. Needed after changing chunk sizes, since the documents themselves did not move."
    )]
    pub force: Option<bool>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct RebuildIndexResponse {
    #[property(description = "True when a rebuild was actually started")]
    pub started: bool,

    #[property(description = "What happened")]
    pub status: String,
}

pub struct RebuildIndexHandler {
    app: Arc<AppContext>,
}

impl RebuildIndexHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for RebuildIndexHandler {
    const FUNC_NAME: &'static str = "rebuild_index";
    const DESCRIPTION: &'static str =
        "Re-fetch the guides and rebuild the search index now, instead of waiting for the next \
         poll. Without force it only rebuilds when a document actually changed. Returns as soon \
         as the rebuild is handed off - it runs in the background, and search_docs reports the \
         index build time in its status so you can tell when the new one is live.";
}

#[async_trait::async_trait]
impl McpToolCall<RebuildIndexInput, RebuildIndexResponse> for RebuildIndexHandler {
    async fn execute_tool_call(
        &self,
        model: RebuildIndexInput,
    ) -> Result<RebuildIndexResponse, String> {
        let outcome = poll_and_maybe_rebuild(&self.app, model.force.unwrap_or(false)).await;

        Ok(RebuildIndexResponse {
            started: matches!(outcome, crate::rag::PollOutcome::RebuildStarted { .. }),
            status: outcome.describe(),
        })
    }
}
