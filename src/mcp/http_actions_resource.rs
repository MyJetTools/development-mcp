use flurl::FlUrl;
use mcp_server_middleware::*;

pub struct HttpActionsResource;

impl ResourceDefinition for HttpActionsResource {
    const RESOURCE_URI: &'static str = "resource://http-actions-design-guide";
    const RESOURCE_NAME: &'static str = "HTTP Actions Design Guide";
    const DESCRIPTION: &'static str = "Guide for HTTP action architecture and patterns";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for HttpActionsResource {
    async fn read_resource(&self, uri: &str) -> Result<ResourceReadResult, String> {
        if uri != Self::RESOURCE_URI {
            return Err(format!("Unknown resource URI: {}", uri));
        }

        const HTTP_ACTIONS_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/my-http-server/refs/heads/main/HTTP_ACTIONS_DESIGN.md";

        // Fetch the HTTP Actions Design content using FlUrl
        let mut response = FlUrl::new(HTTP_ACTIONS_URL)
            .get()
            .await
            .map_err(|e| format!("Failed to fetch HTTP Actions Design: {:?}", e))?;

        let content_str = response
            .get_body_as_str()
            .await
            .map_err(|e| format!("Failed to read response body: {:?}", e))?;

        let result = ResourceContent {
            uri: Self::RESOURCE_URI.to_string(),
            mime_type: Self::MIME_TYPE.to_string(),
            text: Some(content_str.to_string()),
            blob: None,
        };

        Ok(ResourceReadResult {
            contents: vec![result],
        })
    }
}
