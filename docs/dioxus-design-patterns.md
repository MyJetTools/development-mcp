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

### Dialog results — always via `EventHandler`

Dialogs **never** mutate external state or call APIs directly. A dialog collects user input and passes the result back through an `EventHandler<T>` prop. The parent component (or `RenderDialog` router) owns the action logic:

```rust
// ✅ CORRECT — dialog just returns data
#[component]
fn EditTpSlDialog(
    order: EditTpSl,
    instrument_name: String,
    accuracy: usize,
    on_submit: EventHandler<TpSlSubmitData>,
) -> Element {
    // ... form inputs, validation ...
    // Save button:
    onclick: move |_| {
        let w = cs.read();
        let data = TpSlSubmitData {
            tp_str: w.tp_price.clone(),
            sl_str: w.sl_price.clone(),
            open_price_str: w.open_price_str.clone(),
        };
        drop(w);
        on_submit.call(data);
    },
}

// Parent — action function handles API + state:
fn on_tp_sl_submit(mut app_state: Signal<AppState>, order: EditTpSl, data: TpSlSubmitData) {
    spawn(async move {
        let result = crate::api::trading::set_tp_sl(...).await;
        match result {
            Ok(resp) => { app_state.write().update_tp_sl(...); app_state.write().close_dialog(); }
            Err(err) => { show_toast(err.message, ToastType::Error); }
        }
    });
}

// Wiring in RenderDialog:
rsx! {
    EditTpSlDialog {
        order, instrument_name, accuracy,
        on_submit: move |data: TpSlSubmitData| {
            on_tp_sl_submit(app_state, order_clone.clone(), data);
        },
    }
}
```

```rust
// ❌ WRONG — dialog calls API and mutates state
#[component]
fn EditTpSlDialog(...) -> Element {
    let mut app_state = consume_context::<Signal<AppState>>();
    // onclick:
    spawn(async move {
        crate::api::trading::set_tp_sl(...).await;  // not dialog's job
        app_state.write().update_tp_sl(...);         // not dialog's job
    });
}
```

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

## 16) NEVER call `signal.write()` / `signal.set()` during render

**NEVER** call `signal.write()` or `signal.set()` in the component body (outside event handler closures). This triggers a re-render → the function runs again → writes again → **infinite loop → browser hangs**.

```rust
// ❌ WRONG — write during render → infinite loop
#[component]
fn MyComponent() -> Element {
    let mut chart_view = use_signal(|| ChartViewState::new(...));

    // This runs on EVERY render → triggers re-render → loop
    chart_view.write().positions = Some(new_positions);

    rsx! { ... }
}

// ✅ CORRECT — compute as local variable, pass as prop
#[component]
fn MyComponent() -> Element {
    let chart_view = use_signal(|| ChartViewState::new(...));

    // Computed before rsx, no signal write
    let positions = vec![PositionOverlay { ... }];

    rsx! {
        CanvasChart {
            view: chart_view,
            positions: Some(positions),  // passed as prop, not written to signal
        }
    }
}
```

**Rule:** Only event handlers (`onclick`, `oninput`, `onkeydown`, etc.) and `spawn(async move { ... })` blocks may call `signal.write()` / `signal.set()`. The component body is for **reading** state and building the virtual DOM — never for mutating it.

## 17) Single state access per operation

Open a signal once — read/write everything needed — drop. Never open the same signal multiple times in sequence:

```rust
// ✅ CORRECT — one read, extract all fields
let ra = cs.read();
let data = SubmitData {
    tp_str: ra.tp_price.clone(),
    sl_str: ra.sl_price.clone(),
    open_price_str: ra.open_price_str.clone(),
};
drop(ra);

// ❌ WRONG — three separate reads
let data = SubmitData {
    tp_str: cs.read().tp_price.clone(),
    sl_str: cs.read().sl_price.clone(),
    open_price_str: cs.read().open_price_str.clone(),
};
```

Same applies to writes — batch mutations in one `.write()` or use a state method (see §7).

## 18) Browser storage initialises the state — the screen is never drawn from it

Browser storage (`sessionStorage` / `localStorage`) is **not a source for rendering**. It is what the state is initialised from, so that a refresh comes back to the same screen:

- **Read once — in `new()`.** The state is created from the stored record, so the very first render is already the right one. Render and effects **never** read storage.
- **Write-through.** A state method that changes a stored value writes storage **in the same method** — the state and storage change together.

Applies to screens whose first render happens in the browser (client-side projects). A server-rendered fullstack page is first drawn on the server, which has no browser storage, and hydration expects the client's first render to match that HTML.

### Read once — in `new()`

```rust
// ✅ CORRECT — the state is created from the record: the first render is already right
#[component]
pub fn Configurator(discount_id: String) -> Element {
    let link = ConfiguratorLink { discount_id };
    let mut cs = use_signal(move || ComponentState::new(link));
    let cs_ra = cs.read();
    // ... draw from cs_ra and from nothing else
}

// state.rs
impl ComponentState {
    pub fn new(link: ConfiguratorLink) -> Self {
        // The ONE read of storage
        let stored = crate::web::storage::configurator::get();
        // Only a record created at this address counts — see "The address" below
        let record = ConfiguratorRecord::for_link(stored.clone(), &link).unwrap_or_default();
        Self {
            plan_id: record.plan_id,
            options: record.options,
            link,
            stored, // what storage holds now — lets persist() skip a write that changes nothing
            // ... all other fields = defaults
        }
    }
}
```

```rust
// ❌ WRONG — created empty, storage read after the first render:
// the first frame is wrong, the second one corrects it
let mut cs = use_signal(ComponentState::default);
use_effect(move || {
    if let Some(record) = crate::web::storage::configurator::get() {
        cs.write().apply_record(record);
    }
});

// ❌ WRONG — drawn from storage
let plan_id = crate::web::storage::configurator::get().map(|r| r.plan_id);
```

### Write-through — the method that changes the state writes storage

Storage is written by the state method (§7) that makes the change — not by an effect and not by the handler:

```rust
// ✅ CORRECT — state.rs
impl ComponentState {
    pub fn pick_plan(&mut self, plan_id: &str) {
        self.plan_id = plan_id.to_string();
        self.options.clear();
        self.persist();
    }

    // The only writer of the record
    fn persist(&mut self) {
        let record = ConfiguratorRecord {
            link: self.link.clone(),
            plan_id: self.plan_id.clone(),
            options: self.options.clone(),
        };
        if self.stored.as_ref() == Some(&record) {
            return;
        }
        crate::web::storage::configurator::set(&record);
        self.stored = Some(record);
    }
}

// Handler — one line, knows nothing about storage
onclick: move |_| cs.write().pick_plan(&plan_id),
```

```rust
// ❌ WRONG — an effect subscribes to the whole state: every change of any field,
// each keystroke included, pays for a synchronous storage write
use_effect(move || {
    crate::web::storage::configurator::set(&cs.read().to_record());
});

// ❌ WRONG — the handler has to remember to write; the next handler will not
onclick: move |_| {
    cs.write().plan_id = plan_id.clone();
    crate::web::storage::configurator::set(&cs.read().to_record());
},
```

`persist()` compares before it writes, so it is safe to call at the end of every method that may change the record — a keystroke that leaves the record as it was is not a storage write. Keep text that is still being typed out of the record; store the applied value.

### The record

One serde struct per screen, and one module that reads and writes it — through `dioxus_utils::js::SESSION_STORAGE` / `LOCAL_STORAGE`, never through the browser API itself:

```rust
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfiguratorRecord {
    // the address the record was created at
    #[serde(default)]
    pub link: ConfiguratorLink,
    #[serde(default)]
    pub plan_id: String,
    #[serde(default)]
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfiguratorLink {
    #[serde(default)]
    pub discount_id: String,
}
```

```rust
// web/storage/configurator.rs
use dioxus_utils::js::SESSION_STORAGE;

const KEY: &str = "configurator";

pub fn get() -> Option<ConfiguratorRecord> {
    let raw = SESSION_STORAGE.get(KEY)?;
    serde_json::from_str(&raw).ok()
}

pub fn set(record: &ConfiguratorRecord) {
    if let Ok(json) = serde_json::to_string(record) {
        SESSION_STORAGE.set(KEY, &json);
    }
}

pub fn clear() {
    SESSION_STORAGE.delete(KEY);
}
```

```rust
// ❌ WRONG — an accessor of the project's own: storage that could not be obtained reads as "no record"
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

// ❌ WRONG — a write the browser refused is dropped, and the screen goes on as if it was saved
let _ = storage.set_item(KEY, &json);
```

- Every field is `#[serde(default)]` — a record written by an older build still loads.
- **Storage only through `dioxus_utils::js::LOCAL_STORAGE` / `SESSION_STORAGE`** (dioxus-utils `web` feature) — the project has no storage accessor of its own.
- **Two different absences — never read one as the other:**
  - **No record** — the key is absent, or this build cannot parse the value: `get()` answers `None` and the screen starts from scratch. The ordinary case.
  - **No storage** — it cannot be obtained, or the browser refuses a read, a write or a delete (e.g. the quota is exceeded): `dioxus-utils` panics. An app that keeps its session there is a different program without storage — a failure is never read as "nothing is stored".
- **`sessionStorage` for screen state** — a refresh is the same visit and comes back to the same screen, a new tab is a new visit. **`localStorage` for preferences and tokens.**
- A record that holds anything personal is **cleared when the session ends** — call `clear()` where the tokens are cleared.

### Navigating with data — write the destination's record, then navigate

The destination's state is created from the record, so it arrives already drawn right:

```rust
// ✅ CORRECT
fn open_configurator(plan_id: &str, discount_id: &str) {
    crate::web::storage::configurator::set(&ConfiguratorRecord {
        link: ConfiguratorLink { discount_id: discount_id.to_string() },
        plan_id: plan_id.to_string(),
        ..Default::default()
    });
    navigator().push(AppRoute::Configurator { discount_id: discount_id.to_string() });
}

// ❌ WRONG — the address describes the screen
navigator().push(AppRoute::Configurator { plan_id, options, discount_pct });
```

### The address — only what the server needs

**The query string carries only what the server needs on every request** — e.g. a discount id. It is put into the state in `new()` like everything else (the `link` above), and render checks that it applies to what is being drawn:

```rust
// a discount tied to another plan is not applied to this one
let discount = cs_ra.discount_for(plan);
```

**Never a value the screen states as a fact** — a price, a discount size: the address bar is editable. The id travels in the address; the figure comes from the server.

**A record belongs to the address it was created at.** The same address is a refresh — the record stands. Another address that names something else starts a new record:

```rust
impl ConfiguratorRecord {
    pub fn for_link(stored: Option<Self>, link: &ConfiguratorLink) -> Option<Self> {
        stored.filter(|record| record.link == *link)
    }
}
```

### What storage cannot hold — the first picture waits

What is not in storage arrives from the server through `DataState` (§10). The first picture **waits for exactly that part** (a skeleton until it lands) — it is never drawn without it and then corrected. The wait has a ceiling:

```rust
// state.rs — new() sets awaits_discount = !link.discount_id.is_empty()
impl ComponentState {
    // Everything that decides the first picture is in hand
    pub fn ready_to_paint(&self) -> bool {
        self.waited_out || !self.awaits_discount
    }

    pub fn discount_landed(&mut self, discount: Option<DiscountModel>) {
        self.discount.set_loaded(discount);
        self.awaits_discount = false;
    }

    // The ceiling was reached — draw what there is
    pub fn stop_waiting(&mut self) {
        self.waited_out = true;
    }
}

// render.rs — every read the first picture needs is started BEFORE the first early return
start_discount_read(cs, &cs_ra); // spawns on RenderState::None, as in §10
let plans = match get_plans(cs, &cs_ra) {
    Ok(plans) => plans,
    Err(el) => return el,
};
if !cs_ra.ready_to_paint() {
    return render_loading();
}

// actions.rs — the ceiling, armed in the same RenderState::None arm that starts the read
spawn(async move {
    dioxus_utils::js::sleep(std::time::Duration::from_secs(3)).await;
    if !cs.peek().ready_to_paint() {
        cs.write().stop_waiting();
    }
});
```

### Storage is a copy — when the server holds the same data

If the same data also lives on the server (a draft the server keeps), add `#[serde(default)] pub sent: bool` to the record — *the server holds exactly this*. Every method that changes the record clears it, a confirmed write to the server sets it; both go through `persist()`. When the server's copy arrives beside the state:

| `sent` | Server's copy | Which one stands |
| --- | --- | --- |
| any | the same | Nothing moves |
| `false` | differs | The local one — the server was never told, so it is newer; send it |
| `true` | differs | The server's — it was changed elsewhere (another tab, another device); put it into the state |

A state created with no record, while the server may hold one, waits for the server's copy before the first picture — the same rule as above.

### `sent` — only if the record did not change while the request was in flight

A confirmation covers the record **the request carried** — a change made while it was in flight is not on the server. The state counts revisions: every change of the record bumps `revision` and clears `sent`; the push remembers the revision it left with; the confirmation sets `sent = (revision == the one that was sent)`. `revision` is a field of the state only — the record keeps `sent`.

```rust
// ✅ CORRECT — state.rs
impl ComponentState {
    // Called by every method that changes the record
    fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.sent = false;
        self.persist();
    }

    // The push starts: the revision it leaves with, and the record it carries
    pub fn sync_started(&self) -> (u32, ConfiguratorRecord) {
        (self.revision, self.to_record())
    }

    // The server confirmed the push that left with `revision`
    pub fn sync_finished(&mut self, revision: u32) {
        self.sent = self.revision == revision;
        self.persist();
    }
}

// actions.rs
spawn(async move {
    let (revision, record) = cs.read().sync_started();
    if crate::api::configurator::save_draft(&record).await.is_ok() {
        cs.write().sync_finished(revision);
    }
});
```

```rust
// ❌ WRONG — a change made during the request is marked as known to the server:
// by the table above, the server's older copy then replaces it
if crate::api::configurator::save_draft(&record).await.is_ok() {
    cs.write().set_sent();
}
```

### `localStorage` is shared by all tabs — compare and clear

`localStorage` is one slot for every tab of the site (`sessionStorage` is per tab). A page that learns its record is finished — the invoice is paid, the draft is submitted — clears it by **compare-and-clear**: the test is made against the **slot, at the moment of the write**, not against what the page read when it opened. Another tab may have put a newer record there since.

```rust
// ✅ CORRECT — web/storage/invoice.rs: cleared only if the slot still holds this invoice
pub fn clear_if(id: &str) {
    if get().is_some_and(|held| held.id == id) {
        clear();
    }
}

// The page that learned invoice `id` is settled
crate::web::storage::invoice::clear_if(&id);
```

```rust
// ❌ WRONG — clears whatever the slot holds now: the newer invoice of another tab is lost
crate::web::storage::invoice::clear();

// ❌ WRONG — tested against what this page read when it opened, not against the slot
if cs.read().stored.as_ref().is_some_and(|held| held.id == id) {
    crate::web::storage::invoice::clear();
}
```

### Above the router — a `GlobalSignal` born from storage

What is drawn **above the router** — a toast, an error banner — lives outside any screen's state: there is no `new()` to read storage in. A stored value it needs (the language) is a `GlobalSignal`: born from storage **once**, and changed by the **same method that writes storage** — never a storage read in render.

```rust
// ✅ CORRECT — states/app_state.rs
// Born from storage once — the first time anything reads it
pub static CHROME_LANG: GlobalSignal<LangId> =
    Signal::global(crate::web::storage::language::get_lang);

impl AppState {
    // The one place the language changes: the state, storage and the global signal move together
    pub fn set_lang(&mut self, lang: LangId) {
        self.lang = lang;
        crate::web::storage::language::set_lang(lang);
        *CHROME_LANG.write() = lang;
    }
}

// The toast drawn above the router
let lang = *CHROME_LANG.read();
```

```rust
// ❌ WRONG — storage is read on every render, and the toast is not redrawn when the language changes
let lang = crate::web::storage::language::get_lang();
```
