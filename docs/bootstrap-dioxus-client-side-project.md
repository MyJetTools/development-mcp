---
alwaysApply: false
---
# Bootstrap Dioxus Client-Side Project

This document describes how to bootstrap a new Dioxus **client-side only** web application (no server component). The app compiles to WASM and runs entirely in the browser. API calls go to a separate backend service. WebSocket connections are made directly from the browser.

For component design patterns (state management, folder structure, DataState, dialogs, etc.) see **dioxus-design-patterns.md**.

## When to Use Client-Side vs Fullstack

| Use case | Project type |
|---|---|
| Admin panel served by its own backend | Fullstack (`dioxus/fullstack`) |
| Trading terminal, SPA calling external API | **Client-side** (`dioxus/web`) |
| Static site with no server functions | **Client-side** (`dioxus/web`) |

Key difference: client-side project has **no `#[server]` functions**, no `server` feature, no `src/server/` module. All data comes from HTTP API calls (`flurl`) or WebSocket (`reqwasm`).

## Project Structure

```
project-root/
├── Cargo.toml
├── Dioxus.toml
├── build.rs
├── build.py
├── Dockerfile
├── css/
│   ├── 01-common.css
│   ├── 02-full-screen.css
│   ├── 03-inputs.css
│   ├── 04-buttons.css
│   ├── 05-layout.css
│   ├── 06-loading.css
│   └── 99-desktop.css
├── public/
│   ├── favicon.ico
│   └── assets/
│       └── app.css          ← compiled by build.rs from css/ files
└── src/
    ├── main.rs
    ├── api/
    │   └── mod.rs           ← HTTP API calls (FlUrl) + shared response helpers
    ├── components/
    │   └── mod.rs           ← reusable UI components
    ├── dialogs/
    │   ├── mod.rs
    │   ├── dialog_state.rs
    │   └── render.rs
    ├── icons/
    │   └── mod.rs
    ├── models/
    │   └── mod.rs           ← client view-state + WS message parser (wire models live in rest-api-shared)
    ├── states/
    │   ├── mod.rs
    │   ├── app_state.rs
    │   └── location.rs
    ├── templates/
    │   ├── mod.rs
    │   ├── content_panel.rs
    │   ├── full_screen_form.rs
    │   └── menu_panel.rs
    ├── views/
    │   └── mod.rs           ← page views, one folder per page
    └── web/
        └── storage/
            └── session.rs   ← localStorage helpers
```

## Cargo.toml

```toml
[package]
name = "your-project-name"
version = "0.1.0"
edition = "2024"

[features]
default = ["web"]
web = ["dioxus/web"]

[dependencies]
dioxus = { version = "0.7", features = ["router"] }
dioxus-utils = { tag = "0.7.0", git = "https://github.com/MyJetTools/dioxus-utils.git", features = [
    "web",
] }

# Shared wire models — reused verbatim by the REST-API server and this client.
# On the client the crate is pulled in WITHOUT the "server" feature.
rest-api-shared = { path = "../rest-api-shared" }
# HTTP client; compiles to the browser fetch API under wasm and resolves relative "/api/..." URLs.
flurl = { tag = "0.7.0", git = "https://github.com/MyJetTools/fl-url.git" }
# Provides the THttpRequestBuilder bound named by the generic authed-POST helper.
my-http-utils = { tag = "0.1.0", git = "https://github.com/MyJetTools/my-http-utils.git" }

reqwasm = "*"
futures = { version = "*" }

serde_json = { version = "*" }
serde = { version = "*", features = ["derive"] }

web-sys = { version = "*", features = ["Storage"] }
js-sys = { version = "*" }

[build-dependencies]
ci-utils = { git = "https://github.com/MyJetTools/ci-utils.git", tag = "0.1.3" }
```

Key differences from fullstack:
- **No** `dioxus/fullstack` or `dioxus/server` features
- **No** `tokio` dependency
- `dioxus-utils` uses `"web"` feature, not `"fullstack"`
- `flurl` for HTTP API calls (compiles to the WASM fetch API; resolves relative `/api/...` URLs)
- `rest-api-shared` (path dep, **without** the `server` feature) for wire models shared with the REST-API server; `my-http-utils` names the `THttpRequestBuilder` bound used by the generic authed-POST helper
- `reqwasm` for WebSocket connections from the browser

## Dioxus.toml

```toml
[application]
name = "your-project-name"
default_platform = "web"
asset_dir = "assets"

[web.app]
title = "Your Project Title"

[web.watcher]
reload_html = false
index_on_404 = true

[web.resource]
style = ["/assets/app.css"]
script = []

[web.resource.dev]
script = []
```

## build.rs — CSS Compilation

```rust
fn main() {
    ci_utils::css::CssCompiler::new("./css")
        .add_file("01-common.css")
        .add_file("02-full-screen.css")
        .add_file("03-inputs.css")
        .add_file("04-buttons.css")
        .add_file("05-layout.css")
        .add_file("06-loading.css")
        .add_file("99-desktop.css")
        .compile("./public/assets/app.css");
}
```

CSS source files live in `css/`, numbered for ordering. `build.rs` compiles them into a single `public/assets/app.css`.

**NEVER** edit `public/assets/app.css` directly — it is auto-generated on every build and all manual changes will be lost. Always add or edit CSS in the `css/` directory. To add new styles, create a new numbered file (e.g. `07-toast.css`) and register it in `build.rs`.

## main.rs — Routing and WebSocket

Client-side apps typically have **pre-auth pages** (login, code verification) and **post-auth pages** (dashboard, etc.). The `App` component decides whether to start a WebSocket connection based on the current route.

```rust
use dioxus::prelude::*;
use futures::StreamExt;
use reqwasm::websocket::{futures::WebSocket, Message};

mod api;
mod components;
mod dialogs;
mod icons;
mod models;
mod states;
mod templates;
mod views;
mod web;

use models::ServerWsMessage;
use states::*;

#[derive(Routable, PartialEq, Clone)]
enum AppRoute {
    // Pre-auth
    #[route("/")]
    Login {},
    #[route("/enter-code?:email")]
    EnterCode { email: String },

    // Post-auth
    #[route("/dashboard")]
    Dashboard {},
    #[route("/logout")]
    Logout {},
}

fn main() {
    dioxus::LaunchBuilder::new().launch(|| {
        rsx! {
            document::Link { rel: "icon", href: asset!("/public/favicon.ico") }
            document::Meta {
                name: "viewport",
                content: "width=device-width, initial-scale=1.0, maximum-scale=1.0, minimum-scale=1.0, user-scalable=no",
            }
            Router::<AppRoute> {}
        }
    });
}

// Each route component provides its own LocationState and decides if WS is needed
#[component]
fn Login() -> Element {
    use_context_provider(|| Signal::new(LocationState::Login));
    rsx! { App { with_ws: false } }
}

#[component]
fn EnterCode(email: String) -> Element {
    if email.is_empty() {
        navigator().push(AppRoute::Login {});
        return rsx! {};
    }
    use_context_provider(|| Signal::new(LocationState::EnterCode(email)));
    rsx! { App { with_ws: false } }
}

#[component]
fn Dashboard() -> Element {
    use_context_provider(|| Signal::new(LocationState::Dashboard));
    rsx! { App { with_ws: true } }
}

#[component]
fn Logout() -> Element {
    use_context_provider(|| Signal::new(LocationState::Logout));
    rsx! { App { with_ws: false } }
}

#[component]
fn App(with_ws: bool) -> Element {
    use crate::dialogs::*;

    use_context_provider(|| Signal::new(AppState::default()));

    let app_state = consume_context::<Signal<AppState>>();
    let app_state_ra = app_state.read();

    if with_ws && !app_state_ra.ws_is_kicked_off {
        kick_off_ws();
    }

    let location_state = consume_context::<Signal<LocationState>>();
    let location = { location_state.read().clone() };

    let main_content = match location {
        LocationState::Login => rsx! {
            crate::views::login::RenderLogin {}
        },
        LocationState::EnterCode(email) => rsx! {
            crate::views::enter_code::RenderEnterCode { email }
        },
        LocationState::Dashboard => rsx! {
            crate::views::dashboard::RenderDashboard {}
        },
        LocationState::Logout => rsx! {
            crate::views::logout::RenderLogout {}
        },
    };

    rsx! {
        div { id: "main-panel", {main_content} }
        RenderDialog {}
    }
}
```

## WebSocket Connection

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

### WebSocket Message Wire Format

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

### Handling WS Messages

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

## States

### AppState — global application state

```rust
// states/app_state.rs
use crate::dialogs::DialogState;
use crate::models::InstrumentWsModel;

#[derive(Default)]
pub struct AppState {
    dialog_state: DialogState,
    pub ws_is_kicked_off: bool,
    pub instruments: Vec<InstrumentWsModel>,
}

impl AppState {
    pub fn get_dialog_state(&self) -> &DialogState {
        &self.dialog_state
    }

    pub fn set_instruments(&mut self, instruments: Vec<InstrumentWsModel>) {
        self.instruments = instruments;
    }
}
```

### LocationState — tracks current page

```rust
// states/location.rs
#[derive(Clone)]
pub enum LocationState {
    Login,
    EnterCode(String),
    Dashboard,
    Logout,
}
```

## Shared Wire Models (`rest-api-shared`)

Every model that crosses the wire lives in a **separate crate `rest-api-shared`** and is reused
**verbatim** by both the REST-API server and the wasm client. One definition — both ends. This kills
request/response drift: the client cannot serialize a shape the server does not accept.

- **Request models** derive `MyHttpInput`:

  ```rust
  // rest-api-shared/src/auth.rs
  use my_http_utils::macros::MyHttpInput;

  #[derive(MyHttpInput)]
  pub struct SendCodeRequest {
      #[http_body(name = "email", description = "")]
      pub email: String,
  }

  #[derive(MyHttpInput)]
  pub struct VerifyCodeRequest {
      #[http_body(name = "email", description = "")]
      pub email: String,
      #[http_body(name = "code", description = "")]
      pub code: String,
  }
  ```

- **Response models** derive `Serialize, Deserialize, MyHttpObjectStructure, Clone, Debug, PartialEq`:

  ```rust
  use serde::{Deserialize, Serialize};
  use my_http_utils::macros::MyHttpObjectStructure;

  #[derive(Serialize, Deserialize, MyHttpObjectStructure, Clone, Debug, PartialEq)]
  pub struct SessionTokenResponse {
      pub token: String,
      #[serde(rename = "refreshToken")]
      pub refresh_token: String,
  }

  #[derive(Serialize, Deserialize, MyHttpObjectStructure, Clone, Debug, PartialEq)]
  pub struct UserInfoResponse {
      pub email: String,
      pub first_name: String,
      pub last_name: String,
  }
  ```

### The crate is wasm-clean

`rest-api-shared` depends **only on `my-http-utils`** (never on `my-http-server`), so it compiles to
`wasm32`. A `server` feature turns on the server-side request parsing that only the REST-API service
needs:

```toml
# rest-api-shared/Cargo.toml
[dependencies]
my-http-utils = { tag = "0.1.0", git = "https://github.com/MyJetTools/my-http-utils.git" }
serde = { version = "*", features = ["derive"] }

[features]
server = ["my-http-utils/server"]
```

- **Server** depends on it **with** `features = ["server"]` — it needs to parse the incoming request.
- **Client** depends on it **without** any feature — it only needs the schema plus the FlUrl request
  builder (the `THttpRequestBuilder` impl used by `execute_request`).

### Models stay pure data

Shared types carry **no methods and no behaviour** — only fields. Presentational or derived helpers
belong to the **consumer** as an extension trait, so the wire contract never picks up client-only logic:

```rust
// client: src/models/user_info_ext.rs
use rest_api_shared::auth::UserInfoResponse as UserInfo;

pub trait UserInfoExt {
    fn full_name(&self) -> String;
}

impl UserInfoExt for UserInfo {
    fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}
```

When a shared type reads better under a local name, re-export it:

```rust
pub use rest_api_shared::auth::UserInfoResponse as UserInfo;
```

> **NEVER** put `///` doc-comments on the fields of a struct that derives `MyHttpInput` or
> `MyHttpObjectStructure` — the proc-macro panics. Document the struct itself instead, or use the
> `description = "..."` attribute argument on the field.

### What stays in `src/models/`

Only **client view-state** that never crosses the wire — dialog state, form buffers, UI toggles,
the `RequestError` type, WS message enums. Anything that goes to or comes from the REST API belongs in
`rest-api-shared`, not here.

## API Calls

API calls use **FlUrl**, which compiles to the browser `fetch` API under wasm. Two rules:

1. **Use relative URLs.** Pass `"/api/..."` straight into `FlUrl::new(...)` — the wasm backend resolves
   it against the current page origin for you. **Do not** compute a base URL; there is no
   `get_base_url()`. (`GlobalAppSettings::get_origin()` is still needed for the **WebSocket** URL only —
   the browser WebSocket API requires an absolute `ws`/`wss` URL.)
2. **Build the request from a shared `MyHttpInput` model** via `.execute_request(HttpVerb::X, model)`.
   Never hand-assemble a JSON body. For a request that has no parameters, pass `EmptyRequestModel`
   instead of declaring an empty model.

### Centralized response handling (`src/api/mod.rs`)

Status/error handling lives in **three helpers**, each taking the raw `Result<FlUrlResponse, FlUrlError>`
straight off FlUrl and decoding status + error **once**. Every API function forwards FlUrl's `Result`
into a helper — there is no repeated `if !is_success(...) { return Err(...) }` in each method.

```rust
// api/mod.rs
mod auth;
pub use auth::*;

use flurl::{FlUrl, FlUrlError, FlUrlResponse, HttpVerb};
use my_http_utils::THttpRequestBuilder;
use serde::de::DeserializeOwned;

use crate::models::RequestError;

fn is_success(status: u16) -> bool {
    (200..300).contains(&status)
}

async fn read_error_body(response: &mut FlUrlResponse) -> RequestError {
    let message = response
        .get_body_as_str()
        .await
        .map(|body| body.to_string())
        .unwrap_or_else(|err| err.to_string());

    RequestError { message }
}

/// 2xx → deserialize the body into `T`; any other status → `Err` carrying the response body.
pub async fn handle_http_response<T: DeserializeOwned>(
    response: Result<FlUrlResponse, FlUrlError>,
) -> Result<T, RequestError> {
    let mut response = response?;

    if is_success(response.get_status_code()) {
        return Ok(response.get_json().await?);
    }

    Err(read_error_body(&mut response).await)
}

/// Endpoints with no response body: 2xx → `Ok(())`, otherwise `Err` carrying the response body.
pub async fn handle_http_empty(
    response: Result<FlUrlResponse, FlUrlError>,
) -> Result<(), RequestError> {
    let mut response = response?;

    if is_success(response.get_status_code()) {
        return Ok(());
    }

    Err(read_error_body(&mut response).await)
}

/// Like `handle_http_response`, but `401`/`403` map to `Ok(None)` (not logged in / no rights).
pub async fn handle_http_response_opt<T: DeserializeOwned>(
    response: Result<FlUrlResponse, FlUrlError>,
) -> Result<Option<T>, RequestError> {
    let mut response = response?;

    let status = response.get_status_code();

    if status == 401 || status == 403 {
        return Ok(None);
    }

    if is_success(status) {
        return Ok(Some(response.get_json().await?));
    }

    Err(read_error_body(&mut response).await)
}

/// Authenticated POST: attaches the session token, then executes any shared `MyHttpInput` model.
/// The `THttpRequestBuilder` bound (from `my-http-utils`) is what lets the model drive the request.
async fn authed_post<TModel: THttpRequestBuilder>(
    url: &str,
    model: TModel,
) -> Result<FlUrlResponse, FlUrlError> {
    let token = crate::web::storage::session::get_session_token().unwrap_or_default();

    FlUrl::new(url)
        .with_header("Authorization", format!("Bearer {}", token))
        .execute_request(HttpVerb::Post, model)
        .await
}
```

### Example: `api/auth.rs`

Each call builds a shared model, hands FlUrl's `Result` to a helper, and returns. No status checks,
no manual JSON, no base URL.

```rust
// api/auth.rs
use flurl::{EmptyRequestModel, FlUrl, HttpVerb};
use rest_api_shared::auth::*;

use crate::models::RequestError;

use super::{authed_post, handle_http_empty, handle_http_response, handle_http_response_opt};

// POST with a body, no response payload.
pub async fn send_code(email: &str) -> Result<(), RequestError> {
    let response = FlUrl::new("/api/auth/v1/SendCode")
        .execute_request(HttpVerb::Post, SendCodeRequest { email: email.to_string() })
        .await;

    handle_http_empty(response).await
}

// POST with a body and a typed response payload.
pub async fn verify_code(email: &str, code: &str) -> Result<SessionTokenResponse, RequestError> {
    let request = VerifyCodeRequest {
        email: email.to_string(),
        code: code.to_string(),
    };

    let response = FlUrl::new("/api/auth/v1/VerifyCode")
        .execute_request(HttpVerb::Post, request)
        .await;

    handle_http_response(response).await
}

// Authenticated GET with no parameters — `EmptyRequestModel`, 401/403 → `Ok(None)`.
pub async fn get_user_info() -> Result<Option<UserInfoResponse>, RequestError> {
    let token = crate::web::storage::session::get_session_token().unwrap_or_default();

    let response = FlUrl::new("/api/auth/v1/UserInfo")
        .with_header("Authorization", format!("Bearer {}", token))
        .execute_request(HttpVerb::Get, EmptyRequestModel)
        .await;

    handle_http_response_opt(response).await
}

// Authenticated POST with no parameters — reuses the generic helper + `EmptyRequestModel`.
pub async fn logout() -> Result<(), RequestError> {
    let response = authed_post("/api/auth/v1/Logout", EmptyRequestModel).await;

    handle_http_empty(response).await
}
```

## Models

`src/models/` holds **only client-side view-state** — no wire models (those live in `rest-api-shared`,
see [Shared Wire Models](#shared-wire-models-rest-api-shared)). One file per type; `mod.rs` uses the
standard `mod x; pub use x::*;` pattern.

`RequestError` is the client-side error type every API function returns. It converts from both FlUrl
transport errors and `serde_json` errors, so the helpers in `api/mod.rs` can use `?` freely.

```rust
// models/request_error.rs
use std::fmt;

pub struct RequestError {
    pub message: String,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<flurl::FlUrlError> for RequestError {
    fn from(err: flurl::FlUrlError) -> Self {
        Self { message: err.to_string() }
    }
}

// get_json() / try_as_json() surface serde_json::Error, so RequestError converts from it too.
impl From<serde_json::Error> for RequestError {
    fn from(err: serde_json::Error) -> Self {
        Self { message: err.to_string() }
    }
}
```

Client view-state — e.g. a form buffer — is a plain struct, and unlike the pure shared wire models it
may carry behaviour:

```rust
// models/login_form.rs
#[derive(Default, Clone)]
pub struct LoginForm {
    pub email: String,
    pub code: String,
}

impl LoginForm {
    pub fn is_email_valid(&self) -> bool {
        self.email.contains('@')
    }
}
```

## localStorage Helpers

```rust
// web/storage/session.rs
const SESSION_TOKEN_KEY: &str = "mt_session_token";
const REFRESH_TOKEN_KEY: &str = "mt_refresh_token";

fn get_local_storage() -> web_sys::Storage {
    web_sys::window()
        .expect("no window")
        .local_storage()
        .expect("no local storage")
        .expect("local storage is None")
}

pub fn save_tokens(session_token: &str, refresh_token: &str) {
    let storage = get_local_storage();
    storage.set_item(SESSION_TOKEN_KEY, session_token).expect("failed to save session token");
    storage.set_item(REFRESH_TOKEN_KEY, refresh_token).expect("failed to save refresh token");
}

pub fn get_session_token() -> Option<String> {
    get_local_storage()
        .get_item(SESSION_TOKEN_KEY)
        .expect("failed to read session token")
}

pub fn clear_tokens() {
    let storage = get_local_storage();
    let _ = storage.remove_item(SESSION_TOKEN_KEY);
    let _ = storage.remove_item(REFRESH_TOKEN_KEY);
}
```

## Templates

Templates provide layout wrappers for pages.

### Full Screen Form (pre-auth pages)

```rust
// templates/full_screen_form.rs
use dioxus::prelude::*;

pub fn full_screen_form(content: Element) -> Element {
    rsx! {
        div { class: "full-screen",
            div { class: "full-screen-form",
                {content}
            }
        }
    }
}
```

### Content Panel (post-auth pages with sidebar)

```rust
// templates/content_panel.rs
use dioxus::prelude::*;

#[component]
pub fn ContentPanel(content: Element) -> Element {
    rsx! {
        super::MenuPanel {}
        div { class: "main-content",
            {content}
        }
    }
}
```

### Menu Panel (sidebar navigation)

```rust
// templates/menu_panel.rs
use dioxus::prelude::*;
use crate::AppRoute;

#[component]
pub fn MenuPanel() -> Element {
    rsx! {
        div { class: "sidebar",
            div { class: "sidebar-logo", "App Name" }
            nav { class: "sidebar-nav",
                Link { class: "sidebar-link", to: AppRoute::Dashboard {},
                    "Dashboard"
                }
            }
            div { class: "sidebar-bottom",
                Link { class: "sidebar-link", to: AppRoute::Logout {},
                    "Logout"
                }
            }
        }
    }
}
```

## Dialogs — initial scaffold

See **dioxus-design-patterns.md** for full dialog lifecycle, DataState, state management patterns. Below is the minimal bootstrap scaffold:

```rust
// dialogs/dialog_state.rs
#[derive(Default)]
pub enum DialogState {
    #[default]
    None,
}

impl DialogState {
    pub fn is_hidden(&self) -> bool {
        matches!(self, Self::None)
    }
}
```

```rust
// dialogs/render.rs
use dioxus::prelude::*;
use crate::states::AppState;

#[component]
pub fn RenderDialog() -> Element {
    let app_state = consume_context::<Signal<AppState>>();
    let app_state_ra = app_state.read();

    if app_state_ra.get_dialog_state().is_hidden() {
        return rsx! {};
    }

    rsx! {}
}
```

## Dockerfile — Static Hosting

Client-side Dioxus compiles to static WASM + HTML + JS + CSS. Use `myjettools/web-app-host` to serve:

```dockerfile
FROM myjettools/web-app-host:0.1.1
ARG BUILD_VERSION
ENV BUILD_VERSION=$BUILD_VERSION

ARG RFC3339_TIME
ENV COMPILE_TIME=$RFC3339_TIME

WORKDIR /app
COPY ./target/dx/your-project-name/release/web/public ./wwwroot
```

## build.py — Cache Busting

Place `build.py` in project root. CI runs it after `dx build` to append random query strings to `.wasm`, `.js`, `.css` references in `index.html`:

```python
import random
import string
import argparse

def replace_wasm_with_random_string(file_path):
    def generate_random_string(length=16):
        characters = string.ascii_letters + string.digits
        return ''.join(random.choice(characters) for _ in range(length))

    with open(file_path, 'r') as file:
        content = file.read()

    updated_content = content.replace('.wasm', f'.wasm?id={generate_random_string()}')
    updated_content = updated_content.replace('.js', f'.js?id={generate_random_string()}')
    updated_content = updated_content.replace('.css', f'.css?id={generate_random_string()}')

    with open(file_path, 'w') as file:
        file.write(updated_content)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Cache-bust static assets in index.html")
    parser.add_argument("file_path", help="Path to the index.html file")
    args = parser.parse_args()
    replace_wasm_with_random_string(args.file_path)
```

## CI Workflow (GitHub Actions)

Tag trigger: `your-project-name-*`. Two jobs: build (in `dioxus-docker` container) and publish (Docker image to GHCR).

```yaml
name: Release App
on:
  push:
    tags:
      - "your-project-name-*"

env:
  IMAGE_NAME: ghcr.io/your-org/your-project-name
  DIR: your-project-name
  APP_NAME: your-project-name
  DELIVER_NAME: your-project-name

jobs:
  build:
    runs-on: ubuntu-22.04
    container:
      image: myjettools/dioxus-docker:0.7.3
    steps:
      - uses: actions/checkout@v6.0.2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Get the version
        id: get_version
        run: |
          TAG="${{ github.ref_name }}"
          VERSION="${TAG##*-}"
          echo "VERSION=$VERSION" >> "$GITHUB_OUTPUT"

      - name: Updating version
        run: |
          cd ${DIR}
          sed -i -e 's/^version = .*/version = "${{ steps.get_version.outputs.VERSION }}"/' Cargo.toml

      - run: |
          export GIT_HUB_TOKEN="${{ secrets.PUBLISH_TOKEN }}"
          cd ${DIR}
          dx build --release --web
          ls ./target/dx/${APP_NAME}/release/web/public
          python3 build.py ./target/dx/${APP_NAME}/release/web/public/index.html

      - name: Zip and upload
        run: |
          cd ${DIR}
          FILE_NAME="https://jetdev.eu/file/${DELIVER_NAME}-build.zip"
          apt install zip
          zip -r data.zip ./target/dx/${APP_NAME}/release/web ./Dockerfile
          curl -X 'POST' $FILE_NAME -H 'accept: */*' -H 'Content-Type: multipart/form-data' -F 'file=@data.zip;type=application/zip'

  publish:
    runs-on: ubuntu-22.04
    needs: build
    steps:
      - uses: actions/checkout@v6.0.2

      - name: Download Build Artifacts
        run: |
          cd ${DIR}
          FILE_NAME="https://jetdev.eu/file/${DELIVER_NAME}-build.zip"
          curl -L -o data.zip $FILE_NAME
          unzip -o data.zip

      - name: Get the version
        id: get_version
        run: |
          TAG="${{ github.ref_name }}"
          VERSION="${TAG##*-}"
          echo "VERSION=$VERSION" >> "$GITHUB_OUTPUT"

      - name: Docker login
        run: |
          echo "${{ secrets.PUBLISH_TOKEN }}" | docker login https://ghcr.io -u "${{ github.actor }}" --password-stdin

      - name: Docker Build and Publish
        run: |
          cd ${DIR}
          docker build -t ${IMAGE_NAME}:${{ steps.get_version.outputs.VERSION }} .
          docker push ${IMAGE_NAME}:${{ steps.get_version.outputs.VERSION }}
```

## Build and Dev Commands

```bash
# Development (hot-reload)
dx serve --package your-project-name

# Production build
dx build --release --web --package your-project-name

# Check compilation (use dx, not cargo check)
dx build --package your-project-name
```

**NEVER** use `cargo check` or `cargo build` for client-side Dioxus — always use `dx build` or `dx serve`.

## Critical Reminders

1. **No server module** — client-side project has no `src/server/`, no `#[server]` functions, no `#[cfg(feature = "server")]`
2. **`dioxus-utils` feature is `"web"`**, not `"fullstack"`
3. **API calls via FlUrl with relative `/api/...` URLs** — the wasm backend resolves them against the page origin; build requests from shared `rest-api-shared` models via `.execute_request(HttpVerb::X, model)` (never hand-assemble JSON)
4. **WebSocket via `reqwasm`** — browser WebSocket API, no custom headers, token via query parameter
5. **`GlobalAppSettings::get_origin()`** — use it **only** for the WebSocket URL (the browser WS API needs an absolute `ws`/`wss` URL); API calls use relative URLs and never compute a base URL
6. **`dioxus_utils::console_log()`** — for browser console logging
7. **Pre-auth vs post-auth pages** — controlled by `LocationState` and `with_ws` flag on `App` component
8. **CSS compiled by `build.rs`** — source in `css/`, output in `public/assets/app.css`. **NEVER** edit `app.css` directly
9. **Favicon required** — `public/favicon.ico`
10. **Docker image** — `myjettools/web-app-host:0.1.1` serves static files from `./wwwroot`
11. **Wire models live in `rest-api-shared`** — request models (`MyHttpInput`) and response models (`Serialize, Deserialize, MyHttpObjectStructure, Clone, Debug, PartialEq`) are defined once and shared verbatim by the REST-API server and the client; the client depends on the crate **without** the `server` feature. `src/models/` keeps only client view-state
12. **Centralize HTTP handling** — every API function forwards FlUrl's raw `Result<FlUrlResponse, FlUrlError>` to `handle_http_response` / `handle_http_empty` / `handle_http_response_opt` in `api/mod.rs`; no per-call status checks
13. **Parameter-less requests use `EmptyRequestModel`** — don't declare an empty `MyHttpInput` model just to satisfy `execute_request`
14. **Never put `///` doc-comments on `MyHttpInput` / `MyHttpObjectStructure` struct fields** — the proc-macro panics; use the `description = "..."` attribute instead
