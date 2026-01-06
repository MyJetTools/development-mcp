use flurl::FlUrl;
use mcp_server_middleware::*;

pub struct FlUrlResource;

impl ResourceDefinition for FlUrlResource {
    const RESOURCE_URI: &'static str = "resource://flurl-usage-guide";
    const RESOURCE_NAME: &'static str = "FlUrl Usage Guide";
    const DESCRIPTION: &'static str = "How to use FlUrl library";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for FlUrlResource {
    async fn read_resource(&self, uri: &str) -> Result<ResourceReadResult, String> {
        if uri != Self::RESOURCE_URI {
            return Err(format!("Unknown resource URI: {}", uri));
        }

        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/fl-url/refs/heads/main/README.md";

        // Fetch the README content using FlUrl
        let mut response = FlUrl::new(README_URL)
            .get()
            .await
            .map_err(|e| format!("Failed to fetch FlUrl README: {:?}", e))?;

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
