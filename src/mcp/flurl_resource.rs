use mcp_server_middleware::*;

use crate::mcp::{
    scripts::load_resource_by_http, TopicResource, TopicsDocDefinition, TopicsDocTool,
};

pub struct FlUrlResource;

impl ResourceDefinition for FlUrlResource {
    const RESOURCE_URI: &'static str = "resource://flurl-usage-guide";
    const RESOURCE_NAME: &'static str = "FlUrl Usage Guide";
    const DESCRIPTION: &'static str = "MANDATORY HTTP client for MyJetTools projects. Load this BEFORE writing any HTTP/REST code. reqwest is NOT allowed — use FlUrl instead. This resource is the index: installation and TLS, requests, url building, headers, bodies, responses, redirects, errors, and the list of topics; each topic is resource://flurl-usage-guide/{topic}";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for FlUrlResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}

impl TopicsDocDefinition for FlUrlResource {
    const FILENAME: &'static str = "flurl-usage-guide.md";
    const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/fl-url/refs/heads/main/docs/index_resource.md";
    const DOCS_URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/fl-url/refs/heads/main/docs";
    const TOOL_FN: &'static str = "get_flurl_usage_guide";
    const TOOL_DESCRIPTION: &'static str =
        "Fetch the FlUrl usage guide. MANDATORY prerequisite before writing any HTTP request code \
         in MyJetTools projects — FlUrl is the only allowed HTTP client (no reqwest). Without \
         `topic` - the index: installation and the TLS decision, building and sending requests, \
         url building, headers, bodies, responses, redirects and errors - everything an \
         ordinary request needs. With `topic` (e.g. `retries`, `streamed-body`, `wasm`, `ssh`, \
         `model-driven-requests`) - that feature in depth";
    const TOPIC_URI_TEMPLATE: &'static str = "resource://flurl-usage-guide/{topic}";
    const TOPIC_TEMPLATE_NAME: &'static str = "flurl topic";
    const TOPIC_DESCRIPTION: &'static str =
        "One feature of FlUrl in depth, with examples. The topics are listed in \
         resource://flurl-usage-guide";
    const TOPIC_NAME_PREFIX: &'static str = "flurl";
}

pub type FlUrlTopicResource = TopicResource<FlUrlResource>;
pub type FlUrlUsageGuideTool = TopicsDocTool<FlUrlResource>;
