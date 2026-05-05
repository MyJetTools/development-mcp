use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct DioxusBootstrapResource;

impl DioxusBootstrapResource {
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/amigin/ai-templates/refs/heads/main/cursor/bootstrap-empty-dioxus-fullstack-project.mdc";
    pub const TOOL_FN: &'static str = "get_dioxus_bootstrap_guide";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch Dioxus bootstrap guide resource content";
}

impl ResourceDefinition for DioxusBootstrapResource {
    const RESOURCE_URI: &'static str = "resource://dioxus-bootstrap";
    const RESOURCE_NAME: &'static str = "Dioxus Fullstack Bootstrap Guide";
    const DESCRIPTION: &'static str =
        "Step-by-step instructions for bootstrapping a new empty Dioxus fullstack web application";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for DioxusBootstrapResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
