use mcp_server_middleware::*;

use crate::mcp::{
    scripts::load_resource_by_http, TopicResource, TopicsDocDefinition, TopicsDocTool,
};

pub struct FlutterDesignPatternsResource;

impl ResourceDefinition for FlutterDesignPatternsResource {
    const RESOURCE_URI: &'static str = "resource://flutter-design-patterns";
    const RESOURCE_NAME: &'static str = "Flutter Design Patterns";
    const DESCRIPTION: &'static str =
        "Framework-level Flutter conventions for the phone twin of a Dioxus client, with the \
         section numbers of the Dioxus design patterns. This resource is the index: one state \
         class per component (HookWidget + ChangeNotifier + provider, the twin of ComponentState \
         and signals), the render / state / actions / mod folder, dialogs (DialogState + \
         RenderDialog + dialogTemplate), DataState and async reads, NotifyChildComponent, stored \
         state, the rules for every line of Dart, and the list of topics - a widget that draws on \
         a canvas (CustomPainter, a chart engine in the Rust core) and the theme generated from \
         the Dioxus CSS tokens; each topic is resource://flutter-design-patterns/{topic}";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for FlutterDesignPatternsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}

impl TopicsDocDefinition for FlutterDesignPatternsResource {
    const FILENAME: &'static str = "flutter-design-patterns.md";
    const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/flutter-design-patterns/index_resource.md";
    const DOCS_URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/flutter-design-patterns";
    const TOOL_FN: &'static str = "get_flutter_design_patterns";
    const TOOL_DESCRIPTION: &'static str =
        "Fetch Flutter design patterns - the twins of the Dioxus design patterns, section by \
         section, for a Flutter app whose logic is a Rust core. Without `topic` - the index: \
         state (HookWidget + ChangeNotifier + provider), component folders, dialogs, DataState, \
         async reads, the Dart rules and the list of topics. With `topic` - that pattern in \
         depth: `canvas` is a widget that draws on a canvas (CustomPainter, a chart engine in \
         the Rust core) - the picture is drawn by frames, not from state; `theme` is the look of \
         the web client on a phone - tokens generated from its CSS, safe areas, touch, sheets. \
         Load it before writing or changing a Flutter widget.";
    const TOPIC_URI_TEMPLATE: &'static str = "resource://flutter-design-patterns/{topic}";
    const TOPIC_TEMPLATE_NAME: &'static str = "flutter-design-patterns topic";
    const TOPIC_DESCRIPTION: &'static str =
        "One Flutter design pattern in depth. The topics are listed in \
         resource://flutter-design-patterns";
    const TOPIC_NAME_PREFIX: &'static str = "flutter-design-patterns";
}

pub type FlutterDesignPatternsTopicResource = TopicResource<FlutterDesignPatternsResource>;
pub type FlutterDesignPatternsTool = TopicsDocTool<FlutterDesignPatternsResource>;
