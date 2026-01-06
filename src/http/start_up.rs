use std::{net::SocketAddr, sync::Arc};

use mcp_server_middleware::*;

use my_http_server::MyHttpServer;

use crate::{
    app::AppContext,
    mcp::{FlUrlResource, McpResource},
};

pub async fn start(app: &Arc<AppContext>) {
    let mut http_server = MyHttpServer::new(SocketAddr::from(([0, 0, 0, 0], 8000)));

    let mut mcp = McpMiddleware::new(
        "/",
        crate::app::APP_NAME,
        crate::app::APP_VERSION,
        "Provides access to devops tools",
    );

    mcp.register_resource(Arc::new(McpResource)).await;
    mcp.register_resource(Arc::new(FlUrlResource)).await;

    http_server.add_middleware(Arc::new(mcp));

    http_server.start(app.app_states.clone(), my_logger::LOGGER.clone());
}
