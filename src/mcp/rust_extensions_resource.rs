use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct RustExtensionsResource;

impl RustExtensionsResource {
    pub const FILENAME: &'static str = "rust-extensions.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/rust-extensions/main/README.md";
    pub const TOOL_FN: &'static str = "get_rust_extensions_readme";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch rust-extensions README resource content";
}

impl ResourceDefinition for RustExtensionsResource {
    const RESOURCE_URI: &'static str = "resource://rust-extensions";
    const RESOURCE_NAME: &'static str = "rust-extensions for each project";
    const DESCRIPTION: &'static str =
        "Low-level utils, queues and other helpers to glue together Rust code";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for RustExtensionsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
