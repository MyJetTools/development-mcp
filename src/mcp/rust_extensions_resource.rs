use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct RustExtensionsResource;

impl ResourceDefinition for RustExtensionsResource {
    const RESOURCE_URI: &'static str = "resource://rust-extensions";
    const RESOURCE_NAME: &'static str = "rust-extensions for each project";
    const DESCRIPTION: &'static str =
        "Low-level utils, queues and other helpers to glue together Rust code";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for RustExtensionsResource {
    async fn read_resource(&self, uri: &str) -> Result<ResourceReadResult, String> {
        if uri != Self::RESOURCE_URI {
            return Err(format!("Unknown resource URI: {}", uri));
        }

        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/rust-extensions/main/README.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, README_URL).await
    }
}
