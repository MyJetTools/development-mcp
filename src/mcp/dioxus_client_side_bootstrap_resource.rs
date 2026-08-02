use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct DioxusClientSideBootstrapResource;

impl DioxusClientSideBootstrapResource {
    pub const FILENAME: &'static str = "dioxus-client-side-bootstrap.md";
    pub const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/bootstrap-dioxus-client-side-project.md";
    pub const TOOL_FN: &'static str = "get_dioxus_client_side_bootstrap_guide";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch Dioxus client-side (WASM-only) bootstrap guide: project skeleton with WebSocket and \
         API calls, plus the CI workflow for a Dioxus WASM app — dx build inside the \
         myjettools/dioxus-docker container, cache-busting build.py, static-hosting Dockerfile. \
         Load this before writing CI for any Dioxus client; the native-service builder-image \
         pattern does not apply to it.";
}

impl ResourceDefinition for DioxusClientSideBootstrapResource {
    const RESOURCE_URI: &'static str = "resource://dioxus-client-side-bootstrap";
    const RESOURCE_NAME: &'static str = "Dioxus Client-Side Bootstrap Guide";
    const DESCRIPTION: &'static str =
        "Bootstrap a new Dioxus client-side (WASM-only) web application with WebSocket and API calls";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for DioxusClientSideBootstrapResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
