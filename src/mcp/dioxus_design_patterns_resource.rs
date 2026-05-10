use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct DioxusDesignPatternsResource;

impl DioxusDesignPatternsResource {
    pub const FILENAME: &'static str = "dioxus-design-patterns.md";
    pub const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/dioxus-design-patterns.md";
    pub const TOOL_FN: &'static str = "get_dioxus_design_patterns";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch Dioxus design patterns resource content (framework-level conventions for fullstack and client-side projects: dialogs, state, components, signals)";
}

impl ResourceDefinition for DioxusDesignPatternsResource {
    const RESOURCE_URI: &'static str = "resource://dioxus-design-patterns";
    const RESOURCE_NAME: &'static str = "Dioxus Design Patterns";
    const DESCRIPTION: &'static str =
        "Framework-level Dioxus conventions: ComponentState, signals, dialogs (DialogState + RenderDialog + dialog_template), DataState, async data loading, CSS pipeline";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for DioxusDesignPatternsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
