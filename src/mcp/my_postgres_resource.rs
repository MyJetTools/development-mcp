use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyPostgresResource;

impl MyPostgresResource {
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/my-postgres/refs/heads/main/README.md";
    pub const TOOL_FN: &'static str = "get_my_postgres_readme";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch my-postgres README resource content";
}

impl ResourceDefinition for MyPostgresResource {
    const RESOURCE_URI: &'static str = "resource://my-postgres-readme";
    const RESOURCE_NAME: &'static str = "Postgres Design Library";
    const DESCRIPTION: &'static str = "Documentation for my-postgres library";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyPostgresResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
