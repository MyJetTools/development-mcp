use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct HttpActionsResource;

impl HttpActionsResource {
    pub const FILENAME: &'static str = "http-actions-design-guide.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/my-http-server/refs/heads/main/HTTP_ACTIONS_DESIGN.md";
    pub const TOOL_FN: &'static str = "get_http_actions_design_guide";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch HTTP actions design guide resource content";
}

impl ResourceDefinition for HttpActionsResource {
    const RESOURCE_URI: &'static str = "resource://http-actions-design-guide";
    const RESOURCE_NAME: &'static str = "HTTP Actions Design Guide";
    const DESCRIPTION: &'static str = "Guide for HTTP action architecture and patterns";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for HttpActionsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
