use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct ServiceSdkResource;

impl ServiceSdkResource {
    pub const FILENAME: &'static str = "service-sdk.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/service-sdk/refs/heads/main/README.md";
    pub const TOOL_FN: &'static str = "get_service_sdk_readme";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch the service-sdk README: the source of truth for ServiceContext - the startup order of start_application, and how timers, queues, events loops, background executors, HTTP / gRPC servers, MyNoSql readers and writers, Service Bus and Postgres are created on it";
}

impl ResourceDefinition for ServiceSdkResource {
    const RESOURCE_URI: &'static str = "resource://service-sdk";
    const RESOURCE_NAME: &'static str = "service-sdk README";
    const DESCRIPTION: &'static str =
        "ServiceContext: startup order, feature flags, and the methods every component of a service is created with";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for ServiceSdkResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
