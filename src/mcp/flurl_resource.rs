use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct FlUrlResource;

impl FlUrlResource {
    pub const FILENAME: &'static str = "flurl-usage-guide.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/fl-url/refs/heads/main/README.md";
    pub const TOOL_FN: &'static str = "get_flurl_usage_guide";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch the FlUrl usage guide. MANDATORY prerequisite before writing any HTTP request code in MyJetTools projects — FlUrl is the only allowed HTTP client (no reqwest).";
}

impl ResourceDefinition for FlUrlResource {
    const RESOURCE_URI: &'static str = "resource://flurl-usage-guide";
    const RESOURCE_NAME: &'static str = "FlUrl Usage Guide";
    const DESCRIPTION: &'static str = "MANDATORY HTTP client for MyJetTools projects. Load this BEFORE writing any HTTP/REST code. reqwest is NOT allowed — use FlUrl instead.";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for FlUrlResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
