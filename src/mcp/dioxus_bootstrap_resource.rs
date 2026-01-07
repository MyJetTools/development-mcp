use flurl::FlUrl;
use mcp_server_middleware::*;

pub struct DioxusBootstrapResource;

impl ResourceDefinition for DioxusBootstrapResource {
    const RESOURCE_URI: &'static str = "resource://development/dioxus-bootstrap";
    const RESOURCE_NAME: &'static str = "Dioxus Fullstack Bootstrap Guide";
    const DESCRIPTION: &'static str =
        "Step-by-step instructions for bootstrapping a new empty Dioxus fullstack web application";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for DioxusBootstrapResource {
    async fn read_resource(&self, uri: &str) -> Result<ResourceReadResult, String> {
        if uri != Self::RESOURCE_URI {
            return Err(format!("Unknown resource URI: {}", uri));
        }

        const DIOXUS_BOOTSTRAP_URL: &str =
            "https://raw.githubusercontent.com/amigin/ai-templates/refs/heads/main/cursor/bootstrap-empty-dioxus-fullstack-project.mdc";

        // Fetch the Dioxus bootstrap guide content using FlUrl
        let mut response = FlUrl::new(DIOXUS_BOOTSTRAP_URL)
            .get()
            .await
            .map_err(|e| format!("Failed to fetch Dioxus Bootstrap guide: {:?}", e))?;

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
