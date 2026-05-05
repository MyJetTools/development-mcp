use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct CargoDependenciesResource;

impl CargoDependenciesResource {
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/cargo-dependencies-guide.md";
    pub const TOOL_FN: &'static str = "get_cargo_dependencies_guide";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch Cargo dependencies guide resource content";
}

impl ResourceDefinition for CargoDependenciesResource {
    const RESOURCE_URI: &'static str = "resource://cargo-dependencies-guide";
    const RESOURCE_NAME: &'static str = "Cargo Dependencies Guide";
    const DESCRIPTION: &'static str = "How to add dependencies to Cargo.toml";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for CargoDependenciesResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
