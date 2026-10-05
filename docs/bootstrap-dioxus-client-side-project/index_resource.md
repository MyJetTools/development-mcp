---
alwaysApply: false
---
# Bootstrap Dioxus Client-Side Project

This document describes how to bootstrap a new Dioxus **client-side only** web application (no server component). The app compiles to WASM and runs entirely in the browser. API calls go to a separate backend service. WebSocket connections are made directly from the browser.

For component design patterns (state management, folder structure, DataState, dialogs, etc.) see **dioxus-design-patterns** (`get_dioxus_design_patterns`).

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

## Topics

Every part of the skeleton is a topic. Bootstrapping a new project means going through all of them in the order of the table; a change to one part needs only its topic. Read a topic: `resource://dioxus-client-side-bootstrap/{topic}`, or `get_dioxus_client_side_bootstrap_guide` with `topic`.

| Topic | What is inside |
| --- | --- |
| [`project-files`](project-files.md) | `Cargo.toml` (`dioxus/web`, `dioxus-utils` with `web`, FlUrl, `rest-api-shared`, `reqwasm`), `Dioxus.toml`, CSS compilation in `build.rs` |
| [`routing`](routing.md) | `main.rs`: the route enum, pre-auth and post-auth pages, the `App` component that kicks off the WebSocket; `AppState` and `LocationState` |
| [`websocket`](websocket.md) | Browser WebSocket via `reqwasm` with the token in the query string; the `event_id:json_payload` wire format and its handler |
| [`shared-wire-models`](shared-wire-models.md) | The `rest-api-shared` crate shared verbatim with the REST-API server: request / response derives, WASM-clean with a `server` feature, pure-data models |
| [`api-calls`](api-calls.md) | FlUrl with relative `/api/...` URLs and shared `MyHttpInput` models; the three centralized response helpers; `RequestError` and client view-state models |
| [`local-storage`](local-storage.md) | Session and refresh tokens in `localStorage` |
| [`templates-and-dialogs`](templates-and-dialogs.md) | Page layout templates (full-screen form, content panel, menu panel) and the initial dialogs scaffold |
| [`ci`](ci.md) | Static-hosting Dockerfile on `web-app-host`, `build.py` cache busting, the GitHub Actions release workflow inside the `dioxus-docker` container — the whole CI story for a Dioxus WASM client |

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
10. **Docker image** — `ghcr.io/my-jet-tools/web-app-host:0.1.0` serves static files from `./wwwroot`
11. **Wire models live in `rest-api-shared`** — request models (`MyHttpInput`) and response models (`Serialize, Deserialize, MyHttpObjectStructure, Clone, Debug, PartialEq`) are defined once and shared verbatim by the REST-API server and the client; the client depends on the crate **without** the `server` feature. `src/models/` keeps only client view-state
12. **Centralize HTTP handling** — every API function forwards FlUrl's raw `Result<FlUrlResponse, FlUrlError>` to `handle_http_response` / `handle_http_empty` / `handle_http_response_opt` in `api/mod.rs`; no per-call status checks
13. **Parameter-less requests use `EmptyRequestModel`** — don't declare an empty `MyHttpInput` model just to satisfy `execute_request`
14. **Never put `///` doc-comments on `MyHttpInput` / `MyHttpObjectStructure` struct fields** — the proc-macro panics; use the `description = "..."` attribute instead
