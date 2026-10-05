use mcp_server_middleware::*;

use crate::mcp::{
    scripts::load_resource_by_http, TopicResource, TopicsDocDefinition, TopicsDocTool,
};

pub struct RustExtensionsResource;

impl ResourceDefinition for RustExtensionsResource {
    const RESOURCE_URI: &'static str = "resource://rust-extensions";
    const RESOURCE_NAME: &'static str = "rust-extensions for each project";
    const DESCRIPTION: &'static str =
        "Low-level utils, queues and other helpers to glue together Rust code. This resource is \
         the index of topics; each topic is resource://rust-extensions/{topic}";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for RustExtensionsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}

impl TopicsDocDefinition for RustExtensionsResource {
    const FILENAME: &'static str = "rust-extensions.md";
    const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/rust-extensions/main/docs/index_resource.md";
    const DOCS_URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/rust-extensions/main/docs";
    const TOOL_FN: &'static str = "get_rust_extensions_readme";
    const TOOL_DESCRIPTION: &'static str =
        "Fetch rust-extensions docs. Without `topic` - the index: what the crate is, its features \
         and the list of topics with their Startable types. With `topic` (e.g. `events-loop`, \
         `date-time`) - that topic in depth, with examples for every case";
    const TOPIC_URI_TEMPLATE: &'static str = "resource://rust-extensions/{topic}";
    const TOPIC_TEMPLATE_NAME: &'static str = "rust-extensions topic";
    const TOPIC_DESCRIPTION: &'static str =
        "One topic of rust-extensions in depth: contracts and examples for every case. The \
         topics are listed in resource://rust-extensions";
    const TOPIC_NAME_PREFIX: &'static str = "rust-extensions";
    const NOTE_COLUMN: Option<&'static str> = Some("Startable");
}

pub type RustExtensionsTopicResource = TopicResource<RustExtensionsResource>;
pub type RustExtensionsReadmeTool = TopicsDocTool<RustExtensionsResource>;
