use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct RustFixResource;

impl ResourceDefinition for RustFixResource {
    const RESOURCE_URI: &'static str = "resource://rust-fix-readme";
    const RESOURCE_NAME: &'static str = "rust-fix FIX Protocol Library";
    const DESCRIPTION: &'static str = "Zero-dependency FIX protocol library for low-latency trading: message writer, reader, and builder";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for RustFixResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/rust-fix/refs/heads/main/readme.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, README_URL).await
    }
}
