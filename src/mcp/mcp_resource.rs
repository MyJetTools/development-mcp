use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct McpResource;

impl ResourceDefinition for McpResource {
    const RESOURCE_URI: &'static str = "resource://mcp-development-guide";
    const RESOURCE_NAME: &'static str = "MCP Development Guide";
    const DESCRIPTION: &'static str = "Guide for creating Prompts and Tool Calls";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for McpResource {
    async fn read_resource(&self, uri: &str) -> Result<ResourceReadResult, String> {
        if uri != Self::RESOURCE_URI {
            return Err(format!("Unknown resource URI: {}", uri));
        }

        const GUIDE_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/mcp-development-guide.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, GUIDE_URL).await
    }
}
