use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct ApplicationArchitectureResource;

impl ApplicationArchitectureResource {
    pub const FILENAME: &'static str = "application-architecture-best-practices.md";
    pub const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/application-architecture-best-practices.md";
    pub const TOOL_FN: &'static str = "get_application_architecture_best_practices";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch application architecture best practices: project structure, flows/scripts, gRPC, Postgres, Service Bus, HTTP actions, mappers, MyNoSql, settings, logging, error handling";
}

impl ResourceDefinition for ApplicationArchitectureResource {
    const RESOURCE_URI: &'static str = "resource://application-architecture-best-practices";
    const RESOURCE_NAME: &'static str = "Application Architecture Best Practices";
    const DESCRIPTION: &'static str =
        "Complete coding standards: project structure, flows/scripts, gRPC, Postgres, Service Bus, HTTP actions, mappers, MyNoSql, settings, logging, error handling";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for ApplicationArchitectureResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
