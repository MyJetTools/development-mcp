use std::sync::Arc;

use mcp_server_middleware::*;
use serde::{Deserialize, Serialize};

use crate::app::AppContext;
use crate::rag::SearchMode;

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct SearchDocsInputData {
    #[property(
        description = "The question, in your own words. Natural language, not keywords. Russian and English both work."
    )]
    pub query: String,

    #[property(description = "How many chunks to return. Defaults to 6, maximum 15.")]
    pub top_k: Option<i32>,

    #[property(
        description = "Ranking to use: 'dense' (embeddings), 'bm25' (lexical), or 'hybrid' (both, fused). Omit for the server default."
    )]
    pub mode: Option<String>,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct SearchDocsHit {
    #[property(description = "Source document filename - pass it to get_doc to read the whole document")]
    pub filename: String,

    #[property(description = "Human readable document name")]
    pub doc_name: String,

    #[property(description = "Heading breadcrumb this fragment sits under inside the document")]
    pub heading_path: String,

    #[property(description = "The relevant fragment itself")]
    pub text: String,

    #[property(description = "Cosine similarity against the query, between 0 and 1")]
    pub score: f32,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct SearchDocsResponse {
    #[property(description = "Relevant fragments, best first. Empty when nothing matched.")]
    pub hits: Vec<SearchDocsHit>,

    #[property(
        description = "Status of the search. Read it before answering - it tells you whether an empty result means 'not in the guides' or 'index not ready'."
    )]
    pub status: String,
}

pub struct SearchDocsHandler {
    app: Arc<AppContext>,
}

impl SearchDocsHandler {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

impl ToolDefinition for SearchDocsHandler {
    const FUNC_NAME: &'static str = "search_docs";

    const DESCRIPTION: &'static str = "Semantic search across the MyJetTools development guides. \
         Ask a real question in your own words - the index matches by meaning, not by filename.\n\n\
         USE THIS FIRST, before writing any code that touches a MyJetTools library or convention. \
         Training-data muscle memory is wrong for these crates.\n\n\
         Covers: MCP server development; HTTP via FlUrl (never reqwest) and my-http-utils; \
         HTTP actions design; my-grpc-extensions; my-postgres; MyNoSql entity and partitioning \
         patterns; my-json; my-tcp-sockets; my-ssh; WebSocket clients (native and WASM); \
         Dioxus design patterns, fullstack patterns, client-side and fullstack bootstrap, \
         admin UI kit, dioxus-utils; application architecture best practices and the architect \
         playbook; app bootstrap; cargo dependency conventions; rust-extensions; rust-fix; \
         ci-utils; my-ai-agent; release and deployment flows; single-VM unix-socket setup; \
         performance considerations (ArcSwap, parking_lot, AHash).\n\n\
         Returns fragments with their source document and heading. If a fragment looks \
         truncated or you need the surrounding rules, follow up with get_doc using the \
         returned filename.\n\n\
         IMPORTANT: if this returns no hits, say plainly that the guides do not cover it. \
         Do not answer from general knowledge as if it were a MyJetTools convention.";
}

#[async_trait::async_trait]
impl McpToolCall<SearchDocsInputData, SearchDocsResponse> for SearchDocsHandler {
    async fn execute_tool_call(
        &self,
        model: SearchDocsInputData,
    ) -> Result<SearchDocsResponse, String> {
        let query = model.query.trim();

        if query.is_empty() {
            return Err("query must not be empty".to_string());
        }

        let Some(index) = self.app.get_index() else {
            return Ok(SearchDocsResponse {
                hits: Vec::new(),
                status: "Index is still being built. Use list_resource_tools and get_doc \
                         to read documents directly in the meantime."
                    .to_string(),
            });
        };

        let settings = self.app.get_settings();

        let top_k = model
            .top_k
            .unwrap_or(settings.default_top_k)
            .clamp(1, settings.max_top_k) as usize;

        let mode = match model.mode.as_deref() {
            None => settings.search_mode,
            Some(value) => SearchMode::parse(value)
                .ok_or_else(|| format!("Unknown mode '{}'. Use dense, bm25 or hybrid.", value))?,
        };

        // The embedding is only needed by the rankings that use it.
        let query_vector = match mode {
            SearchMode::Bm25 => Vec::new(),
            _ => self.app.embedder.embed_query(query).await?,
        };

        let hits = index.search(mode, &query_vector, query, top_k, &settings);

        let status = if hits.is_empty() {
            format!(
                "No fragment in the guides matched this query in {} mode (searched {} chunks \
                 from {} documents). Treat this as 'the guides do not cover it'.",
                mode.as_str(),
                index.chunks_amount(),
                index.documents_indexed
            )
        } else {
            format!(
                "{} fragment(s) found in {} mode across {} indexed chunks (index built at {}).",
                hits.len(),
                mode.as_str(),
                index.chunks_amount(),
                index.built_at.to_rfc3339()
            )
        };

        let hits = hits
            .into_iter()
            .map(|it| SearchDocsHit {
                filename: it.filename,
                doc_name: it.doc_name,
                heading_path: it.heading_path,
                text: it.text,
                score: it.score,
            })
            .collect();

        Ok(SearchDocsResponse { hits, status })
    }
}
