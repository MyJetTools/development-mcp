---
alwaysApply: false
---
# my-web-socket-client Usage Guide

WebSocket client library for non-WASM Rust applications (CLI tools, backend services, daemons). Provides automatic reconnection, heartbeat, and a callback-driven API.

For browser/WASM WebSocket connections see **bootstrap-dioxus-client-side-project.md** (uses `reqwasm`).

## Cargo.toml

```toml
[dependencies]
my-web-socket-client = { tag = "0.2.0", git = "https://github.com/MyJetTools/my-web-socket-client.git" }
rust-extensions = { tag = "0.1.5", git = "https://github.com/MyJetTools/rust-extensions.git" }
tokio = { version = "*", features = ["full"] }
async-trait = "*"
```

## Minimal Example

### 1. Implement `WsClientSettings` — provides the URL

```rust
struct MySettings;

#[async_trait::async_trait]
impl WsClientSettings for MySettings {
    async fn get_url(&self, _client_name: &str) -> Option<String> {
        Some("wss://example.com/ws".to_string())
    }
}
```

The URL is fetched on every reconnection attempt, so it can be dynamic (e.g. read from config or include a changing list of subscriptions).

### 2. Implement `WsCallback` — handle lifecycle events

```rust
struct MyWsCallback;

#[async_trait::async_trait]
impl WsCallback for MyWsCallback {
    async fn before_start_ws_connect(
        &self,
        _url: String,
    ) -> Result<StartWsConnectionDataToApply, String> {
        // Optionally override URL or add headers
        Ok(StartWsConnectionDataToApply::default())
    }

    async fn on_connected(&self, ws_connection: Arc<WsConnection>) {
        println!("Connected");
        // Send an initial message after connection
        ws_connection
            .send_message(Message::Text(r#"{"subscribe":"data"}"#.into()))
            .await;
    }

    async fn on_disconnected(&self, _ws_connection: Arc<WsConnection>) {
        println!("Disconnected");
    }

    async fn on_data(&self, ws_connection: Arc<WsConnection>, data: Message) {
        match data {
            Message::Text(text) => {
                println!("Received: {}", text);
            }
            Message::Binary(bin) => {
                println!("Binary: {} bytes", bin.len());
            }
            Message::Ping(msg) => {
                ws_connection.send_message(Message::Pong(msg)).await;
            }
            _ => {}
        }
    }
}
```

### 3. Implement `Logger` — required by `WebSocketClient`

If using `service-sdk`, pass `service_sdk::my_logger::LOGGER.clone()`. Otherwise implement a simple console logger:

```rust
use std::collections::HashMap;

struct ConsoleLogger;

impl rust_extensions::Logger for ConsoleLogger {
    fn write_info(&self, process: String, message: String, _ctx: Option<HashMap<String, String>>) {
        println!("[INFO] {}: {}", process, message);
    }
    fn write_warning(&self, process: String, message: String, _ctx: Option<HashMap<String, String>>) {
        println!("[WARN] {}: {}", process, message);
    }
    fn write_error(&self, process: String, message: String, _ctx: Option<HashMap<String, String>>) {
        eprintln!("[ERROR] {}: {}", process, message);
    }
    fn write_fatal_error(&self, process: String, message: String, _ctx: Option<HashMap<String, String>>) {
        eprintln!("[FATAL] {}: {}", process, message);
    }
    fn write_debug_info(&self, process: String, message: String, _ctx: Option<HashMap<String, String>>) {
        println!("[DEBUG] {}: {}", process, message);
    }
}
```

### 4. Wire everything together in `main()`

```rust
use std::sync::Arc;
use my_web_socket_client::*;
use my_web_socket_client::hyper_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() {
    // Required for TLS (wss://) connections
    my_web_socket_client::my_tls::install_default_crypto_providers();

    let client = WebSocketClient::new(
        Arc::new("MyClient".into()),   // client name (for logging)
        Arc::new(MySettings),           // WsClientSettings impl
        Arc::new(ConsoleLogger),        // Logger impl
    );

    // Start with optional heartbeat message
    // Pass None if server doesn't need periodic pings
    client.start(
        Some(Message::Text("ping".into())),  // sent every ~3s
        Arc::new(MyWsCallback),
    );

    // Keep process alive
    tokio::signal::ctrl_c().await.unwrap();
}
```

## Key Concepts

### Automatic Reconnection

The client automatically reconnects on connection loss. Default timeouts:

| Parameter | Default | Description |
|---|---|---|
| `reconnect_timeout` | 3s | Wait before each reconnection attempt |
| `ping_interval` | 3s | How often to send the heartbeat message |
| `disconnect_timeout` | 9s | If no message received within this period, disconnect |
| `send_timeout` | 30s | Max time to wait for a send to complete |

### Heartbeat / Ping

The first argument to `client.start()` is `Option<Message>`:

- `Some(Message::Text("7".into()))` — sends this text message every `ping_interval` (3s). Use for servers that expect a custom heartbeat.
- `Some(Message::Ping(vec![]))` — sends a standard WebSocket ping frame.
- `None` — no heartbeat; handle manually if needed.

### Sending Messages

Use `WsConnection` received in callbacks:

```rust
// Single message
ws_connection.send_message(Message::Text("hello".into())).await;

// Multiple messages
ws_connection.send_messages(
    vec![
        Message::Text("msg1".into()),
        Message::Text("msg2".into()),
    ].into_iter()
).await;
```

### Adding Custom Headers

Override `before_start_ws_connect` to inject headers (e.g. auth tokens):

```rust
async fn before_start_ws_connect(
    &self,
    _url: String,
) -> Result<StartWsConnectionDataToApply, String> {
    Ok(StartWsConnectionDataToApply {
        headers: Some(vec![
            ("Authorization".into(), "Bearer my-token".into()),
        ]),
        url: None,
    })
}
```

### Dynamic URL

Return different URLs from `get_url()` based on runtime state. This is called on every reconnection attempt:

```rust
async fn get_url(&self, _client_name: &str) -> Option<String> {
    let instruments = self.get_instruments().await;
    let streams = instruments.iter()
        .map(|i| format!("{}@ticker", i.to_lowercase()))
        .collect::<Vec<_>>()
        .join("/");
    Some(format!("wss://stream.example.com?streams={}", streams))
}
```

### Handling Compressed Binary Messages (per-message deflate)

If the server sends DEFLATE-compressed binary frames, use `flate2` for stateful decompression:

```toml
[dependencies]
flate2 = "*"
```

```rust
use tokio::sync::Mutex;

struct MyWsCallback {
    decompressor: Mutex<flate2::Decompress>,
}

impl MyWsCallback {
    fn new() -> Self {
        Self {
            decompressor: Mutex::new(flate2::Decompress::new(false)), // raw deflate
        }
    }

    async fn decompress_binary(&self, data: &[u8]) -> Option<String> {
        let mut decompressor = self.decompressor.lock().await;

        let mut input = data.to_vec();
        input.extend_from_slice(&[0x00, 0x00, 0xFF, 0xFF]); // sync flush marker

        let mut output = vec![0u8; 32768];
        let before_out = decompressor.total_out();

        match decompressor.decompress(&input, &mut output, flate2::FlushDecompress::Sync) {
            Ok(_) => {
                let written = (decompressor.total_out() - before_out) as usize;
                Some(String::from_utf8_lossy(&output[..written]).to_string())
            }
            Err(e) => {
                eprintln!("Decompress error: {}", e);
                None
            }
        }
    }
}
```

**Important:** The `Decompress` instance must be kept alive across messages (stateful). Wrap in `Mutex` inside the callback struct.

## Integration with service-sdk

In projects using `service-sdk`, the logger is already available:

```rust
let client = WebSocketClient::new(
    Arc::new("BinanceWs".into()),
    app.clone(),                              // AppContext implements WsClientSettings
    service_sdk::my_logger::LOGGER.clone(),   // shared logger
);
client.start(None, Arc::new(MyCallback::new(app)));
```

Implement `WsClientSettings` on `AppContext` so the URL is resolved from the application's configuration or NoSQL state.

## Re-exports

The library re-exports these crates for convenience:

- `my_web_socket_client::hyper_tungstenite` — access to `tungstenite::Message` enum
- `my_web_socket_client::my_tls` — TLS utilities, `install_default_crypto_providers()`
- `my_web_socket_client::url_utils` — URL parsing helpers