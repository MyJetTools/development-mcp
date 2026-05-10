use std::sync::Arc;

use my_http_server::macros::*;
use my_http_server::*;

use crate::app::AppContext;
use crate::mcp::get_doc_url_by_filename;

#[derive(MyHttpInput)]
pub struct GetAiDocInputModel {
    #[http_path(name = "filename", description = "Document filename, e.g. flurl-usage-guide.md")]
    pub filename: String,
}

#[http_route(
    method: "GET",
    route: "/ai-docs/{filename}",
    controller: "AiDocs",
    summary: "Get AI documentation",
    description: "Returns a markdown document by filename",
    input_data: GetAiDocInputModel,
    result: [
        {status_code: 200, description: "Markdown document content"},
        {status_code: 404, description: "Document not found"},
        {status_code: 500, description: "Failed to fetch document"},
    ]
)]
pub struct GetAiDocAction {
    _app: Arc<AppContext>,
}

impl GetAiDocAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { _app: app }
    }
}

async fn handle_request(
    _action: &GetAiDocAction,
    input_data: GetAiDocInputModel,
    _ctx: &HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let url = match get_doc_url_by_filename(&input_data.filename) {
        Some(url) => url,
        None => {
            return HttpFailResult::as_not_found(
                format!("Document '{}' not found", input_data.filename),
                false,
            )
            .into_err();
        }
    };

    let mut response = flurl::FlUrl::new(url)
        .get()
        .await
        .map_err(|e| {
            HttpFailResult::as_fatal_error(format!("Failed to fetch {}: {:?}", url, e))
        })?;

    let content = response
        .get_body_as_str()
        .await
        .map_err(|e| {
            HttpFailResult::as_fatal_error(format!("Failed to read body from {}: {:?}", url, e))
        })?;

    HttpOutput::as_text(content.to_string())
        .add_header("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0")
        .into_ok_result(true)
        .into()
}
