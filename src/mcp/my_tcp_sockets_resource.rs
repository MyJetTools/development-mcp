use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyTcpSocketsResource;

impl MyTcpSocketsResource {
    pub const FILENAME: &'static str = "my-tcp-sockets-readme.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/my-tcp-sockets/refs/heads/main/README.md";
    pub const TOOL_FN: &'static str = "get_my_tcp_sockets_readme";
    pub const TOOL_DESCRIPTION: &'static str = "Fetch my-tcp-sockets README resource content";
}

impl ResourceDefinition for MyTcpSocketsResource {
    const RESOURCE_URI: &'static str = "resource://tcp-sockets-design-library";
    const RESOURCE_NAME: &'static str = "TcpSockets design library";
    const DESCRIPTION: &'static str =
        "Async TCP server/client building blocks with ping/pong and TLS options";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyTcpSocketsResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
