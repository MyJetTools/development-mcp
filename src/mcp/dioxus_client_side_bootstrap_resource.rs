use mcp_server_middleware::*;

use crate::mcp::{
    scripts::load_resource_by_http, TopicResource, TopicsDocDefinition, TopicsDocTool,
};

pub struct DioxusClientSideBootstrapResource;

impl ResourceDefinition for DioxusClientSideBootstrapResource {
    const RESOURCE_URI: &'static str = "resource://dioxus-client-side-bootstrap";
    const RESOURCE_NAME: &'static str = "Dioxus Client-Side Bootstrap Guide";
    const DESCRIPTION: &'static str =
        "Bootstrap a new Dioxus client-side (WASM-only) web application with WebSocket and API \
         calls. This resource is the index: when to use client-side, the project structure, the \
         build commands, the critical reminders and the list of topics; each topic is \
         resource://dioxus-client-side-bootstrap/{topic}";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for DioxusClientSideBootstrapResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}

impl TopicsDocDefinition for DioxusClientSideBootstrapResource {
    const FILENAME: &'static str = "dioxus-client-side-bootstrap.md";
    const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/bootstrap-dioxus-client-side-project/index_resource.md";
    const DOCS_URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/bootstrap-dioxus-client-side-project";
    const TOOL_FN: &'static str = "get_dioxus_client_side_bootstrap_guide";
    const TOOL_DESCRIPTION: &'static str =
        "Fetch the Dioxus client-side (WASM-only) bootstrap guide. Without `topic` - the index: \
         project structure, build commands, critical reminders and the list of topics. With \
         `topic` (e.g. `project-files`, `websocket`, `api-calls`, `shared-wire-models`) - that \
         part of the skeleton. Topic `ci` is the CI workflow for a Dioxus WASM app — dx build \
         inside the ghcr.io/my-jet-tools/dioxus-docker container, cache-busting build.py, \
         static-hosting Dockerfile. Load it before writing CI for any Dioxus client; the \
         native-service builder-image pattern does not apply to it.";
    const TOPIC_URI_TEMPLATE: &'static str = "resource://dioxus-client-side-bootstrap/{topic}";
    const TOPIC_TEMPLATE_NAME: &'static str = "dioxus-client-side-bootstrap topic";
    const TOPIC_DESCRIPTION: &'static str =
        "One part of the Dioxus client-side project skeleton, with the code to start from. The \
         topics are listed in resource://dioxus-client-side-bootstrap";
    const TOPIC_NAME_PREFIX: &'static str = "dioxus-client-side-bootstrap";
}

pub type DioxusClientSideBootstrapTopicResource = TopicResource<DioxusClientSideBootstrapResource>;
pub type DioxusClientSideBootstrapTool = TopicsDocTool<DioxusClientSideBootstrapResource>;
