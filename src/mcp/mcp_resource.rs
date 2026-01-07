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

        let content = ResourceContent {
            uri: Self::RESOURCE_URI.to_string(),
            mime_type: Self::MIME_TYPE.to_string(),
            text: Some(GUIDE_CONTENT.to_string()),
            blob: None,
        };

        Ok(ResourceReadResult {
            contents: vec![content],
        })
    }
}

const GUIDE_CONTENT: &str = std::include_str!("../../docs/mcp-development-guide.md");
