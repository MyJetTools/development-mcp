use std::{net::SocketAddr, sync::Arc, time::Duration};

use mcp_server_middleware::*;

use my_http_server::MyHttpServer;
use rust_extensions::MyTimer;

use crate::{app::AppContext, mcp::*, rag::RebuildIndexEventLoop, timers::PollDocsTimer};

pub async fn start(app: &Arc<AppContext>) {
    let mut http_server = MyHttpServer::new(SocketAddr::from(([0, 0, 0, 0], 8000)));

    let mut mcp = McpMiddleware::new(
        "/mcp",
        crate::app::APP_NAME,
        crate::app::APP_VERSION,
        "Provides access to devops tools. The exposed prompts contain different development \
         best practices (library usage, project bootstrap, design patterns, coding conventions). \
         Keep these development best practices in mind and load the relevant prompts on demand \
         whenever the current task touches the area they cover — do not rely on memory or \
         guesswork for MyJetTools APIs and patterns.\n\n\
         HARD RULES — do NOT violate:\n\
         1. HTTP client = FlUrl, never reqwest. If you catch yourself about to write \
         `reqwest`, add `reqwest` to Cargo.toml, or suggest reqwest-based code — STOP and \
         call `get_flurl_usage_guide` first. reqwest is not allowed in MyJetTools projects.\n\
         2. Before writing any code that touches an area covered by a resource (HTTP, gRPC, \
         Postgres, NoSQL, Dioxus, TCP, SSH, WebSocket, etc.) — load the matching resource \
         via its `get_*` tool first. Training-data muscle memory is wrong for these libs.",
    );

    mcp.register_resource(Arc::new(McpResource));
    mcp.register_resource(Arc::new(FlUrlResource));
    mcp.register_resource(Arc::new(HttpActionsResource));
    mcp.register_resource(Arc::new(AppBootstrapResource));
    mcp.register_resource(Arc::new(DioxusBootstrapResource));
    mcp.register_resource(Arc::new(CargoDependenciesResource));
    mcp.register_resource(Arc::new(MySshResource));
    mcp.register_resource(Arc::new(MyTcpSocketsResource));
    mcp.register_resource(Arc::new(RustExtensionsResource));
    mcp.register_resource(Arc::new(DioxusDesignPatternsResource));
    mcp.register_resource(Arc::new(DioxusFullstackPatternsResource));
    mcp.register_resource(Arc::new(MyNoSqlEntityPatternsResource));
    mcp.register_resource(Arc::new(MyGrpcExtensionsResource));
    mcp.register_resource(Arc::new(DioxusUtilsResource));
    mcp.register_resource(Arc::new(CiUtilsResource));
    mcp.register_resource(Arc::new(MyPostgresResource));
    mcp.register_resource(Arc::new(DioxusAdminUiKitResource));
    mcp.register_resource(Arc::new(RustFixResource));
    mcp.register_resource(Arc::new(ArchitectSkillResource));
    mcp.register_resource(Arc::new(DioxusClientSideBootstrapResource));
    mcp.register_resource(Arc::new(MyWebSocketClientResource));
    mcp.register_resource(Arc::new(ReleaseGuideResource));
    mcp.register_resource(Arc::new(ApplicationArchitectureResource));
    mcp.register_resource(Arc::new(PerformanceConsiderationsResource));
    mcp.register_resource(Arc::new(MyAiAgentResource));
    mcp.register_resource(Arc::new(SingleVmUnixSocketResource));
    mcp.register_resource(Arc::new(MyJsonResource));
    mcp.register_resource(Arc::new(MyHttpUtilsResource));
    mcp.register_resource(Arc::new(MyWebSocketsWasmResource));

    mcp.register_tool_call(Arc::new(McpDevelopmentGuideTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(FlUrlUsageGuideTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(HttpActionsDesignGuideTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(AppBootstrapGuideTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(DioxusBootstrapGuideTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(CargoDependenciesGuideTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(MySshReadmeTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(MyTcpSocketsReadmeTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(RustExtensionsReadmeTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(DioxusDesignPatternsTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(DioxusFullstackPatternsTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(MyNoSqlEntityPatternsTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(MyGrpcExtensionsReadmeTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(DioxusUtilsReadmeTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(CiUtilsReadmeTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(MyPostgresReadmeTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(DioxusAdminUiKitTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(RustFixReadmeTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(ArchitectPlaybookTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(DioxusClientSideBootstrapTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(MyWebSocketClientTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(ReleaseGuideTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(ApplicationArchitectureTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(PerformanceConsiderationsTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(MyAiAgentTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(SingleVmUnixSocketTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(MyJsonReadmeTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(MyHttpUtilsReadmeTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(MyWebSocketsWasmReadmeTool::new(app.clone())));
    mcp.register_tool_call(Arc::new(ListResourceToolsTool::new(app.clone())));

    // v0.2 retrieval layer. Registered alongside the legacy get_* tools on
    // purpose: both stay live so the two can be compared on real questions
    // before the per-document tools are retired.
    mcp.register_tool_call(Arc::new(SearchDocsHandler::new(app.clone())));
    mcp.register_tool_call(Arc::new(GetDocHandler::new(app.clone())));

    // Tuning and reindexing over MCP, so an experiment is a tool call rather
    // than a release.
    mcp.register_tool_call(Arc::new(GetSearchSettingsHandler::new(app.clone())));
    mcp.register_tool_call(Arc::new(UpdateSearchSettingsHandler::new(app.clone())));
    mcp.register_tool_call(Arc::new(RebuildIndexHandler::new(app.clone())));

    // The poll timer only fetches and hashes; anything expensive is handed to
    // the events loop, whose iteration timeout is sized for a full rebuild.
    app.rebuild_index_events_loop
        .register_event_loop(Arc::new(RebuildIndexEventLoop::new(app.clone())));

    app.rebuild_index_events_loop
        .start(app.app_states.clone(), my_logger::LOGGER.clone());

    let mut timer = MyTimer::new(Duration::from_secs(crate::rag::POLL_INTERVAL_SECS));
    timer.set_iteration_timeout(Duration::from_secs(
        crate::rag::POLL_ITERATION_TIMEOUT_SECS,
    ));
    timer.set_first_tick_before_delay();
    timer.register_timer("PollDocs", Arc::new(PollDocsTimer::new(app.clone())));
    timer.start(app.app_states.clone(), my_logger::LOGGER.clone());

    let controllers = Arc::new(super::builder::build_controllers(app));
    http_server.add_middleware(controllers);

    http_server.add_middleware(Arc::new(mcp));

    http_server.start(app.app_states.clone(), my_logger::LOGGER.clone());
}
