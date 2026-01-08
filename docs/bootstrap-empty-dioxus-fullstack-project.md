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
        └── styled.css
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
style = ["/assets/styled.css", "/assets/app.css"]

# Javascript code file
script = ["/assets/bootstrap.js"]


[web.resource.dev]

# Javascript code file
# serve: [dev-server] only
script = []
```

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

Since it's a fullstack project, the server module is required:

1. Create `src/server/mod.rs` with your server configuration (can be empty initially)
2. Use `#[cfg(feature = "server")]` to conditionally compile server code
3. Create server functions using `#[server]` attribute

### src/server/mod.rs
```rust
// Server module - server-side functionality
```

## Build.rs

Create a `build.rs` file:

```rust
fn main() {

}
```

## Summary

- Create the directory structure as described
- Set up Cargo.toml with Dioxus fullstack dependencies
- Set up Dioxus.toml with web configuration
- Create main.rs with routing but NO init_app call on startup
- Create empty module files for all directories
- The App component should render immediately without any async initialization
- When you check if project compiles successfully, run not only `cargo check` command but as well dx build - since it's a fullstack project, we need to build both server and client parts of the project. As well Add feature server - compile it and remove feature server.