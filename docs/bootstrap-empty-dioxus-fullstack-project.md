---
alwaysApply: false
---
# Bootstrap Empty Dioxus Fullstack Project

This document describes how to bootstrap a new empty Dioxus fullstack web application with the standard project structure.

## Project Structure

The project should have the following directory structure:

```
project-root/
├── Cargo.toml
├── Dioxus.toml
├── build.rs
├── css/
│   ├── 01-common.css
│   └── 99-desktop.css
├── src/
│   ├── main.rs
│   ├── api/
│   │   └── mod.rs
│   ├── components/
│   │   └── mod.rs
│   ├── models/
│   │   └── mod.rs
│   ├── states/
│   │   └── mod.rs
│   ├── views/
│   │   └── mod.rs
│   ├── web/
│   │   └── mod.rs
│   ├── dialogs/
│   │   └── mod.rs
│   └── server/
│       └── mod.rs
└── public/
    ├── favicon.ico
    └── assets/
        └── app.css          ← compiled by build.rs from css/ files
```

## Cargo.toml Setup

Create a `Cargo.toml` with the following structure:

```toml
[package]
name = "your-project-name"
version = "0.1.0"
edition = "2021"

[features]
default = []
server = ["dioxus/server", "tokio", "dioxus-utils/server"]
web = ["dioxus/web"]

[dependencies]
dioxus = { version = "0.7", features = ["fullstack", "router"] }
dioxus-utils = { tag = "0.7.0", git = "https://github.com/MyJetTools/dioxus-utils.git", features = [
    "fullstack",
] }
serde = "*"
serde_json = "*"
web-sys = { version = "*", features = ["Storage"] }
js-sys = "*"

tokio = { version = "*", features = ["full"], optional = true }
futures = "*"

[build-dependencies]
ci-utils = { git = "https://github.com/MyJetTools/ci-utils.git", tag = "0.1.3" }

[profile]
[profile.wasm-dev]
inherits = "dev"
opt-level = 1

[profile.server-dev]
inherits = "dev"

[profile.android-dev]
inherits = "dev"
```

## Dioxus.toml Setup

Create a `Dioxus.toml` with:

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
# CSS style file
style = ["/assets/app.css"]

# Javascript code file
script = ["/assets/bootstrap.js"]


[web.resource.dev]

# Javascript code file
# serve: [dev-server] only
script = []
```

## build.rs — CSS Compilation

CSS source files live in `css/`, compiled into `public/assets/app.css`. See the Dioxus design patterns (`get_dioxus_design_patterns`), §15, for full details.

```rust
fn main() {
    ci_utils::css::CssCompiler::new("./css")
        .add_file("01-common.css")
        .add_file("99-desktop.css")
        .compile("./public/assets/app.css");
}
```

**NEVER** edit `public/assets/app.css` directly — it is auto-generated on every build. Always add or edit CSS in the `css/` directory. To add new styles, create a new numbered file (e.g. `02-layout.css`) and register it in `build.rs`.

## Main.rs Structure

The `src/main.rs` should have this structure:

```rust
#[cfg(feature = "server")]
mod server;

use dioxus::prelude::*;
#[cfg(feature = "server")]
use dioxus::server::IncrementalRendererConfig;

mod api;
mod components;
mod dialogs;
mod models;
mod states;
mod views;
mod web;

// Define your routes enum
#[derive(Routable, PartialEq, Clone)]
enum AppRoute {
    #[route("/")]
    Dashboard {},
}

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(server_only!(ServeConfig::builder().incremental(
            IncrementalRendererConfig::default()
                .invalidate_after(std::time::Duration::from_secs(120)),
        )))
        .launch(|| {
            rsx! {
                document::Link { rel: "icon", href: asset!("/public/favicon.ico") }
                Router::<AppRoute> {}
            }
        })
}

#[component]
fn Dashboard() -> Element {
    rsx! {
        App {}
    }
}

#[component]
fn App() -> Element {
    rsx! {
        div { "Welcome to your new Dioxus application" }
    }
}
```

## Critical Requirements

### NO init_app Call on Startup

**IMPORTANT:** The `App` component must NOT call any initialization server function on startup. 

The App component should be a simple component that renders immediately without:
- No `init_app()` or similar server function calls
- No loading states waiting for server responses
- No `use_signal` with DataState that triggers async initialization
- No `spawn` calls that fetch data on component mount

The App component should render immediately without any server-side initialization.


### Example of Correct App Component

DO create an App component like this:

```rust
// ✅ CORRECT - No initialization on startup
#[component]
fn App() -> Element {
    rsx! {
        div { "Your application content here" }
    }
}
```

## Module Files

Create empty module files for each directory:

### src/api/mod.rs
```rust
// API module - server and client functions
```

### src/components/mod.rs
```rust
// UI components module
```

### src/models/mod.rs
```rust
// Data models for communication between server and client
```

### src/states/mod.rs
```rust
// Application state management
```

### src/views/mod.rs
```rust
// Page views module
```

### src/web/mod.rs
```rust
// Web-specific utilities (localStorage, etc.)
```

### src/dialogs/mod.rs
```rust
// Dialog components module
```

## Left Panel Navigation (Optional)

If you need a left panel navigation system:

1. Create `AppRoute` enum with all your routes
2. Create `LocationState` enum in `src/states/location.rs` to track current page
3. Use `use_context_provider` to provide location state to child components
4. Create a `LeftPanel` component in `src/views/` that renders navigation items

## Server Module

The server module has a standard structure. **Always** organize it as follows:

```
src/server/
├── mod.rs                  — lazy_static APP_CTX + proto include + module re-exports
├── settings.rs             — AppSettingsReader + GrpcClientSettings impl
├── app/
│   ├── mod.rs
│   ├── app_ctx.rs          — AppContext struct (holds gRPC clients, repos)
│   └── app_ctx_wrapper.rs  — lazy initialization wrapper
└── grpc_client/
    ├── mod.rs
    └── my_service.rs       — generate_grpc_client macro
```

### src/server/mod.rs
```rust
mod settings;
pub use settings::*;
mod app;
pub use app::*;
pub mod grpc_client;
pub use grpc_client::*;

pub const APP_NAME: &'static str = env!("CARGO_PKG_NAME");

lazy_static::lazy_static! {
    pub static ref APP_CTX: AppContextWrapper = AppContextWrapper::new();
}
```

### src/server/settings.rs
```rust
use serde::{Deserialize, Serialize};
use crate::server::grpc_client::MyServiceGrpcClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub my_service_grpc_url: String,
}

pub struct AppSettingsReader {
    pub settings_reader: my_settings_reader::SettingsReader<Settings>,
}

impl AppSettingsReader {
    pub fn new() -> Self {
        Self {
            settings_reader: my_settings_reader::SettingsReader::new("~/.my-app"),
        }
    }
}

#[async_trait::async_trait]
impl service_sdk::my_grpc_extensions::GrpcClientSettings for AppSettingsReader {
    async fn get_grpc_url(&self, name: &'static str) -> service_sdk::my_grpc_extensions::GrpcUrl {
        if name == MyServiceGrpcClient::get_service_name() {
            return self.settings_reader
                .get(|s| s.my_service_grpc_url.to_string().into())
                .await;
        }
        panic!("Unknown grpc service: {}", name)
    }
}
```

**Note**: Use `.get(|s| ...)` — this is `my_settings_reader::SettingsReader<T>`'s method. It's different from `use_settings(|s| ...)` which is from service-sdk's `SettingsReader`.

### src/server/grpc_client/my_service.rs
```rust
service_sdk::macros::use_grpc_client!();

#[generate_grpc_client(
    proto_file: "./proto/MyService.proto",
    crate_ns: "crate::my_service_grpc",
    retries: 3,
    request_timeout_sec: 30,
    ping_timeout_sec: 5,
    ping_interval_sec: 10,
)]
pub struct MyServiceGrpcClient;
```

### src/server/app/app_ctx_wrapper.rs
```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::server::settings::AppSettingsReader;
use super::AppContext;

pub struct AppContextWrapper {
    pub settings_reader: Arc<AppSettingsReader>,
    inner: Mutex<Option<Arc<AppContext>>>,
}

impl AppContextWrapper {
    pub fn new() -> Self {
        Self {
            settings_reader: Arc::new(AppSettingsReader::new()),
            inner: Mutex::new(None),
        }
    }

    pub async fn get(&self) -> Arc<AppContext> {
        let mut inner = self.inner.lock().await;
        if let Some(ctx) = inner.clone() { return ctx; }
        let ctx = Arc::new(AppContext::new(self.settings_reader.clone()).await);
        *inner = Some(ctx.clone());
        ctx
    }
}
```

### Cargo.toml server features (when using gRPC)
```toml
[features]
server = [
    "dioxus/server",
    "tokio",
    "tonic",
    "prost",
    "tonic-prost",
    "async-trait",
    "lazy_static",
    "my-settings-reader",
    "dioxus-utils/server",
    "service-sdk",
]

[dependencies]
lazy_static = { version = "*", optional = true }
my-settings-reader = { tag = "0.4.1", git = "https://github.com/MyJetTools/my-settings-reader.git", optional = true }
tonic = { version = "*", optional = true }
tonic-prost = { version = "*", optional = true }
prost = { version = "*", optional = true }
service-sdk = { tag = "...", git = "...", features = ["grpc", "macros"], optional = true }

# ⚠️ IMPORTANT: rust-extensions must be a DIRECT (non-optional) dependency
# service-sdk re-exports rust-extensions on server side, but service-sdk is optional.
# Shared models (enums with AsStr/EnumIterator) compile on BOTH web and server targets,
# so rust-extensions must be always available.
rust-extensions = { tag = "0.1.5", git = "https://github.com/MyJetTools/rust-extensions.git" }
```

### main.rs — proto module (server only)
```rust
#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
use dioxus::server::*;

#[cfg(feature = "server")]
pub mod my_service_grpc {
    tonic::include_proto!("my_service");  // package name from proto file
}
```

## Server Functions — Use `#[get]` / `#[post]`, NOT `#[server]`

Always use explicit HTTP method attributes with a clear path. Convention: `/api/{domain}/{action}`.

```rust
// ✅ CORRECT — explicit method + path
#[get("/api/swap-profiles/get")]
pub async fn get_swap_profiles() -> Result<Vec<SwapProfileModel>, ServerFnError> { ... }

#[post("/api/swap-profiles/save")]
pub async fn save_swap_profile(profile: SwapProfileModel) -> Result<(), ServerFnError> { ... }

// ❌ WRONG — random generated path, unclear method
#[server]
pub async fn get_swap_profiles() -> Result<Vec<SwapProfileModel>, ServerFnError> { ... }
```

**Why:** `#[server]` generates a random internal path which makes debugging and network inspection hard. `#[get]`/`#[post]` give predictable, readable URLs.

Organize API files by domain in `src/api/`:
```
src/api/
├── mod.rs             — pub mod for each domain file
├── swap_profiles.rs   — #[get] + #[post] for swap profiles
└── instruments.rs     — #[get] for instruments
```

## gRPC Client in Server Functions

When `service-sdk` is used with `grpc` feature, it forces `with-telemetry` in `my-grpc-extensions`. All generated client methods require a telemetry context as second argument:

```rust
#[get("/api/items/get")]
pub async fn get_items() -> Result<Vec<ItemModel>, ServerFnError> {
    use crate::server::APP_CTX;

    let ctx = APP_CTX.get().await;

    // Pass MyTelemetryContext::Empty when no telemetry needed
    let items = ctx.my_service
        .get_items((), &service_sdk::my_telemetry::MyTelemetryContext::Empty)
        .await
        .map_err(|e| ServerFnError::new(format!("get_items gRPC call failed: {:?}", e)))?;
    // ...
}
```

**Always add context to map_err** — describe what operation failed, not just the error.

## Proto Setup (when admin talks to gRPC backend)

```
my-admin/
└── proto/
    └── MyService.proto   ← copy manually from shared proto-files/
```

`build.rs` compiles the proto next to the CSS — see **Build.rs** below.

**Note**: Create the `proto/` directory manually and copy the proto file there. `ProtoFileBuilder` compiles but does not auto-create the directory.

## Models — Standard Derives

All shared models (used in server functions, serialized over wire) must have:

```rust
#[derive(Debug, Serialize, Deserialize, PartialEq, Default, Clone)]
pub struct MyModel {
    pub id: String,
    pub name: String,
}
```

`PartialEq` is required by Dioxus `Signal<T>` for change detection. `Default` is needed for "new item" dialogs.

## Build.rs

```rust
fn main() {
    // keep this line only if the project talks to a gRPC backend
    ci_utils::ProtoFileBuilder::new("proto/").sync_and_build("MyService.proto");

    ci_utils::css::CssCompiler::new("./css")
        .add_file("01-common.css")
        .add_file("99-desktop.css")
        .compile("./public/assets/app.css");
}
```

Without a gRPC backend `build.rs` is the CSS compilation alone.

## CI / GitHub Actions

**Always ask the user:** *"Should I create a CI workflow for this project?"*

If yes and the project is its own GitHub repo, `ci-utils` generates both the Dockerfile and the workflow. Add the `CiGenerator` call to the same `build.rs`:

```rust
fn main() {
    CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_dioxus_fullstack_service()
        .generate_github_ci_file()
        .build();

    // keep this line only if the project talks to a gRPC backend
    ci_utils::ProtoFileBuilder::new("proto/").sync_and_build("MyService.proto");

    ci_utils::css::CssCompiler::new("./css")
        .add_file("01-common.css")
        .add_file("99-desktop.css")
        .compile("./public/assets/app.css");
}
```

Rules:
- always pass `env!("CARGO_PKG_NAME")` — never hardcode the service name;
- add `.with_ci_test()` only if the project has at least one `#[test]`;
- run `cargo build` once — it writes `.github/workflows/release.yaml` and `Dockerfile`. Commit both;
- never hand-edit a generated file: the next `cargo build` overwrites it.

If the project lives in a **monorepo**, do not use `CiGenerator` — both workflow files are written by hand. Fetch the app-bootstrap guide, topic `ci-monorepo` (`get_app_bootstrap_guide`), for the templates. Note that the pre-baked builder image described there is for native Rust services: a Dioxus build runs `dx build` inside the `ghcr.io/my-jet-tools/dioxus-docker` container instead (the Dioxus client-side bootstrap guide, topic `ci`, has that workflow), so do not add a builder-image workflow to a Dioxus project.

## Summary

- Create the directory structure as described
- Set up Cargo.toml with Dioxus fullstack dependencies
- Set up Dioxus.toml with web configuration
- CSS lives in `css/` and `build.rs` compiles it into `public/assets/app.css` — never edit `app.css` directly
- Create main.rs with routing but NO init_app call on startup
- Create empty module files for all directories
- The App component should render immediately without any async initialization
- Server module structure: `server/{mod.rs, settings.rs, app/, grpc_client/}`
- Models derive: `Debug, Serialize, Deserialize, PartialEq, Default, Clone`
- When you check if project compiles successfully, run not only `cargo check` command but as well `dx build` and `cargo build --features server` — since it's a fullstack project, we need to build both server and client parts.
