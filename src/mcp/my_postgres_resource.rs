use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyPostgresResource;

impl ResourceDefinition for MyPostgresResource {
    const RESOURCE_URI: &'static str = "resource://my-postgres-readme";
    const RESOURCE_NAME: &'static str = "Postgres Design Library";
    const DESCRIPTION: &'static str = "Documentation for my-postgres library";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyPostgresResource {
    async fn read_resource(&self, uri: &str) -> Result<ResourceReadResult, String> {
        if uri != Self::RESOURCE_URI {
            return Err(format!("Unknown resource URI: {}", uri));
        }

        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/my-postgres/refs/heads/main/README.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, README_URL).await
    }
}
