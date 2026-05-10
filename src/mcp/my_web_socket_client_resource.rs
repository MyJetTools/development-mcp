use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyWebSocketClientResource;

impl MyWebSocketClientResource {
    pub const FILENAME: &'static str = "my-web-socket-client-guide.md";
    pub const URL: &'static str = "https://raw.githubusercontent.com/MyJetTools/development-mcp/refs/heads/main/docs/my-web-socket-client-guide.md";
    pub const TOOL_FN: &'static str = "get_my_web_socket_client_guide";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch my-web-socket-client usage guide: auto-reconnect, heartbeat, callbacks, compression";
}

impl ResourceDefinition for MyWebSocketClientResource {
    const RESOURCE_URI: &'static str = "resource://my-web-socket-client-guide";
    const RESOURCE_NAME: &'static str = "WebSocket Client Guide";
    const DESCRIPTION: &'static str =
        "WebSocket client for non-WASM apps: auto-reconnect, heartbeat, callbacks, compression";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyWebSocketClientResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
