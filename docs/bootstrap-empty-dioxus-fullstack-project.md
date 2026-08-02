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

CSS source files live in `css/`, compiled into `public/assets/app.css`. See **dioxus-design-patterns.md §15** for full details.

```rust
fn main() {
    ci_utils::css::CssCompiler::new("./css")
        .add_file("01-common.css")
        .add_file("99-desktop.css")
        .compile("./public/assets/app.css");
}
```

**NEVER** edit `public/assets/app.css` directly — it is auto-generated on every build. Always add or edit CSS in the `css/` directory. To add new styles, create a new numbered file (e.g. `02-layout.css`) and register it in `build.rs`.

## CI / GitHub Actions

**Always ask the user:** *"Should I create a CI workflow for this project?"*

If yes and the project is its own GitHub repo, `ci-utils` generates both the Dockerfile and the
workflow — add the `CiGenerator` call to the same `build.rs` that compiles the CSS:

```rust
fn main() {
    CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_dioxus_fullstack_service()
        .generate_github_ci_file()
        .build();

    ci_utils::css::CssCompiler::new("./css")
        .add_file("01-common.css")
        .add_file("99-desktop.css")
        .compile("./public/assets/app.css");
}
```

Then run `cargo build` once — it writes `.github/workflows/release.yaml` and `Dockerfile`. Commit
both. Never hand-edit a generated file: the next `cargo build` overwrites it.

If the project lives in a **monorepo**, do not use `CiGenerator` — the workflow is written by hand.
Fetch the CI section of the app-bootstrap guide (`get_app_bootstrap_guide`) for the templates; note
that the pre-baked builder image described there is for native Rust services, while a Dioxus build
runs inside the `myjettools/dioxus-docker` container instead (see the Dioxus client-side bootstrap
guide for that workflow).

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

## Summary

- Create the directory structure as described
- Set up Cargo.toml with Dioxus fullstack dependencies
- Set up Dioxus.toml with web configuration
- Create main.rs with routing but NO init_app call on startup
- Create empty module files for all directories
- The App component should render immediately without any async initialization
- When you check if project compiles successfully, run not only `cargo check` command but as well dx build - since it's a fullstack project, we need to build both server and client parts of the project. As well Add feature server - compile it and remove feature server.
