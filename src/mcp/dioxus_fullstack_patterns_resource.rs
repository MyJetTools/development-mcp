use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct DioxusFullstackPatternsResource;

impl DioxusFullstackPatternsResource {
    pub const FILENAME: &'static str = "dioxus-fullstack-design-patterns.md";
    pub const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/DIOXUS_FULLSTACK_DESIGN_PATTERS.md";
    pub const TOOL_FN: &'static str = "get_dioxus_fullstack_design_patterns";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch Dioxus fullstack design patterns resource content";
}

impl ResourceDefinition for DioxusFullstackPatternsResource {
    const RESOURCE_URI: &'static str = "resource://dioxus-fullstack-design-patterns";
    const RESOURCE_NAME: &'static str = "Dioxus Fullstack Design Patterns";
    const DESCRIPTION: &'static str =
        "Project playbook for dialogs, forms, lists, and server functions";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for DioxusFullstackPatternsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
