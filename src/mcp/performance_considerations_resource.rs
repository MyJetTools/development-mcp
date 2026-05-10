use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct PerformanceConsiderationsResource;

impl PerformanceConsiderationsResource {
    pub const FILENAME: &'static str = "performance-considerations.md";
    pub const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/performance-considerations.md";
    pub const TOOL_FN: &'static str = "get_performance_considerations";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch performance considerations: ArcSwap for read-mostly state, parking_lot vs tokio locks, AHash, no heavy work under locks, Arc-based snapshots, bounded async parallelism";
}

impl ResourceDefinition for PerformanceConsiderationsResource {
    const RESOURCE_URI: &'static str = "resource://performance-considerations";
    const RESOURCE_NAME: &'static str = "Performance Considerations";
    const DESCRIPTION: &'static str =
        "Default performance principles: ArcSwap for read-mostly state, parking_lot vs tokio locks, AHash, no heavy work under locks, Arc-based snapshots, bounded async parallelism";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for PerformanceConsiderationsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
