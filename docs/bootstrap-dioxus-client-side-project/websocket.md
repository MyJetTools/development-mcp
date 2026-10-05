# WebSocket Connection

Browser WebSocket API does **not** support custom HTTP headers. Pass the auth token as a query parameter.

```rust
fn kick_off_ws() {
    spawn(async move {
        let mut app_state = consume_context::<Signal<AppState>>();
        {
            let mut w = app_state.write();
            if w.ws_is_kicked_off {
                return;
            }
            w.ws_is_kicked_off = true;
        }

        let token = crate::web::storage::session::get_session_token().unwrap_or_default();

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

        match WebSocket::open(&ws_url) {
            Ok(mut ws) => {
                while let Some(msg) = ws.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            handle_ws_message(app_state, &text);
                        }
                        Ok(Message::Bytes(_)) => {}
                        Err(err) => {
                            dioxus_utils::console_log(format!("WS error: {:?}", err));
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                dioxus_utils::console_log(format!("Cannot connect to WS: {:?}", err));
            }
        }
    });
}
```

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
