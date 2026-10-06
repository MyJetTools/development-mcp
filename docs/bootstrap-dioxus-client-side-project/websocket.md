# WebSocket Connection

A browser WebSocket goes through **`my-web-sockets-wasm`** — it reconnects by itself and drops a socket that went silent. Implement `WsCallback`, drive it with `WebSocketClient`. Read its README (`get_my_web_sockets_wasm_readme`) before writing the handler — never write a connect / read loop by hand.

Browser WebSocket API does **not** support custom HTTP headers. Pass the auth token as a query parameter.

## Starting and stopping — `WebSocketClient`

```rust
// main.rs
// Called on every render of `App` — the client is created once, inside `use_hook`
fn kick_off_ws(app_state: Signal<AppState>, with_ws: bool) {
    let client = use_hook(move || {
        with_ws.then(|| {
            let client = Rc::new(WebSocketClient::new());
            client.start(Rc::new(WsHandler { app_state }));
            client
        })
    });

    // The socket lives exactly as long as this `App` is mounted
    use_drop(move || {
        if let Some(client) = client {
            client.stop();
        }
    });
}
```

- `start()` spawns the reconnect loop on the Dioxus runtime — call it only inside a component or a spawned task.
- `kick_off_ws` uses hooks, so `App` calls it unconditionally and passes `with_ws` in — never `if with_ws { kick_off_ws(..) }`.
- No "already started" flag in the state: `use_hook` is what makes it one client per mounted `App`.

## The handler — `WsCallback`

```rust
// main.rs
struct WsHandler {
    app_state: Signal<AppState>,
}

impl WsCallback for WsHandler {
    // Called on EVERY (re)connect attempt. `None` — no token yet: the client stays idle and asks again
    fn get_url(&self) -> Option<String> {
        let token = crate::web::storage::session::get_session_token()?;

        let settings = dioxus_utils::js::GlobalAppSettings::new();
        let origin = settings.get_origin();

        let ws_url = if origin.starts_with("https") {
            origin.replacen("https", "wss", 1)
        } else {
            origin.replacen("http", "ws", 1)
        };

        let ws_url = if ws_url.ends_with('/') {
            format!("{}ws?token={}", ws_url, token)
        } else {
            format!("{}/ws?token={}", ws_url, token)
        };

        Some(ws_url)
    }

    async fn on_connected(&self, conn: Rc<WsConnection>) -> Result<(), ()> {
        // MANDATORY — without it the socket is dropped and reopened every ~13 seconds.
        // This server sends no init frame, so the connection is ready as soon as it is open
        conn.mark_initialized();
        Ok(())
    }

    async fn on_data(&self, _conn: Rc<WsConnection>, msg: Message) -> Result<(), ()> {
        match msg {
            Message::Text(text) => handle_ws_message(self.app_state, &text),
            Message::Bytes(_) => {}
        }
        Ok(())
    }

    async fn on_disconnected(&self, _conn: Rc<WsConnection>) {
        dioxus_utils::console_log("WS: disconnected");
    }
}
```

Rules:

- **`conn.mark_initialized()` is mandatory** and is never called for you. No init frame in the protocol — call it at the end of `on_connected`; the server sends one — call it in `on_data` when that frame arrives.
- **The server must send a frame at least every 10 seconds.** After `mark_initialized()` 10 seconds without an inbound frame drop the socket and reconnect; the crate has no heartbeat of its own and the timeouts are not configurable. A server that can stay silent longer sends a keep-alive frame — give it its own `event_id` and a match arm that does nothing.
- **The URL and the token are read in `get_url()` only** — it runs before every attempt, so a reconnect goes out with the token that is in storage now.
- **`Ok(())` keeps the connection; `Err(())`** from `on_connected` / `on_data` disconnects and reconnects.
- **Keep `on_data` fast** — it is awaited inline in the read loop. Heavy work goes into a separate `spawn`.

## Sending a frame

`WsConnection` is handed to every callback:

```rust
// Client frames use the same `event_id:json_payload` format; the event ids are the app's own
if let Err(err) = conn.send_text("subscribe-instruments:{}") {
    dioxus_utils::console_log(format!("WS: send failed: {}", err));
    return Err(()); // disconnect + reconnect
}
```

A frame the server needs on every connection is sent from `on_connected` — a reconnect is a new socket. Sending from UI code, outside the callbacks: see **Sending messages** in the README.

## WebSocket Message Wire Format

Server sends messages in format `event_id:json_payload`. Client parses them:

```rust
// models/ws.rs
pub enum ServerWsMessage {
    Instruments(Vec<InstrumentWsModel>),
    InvalidToken,
    Close,
    Unknown(String),
}

impl ServerWsMessage {
    pub fn parse(raw: &str) -> Self {
        let Some(colon_pos) = raw.find(':') else {
            return Self::Unknown(raw.to_string());
        };
        let msg_id = &raw[..colon_pos];
        let payload = &raw[colon_pos + 1..];

        match msg_id {
            "instruments" => {
                match serde_json::from_str::<InstrumentsWsContract>(payload) {
                    Ok(contract) => Self::Instruments(contract.instruments),
                    Err(_) => Self::Unknown(raw.to_string()),
                }
            }
            "invalid-token" => Self::InvalidToken,
            "close" => Self::Close,
            _ => Self::Unknown(msg_id.to_string()),
        }
    }
}
```

## Handling WS Messages

```rust
fn handle_ws_message(mut app_state: Signal<AppState>, raw: &str) {
    match ServerWsMessage::parse(raw) {
        ServerWsMessage::Instruments(instruments) => {
            app_state.write().set_instruments(instruments);
        }
        ServerWsMessage::InvalidToken => {
            crate::web::storage::session::clear_tokens();
            navigator().push(AppRoute::Login {});
        }
        ServerWsMessage::Close => {
            dioxus_utils::console_log("WS: server closed connection");
        }
        ServerWsMessage::Unknown(msg_id) => {
            dioxus_utils::console_log(format!("WS: unknown message: {}", msg_id));
        }
    }
}
```

After `clear_tokens()` `get_url()` answers `None`, so nothing reconnects with a dead token; navigating to a pre-auth page unmounts this `App`, and `use_drop` stops the client.
