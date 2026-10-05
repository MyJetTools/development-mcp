# Routing — main.rs, pre-auth and post-auth pages, states

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
