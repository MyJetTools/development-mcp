use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct ArchitectSkillResource;

impl ArchitectSkillResource {
    pub const FILENAME: &'static str = "architect-playbook.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/architect-playbook.md";
    pub const TOOL_FN: &'static str = "get_architect_playbook";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch architect playbook resource content";
}

impl ResourceDefinition for ArchitectSkillResource {
    const RESOURCE_URI: &'static str = "resource://architect-skill";
    const RESOURCE_NAME: &'static str = "architect-playbook";
    const DESCRIPTION: &'static str = "Architectural decision playbook. Read BEFORE development whenever a task requires designing changes at the architectural level — microservices boundaries, data ownership, transports between services, service archetypes, queue/event contracts. Make the architecture-level decision here first, then hand the chosen design off to implementation.";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for ArchitectSkillResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
