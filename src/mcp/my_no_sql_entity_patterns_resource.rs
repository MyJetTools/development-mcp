use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyNoSqlEntityPatternsResource;

impl ResourceDefinition for MyNoSqlEntityPatternsResource {
    const RESOURCE_URI: &'static str = "resource://my-no-sql-entity-design-patterns";
    const RESOURCE_NAME: &'static str = "MyNoSql Entity Design Patterns";
    const DESCRIPTION: &'static str = "Design patterns for MyNoSql entities and enums";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyNoSqlEntityPatternsResource {
    async fn read_resource(&self, uri: &str) -> Result<ResourceReadResult, String> {
        if uri != Self::RESOURCE_URI {
            return Err(format!("Unknown resource URI: {}", uri));
        }

        const README_URL: &str = "https://raw.githubusercontent.com/MyJetTools/my-no-sql-sdk/refs/heads/main/MY_NO_SQL_ENTITY_DESIGN_PATTERNS.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, README_URL).await
    }
}
