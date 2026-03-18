use std::sync::Arc;

use my_http_server::macros::*;
use my_http_server::*;

use crate::app::AppContext;

fn get_doc_url(filename: &str) -> Option<&'static str> {
    match filename {
        "mcp-development-guide.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/mcp-development-guide.md",
        ),
        "flurl-usage-guide.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/fl-url/refs/heads/main/README.md",
        ),
        "http-actions-design-guide.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/my-http-server/refs/heads/main/HTTP_ACTIONS_DESIGN.md",
        ),
        "app-bootstrap.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/app-bootstrap.md",
        ),
        "dioxus-bootstrap.md" => Some(
            "https://raw.githubusercontent.com/amigin/ai-templates/refs/heads/main/cursor/bootstrap-empty-dioxus-fullstack-project.mdc",
        ),
        "dioxus-client-side-bootstrap.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/bootstrap-dioxus-client-side-project.md",
        ),
        "cargo-dependencies-guide.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/cargo-dependencies-guide.md",
        ),
        "my-ssh-readme.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/my-ssh/refs/heads/main/README.md",
        ),
        "my-tcp-sockets-readme.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/my-tcp-sockets/refs/heads/main/README.md",
        ),
        "rust-extensions.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/rust-extensions/refs/heads/main/README.md",
        ),
        "dioxus-fullstack-design-patterns.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/DIOXUS_FULLSTACK_DESIGN_PATTERS.md",
        ),
        "my-no-sql-entity-design-patterns.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/my-no-sql-sdk/refs/heads/main/MY_NO_SQL_ENTITY_DESIGN_PATTERNS.md",
        ),
        "my-grpc-extensions.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/my-grpc-extensions/refs/heads/main/README.md",
        ),
        "dioxus-utils-readme.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/dioxus-utils/refs/heads/main/README.md",
        ),
        "ci-utils-readme.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/ci-utils/refs/heads/main/README.md",
        ),
        "my-postgres-readme.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/my-postgres/refs/heads/main/README.md",
        ),
        "dioxus-admin-ui-kit.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/dioxus-admin-ui-kit/refs/heads/main/README.md",
        ),
        "application-architecture-best-practices.md" => Some(
            "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/application-architecture-best-practices.md",
        ),
        _ => None,
    }
}

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
    let url = match get_doc_url(&input_data.filename) {
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
