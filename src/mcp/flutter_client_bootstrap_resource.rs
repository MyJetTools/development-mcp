use mcp_server_middleware::*;

use crate::mcp::{
    scripts::load_resource_by_http, TopicResource, TopicsDocDefinition, TopicsDocTool,
};

pub struct FlutterClientBootstrapResource;

impl ResourceDefinition for FlutterClientBootstrapResource {
    const RESOURCE_URI: &'static str = "resource://flutter-client-bootstrap";
    const RESOURCE_NAME: &'static str = "Flutter Client Bootstrap Guide";
    const DESCRIPTION: &'static str =
        "Bootstrap a Flutter app for iOS and Android built in parallel with a Dioxus client-side \
         web app: the business logic is the same Rust core, embedded through flutter_rust_bridge \
         and kept without I/O; Dart does the HTTP, the WebSocket, the storage and the rendering. \
         This resource is the index: the architecture, when to use it, how a feature is built in \
         parallel, the project structure that mirrors the Dioxus client file for file, the build \
         commands, the critical reminders and the list of topics; each topic is \
         resource://flutter-client-bootstrap/{topic}";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for FlutterClientBootstrapResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}

impl TopicsDocDefinition for FlutterClientBootstrapResource {
    const FILENAME: &'static str = "flutter-client-bootstrap.md";
    const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/bootstrap-flutter-client-project/index_resource.md";
    const DOCS_URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/bootstrap-flutter-client-project";
    const TOOL_FN: &'static str = "get_flutter_client_bootstrap_guide";
    const TOOL_DESCRIPTION: &'static str =
        "Fetch the Flutter client (iOS / Android) bootstrap guide - the phone twin of a Dioxus \
         client-side app: the Rust core embedded through flutter_rust_bridge, without I/O; HTTP, \
         WebSocket and storage in Dart. Without `topic` - the index: architecture, project \
         structure, build commands, critical reminders and the list of topics. With `topic` \
         (e.g. `rust-core`, `bridge`, `io`, `websocket`, `api-calls`, `local-storage`) - that part \
         of the skeleton. Topic `ci` is the pinned toolchain, the guards before every commit, \
         signing and release of the app. Load it before creating a Flutter app, or before \
         changing its bridge to the Rust core or its I/O.";
    const TOPIC_URI_TEMPLATE: &'static str = "resource://flutter-client-bootstrap/{topic}";
    const TOPIC_TEMPLATE_NAME: &'static str = "flutter-client-bootstrap topic";
    const TOPIC_DESCRIPTION: &'static str =
        "One part of the Flutter client project skeleton, with the code to start from. The \
         topics are listed in resource://flutter-client-bootstrap";
    const TOPIC_NAME_PREFIX: &'static str = "flutter-client-bootstrap";
}

pub type FlutterClientBootstrapTopicResource = TopicResource<FlutterClientBootstrapResource>;
pub type FlutterClientBootstrapTool = TopicsDocTool<FlutterClientBootstrapResource>;
