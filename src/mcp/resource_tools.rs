use crate::app::AppContext;
use crate::mcp::scripts::load_resource_by_http;
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

macro_rules! define_resource_tool {
    ($name:ident, $func:expr, $tool_desc:expr, $resource_name:expr, $resource_desc:expr, $uri:expr, $mime:expr, $url:expr) => {
        pub struct $name {
            _app: Arc<AppContext>,
        }

        impl $name {
            pub fn new(app: Arc<AppContext>) -> Self {
                Self { _app: app }
            }
        }

        impl ToolDefinition for $name {
            const FUNC_NAME: &'static str = $func;
            const DESCRIPTION: &'static str = $tool_desc;
        }

        #[async_trait::async_trait]
        impl McpToolCall<EmptyToolInput, ResourceToolResponse> for $name {
            async fn execute_tool_call(
                &self,
                _model: EmptyToolInput,
            ) -> Result<ResourceToolResponse, String> {
                fetch_resource_text($uri, $resource_name, $resource_desc, $mime, $url).await
            }
        }
    };
}

fn all_resource_tools() -> Vec<ResourceToolInfo> {
    vec![
        ResourceToolInfo {
            func_name: "get_mcp_development_guide".to_string(),
            tool_description: "Fetch MCP development guide resource content".to_string(),
            resource_uri: "resource://mcp-development-guide".to_string(),
            resource_name: "MCP Development Guide".to_string(),
            resource_description: "Guide for creating Prompts and Tool Calls".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_flurl_usage_guide".to_string(),
            tool_description: "Fetch FlUrl usage guide resource content".to_string(),
            resource_uri: "resource://flurl-usage-guide".to_string(),
            resource_name: "FlUrl Usage Guide".to_string(),
            resource_description: "How to use FlUrl library".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_http_actions_design_guide".to_string(),
            tool_description: "Fetch HTTP actions design guide resource content".to_string(),
            resource_uri: "resource://http-actions-design-guide".to_string(),
            resource_name: "HTTP Actions Design Guide".to_string(),
            resource_description: "Guide for HTTP action architecture and patterns".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_app_bootstrap_guide".to_string(),
            tool_description: "Fetch app bootstrap guide resource content".to_string(),
            resource_uri: "resource://app-bootstrap".to_string(),
            resource_name: "App Bootstrap Guide".to_string(),
            resource_description: "Step-by-step instructions for bootstrapping a new project".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_dioxus_bootstrap_guide".to_string(),
            tool_description: "Fetch Dioxus bootstrap guide resource content".to_string(),
            resource_uri: "resource://dioxus-bootstrap".to_string(),
            resource_name: "Dioxus Fullstack Bootstrap Guide".to_string(),
            resource_description: "Step-by-step instructions for bootstrapping a new empty Dioxus fullstack web application".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_cargo_dependencies_guide".to_string(),
            tool_description: "Fetch Cargo dependencies guide resource content".to_string(),
            resource_uri: "resource://cargo-dependencies-guide".to_string(),
            resource_name: "Cargo Dependencies Guide".to_string(),
            resource_description: "How to add dependencies to Cargo.toml".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_my_ssh_readme".to_string(),
            tool_description: "Fetch my-ssh README resource content".to_string(),
            resource_uri: "resource://my-ssh-readme".to_string(),
            resource_name: "Ssh connections design library".to_string(),
            resource_description: "Async SSH helpers for commands, file transfer, and port forwarding.".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_my_tcp_sockets_readme".to_string(),
            tool_description: "Fetch my-tcp-sockets README resource content".to_string(),
            resource_uri: "resource://tcp-sockets-design-library".to_string(),
            resource_name: "TcpSockets design library".to_string(),
            resource_description: "Async TCP server/client building blocks with ping/pong and TLS options".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_rust_extensions_readme".to_string(),
            tool_description: "Fetch rust-extensions README resource content".to_string(),
            resource_uri: "resource://rust-extensions".to_string(),
            resource_name: "rust-extensions for each project".to_string(),
            resource_description: "Low-level utils, queues and other helpers to glue together Rust code".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_dioxus_fullstack_design_patterns".to_string(),
            tool_description: "Fetch Dioxus fullstack design patterns resource content".to_string(),
            resource_uri: "resource://dioxus-fullstack-design-patterns".to_string(),
            resource_name: "Dioxus Fullstack Design Patterns".to_string(),
            resource_description: "Project playbook for dialogs, forms, lists, and server functions".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_my_no_sql_entity_patterns".to_string(),
            tool_description: "Fetch MyNoSql entity design patterns resource content".to_string(),
            resource_uri: "resource://my-no-sql-entity-design-patterns".to_string(),
            resource_name: "MyNoSql Entity Design Patterns".to_string(),
            resource_description: "Design patterns for MyNoSql entities and enums".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_my_grpc_extensions_readme".to_string(),
            tool_description: "Fetch my-grpc-extensions README resource content".to_string(),
            resource_uri: "resource://my-grpc-extensions.md".to_string(),
            resource_name: "Grpc extensions".to_string(),
            resource_description: "Utilities and macros for building gRPC clients and servers".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_dioxus_utils_readme".to_string(),
            tool_description: "Fetch dioxus-utils README resource content".to_string(),
            resource_uri: "resource://dioxus-utils-readme".to_string(),
            resource_name: "dioxus-utils Usage Cases Guide".to_string(),
            resource_description: "Utilities for Dioxus apps: data state, dialogs, JS helpers".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_ci_utils_readme".to_string(),
            tool_description: "Fetch ci-utils README resource content".to_string(),
            resource_uri: "resource://ci-utils-readme".to_string(),
            resource_name: "ci-utils for each project".to_string(),
            resource_description: "Utility crate for build-time helpers".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        ResourceToolInfo {
            func_name: "get_my_postgres_readme".to_string(),
            tool_description: "Fetch my-postgres README resource content".to_string(),
            resource_uri: "resource://my-postgres-readme".to_string(),
            resource_name: "Postgres Design Library".to_string(),
            resource_description: "Documentation for my-postgres library".to_string(),
            mime_type: "text/markdown".to_string(),
        },
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

define_resource_tool!(
    McpDevelopmentGuideTool,
    "get_mcp_development_guide",
    "Fetch MCP development guide resource content",
    "MCP Development Guide",
    "Guide for creating Prompts and Tool Calls",
    "resource://mcp-development-guide",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/mcp-development-guide.md"
);

define_resource_tool!(
    FlUrlUsageGuideTool,
    "get_flurl_usage_guide",
    "Fetch FlUrl usage guide resource content",
    "FlUrl Usage Guide",
    "How to use FlUrl library",
    "resource://flurl-usage-guide",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/fl-url/refs/heads/main/README.md"
);

define_resource_tool!(
    HttpActionsDesignGuideTool,
    "get_http_actions_design_guide",
    "Fetch HTTP actions design guide resource content",
    "HTTP Actions Design Guide",
    "Guide for HTTP action architecture and patterns",
    "resource://http-actions-design-guide",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/my-http-server/refs/heads/main/HTTP_ACTIONS_DESIGN.md"
);

define_resource_tool!(
    AppBootstrapGuideTool,
    "get_app_bootstrap_guide",
    "Fetch app bootstrap guide resource content",
    "App Bootstrap Guide",
    "Step-by-step instructions for bootstrapping a new project",
    "resource://app-bootstrap",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/service-sdk/refs/heads/main/APP_BOOTSTRAP.md"
);

define_resource_tool!(
    DioxusBootstrapGuideTool,
    "get_dioxus_bootstrap_guide",
    "Fetch Dioxus bootstrap guide resource content",
    "Dioxus Fullstack Bootstrap Guide",
    "Step-by-step instructions for bootstrapping a new empty Dioxus fullstack web application",
    "resource://dioxus-bootstrap",
    "text/markdown",
    "https://raw.githubusercontent.com/amigin/ai-templates/refs/heads/main/cursor/bootstrap-empty-dioxus-fullstack-project.mdc"
);

define_resource_tool!(
    CargoDependenciesGuideTool,
    "get_cargo_dependencies_guide",
    "Fetch Cargo dependencies guide resource content",
    "Cargo Dependencies Guide",
    "How to add dependencies to Cargo.toml",
    "resource://cargo-dependencies-guide",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/cargo-dependencies-guide.md"
);

define_resource_tool!(
    MySshReadmeTool,
    "get_my_ssh_readme",
    "Fetch my-ssh README resource content",
    "Ssh connections design library",
    "Async SSH helpers for commands, file transfer, and port forwarding.",
    "resource://my-ssh-readme",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/my-ssh/main/README.md"
);

define_resource_tool!(
    MyTcpSocketsReadmeTool,
    "get_my_tcp_sockets_readme",
    "Fetch my-tcp-sockets README resource content",
    "TcpSockets design library",
    "Async TCP server/client building blocks with ping/pong and TLS options",
    "resource://tcp-sockets-design-library",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/my-tcp-sockets/refs/heads/main/README.md"
);

define_resource_tool!(
    RustExtensionsReadmeTool,
    "get_rust_extensions_readme",
    "Fetch rust-extensions README resource content",
    "rust-extensions for each project",
    "Low-level utils, queues and other helpers to glue together Rust code",
    "resource://rust-extensions",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/rust-extensions/main/README.md"
);

define_resource_tool!(
    DioxusFullstackPatternsTool,
    "get_dioxus_fullstack_design_patterns",
    "Fetch Dioxus fullstack design patterns resource content",
    "Dioxus Fullstack Design Patterns",
    "Project playbook for dialogs, forms, lists, and server functions",
    "resource://dioxus-fullstack-design-patterns",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/development-mcp/main/docs/DIOXUS_FULLSTACK_DESIGN_PATTERS.md"
);

define_resource_tool!(
    MyNoSqlEntityPatternsTool,
    "get_my_no_sql_entity_patterns",
    "Fetch MyNoSql entity design patterns resource content",
    "MyNoSql Entity Design Patterns",
    "Design patterns for MyNoSql entities and enums",
    "resource://my-no-sql-entity-design-patterns",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/my-no-sql-sdk/refs/heads/main/MY_NO_SQL_ENTITY_DESIGN_PATTERNS.md"
);

define_resource_tool!(
    MyGrpcExtensionsReadmeTool,
    "get_my_grpc_extensions_readme",
    "Fetch my-grpc-extensions README resource content",
    "Grpc extensions",
    "Utilities and macros for building gRPC clients and servers",
    "resource://my-grpc-extensions.md",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/my-grpc-extensions/main/README.md"
);

define_resource_tool!(
    DioxusUtilsReadmeTool,
    "get_dioxus_utils_readme",
    "Fetch dioxus-utils README resource content",
    "dioxus-utils Usage Cases Guide",
    "Utilities for Dioxus apps: data state, dialogs, JS helpers",
    "resource://dioxus-utils-readme",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/dioxus-utils/refs/heads/main/README.md"
);

define_resource_tool!(
    CiUtilsReadmeTool,
    "get_ci_utils_readme",
    "Fetch ci-utils README resource content",
    "ci-utils for each project",
    "Utility crate for build-time helpers",
    "resource://ci-utils-readme",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/ci-utils/refs/heads/main/README.md"
);

define_resource_tool!(
    MyPostgresReadmeTool,
    "get_my_postgres_readme",
    "Fetch my-postgres README resource content",
    "Postgres Design Library",
    "Documentation for my-postgres library",
    "resource://my-postgres-readme",
    "text/markdown",
    "https://raw.githubusercontent.com/MyJetTools/my-postgres/refs/heads/main/README.md"
);
