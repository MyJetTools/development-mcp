use crate::app::AppContext;
use crate::mcp::scripts::load_resource_by_http;
use crate::mcp::*;
use mcp_server_middleware::*;
use my_ai_agent::{macros::*, *};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct EmptyToolInput {}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ResourceToolResponse {
    #[property(description = "Resource URI")]
    pub uri: String,
    #[property(description = "Resource name")]
    pub resource_name: String,
    #[property(description = "Resource description")]
    pub resource_description: String,
    #[property(description = "MIME type for the resource content")]
    pub mime_type: String,
    #[property(description = "Resource content text")]
    pub text: String,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ResourceToolInfo {
    #[property(description = "Tool function name")]
    pub func_name: String,
    #[property(description = "Tool description")]
    pub tool_description: String,
    #[property(description = "Resource URI")]
    pub resource_uri: String,
    #[property(description = "Resource name")]
    pub resource_name: String,
    #[property(description = "Resource description")]
    pub resource_description: String,
    #[property(description = "MIME type for the resource content")]
    pub mime_type: String,
}

#[derive(ApplyJsonSchema, Debug, Serialize, Deserialize)]
pub struct ResourceToolListResponse {
    #[property(description = "List of resource tools exposed by the server")]
    pub items: Vec<ResourceToolInfo>,
}

async fn fetch_resource_text(
    uri: &str,
    resource_name: &str,
    resource_description: &str,
    mime_type: &str,
    url: &str,
) -> Result<ResourceToolResponse, String> {
    let result = load_resource_by_http(uri, mime_type, url).await?;

    let content = result
        .contents
        .into_iter()
        .next()
        .ok_or_else(|| format!("Resource {} returned no content", uri))?;

    let text = content
        .text
        .ok_or_else(|| format!("Resource {} returned no text content", uri))?;

    Ok(ResourceToolResponse {
        uri: content.uri,
        resource_name: resource_name.to_string(),
        resource_description: resource_description.to_string(),
        mime_type: content.mime_type,
        text,
    })
}

/// Defines an MCP tool wrapper around a resource type. Pulls every piece of
/// metadata (URL, function name, descriptions, URI, MIME) from constants on
/// the resource type itself so there is exactly one source of truth per
/// resource.
macro_rules! define_resource_tool {
    ($tool:ident, $res:ty) => {
        pub struct $tool {
            _app: Arc<AppContext>,
        }

        impl $tool {
            pub fn new(app: Arc<AppContext>) -> Self {
                Self { _app: app }
            }
        }

        impl ToolDefinition for $tool {
            const FUNC_NAME: &'static str = <$res>::TOOL_FN;
            const DESCRIPTION: &'static str = <$res>::TOOL_DESCRIPTION;
        }

        #[async_trait::async_trait]
        impl McpToolCall<EmptyToolInput, ResourceToolResponse> for $tool {
            async fn execute_tool_call(
                &self,
                _model: EmptyToolInput,
            ) -> Result<ResourceToolResponse, String> {
                fetch_resource_text(
                    <$res as ResourceDefinition>::RESOURCE_URI,
                    <$res as ResourceDefinition>::RESOURCE_NAME,
                    <$res as ResourceDefinition>::DESCRIPTION,
                    <$res as ResourceDefinition>::MIME_TYPE,
                    <$res>::URL,
                )
                .await
            }
        }
    };
}

/// Builds a `ResourceToolInfo` directly from a resource type's constants.
macro_rules! tool_info {
    ($res:ty) => {
        ResourceToolInfo {
            func_name: <$res>::TOOL_FN.to_string(),
            tool_description: <$res>::TOOL_DESCRIPTION.to_string(),
            resource_uri: <$res as ResourceDefinition>::RESOURCE_URI.to_string(),
            resource_name: <$res as ResourceDefinition>::RESOURCE_NAME.to_string(),
            resource_description: <$res as ResourceDefinition>::DESCRIPTION.to_string(),
            mime_type: <$res as ResourceDefinition>::MIME_TYPE.to_string(),
        }
    };
}

fn all_resource_tools() -> Vec<ResourceToolInfo> {
    vec![
        tool_info!(McpResource),
        tool_info!(FlUrlResource),
        tool_info!(HttpActionsResource),
        tool_info!(AppBootstrapResource),
        tool_info!(DioxusBootstrapResource),
        tool_info!(CargoDependenciesResource),
        tool_info!(MySshResource),
        tool_info!(MyTcpSocketsResource),
        tool_info!(RustExtensionsResource),
        tool_info!(DioxusFullstackPatternsResource),
        tool_info!(MyNoSqlEntityPatternsResource),
        tool_info!(MyGrpcExtensionsResource),
        tool_info!(DioxusUtilsResource),
        tool_info!(CiUtilsResource),
        tool_info!(MyPostgresResource),
        tool_info!(DioxusAdminUiKitResource),
        tool_info!(RustFixResource),
    ]
}

pub struct ListResourceToolsTool {
    _app: Arc<AppContext>,
}

impl ListResourceToolsTool {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { _app: app }
    }
}

impl ToolDefinition for ListResourceToolsTool {
    const FUNC_NAME: &'static str = "list_resource_tools";
    const DESCRIPTION: &'static str = "List resource tools exposed by this server";
}

#[async_trait::async_trait]
impl McpToolCall<EmptyToolInput, ResourceToolListResponse> for ListResourceToolsTool {
    async fn execute_tool_call(
        &self,
        _model: EmptyToolInput,
    ) -> Result<ResourceToolListResponse, String> {
        Ok(ResourceToolListResponse {
            items: all_resource_tools(),
        })
    }
}

define_resource_tool!(McpDevelopmentGuideTool, McpResource);
define_resource_tool!(FlUrlUsageGuideTool, FlUrlResource);
define_resource_tool!(HttpActionsDesignGuideTool, HttpActionsResource);
define_resource_tool!(AppBootstrapGuideTool, AppBootstrapResource);
define_resource_tool!(DioxusBootstrapGuideTool, DioxusBootstrapResource);
define_resource_tool!(CargoDependenciesGuideTool, CargoDependenciesResource);
define_resource_tool!(MySshReadmeTool, MySshResource);
define_resource_tool!(MyTcpSocketsReadmeTool, MyTcpSocketsResource);
define_resource_tool!(RustExtensionsReadmeTool, RustExtensionsResource);
define_resource_tool!(DioxusFullstackPatternsTool, DioxusFullstackPatternsResource);
define_resource_tool!(MyNoSqlEntityPatternsTool, MyNoSqlEntityPatternsResource);
define_resource_tool!(MyGrpcExtensionsReadmeTool, MyGrpcExtensionsResource);
define_resource_tool!(DioxusUtilsReadmeTool, DioxusUtilsResource);
define_resource_tool!(CiUtilsReadmeTool, CiUtilsResource);
define_resource_tool!(MyPostgresReadmeTool, MyPostgresResource);
define_resource_tool!(DioxusAdminUiKitTool, DioxusAdminUiKitResource);
define_resource_tool!(RustFixReadmeTool, RustFixResource);
