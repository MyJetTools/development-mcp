use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct RustFixResource;

impl RustFixResource {
    pub const FILENAME: &'static str = "rust-fix-readme.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/rust-fix/refs/heads/main/readme.md";
    pub const TOOL_FN: &'static str = "get_rust_fix_readme";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch rust-fix README resource content";
}

impl ResourceDefinition for RustFixResource {
    const RESOURCE_URI: &'static str = "resource://rust-fix-readme";
    const RESOURCE_NAME: &'static str = "rust-fix FIX Protocol Library";
    const DESCRIPTION: &'static str = "Zero-dependency FIX protocol library for low-latency trading: message writer, reader, and builder";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for RustFixResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
