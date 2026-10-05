# Templates and the dialogs scaffold

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

See **dioxus-design-patterns** (`get_dioxus_design_patterns`) for full dialog lifecycle, DataState, state management patterns. Below is the minimal bootstrap scaffold:

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
