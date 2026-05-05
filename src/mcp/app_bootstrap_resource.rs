use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct AppBootstrapResource;

impl AppBootstrapResource {
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/app-bootstrap.md";
    pub const TOOL_FN: &'static str = "get_app_bootstrap_guide";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch app bootstrap guide resource content";
}

impl ResourceDefinition for AppBootstrapResource {
    const RESOURCE_URI: &'static str = "resource://app-bootstrap";
    const RESOURCE_NAME: &'static str = "App Bootstrap Guide";
    const DESCRIPTION: &'static str = "Step-by-step instructions for bootstrapping a new project";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for AppBootstrapResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
