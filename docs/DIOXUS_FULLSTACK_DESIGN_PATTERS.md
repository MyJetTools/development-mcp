# Dioxus Fullstack Design Patterns (Project Playbook)

Applies to Dioxus **fullstack** projects (shared code server/web). Use these when adding dialogs, forms, list views, or server functions. Adapt names as needed.

## Naming conventions
- **`cs`** – the mutable signal holding component state (`let mut cs = use_signal(...)`)
- **`cs_ra`** – a read-access snapshot of `cs` (`let cs_ra = cs.read()`)
- Use `cs` for writes (`cs.write().field = value`) and `cs_ra` for reads in the render phase.

## 0) Shared models (client + server)
- **Rule**: If a struct is used both on server **and** client (e.g., returned from a server function and rendered in UI) → put it in `src/models`.
- **Rule**: If a struct is only used inside a server function (e.g., parsing an external API response) → keep it private in the `src/api/*.rs` file, gated with `#[cfg(feature = "server")]`.
- **Naming**: Models returned from server functions (HTTP/server fn boundary) use the `HttpModel` suffix: `BalanceHistoryHttpModel`, `InstrumentHttpModel`.
- **One file per type**: each model lives in its own file (`models/balance_history.rs`, `models/instrument.rs`).
- `mod.rs` uses the standard re-export pattern: `mod x; pub use x::*;`
- In components: `use crate::models::*;` — never enumerate types explicitly.
- Derive `Serialize`/`Deserialize` for anything crossing the wire; keep structs minimal and web-safe.
- **Examples**:
  - `BalanceHistoryHttpModel` → in `src/models/balance_history.rs` (returned to client, shown in UI)
  - `BinanceExchangeInfo` → private in `src/api/binance.rs` with `#[cfg(feature = "server")]` (only used to parse external API response)

## 0.1) Mappers (server-only)
- Mappers know about gRPC/server contracts — they live in `src/server/` (not visible to client code).
- **1:1 mapping** → `impl From<GrpcModel> for HttpModel`, then in api: `.map(|i| i.into()).collect()`
- **Complex mapping** (multiple structs, logic) → a mapper function in `src/server/mappers/`

```rust
// ✅ 1:1 — impl From in server/mappers/ or alongside the model
impl From<BalanceHistoryGrpcModel> for BalanceHistoryHttpModel {
    fn from(i: BalanceHistoryGrpcModel) -> Self {
        Self { id: i.id, delta: i.delta, balance_after: i.balance_after, comment: i.comment, moment: i.moment }
    }
}

// api/balance.rs — clean, no manual field mapping
Ok(items.into_iter().map(|i| i.into()).collect())
```

## 0.2) API calls — always full path, never `use`
```rust
// ✅ CORRECT — visible that this is a server function call
crate::api::accounts::get_account(id).await
crate::api::balance::balance_update(id, delta, comment).await

// ❌ WRONG — looks like a local function, hides the boundary
use crate::api::accounts::get_account;
get_account(id).await
```

## 1) Dialogs: lifecycle and rendering
- Keep a global `DialogState` in context (`Signal<DialogState>`). Define variants per dialog (`Confirmation`, `EditInstrument`, etc.).
- Render all dialogs centrally via `RenderDialog`, matching on `DialogState` and embedding the concrete dialog component.
- Use `DialogTemplate` for consistent header, close "X", cancel button, and optional OK slot.
- Close dialogs by setting state to `DialogState::None` (either via `close()` or `set(DialogState::None)`).
- **Example: `DialogState` and renderer**
  ```rust
  #[derive(Clone)]
  pub enum DialogState {
      None,
      Confirmation { content: String, on_ok: EventHandler<()> },
      EditInstrument { item: Rc<InstrumentHttpModel>, on_ok: EventHandler<InstrumentHttpModel> },
  }

  #[component]
  pub fn RenderDialog() -> Element {
      let dialog_state = consume_context::<Signal<DialogState>>().read().clone();
      match dialog_state {
          DialogState::Confirmation { content, on_ok } => rsx! { ConfirmationDialog { content, on_ok } },
          DialogState::EditInstrument { item, on_ok } => rsx! { EditInstrumentDialog { item, on_ok } },
          DialogState::None => rsx! {}
      }
  }
  ```

## 2) Opening dialogs from views
- In tables or lists, set `DialogState` with the target item and an `on_ok` callback.
- `on_ok` should perform the mutation (e.g., save) and then reset any list state so the data reloads.
- Example flow:
  - Button click → set `DialogState::EditInstrument { item, on_ok }`
  - `on_ok` → call API → reset list state → dialog closes.
- **Example: open and handle save**
  ```rust
  button {
      onclick: move |_| {
          let item_to_edit = item.clone();
          consume_context::<Signal<DialogState>>().set(DialogState::EditInstrument {
              item: item_to_edit,
              on_ok: EventHandler::new(move |updated| {
                  spawn(async move {
                      crate::api::instruments::save_instrument(updated).await.unwrap();
                      cs.write().data.reset(); // triggers reload
                  });
              }),
          });
      },
      "Edit"
  }
  ```

## 3) Form state management
- Store form state in a struct held by `use_signal` (e.g., `EditInstrumentState`).
- Keep string/parsed numeric fields via `InputValue<T>` so validation is easy: `value_is_valid()` and `get_value()`.
- Drive UI enablement: compute `ok_is_disabled = !state.validation_ok()`.
- Normalize inputs inside handlers (e.g., trim, lowercase for IDs) and clear dependent status fields when input changes.
- **Example: state + validation**
  ```rust
  let mut cs = use_signal(|| EditInstrumentState::from(item.as_ref()));
  let cs_ra = cs.read();
  let ok_is_disabled = !cs_ra.validation_ok();

  InputString {
      caption: "Binance Id",
      value: cs_ra.binance_instr_id.clone().unwrap_or_default(),
      on_input: move |v| {
          let v = v.trim();
          cs.write().binance_instr_id = if v.is_empty() { None } else { Some(v.to_lowercase()) };
          cs.write().binance_check = None; // clear status
      },
  }
  ```

## 4) Inputs and events
- Use the shared input components (`InputString`, `input_i64`, `InputBool`, etc.) and pass `EventHandler` callbacks that update the signal-backed state.
- Keep handlers lightweight: update state, then early-return on invalid input; avoid inline heavy work.
- For Enter/keyboard behaviors, use the provided `on_enter_pressed` in inputs when needed.
- **Example: numeric input with `InputValue`**
  ```rust
  let input_accuracy = input_i64(
      "Accuracy",
      &cs_ra.accuracy,
      EventHandler::new(move |v| cs.write().accuracy = v),
  );
  ```

## 5) Async actions in dialogs
- Wrap network or long work in `spawn` to avoid blocking UI.
- Set boolean flags before/after the await (e.g., `is_checking_binance`) to disable buttons and show progress text.
- On success, update state (e.g., auto-fill accuracy). On failure, store a status message in state for rendering.
- **Example: availability check (abstract)**
  ```rust
  let on_check = {
      move |_| {
          let item_id = cs.read().item_id.clone().unwrap_or_default();
          if item_id.trim().is_empty() { return; }

          spawn({
              let item_id = item_id.clone();
              async move {
                  {
                      let mut s = cs.write();
                      s.is_checking_binance = true;
                      s.binance_check = None;
                  }
                  let resp = crate::api::items::check_availability(item_id).await;
                  let mut s = cs.write();
                  s.is_checking_binance = false;
                  s.binance_check = resp.ok();
              }
          });
      }
  };
  ```

## 6) Data loading — DataState + `get_data` pattern

**Every** piece of async data uses `DataState<T>` and a `get_data` helper function. Never call API directly in a component body or trigger loading via manual `loading: bool` fields.

The helper handles all four states and triggers loading automatically on first render (`None` branch spawns the fetch):

```rust
#[derive(Default)]
struct MyListState {
    data: DataState<Vec<MyHttpModel>>,
}

#[component]
fn MyList(some_id: i64) -> Element {
    let cs = use_signal(MyListState::default);
    let cs_ra = cs.read();

    let items = match get_my_data(cs, &cs_ra, some_id) {
        Ok(d) => d,
        Err(el) => return el,
    };
    rsx! { /* render items */ }
}

fn get_my_data<'a>(
    mut cs: Signal<MyListState>,
    cs_ra: &'a MyListState,
    some_id: i64,
) -> Result<&'a [MyHttpModel], Element> {
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

**Forced reload after mutation**: call `.reset()` on `DataState` — it returns to `None`, and the next render triggers a fresh load automatically.

```rust
// After save/delete — just reset, get_data will reload on next render
cs.write().data.reset();
```

If the reset is triggered from **outside** the component (e.g., parent after a mutation), the `DataState` must be accessible from the caller: either lift it into the parent's state or pass `Signal<ChildState>` as a prop.

## 7) Component structure for pages with tabs and lists

- Each **tab** is its own `#[component]` receiving an ID prop and owning its `DataState`.
- Each **list within a tab** (e.g., active positions + pending orders) is also a separate component with its own `DataState`.
- **Page-level state** holds only UI: current tab, search input, selected item, flags — **never data arrays**.

```
PageComponent              ← PageState: input, selected item, tab, ui flags only
└── ContentComponent
    ├── TabA { id }        ← own State + DataState, get_data pattern
    │   ├── ListOne { id } ← own State + DataState
    │   └── ListTwo { id } ← own State + DataState
    ├── TabB { id }        ← own State + DataState
    └── TabC { id, cs }    ← uses parent Signal if parent needs to trigger reset
```

When a child tab needs to be reloaded from the parent (e.g., balance history after deposit), either:
- Lift the `DataState` into the parent's state and pass `Signal<PageState>` to the tab, or
- Pass `Signal<ChildState>` as a prop so the parent can call `.reset()` directly.

## 8) Server functions as API boundary (fullstack)
- Use Dioxus fullstack server functions (`#[get]`, `#[post]` in `src/api/*`) for all client <-> server calls; they compile to RPCs on web and direct calls on server.
- Keep them thin: fetch app context, perform storage/NoSQL ops, return typed models (`InstrumentHttpModel`, etc.).
- Prefer `Result<T, ServerFnError>`; let the client handle loading/error rendering via `DataState`.
- Gate server-only code behind `#[cfg(feature = "server")]` when needed.
- **Example: save endpoint**
  ```rust
  #[post("/api/instruments/save")]
  pub async fn save_instrument(value: InstrumentHttpModel) -> Result<(), ServerFnError> {
      let app_ctx = crate::server::APP_CTX.get().await;
      let writer = app_ctx.get_instruments();
      writer.insert_or_replace_entity(&InstrumentMyNoSqlEntity::from(value)).await.unwrap();
      Ok(())
  }
  ```

## 9) Dialog template usage
- Provide `header`, optional `header_content`, main `content`, optional `ok_button`, and `allocate_max_space` when needed.
- Cancel/close is built in; for custom OK, pass a button element to `ok_button`.
- The close "X" uses the dialog context; no per-dialog wiring required.
- **Example: template with OK**
  ```rust
  DialogTemplate {
      header: "Edit asset".into(),
      header_content: None,
      content: rsx! { /* form content */ },
      allocate_max_space: None,
      ok_button: rsx! {
          button {
              class: "btn btn-success",
              disabled: ok_is_disabled,
              onclick: move |_| { on_ok.call(cs.read().unwrap_as_http_model()); },
              "Save"
          }
      }
  }
  ```

## 10) Signal handling tips
- Signals are `Copy`; capture once in handlers. Only clone when moving into async blocks.
- Avoid nested `cs.clone()` layers unless a separate handle is truly needed.
- Read with `.read()` for an immutable snapshot; write with `.write()` to mutate.
- **Example**
  ```rust
  let on_click = {
      move |_| {
          let current = cs.read().field.clone();
          spawn({
              async move { cs.write().field = current; }
          });
      }
  };
  ```

## 10.1) Client-side "now" date/time
- **Rule**: If "now" date/time must be resolved on the **client side** in a Dioxus fullstack app, use `dioxus_utils::now_date_time()`.
- This avoids server-side resolution and keeps client-local time semantics correct.

## 11) Status messaging
- Store transient statuses (availability checks, errors) in the form state and render inline near the related control.
- Clear stale statuses when the input they depend on changes.
- **Example**
  ```rust
  if let Some(status) = cs_ra.binance_check.clone() {
      span {
          class: if status.available { "text-success" } else { "text-danger" },
          { status.message.unwrap_or_else(|| "OK".into()) }
      }
  }
  ```

## 12) `use` imports in server functions — always inside the function body

In server functions (`#[get]`, `#[post]`), put **all `use` imports inside the function body**, not at the file level. Top-level imports cause `unused import` warnings on the web target where `feature = "server"` is disabled.

```rust
// ✅ CORRECT — imports inside function, no warnings on web target
#[get("/api/swap-profiles/get")]
pub async fn get_swap_profiles() -> Result<Vec<SwapProfileModel>, ServerFnError> {
    use std::collections::HashMap;
    use crate::margin_engine_grpc::SwapProfileGrpcModel;
    use crate::server::APP_CTX;
    // ...
}

// ❌ WRONG — top-level imports cause unused warnings on web target
use std::collections::HashMap;
use crate::margin_engine_grpc::SwapProfileGrpcModel;

#[get("/api/swap-profiles/get")]
pub async fn get_swap_profiles() -> Result<Vec<SwapProfileModel>, ServerFnError> { ... }
```

## 13) GET server functions — query parameters in route

`#[get]` endpoints pass parameters via query string, not request body. You **must** declare each parameter in the route string using `?param` syntax — otherwise the parameter will not be extracted from the URL.

```rust
// ✅ CORRECT — parameter declared in route, extracted from query string
#[get("/api/accounts/get?account_id")]
pub async fn get_account(account_id: i64) -> Result<Option<AccountModel>, ServerFnError> {
    // account_id is extracted from ?account_id=123
}

// ✅ CORRECT — multiple query params
#[get("/api/orders/search?account_id&status")]
pub async fn search_orders(account_id: i64, status: String) -> Result<Vec<OrderModel>, ServerFnError> {
    // ...
}

// ✅ CORRECT — no params, no query string needed
#[get("/api/instruments/get")]
pub async fn get_all_instruments() -> Result<Vec<InstrumentModel>, ServerFnError> {
    // ...
}

// ❌ WRONG — param in signature but NOT in route, will not be received
#[get("/api/accounts/get")]
pub async fn get_account(account_id: i64) -> Result<Option<AccountModel>, ServerFnError> {
    // account_id is always 0 / default — never extracted
}
```

**Rule:** `#[post]` sends parameters in the request body — no `?param` needed. Only `#[get]` requires this.

## 14) `NotifyChildComponent<TValue>` — parent-to-child notification

Use when a parent action (e.g. deposit, save) must trigger a child component to reload its own `DataState`, and the child manages its state independently (not in the parent's state).

`NotifyChildComponent<TValue>` is in `dioxus_utils`. It wraps a `Signal<Option<TValue>>` and is `Copy + Clone`, so it can be passed as a component prop.

**Parent** — create once with `new()`, pass to child as prop, call `notify_other_components(value)` after mutation:

```rust
// In parent component body (hook context):
let notify_balance = NotifyChildComponent::<()>::new();

// Pass to child:
ChildComponent { notify_balance }

// After mutation (e.g. deposit success):
notify_balance.notify_other_components(());
```

**Child** — call `on_notify(callback)` as a hook. Internally sets up `use_effect` that fires when the signal changes, consumes the value, and runs the callback:

```rust
#[component]
fn ChildComponent(notify_balance: NotifyChildComponent<()>) -> Element {
    let mut cs = use_signal(ChildState::default);

    notify_balance.on_notify(move |_| {
        cs.write().data.reset(); // triggers get_data reload on next render
    });

    let cs_ra = cs.read();
    let items = match get_data(cs, &cs_ra) { ... };
    rsx! { /* render */ }
}
```

**Key properties:**
- `NotifyChildComponent` holds `Signal<Option<TValue>>` — `Copy`, safe to pass as prop
- `on_notify` must be called at component top level (it wraps `use_effect`)
- The notification is consumed once — child's `use_effect` fires, clears the value, runs callback
- After `.reset()` on `DataState`, the `get_data` helper sees `None` and spawns a reload automatically

## 15) `dialog_template` / `dialog_template_ex` — standard dialog wrapper

All dialogs use `dialog_template` (or `dialog_template_ex` for custom size) instead of inlining modal HTML. This keeps dialog structure consistent and eliminates boilerplate.

```rust
// Standard size
super::dialog_template(title, content, ok_button)

// Custom size (e.g. modal-xl for wide dialogs)
super::dialog_template_ex(title, content, ok_button, Some("modal-xl"))
```

**Pattern** — compute title, build `content` and `ok_button` as `rsx!` blocks, then delegate:

```rust
#[component]
pub fn EditInstrumentDialog(instrument: Rc<InstrumentModel>, on_ok: EventHandler<InstrumentModel>) -> Element {
    let mut cs = use_signal(|| EditState::from(instrument.as_ref()));
    let cs_ra = cs.read();

    let title = if cs_ra.is_new { "New Instrument" } else { "Edit Instrument" };

    let content = rsx! { /* form inputs */ };

    let ok_button = rsx! {
        button {
            class: "btn btn-success",
            disabled: !cs_ra.is_valid(),
            onclick: move |_| {
                let model = cs.read().to_model();
                consume_context::<Signal<super::DialogState>>().set(super::DialogState::None);
                on_ok.call(model);
            },
            "Save"
        }
    };

    super::dialog_template(title, content, ok_button)
}
```

**When loading data** — pass the loading/error element as `content` with empty `ok_button`:

```rust
let data = match get_data(cs, &cs_ra) {
    Ok(d) => d,
    Err(el) => return super::dialog_template(title, el, rsx! {}),
};
```

Cancel button and close (×) are built into `dialog_template` — no need to add them.

## 16) State methods over inline handler logic

**Rule**: Put domain logic into methods on the state struct. Event handlers call these methods via a single `cs.write().method(...)` — never inline the logic directly in closures.

**Why:**
- **Testable**: methods on plain structs are unit-testable without Dioxus runtime
- **Atomic**: one `.write()` call updates all related fields together — no risk of partial state between multiple `.set()` calls or multiple `.write()` accesses
- **Readable**: handler becomes a one-liner, state struct documents the contract
- **Reliable**: with separate signals and multiple `.set()` calls inside a handler, Dioxus may not batch updates correctly across different signals — one render cycle may see only a subset of changes. A single state struct with one `.write()` guarantees all fields update atomically.

```rust
// ✅ CORRECT — logic in state methods, handlers are one-liners

struct TpSlInputState {
    tp_price: String,
    tp_pct: String,
    sl_price: String,
    sl_pct: String,
}

impl TpSlInputState {
    fn set_tp_price(&mut self, val: String, open_price: f64, is_buy: bool, leverage: f64) {
        self.tp_price = val.clone();
        if val.is_empty() {
            self.tp_pct = String::new();
        } else {
            self.tp_pct = price_to_pct_str(&val, open_price, is_buy, true, leverage);
        }
    }

    fn set_tp_pct(&mut self, val: String, open_price: f64, is_buy: bool, accuracy: usize, leverage: f64) {
        self.tp_pct = val.clone();
        if val.is_empty() {
            self.tp_price = String::new();
        } else {
            self.tp_price = pct_to_price_str(&val, open_price, is_buy, true, accuracy, leverage);
        }
    }
}

// Handler — one line, all updates are atomic
oninput: move |evt: Event<FormData>| {
    input_state.write().set_tp_price(evt.value(), open_price, is_buy, leverage);
},

// ❌ WRONG — inline logic, multiple .set() calls on separate signals
oninput: move |evt: Event<FormData>| {
    let val = evt.value();
    tp_price_str.set(val.clone());
    tp_pct_str.set(price_to_pct_str(&val, open_price, is_buy, true, leverage));
},
```

**When to extract a method:**
- Handler updates **2+ fields** that are logically coupled (e.g., price ↔ percent, status + error)
- The same update logic is used in **multiple handlers** (e.g., `oninput` and `onkeydown` for the same field)
- The logic is **non-trivial** (conditionals, calculations, validation)

**Testing**: since methods are on a plain struct, unit tests don't need Dioxus:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_tp_price_updates_pct() {
        let mut state = TpSlInputState { tp_price: String::new(), tp_pct: String::new(), .. };
        state.set_tp_price("1.10".into(), 1.0, true, 1.0);
        assert_eq!(state.tp_price, "1.10");
        let pct: f64 = state.tp_pct.parse().unwrap();
        assert!((pct - 10.0).abs() < 0.01);
    }

    #[test]
    fn empty_price_clears_pct() {
        let mut state = TpSlInputState { tp_price: "1.10".into(), tp_pct: "10.00".into(), .. };
        state.set_tp_price(String::new(), 1.0, true, 1.0);
        assert_eq!(state.tp_price, "");
        assert_eq!(state.tp_pct, "");
    }
}
```

**Also applies to client-side Dioxus projects** — the pattern is framework-agnostic and works identically with `dioxus/web` (WASM-only) apps.

## 17) Controlled inputs — use cloned values, not format strings

When binding input values to state, **clone values before the `rsx!` block** and pass them directly. Do not use format-string interpolation with signals.

```rust
// ✅ CORRECT — read state, clone values, pass directly
let is_ra = input_state.read();
let tp_price_val = is_ra.tp_price.clone();
let tp_pct_val = is_ra.tp_pct.clone();
drop(is_ra);

rsx! {
    input { value: tp_price_val, oninput: move |evt| { ... } }
    input { value: tp_pct_val, oninput: move |evt| { ... } }
}

// ❌ WRONG — format-string interpolation with signals may not trigger cross-input updates
rsx! {
    input { value: "{tp_price_str}", oninput: move |evt| { ... } }
    input { value: "{tp_pct_str}", oninput: move |evt| { ... } }
}
```

**Why:** Format-string bindings (`"{signal}"`) read signals individually. When one handler updates multiple separate signals, Dioxus may not correctly batch DOM updates for all affected inputs. Cloned values from a single state struct avoid this issue entirely.
