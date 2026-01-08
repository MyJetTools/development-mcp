use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct CargoDependenciesResource;

impl ResourceDefinition for CargoDependenciesResource {
    const RESOURCE_URI: &'static str = "resource://cargo-dependencies-guide";
    const RESOURCE_NAME: &'static str = "Cargo Dependencies Guide";
    const DESCRIPTION: &'static str = "How to add dependencies to Cargo.toml";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for CargoDependenciesResource {
    async fn read_resource(&self, uri: &str) -> Result<ResourceReadResult, String> {
        if uri != Self::RESOURCE_URI {
            return Err(format!("Unknown resource URI: {}", uri));
        }

        const GUIDE_URL: &str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/cargo-dependencies-guide.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, GUIDE_URL).await
    }
}
