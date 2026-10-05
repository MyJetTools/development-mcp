use mcp_server_middleware::*;

use crate::mcp::{
    scripts::load_resource_by_http, TopicResource, TopicsDocDefinition, TopicsDocTool,
};

pub struct ApplicationArchitectureResource;

impl ResourceDefinition for ApplicationArchitectureResource {
    const RESOURCE_URI: &'static str = "resource://application-architecture-best-practices";
    const RESOURCE_NAME: &'static str = "Application Architecture Best Practices";
    const DESCRIPTION: &'static str =
        "Coding standards. This resource is the index: the rules for every line of code (zero \
         warnings, named structs, exhaustive matches, logging levels, project layers, module \
         exports) and the list of topics - AppContext, settings, build.rs, gRPC, Postgres, flows, \
         MyNoSql, ServiceContext, mappers, Service Bus, HTTP actions, shared wire models; each \
         topic is resource://application-architecture-best-practices/{topic}";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for ApplicationArchitectureResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}

impl TopicsDocDefinition for ApplicationArchitectureResource {
    const FILENAME: &'static str = "application-architecture-best-practices.md";
    const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/application-architecture-best-practices/index_resource.md";
    const DOCS_URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/application-architecture-best-practices";
    const TOOL_FN: &'static str = "get_application_architecture_best_practices";
    const TOOL_DESCRIPTION: &'static str =
        "Fetch application architecture best practices. Without `topic` - the index: the rules \
         for every line of code (zero warnings, named structs, exhaustive matches, logging \
         levels, project layers flows/scripts, module exports) and the list of topics. With \
         `topic` (e.g. `grpc`, `postgres`, `service-bus`, `my-no-sql`, `http`) - the pattern of \
         that area with examples and error handling";
    const TOPIC_URI_TEMPLATE: &'static str =
        "resource://application-architecture-best-practices/{topic}";
    const TOPIC_TEMPLATE_NAME: &'static str = "application-architecture topic";
    const TOPIC_DESCRIPTION: &'static str =
        "The coding pattern of one area of a service, with examples and error handling. The \
         topics are listed in resource://application-architecture-best-practices";
    const TOPIC_NAME_PREFIX: &'static str = "application-architecture";
}

pub type ApplicationArchitectureTopicResource = TopicResource<ApplicationArchitectureResource>;
pub type ApplicationArchitectureTool = TopicsDocTool<ApplicationArchitectureResource>;
