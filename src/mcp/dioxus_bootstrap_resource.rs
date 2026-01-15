use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct DioxusBootstrapResource;

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
        const DIOXUS_BOOTSTRAP_URL: &str =
            "https://raw.githubusercontent.com/amigin/ai-templates/refs/heads/main/cursor/bootstrap-empty-dioxus-fullstack-project.mdc";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, DIOXUS_BOOTSTRAP_URL).await
    }
}
