# Project files — Cargo.toml, Dioxus.toml, build.rs

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

# Browser WebSocket client: reconnect loop and timeouts built in.
my-web-sockets-wasm = { tag = "0.1.0", git = "https://github.com/MyJetTools/my-web-sockets-wasm.git" }

serde_json = { version = "*" }
serde = { version = "*", features = ["derive"] }

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
- `my-web-sockets-wasm` for WebSocket connections from the browser — auto-reconnecting, see topic `websocket`

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
