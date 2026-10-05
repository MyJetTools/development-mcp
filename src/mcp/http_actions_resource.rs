use mcp_server_middleware::*;

use crate::mcp::{
    scripts::load_resource_by_http, TopicResource, TopicsDocDefinition, TopicsDocTool,
};

pub struct HttpActionsResource;

impl ResourceDefinition for HttpActionsResource {
    const RESOURCE_URI: &'static str = "resource://http-actions-design-guide";
    const RESOURCE_NAME: &'static str = "HTTP Actions Design Guide";
    const DESCRIPTION: &'static str =
        "Guide for HTTP action architecture and patterns. This resource is the index: the \
         directory structure, the action struct with #[http_route] and handle_request, the \
         steps of creating an action, the rules, and the list of topics; each topic is \
         resource://http-actions-design-guide/{topic}";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for HttpActionsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}

impl TopicsDocDefinition for HttpActionsResource {
    const FILENAME: &'static str = "http-actions-design-guide.md";
    const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/my-http-server/refs/heads/main/docs/http-actions/index_resource.md";
    const DOCS_URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/my-http-server/refs/heads/main/docs/http-actions";
    const TOOL_FN: &'static str = "get_http_actions_design_guide";
    const TOOL_DESCRIPTION: &'static str =
        "Fetch the HTTP actions design guide (my-http-server). Without `topic` - the index: the \
         directory structure, the action struct with #[http_route] and handle_request, the steps \
         of creating an action and the rules. With `topic` (e.g. `input-models`, \
         `output-models`, `responses`, `errors`, `example`, `cookies`, `client-ip`) - that part \
         in depth";
    const TOPIC_URI_TEMPLATE: &'static str = "resource://http-actions-design-guide/{topic}";
    const TOPIC_TEMPLATE_NAME: &'static str = "http-actions topic";
    const TOPIC_DESCRIPTION: &'static str =
        "One part of writing HTTP actions in depth, with examples. The topics are listed in \
         resource://http-actions-design-guide";
    const TOPIC_NAME_PREFIX: &'static str = "http-actions";
}

pub type HttpActionsTopicResource = TopicResource<HttpActionsResource>;
pub type HttpActionsDesignGuideTool = TopicsDocTool<HttpActionsResource>;
