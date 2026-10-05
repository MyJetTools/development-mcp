use mcp_server_middleware::*;

use crate::mcp::{
    scripts::load_resource_by_http, TopicResource, TopicsDocDefinition, TopicsDocTool,
};

pub struct ArchitectSkillResource;

impl ResourceDefinition for ArchitectSkillResource {
    const RESOURCE_URI: &'static str = "resource://architect-skill";
    const RESOURCE_NAME: &'static str = "architect-playbook";
    const DESCRIPTION: &'static str = "Architectural decision playbook. Read BEFORE development whenever a task requires designing changes at the architectural level — microservices boundaries, data ownership, transports between services, service archetypes, queue/event contracts. Make the architecture-level decision here first, then hand the chosen design off to implementation. This resource is the index: the philosophy, the service archetypes, every hard rule in one line, the anti-patterns and the list of topics; each topic is resource://architect-skill/{topic}";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for ArchitectSkillResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}

impl TopicsDocDefinition for ArchitectSkillResource {
    const FILENAME: &'static str = "architect-playbook.md";
    const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/architect-playbook/index_resource.md";
    const DOCS_URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/architect-playbook";
    const TOOL_FN: &'static str = "get_architect_playbook";
    const TOOL_DESCRIPTION: &'static str =
        "Fetch the architect playbook. Without `topic` - the index: decomposition philosophy, \
         service archetypes, every hard rule in one line, the anti-patterns and the list of \
         topics. With `topic` (e.g. `service-bus`, `read-models`, `data-ownership`) - the \
         reasoning, the edge cases and the trade-offs of that area";
    const TOPIC_URI_TEMPLATE: &'static str = "resource://architect-skill/{topic}";
    const TOPIC_TEMPLATE_NAME: &'static str = "architect-playbook topic";
    const TOPIC_DESCRIPTION: &'static str =
        "One area of the architect playbook in depth: the rules, their reasoning and the \
         trade-offs. The topics are listed in resource://architect-skill";
    const TOPIC_NAME_PREFIX: &'static str = "architect-playbook";
}

pub type ArchitectSkillTopicResource = TopicResource<ArchitectSkillResource>;
pub type ArchitectPlaybookTool = TopicsDocTool<ArchitectSkillResource>;
