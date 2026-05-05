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

    mcp.register_resource(Arc::new(McpResource)).await;
    mcp.register_resource(Arc::new(FlUrlResource)).await;
    mcp.register_resource(Arc::new(HttpActionsResource)).await;
    mcp.register_resource(Arc::new(AppBootstrapResource)).await;
    mcp.register_resource(Arc::new(DioxusBootstrapResource))
        .await;
    mcp.register_resource(Arc::new(CargoDependenciesResource))
        .await;
    mcp.register_resource(Arc::new(MySshResource)).await;
    mcp.register_resource(Arc::new(MyTcpSocketsResource)).await;
    mcp.register_resource(Arc::new(RustExtensionsResource))
        .await;
    mcp.register_resource(Arc::new(DioxusFullstackPatternsResource))
        .await;
    mcp.register_resource(Arc::new(MyNoSqlEntityPatternsResource))
        .await;
    mcp.register_resource(Arc::new(MyGrpcExtensionsResource))
        .await;
    mcp.register_resource(Arc::new(DioxusUtilsResource)).await;
    mcp.register_resource(Arc::new(CiUtilsResource)).await;
    mcp.register_resource(Arc::new(MyPostgresResource)).await;
    mcp.register_resource(Arc::new(DioxusAdminUiKitResource)).await;
    mcp.register_resource(Arc::new(RustFixResource)).await;

    mcp.register_tool_call(Arc::new(McpDevelopmentGuideTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(FlUrlUsageGuideTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(HttpActionsDesignGuideTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(AppBootstrapGuideTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(DioxusBootstrapGuideTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(CargoDependenciesGuideTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(MySshReadmeTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(MyTcpSocketsReadmeTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(RustExtensionsReadmeTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(DioxusFullstackPatternsTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(MyNoSqlEntityPatternsTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(MyGrpcExtensionsReadmeTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(DioxusUtilsReadmeTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(CiUtilsReadmeTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(MyPostgresReadmeTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(DioxusAdminUiKitTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(RustFixReadmeTool::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(ListResourceToolsTool::new(app.clone())))
        .await;

    let controllers = Arc::new(super::builder::build_controllers(app));
    http_server.add_middleware(controllers);

    http_server.add_middleware(Arc::new(mcp));

    http_server.start(app.app_states.clone(), my_logger::LOGGER.clone());
}
