use std::{net::SocketAddr, sync::Arc};

use mcp_server_middleware::*;

use my_http_server::MyHttpServer;

use crate::{app::AppContext, mcp::*};

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
         guesswork for MyJetTools APIs and patterns.",
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
    mcp.register_tool_call(Arc::new(ListResourceToolsTool::new(app.clone())));

    let controllers = Arc::new(super::builder::build_controllers(app));
    http_server.add_middleware(controllers);

    http_server.add_middleware(Arc::new(mcp));

    http_server.start(app.app_states.clone(), my_logger::LOGGER.clone());
}
