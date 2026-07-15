use crate::mcp::scripts::load_resource_by_http;
use mcp_server_middleware::*;

pub struct MyWebSocketsWasmResource;

impl MyWebSocketsWasmResource {
    pub const FILENAME: &'static str = "my-web-sockets-wasm-readme.md";
    pub const URL: &'static str =
        "https://raw.githubusercontent.com/MyJetTools/my-web-sockets-wasm/refs/heads/main/README.md";
    pub const TOOL_FN: &'static str = "get_my_web_sockets_wasm_readme";
    pub const TOOL_DESCRIPTION: &'static str =
        "Fetch the my-web-sockets-wasm README: a lightweight, auto-reconnecting WebSocket client \
         for Dioxus browser/WASM clients. Load this whenever a dioxus-client app needs a web \
         socket. Wraps the browser WebSocket API with a reconnect loop and timeouts. Implement \
         the WsCallback trait (get_url, on_connected, on_data, on_disconnected), use WsConnection \
         (send_text / send_bytes, mark_initialized) and drive it with WebSocketClient \
         (new / start / stop). Handles cleanup, backpressure and Dioxus scheduler integration.";
}

impl ResourceDefinition for MyWebSocketsWasmResource {
    const RESOURCE_URI: &'static str = "resource://my-web-sockets-wasm-readme";
    const RESOURCE_NAME: &'static str = "my-web-sockets-wasm Dioxus WebSocket Client";
    const DESCRIPTION: &'static str =
        "If a dioxus-client (browser/WASM) application requires a web socket, use this crate. A \
         lightweight, auto-reconnecting WebSocket client for Dioxus: implement WsCallback, drive \
         it with WebSocketClient, send via WsConnection. Handles reconnection, timeouts, cleanup \
         and Dioxus scheduler integration.";
    const MIME_TYPE: &'static str = "text/markdown";
}

#[async_trait::async_trait]
impl McpResourceService for MyWebSocketsWasmResource {
    async fn read_resource(&self) -> Result<ResourceReadResult, String> {
        load_resource_by_http(Self::RESOURCE_URI, Self::MIME_TYPE, Self::URL).await
    }
}
