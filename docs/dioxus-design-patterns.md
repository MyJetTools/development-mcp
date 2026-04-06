---
alwaysApply: false
---
# Dioxus Design Patterns

Common patterns for all Dioxus projects (fullstack and client-side). These are framework-level conventions, not project-type-specific.

## 1) Naming conventions

- **`cs`** — mutable signal holding component state: `let mut cs = use_signal(|| ComponentState::new(...))`
- **`cs_ra`** — read-access snapshot: `let cs_ra = cs.read()`
- Use `cs` for writes (`cs.write().field = value`) and `cs_ra` for reads in the render phase.

## 2) Single ComponentState — one struct, one signal

**NEVER** use multiple `use_signal` for component state fields. One struct, one signal:

```rust
// ❌ WRONG — separate signals
let mut submitting = use_signal(|| false);
let mut candle_type = use_signal(|| 0);
let mut style = use_signal(|| CandleStyle::Candles);

// ✅ CORRECT — one state struct
let mut cs = use_signal(|| ComponentState::new(&order, accuracy));
let cs_ra = cs.read();
```

**Exception**: `Signal<T>` that must be passed to a child component which requires `Signal<T>` as a prop. In this case, keep it as a separate signal.

Keep `cs_ra` alive for the entire render function — **never drop it early**. Event handlers capture `cs: Signal` (which is `Copy`), not `cs_ra`, so there's no borrow conflict.

## 3) Component folder structure — render / state / actions

Every non-trivial component lives in its own folder:

```
dialogs/edit_tp_sl/
├── mod.rs      ← mod render; pub use render::*; mod state; pub use state::*; mod actions; pub use actions::*;
├── render.rs   ← #[component] fn — only rendering, minimal logic
├── state.rs    ← ComponentState struct + mutation methods
└── actions.rs  ← pure helper functions (conversions, validation, AppState readers)
```

Rules:
- **render.rs** — rendering only. All data preparation via functions from state/actions. Open signals, call functions, build rsx.
- **state.rs** — single `ComponentState` struct. Methods for coupled mutations (e.g. `set_tp_price` updates both price and percent fields atomically).
- **actions.rs** — pure helper functions. Functions that read external state take it as `&T` parameter.

Simple components (no state, no actions) can have just `render.rs`.

## 4) Pass models to components — not individual props

When a component needs data from a model, pass the model directly:

```rust
// ❌ WRONG — 15 params unpacked from model
#[component]
pub fn EditDialog(
    account_id: i64, order_id: i64, instrument_id: String,
    open_price: f64, is_pending: bool, side: OrderSide, ...
) -> Element

// ✅ CORRECT — pass the model
#[component]
pub fn EditDialog(
    order: EditTpSl,
    instrument_name: String,
    accuracy: usize,
) -> Element {
    let order_id = order.id();
    let is_buy = order.side() == OrderSide::Buy;
    // use model methods directly
}
```

The model should implement `Clone` + `PartialEq` (Dioxus requires `PartialEq` for props). When the model holds `Rc<T>` and `T` doesn't implement `PartialEq`, implement it manually via `Rc::ptr_eq`:

```rust
impl PartialEq for EditTpSl {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Active(a), Self::Active(b)) => Rc::ptr_eq(a, b),
            (Self::Pending(a), Self::Pending(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}
```

## 5) External state reads — extract into functions with `&T` param

Logic that reads from `AppState` or other external state should be in named functions, not inlined in render:

```rust
// ❌ WRONG — inline in render
let position_volume = if is_active {
    app_ra.positions.iter().find(|p| p.id == id).map(|p| p.volume).unwrap_or(0.0)
} else {
    app_ra.pending_orders.iter().find(|o| o.id == id).map(|o| o.volume).unwrap_or(0.0)
};

// ✅ CORRECT — function in actions.rs
let position_volume = get_position_volume(&app_ra, order_id, is_active);
```

```rust
// actions.rs
pub fn get_position_volume(app: &AppState, order_id: i64, is_active: bool) -> f64 {
    if is_active {
        app.positions.iter().find(|p| p.id == order_id).map(|p| p.volume).unwrap_or(0.0)
    } else {
        app.pending_orders.iter().find(|o| o.id == order_id).map(|o| o.volume).unwrap_or(0.0)
    }
}
```

## 6) Controlled inputs — `as_str()` from cs_ra, no cloning

Use `.as_str()` references from `cs_ra` directly in rsx — don't clone strings for rendering:

```rust
// ✅ CORRECT — zero-copy references
let cs_ra = cs.read();
let tp_price_str = cs_ra.tp_price.as_str();
let tp_pct_str = cs_ra.tp_pct.as_str();

rsx! {
    input { value: "{tp_price_str}", oninput: move |evt| { cs.write().set_tp_price(...); } }
    input { value: "{tp_pct_str}", oninput: move |evt| { cs.write().set_tp_pct(...); } }
}

// ❌ WRONG — unnecessary cloning
let tp_price_val = cs_ra.tp_price.clone();
rsx! { input { value: tp_price_val, ... } }
```

## 7) State methods over inline handler logic

Put domain logic into methods on the state struct. Handlers call methods via `cs.write().method(...)`:

```rust
// ✅ CORRECT — logic in state method
impl ComponentState {
    pub fn set_tp_price(&mut self, val: String, open_price: f64, is_buy: bool, leverage: f64) {
        self.tp_price = val.clone();
        self.tp_pct = if val.is_empty() {
            String::new()
        } else {
            price_to_pct_str(&val, open_price, is_buy, true, leverage)
        };
    }
}

// Handler — one line
oninput: move |evt: Event<FormData>| {
    cs.write().set_tp_price(evt.value(), open_price, is_buy, leverage);
},
```

Multiple `cs.write()` calls in sequence — antipattern. Use a method to update all fields in one write:

```rust
// ❌ WRONG
cs.write().loading = true;
cs.write().error = None;
cs.write().tab = Tab::Active;

// ✅ CORRECT
impl PageState {
    pub fn reset(&mut self) {
        self.loading = true;
        self.error = None;
        self.tab = Tab::Active;
    }
}
cs.write().reset();
```

Methods on plain structs are unit-testable without Dioxus runtime.

## 8) Structs with many fields — `new()` with required params, defaults for the rest

```rust
impl ChartViewState {
    pub fn new(prefix: impl Into<String>, instrument_id: impl Into<String>, candle_type: i32) -> Self {
        Self {
            canvas_prefix: prefix.into(),
            instrument_id: instrument_id.into(),
            candle_type,
            candle_style: CandleStyle::Candles,
            zoom_idx: DEFAULT_ZOOM_IDX,
            indicators: Vec::new(),
            positions: None,
            // ... all other fields = defaults
        }
    }
}

// Usage — set only what differs
let mut cv = ChartViewState::new("dialog-chart", instrument_id, 0);
cv.pending_mode = Some(false);
cv.container_class = Some("mini-chart-container".to_string());
```

When a child component needs to write back to the struct (e.g. zoom via mouse wheel), pass `Signal<Struct>` — not individual `Signal<usize>` per field.

## 9) Dialogs: lifecycle, rendering, and template

### DialogState — always part of AppState

`DialogState` is **always a field of `AppState`** — dialogs render globally as an overlay, so `RenderDialog` reads from `AppState`:

```rust
#[derive(Default)]
pub struct AppState {
    dialog_state: DialogState,
    // ... other fields
}

impl AppState {
    pub fn get_dialog_state(&self) -> &DialogState {
        &self.dialog_state
    }

    pub fn open_edit_tp_sl(&mut self, order: EditTpSl, instrument_name: String, accuracy: usize) {
        self.dialog_state = DialogState::EditTpSl { order, instrument_name, accuracy };
    }
}
```

### DialogState enum + RenderDialog router

```rust
#[derive(Default)]
pub enum DialogState {
    #[default]
    None,
    EditTpSl { order: EditTpSl, instrument_name: String, accuracy: usize },
    ViewClosedOrder { order: Rc<HistoryEntry>, instrument_name: String, accuracy: usize },
}

#[component]
pub fn RenderDialog() -> Element {
    let app_state = consume_context::<Signal<AppState>>();
    let app_state_ra = app_state.read();
    match app_state_ra.get_dialog_state() {
        DialogState::None => rsx! {},
        DialogState::EditTpSl { order, instrument_name, accuracy } => {
            // clone cheap Rc-based data, drop read guard, render
            let order = order.clone();
            drop(app_state_ra);
            rsx! { EditTpSlDialog { order, instrument_name, accuracy } }
        }
        // ...
    }
}
```

Each dialog = its own folder (see §3). Open dialogs by setting state: `app_state.write().open_edit_tp_sl(...)`.

### `dialog_template` — standard wrapper for all dialogs

All dialogs use `dialog_template` instead of inlining modal HTML. Cancel button and close (x) are **built into** the template — never add them manually:

```rust
// Standard size
super::dialog_template(title, content, ok_button)

// Custom size (e.g. wide dialog)
super::dialog_template_ex(title, content, ok_button, Some("modal-xl"))

// When data is loading — pass loading/error element as content
let data = match get_data(cs, &cs_ra) {
    Ok(d) => d,
    Err(el) => return super::dialog_template(title, el, rsx! {}),
};
```

## 10) DataState + `get_data` pattern — for async data

Every piece of async data uses `DataState<T>` + a `get_data` helper:

```rust
#[derive(Default)]
struct MyState {
    data: DataState<Vec<MyModel>>,
}

#[component]
fn MyComponent(some_id: i64) -> Element {
    let mut cs = use_signal(MyState::default);
    let cs_ra = cs.read();

    let items = match get_my_data(cs, &cs_ra, some_id) {
        Ok(d) => d,
        Err(el) => return el,
    };
    rsx! { /* render items */ }
}

fn get_my_data<'a>(
    mut cs: Signal<MyState>,
    cs_ra: &'a MyState,
    some_id: i64,
) -> Result<&'a [MyModel], Element> {
    match cs_ra.data.as_ref() {
        RenderState::None => {
            spawn(async move {
                cs.write().data.set_loading();
                match crate::api::something::get_items(some_id).await {
                    Ok(data) => cs.write().data.set_loaded(data),
                    Err(e)   => cs.write().data.set_error(e.to_string()),
                }
            });
            Err(render_loading())
        }
        RenderState::Loading      => Err(render_loading()),
        RenderState::Loaded(data) => Ok(data.as_slice()),
        RenderState::Error(err)   => Err(render_error(err.as_str())),
    }
}
```

**Forced reload after mutation**: `.reset()` on `DataState` — returns to `None` — next render triggers fresh load.

## 11) Tabs and lists — each = own component with own state

- Each tab is a `#[component]` that receives an ID prop and owns its `DataState`
- Lists within a tab are also separate components with own `DataState`
- Page-level state holds only UI: current tab, input, selected item — **never data arrays**

```
PageComponent              ← PageState: input, selected item, tab, ui flags
└── ContentComponent
    ├── TabA { id }        ← own State + DataState
    │   ├── ListOne { id } ← own State + DataState
    │   └── ListTwo { id } ← own State + DataState
    ├── TabB { id }        ← own State + DataState
    └── TabC { id, cs }    ← parent Signal if parent needs to trigger reset
```

## 12) API calls — always full path, never `use`

```rust
// ✅ CORRECT — visible that this is an API call
crate::api::accounts::get_account(id).await

// ❌ WRONG — looks like a local function
use crate::api::accounts::get_account;
get_account(id).await
```

## 13) Signal handling tips

- Signals are `Copy` — capture once in handlers, no cloning needed.
- Read with `.read()` for immutable snapshot; write with `.write()` to mutate.
- In closures inside `spawn(async move { ... })`, signal is moved by copy — safe to use.

## 14) `NotifyChildComponent<TValue>` — parent-to-child notification

When a parent action must trigger a child update (e.g. repaint, DataState reset), and the child manages its own state.

### Parent — create, notify, pass as prop

```rust
#[component]
fn ChartPanel() -> Element {
    // 1. Create notifier
    let repaint_notify = dioxus_utils::NotifyChildComponent::<()>::new();

    let mut chart_view = use_signal(|| ChartViewState::new("chart", instrument_id, 0));

    rsx! {
        // Toolbar — notify on style change
        button {
            onclick: move |_| {
                chart_view.write().candle_style = CandleStyle::Candles;
                // 2. Notify child
                repaint_notify.notify_other_components(());
            },
            "Candles"
        }
        button {
            onclick: move |_| {
                chart_view.write().candle_style = CandleStyle::Line;
                repaint_notify.notify_other_components(());
            },
            "Line"
        }

        // 3. Pass notifier as prop to child
        CanvasChart { view: chart_view, repaint_notify }
    }
}
```

### Child — receive as prop, subscribe with `on_notify`

```rust
#[component]
pub fn CanvasChart(
    mut view: Signal<ChartViewState>,
    repaint_notify: dioxus_utils::NotifyChildComponent<()>,
) -> Element {
    let mut cs = use_signal(ChartState::default);

    // Subscribe to parent notifications
    repaint_notify.on_notify(move |_| {
        // React to parent change — e.g. repaint canvas
        let state_ra = cs.read();
        if state_ra.loaded {
            do_repaint(&state_ra);
        }
    });

    // ... render
    rsx! { canvas { id: "chart-canvas" } }
}
```

### Rules

- `NotifyChildComponent` is `Copy` — safe to capture in multiple handlers
- `on_notify()` wraps `use_effect` — call it at the top level of the component, not inside conditions
- Use `()` as the type parameter when the notification carries no data — just a "something changed" signal
- For DataState reloads: `repaint_notify.on_notify(move |_| { cs.write().data.reset(); });`

## 15) CSS — source files in `css/`, compiled by `build.rs`

CSS source files live in `css/` directory, numbered for ordering. `build.rs` compiles them into a single `public/assets/app.css`:

```
css/
├── 01-common.css
├── 02-layout.css
├── 03-inputs.css
├── 04-buttons.css
└── 99-desktop.css
```

```rust
// build.rs
fn main() {
    ci_utils::css::CssCompiler::new("./css")
        .add_file("01-common.css")
        .add_file("02-layout.css")
        .add_file("03-inputs.css")
        .add_file("04-buttons.css")
        .add_file("99-desktop.css")
        .compile("./public/assets/app.css");
}
```

**NEVER** edit `public/assets/app.css` directly — it is auto-generated on every build and all manual changes will be lost. Always add or edit CSS in the `css/` directory. To add new styles, create a new numbered file (e.g. `07-toast.css`) and register it in `build.rs`.
