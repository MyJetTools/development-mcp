use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct AppBootstrapResource;

impl ResourceDefinition for AppBootstrapResource {
    const RESOURCE_URI: &'static str = "resource://app-bootstrap";
    const RESOURCE_NAME: &'static str = "App Bootstrap Guide";
    const DESCRIPTION: &'static str = "Step-by-step instructions for bootstrapping a new project";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for AppBootstrapResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        const BOOTSTRAP_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/service-sdk/refs/heads/main/APP_BOOTSTRAP.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, BOOTSTRAP_URL).await
    }
}
