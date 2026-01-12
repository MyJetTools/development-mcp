use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct DioxusFullstackPatternsResource;

impl ResourceDefinition for DioxusFullstackPatternsResource {
    const RESOURCE_URI: &'static str = "resource://dioxus-fullstack-design-patterns";
    const RESOURCE_NAME: &'static str = "Dioxus Fullstack Design Patterns";
    const DESCRIPTION: &'static str =
        "Project playbook for dialogs, forms, lists, and server functions";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for DioxusFullstackPatternsResource {
    async fn read_resource(&self, uri: &str) -> Result<ResourceReadResult, String> {
        if uri != Self::RESOURCE_URI {
            return Err(format!("Unknown resource URI: {}", uri));
        }

        const README_URL: &str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/main/docs/DIOXUS_FULLSTACK_DESIGN_PATTERS.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, README_URL).await
    }
}
