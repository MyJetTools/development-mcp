use std::sync::Arc;

use my_http_server::macros::*;
use my_http_server::*;

use crate::app::AppContext;

use super::get_docs_index_action::build_yaml;

#[http_route(
    method: "GET",
    route: "/ai-docs.yaml",
    controller: "AiDocs",
    summary: "AI docs YAML index",
    description: "Returns a YAML file listing all available tools with usage guidance",
    result: [
        {status_code: 200, description: "YAML index"},
    ]
)]
pub struct GetAiDocsYamlAction {
    _app: Arc<AppContext>,
}

impl GetAiDocsYamlAction {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { _app: app }
    }
}

async fn handle_request(
    _action: &GetAiDocsYamlAction,
    ctx: &HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    let scheme = ctx.request.get_scheme();
    let host = ctx.request.get_host();
    HttpOutput::as_text(build_yaml(scheme, host))
        .set_content_type(my_http_server::WebContentType::Yaml)
        .add_header("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0")
        .into_ok_result(true)
        .into()
}
