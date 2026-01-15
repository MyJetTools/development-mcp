use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct FlUrlResource;

impl ResourceDefinition for FlUrlResource {
    const RESOURCE_URI: &'static str = "resource://flurl-usage-guide";
    const RESOURCE_NAME: &'static str = "FlUrl Usage Guide";
    const DESCRIPTION: &'static str = "How to use FlUrl library";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for FlUrlResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/fl-url/refs/heads/main/README.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, README_URL).await
    }
}
