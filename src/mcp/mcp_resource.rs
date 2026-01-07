use flurl::FlUrl;
use mcp_server_middleware::*;

pub struct McpResource;

impl ResourceDefinition for McpResource {
    const RESOURCE_URI: &'static str = "resource://mcp-development-guide";
    const RESOURCE_NAME: &'static str = "MCP Development Guide";
    const DESCRIPTION: &'static str = "Guide for creating Prompts and Tool Calls";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for McpResource {
    async fn read_resource(&self, uri: &str) -> Result<ResourceReadResult, String> {
        if uri != Self::RESOURCE_URI {
            return Err(format!("Unknown resource URI: {}", uri));
        }

        const GUIDE_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/mcp-development-guide.md";

        let mut response = FlUrl::new(GUIDE_URL)
            .get()
            .await
            .map_err(|e| format!("Failed to fetch MCP development guide: {:?}", e))?;

        let content_str = response
            .get_body_as_str()
            .await
            .map_err(|e| format!("Failed to read response body: {:?}", e))?;

        let content = ResourceContent {
            uri: Self::RESOURCE_URI.to_string(),
            mime_type: Self::MIME_TYPE.to_string(),
            text: Some(content_str.to_string()),
            blob: None,
        };

        Ok(ResourceReadResult {
            contents: vec![content],
        })
    }
}
