use mcp_server_middleware::*;

use crate::mcp::{
    scripts::load_resource_by_http, TopicResource, TopicsDocDefinition, TopicsDocTool,
};

pub struct DioxusDesignPatternsResource;

impl ResourceDefinition for DioxusDesignPatternsResource {
    const RESOURCE_URI: &'static str = "resource://dioxus-design-patterns";
    const RESOURCE_NAME: &'static str = "Dioxus Design Patterns";
    const DESCRIPTION: &'static str =
        "Framework-level Dioxus conventions. This resource is the index: ComponentState, \
         signals, dialogs (DialogState + RenderDialog + dialog_template), DataState, async data \
         loading, CSS pipeline, state initialised from browser storage (sessionStorage / \
         localStorage), and the list of topics - a component that draws on a canvas (the 2D \
         context, WebGL, WebGPU); each topic is resource://dioxus-design-patterns/{topic}";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for DioxusDesignPatternsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}

impl TopicsDocDefinition for DioxusDesignPatternsResource {
    const FILENAME: &'static str = "dioxus-design-patterns.md";
    const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/dioxus-design-patterns/index_resource.md";
    const DOCS_URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/dioxus-design-patterns";
    const TOOL_FN: &'static str = "get_dioxus_design_patterns";
    const TOOL_DESCRIPTION: &'static str =
        "Fetch Dioxus design patterns (framework-level conventions for fullstack and \
         client-side projects). Without `topic` - the index: dialogs, state, components, \
         signals, state initialised from browser storage - sessionStorage / localStorage - and \
         the list of topics. With `topic` - that pattern in depth: `canvas` is a component that \
         draws on a canvas (the 2D context, WebGL, WebGPU) - the picture is drawn by animation \
         frames, not from state. Load it before writing or changing anything that draws on a \
         canvas.";
    const TOPIC_URI_TEMPLATE: &'static str = "resource://dioxus-design-patterns/{topic}";
    const TOPIC_TEMPLATE_NAME: &'static str = "dioxus-design-patterns topic";
    const TOPIC_DESCRIPTION: &'static str =
        "One Dioxus pattern in depth, with examples. The topics are listed in \
         resource://dioxus-design-patterns";
    const TOPIC_NAME_PREFIX: &'static str = "dioxus-design-patterns";
}

pub type DioxusDesignPatternsTopicResource = TopicResource<DioxusDesignPatternsResource>;
pub type DioxusDesignPatternsTool = TopicsDocTool<DioxusDesignPatternsResource>;
