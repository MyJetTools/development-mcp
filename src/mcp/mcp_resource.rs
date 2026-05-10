use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct McpResource;

impl McpResource {
    pub const FILENAME: &'static str = "mcp-development-guide.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/mcp-development-guide.md";
    pub const TOOL_FN: &'static str = "get_mcp_development_guide";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch MCP development guide resource content";
}

impl ResourceDefinition for McpResource {
    const RESOURCE_URI: &'static str = "resource://mcp-development-guide";
    const RESOURCE_NAME: &'static str = "MCP Development Guide";
    const DESCRIPTION: &'static str = "Guide for creating Prompts and Tool Calls";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for McpResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
