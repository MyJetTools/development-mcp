# Routing — main.rs, pre-auth and post-auth pages, states

## main.rs — Routing and WebSocket

Client-side apps typically have **pre-auth pages** (login, code verification) and **post-auth pages** (dashboard, etc.). The `App` component decides whether to start a WebSocket connection based on the current route.

`main()` installs the panic hook before anything else: `dioxus_utils::set_panic_hook()`. Dioxus installs none in a release build, so without it a panic leaves only `RuntimeError: unreachable` in the browser console — with it, the console shows the panic text and the file, line and column. In a debug build (`dx serve`) Dioxus installs its own hook at launch, which replaces this one. See **Panic Hook** in the dioxus-utils README (`get_dioxus_utils_readme`).

```rust
use std::rc::Rc;

use dioxus::prelude::*;
use my_web_sockets_wasm::{Message, WebSocketClient, WsCallback, WsConnection};

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
    // First thing in main() — a release build prints no panic text without it
    dioxus_utils::set_panic_hook();

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

    // Uses hooks — called on every render, never under `if`. See topic `websocket`
    kick_off_ws(app_state, with_ws);

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

## States

### AppState — global application state

```rust
// states/app_state.rs
use crate::dialogs::DialogState;
use crate::models::InstrumentWsModel;

#[derive(Default)]
pub struct AppState {
    dialog_state: DialogState,
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

## Tabs — an enum in the route, never a string

A tab of a page is a closed set, so it is typed (`Types Over Magic Values`, `get_application_architecture_best_practices`). An address is built from `AppRoute`, never written as text.

```rust
// ❌ WRONG — the compiler sees neither the tab nor the address, and a typo opens the first tab
#[route("/okx-settings/:..tab")]
OkxSettings { tab: Vec<String> },

navigator().push("/okx-settings/bid-ask");

// ✅ CORRECT
#[route("/okx-settings/:..tab")]
OkxSettings { tab: OkxTab },
#[route("/:..rest")]
NotFound { rest: Vec<String> },

navigator().push(AppRoute::OkxSettings { tab: OkxTab::BidAsk });
```

A `:..` segment takes any type with `ToRouteSegments` and `FromRouteSegments`:

```rust
use dioxus::router::routable::{FromRouteSegments, ToRouteSegments};

#[derive(Clone, Copy, PartialEq)]
pub enum OkxTab {
    Instruments,
    Maps,
    BidAsk,
}

impl OkxTab {
    pub const ALL: [Self; 3] = [Self::Instruments, Self::Maps, Self::BidAsk];

    // The ONE place the text of a tab lives. None: the page's own address.
    fn segment(self) -> Option<&'static str> {
        match self {
            Self::Instruments => None,
            Self::Maps => Some("maps"),
            Self::BidAsk => Some("bid-ask"),
        }
    }
}

impl ToRouteSegments for OkxTab {
    fn display_route_segments(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.segment() {
            // WITH the slash: the route writes `/okx-settings`, then this
            Some(segment) => write!(f, "/{segment}"),
            None => Ok(()),
        }
    }
}

impl FromRouteSegments for OkxTab {
    type Err = String;

    fn from_route_segments(segments: &[&str]) -> Result<Self, Self::Err> {
        let tail = segments.join("/");
        Self::ALL
            .into_iter()
            .find(|tab| tab.segment().unwrap_or_default() == tail)
            .ok_or_else(|| format!("no tab `{tail}`"))
    }
}
```

- The two traits are not in `dioxus::prelude`.
- `display_route_segments` writes the leading `/`, whatever the trait's doc comment says. Without it the address is `/okx-settingsmaps`.
- The tab of the page's own address writes nothing. `/okx-settings` and `/okx-settings/` both arrive as that tab.
- An unknown tab is an `Err`: the route does not match, and the address goes to the catch-all `NotFound`. It never opens another tab. So keep the catch-all route.
- The route component stays when only the tab changes, and its hooks do not run again. Pass the tab down as a prop: a `use_context_provider` there keeps the first tab.
- `from_route_segments` reads `ALL`, and the compiler does not check it: a variant left out of it compiles, and its address goes to `NotFound`. Let the tab strip draw `ALL`, filtered where a tab is hidden from someone, and the gap is seen at once.
- `dx build --wasm-split` with the cargo feature `dioxus/wasm-split` makes every variant of the route enum a wasm module. The flag alone builds no modules and says nothing. Link with `RUSTFLAGS="-C link-arg=-O0"`: without it wasm-ld merges constants, and the split zeroes bytes another module reads. `RUSTFLAGS` replaces `rustflags` of `.cargo/config.toml`, it does not add to them, so repeat them in the variable: a project on the Zero Warnings Policy builds with `RUSTFLAGS="-D warnings -A non_snake_case -C link-arg=-O0"`. Under this feature `Routable` generates a camel-case function for every route, so `-D warnings` or `#![deny(warnings)]` alone stops the build.
- With `--wasm-split` the shell must be a layout: `#[layout(App)]` over the routes, with `Outlet` inside `SuspenseBoundary`. An `App` rendered inside each route component, as in the example at the top of this topic, is copied into the module of every route, with every page of its `match location`.
- A variant per tab that draws the whole page is a module per tab, each with the whole page in it: dx 0.7.10 copies into both modules what two routes share and the main module does not. One variant with a typed tab is one module a page, and opening one tab brings all its tabs.
- Several variants that draw one page are a module each, with the page copied into each. One variant with a typed tail is one module: `#[route("/bridges/:..page")] Bridge { page: BridgePage }`.
- Profile, what the main module must keep, dialogs as a module and fetching modules ahead: `get_wasm_optimization_guide`, section 10.
