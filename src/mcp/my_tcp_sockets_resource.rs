use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyTcpSocketsResource;

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
        const README_URL: &str =
            "https://raw.githubusercontent.com/MyJetTools/my-tcp-sockets/refs/heads/main/README.md";

        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, README_URL).await
    }
}
