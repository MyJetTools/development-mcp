use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyNoSqlEntityPatternsResource;

impl MyNoSqlEntityPatternsResource {
    pub const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/my-no-sql-sdk/refs/heads/main/MY_NO_SQL_ENTITY_DESIGN_PATTERNS.md";
    pub const TOOL_FN: &'static str = "get_my_no_sql_entity_patterns";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch MyNoSql entity design patterns resource content";
}

impl ResourceDefinition for MyNoSqlEntityPatternsResource {
    const RESOURCE_URI: &'static str = "resource://my-no-sql-entity-design-patterns";
    const RESOURCE_NAME: &'static str = "MyNoSql Entity Design Patterns";
    const DESCRIPTION: &'static str = "Design patterns for MyNoSql entities and enums";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyNoSqlEntityPatternsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
