use flurl::FlUrl;
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

        let mut response = FlUrl::new(GUIDE_URL)
            .get()
            .await
            .map_err(|e| format!("Failed to fetch Cargo dependencies guide: {:?}", e))?;

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
