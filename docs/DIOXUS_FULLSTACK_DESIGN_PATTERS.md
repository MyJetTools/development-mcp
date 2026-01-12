## Dioxus Fullstack Design Patterns (Project Playbook)

Applies to Dioxus **fullstack** projects (shared code server/web). Use these when adding dialogs, forms, list views, or server functions. Adapt names as needed.

### Naming conventions
- **`cs`** – the mutable signal holding component state (`let mut cs = use_signal(...)`)
- **`cs_ra`** – a read-access snapshot of `cs` (`let cs_ra = cs.read()`)
- Use `cs` for writes (`cs.write().field = value`) and `cs_ra` for reads in the render phase.

### 0) Shared models (client + server)
- **Rule**: If a struct is used both on server **and** client (e.g., returned from a server function and rendered in UI) → put it in `src/models`.
- **Rule**: If a struct is only used inside a server function (e.g., parsing an external API response) → keep it private in the `src/api/*.rs` file, gated with `#[cfg(feature = "server")]`.
- Derive `Serialize`/`Deserialize` for anything crossing the wire; keep structs minimal and web-safe.
- If a model is used in `http_route` responses (server-only), also derive `MyHttpObjectStructure` when required by your HTTP doc tooling; otherwise `Serialize`/`Deserialize` is enough.
- **Examples**:
  - `BinanceInstrumentCheckResponse` → in `src/models` (returned to client, shown in dialog)
  - `BinanceExchangeInfo`, `BinanceSymbolInfo` → private in `src/api/binance.rs` with `#[cfg(feature = "server")]` (only used to parse Binance API response)
  - `InputValue<T>` → in `src/models` (shared validation helper used in components)

### 1) Dialogs: lifecycle and rendering
- Keep a global `DialogState` in context (`Signal<DialogState>`). Define variants per dialog (`Confirmation`, `EditInstrument`, etc.).
- Render all dialogs centrally via `RenderDialog`, matching on `DialogState` and embedding the concrete dialog component.
- Use `DialogTemplate` for consistent header, close “X”, cancel button, and optional OK slot.
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

### 2) Opening dialogs from views
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

### 3) Form state management
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

### 4) Inputs and events
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

### 5) Async actions in dialogs
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

### 6) Data loading lists
- Use the `DataState`/`RenderState` pattern: start `None`, set `Loading`, fire `spawn` to fetch, then `set_value` or `set_error`.
- After a mutation (save/delete), call `data.reset()` to force a reload through the existing load logic.
- Filter/search by keeping the search string in state and applying it before rendering rows.
- **Example: load on first render**
  ```rust
  if matches!(state.data.as_ref(), RenderState::None) {
      spawn(async move {
          state.write().data.set_loading();
          match crate::api::instruments::get_instruments().await {
              Ok(items) => state.write().data.set_value(items.into_iter().map(Rc::new).collect()),
              Err(err) => state.write().data.set_error(err),
          }
      });
      return loading();
  }
  ```

### 7) Server functions as API boundary (fullstack)
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

### 8) Dialog template usage
- Provide `header`, optional `header_content`, main `content`, optional `ok_button`, and `allocate_max_space` when needed.
- Cancel/close is built in; for custom OK, pass a button element to `ok_button`.
- The close “X” uses the dialog context; no per-dialog wiring required.
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

### 9) Signal handling tips
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

### 10) Status messaging
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