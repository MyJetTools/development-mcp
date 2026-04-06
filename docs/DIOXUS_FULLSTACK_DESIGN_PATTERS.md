---
alwaysApply: false
---
# Dioxus Fullstack Design Patterns

Applies to Dioxus **fullstack** projects (shared code server/web). For common Dioxus patterns (component structure, state management, dialogs, DataState, etc.) see **dioxus-design-patterns.md**.

## 1) Shared models (client + server)

- If a struct is used both on server **and** client → `src/models/`.
- If a struct is server-only (e.g., parsing external API) → private in `src/api/*.rs`, gated with `#[cfg(feature = "server")]`.
- **Naming**: Models crossing the wire use `HttpModel` suffix: `BalanceHistoryHttpModel`.
- **One file per type**: `models/balance_history.rs`. `mod.rs` uses `mod x; pub use x::*;`.
- In components: `use crate::models::*;` — never enumerate types.

## 2) Mappers (server-only)

Mappers know about gRPC/server contracts — live in `src/server/`.

- **1:1 mapping** → `impl From<GrpcModel> for HttpModel`
- **Complex mapping** → function in `src/server/mappers/`

```rust
impl From<BalanceHistoryGrpcModel> for BalanceHistoryHttpModel {
    fn from(i: BalanceHistoryGrpcModel) -> Self {
        Self { id: i.id, delta: i.delta, balance_after: i.balance_after, comment: i.comment, moment: i.moment }
    }
}

// api — clean
Ok(items.into_iter().map(|i| i.into()).collect())
```

## 3) Server functions as API boundary

- Use `#[get]`, `#[post]` in `src/api/*` for all client ↔ server calls.
- Keep them thin: fetch app context, perform storage/NoSQL ops, return typed models.
- Prefer `Result<T, ServerFnError>`.
- Gate server-only code behind `#[cfg(feature = "server")]`.

```rust
#[post("/api/instruments/save")]
pub async fn save_instrument(value: InstrumentHttpModel) -> Result<(), ServerFnError> {
    use crate::server::APP_CTX;
    let app_ctx = APP_CTX.get().await;
    let writer = app_ctx.get_instruments();
    writer.insert_or_replace_entity(&InstrumentMyNoSqlEntity::from(value)).await.unwrap();
    Ok(())
}
```

## 4) `use` imports in server functions — always inside the function body

Top-level imports cause `unused import` warnings on the web target where `feature = "server"` is disabled.

```rust
// ✅ CORRECT — imports inside function
#[get("/api/swap-profiles/get")]
pub async fn get_swap_profiles() -> Result<Vec<SwapProfileModel>, ServerFnError> {
    use std::collections::HashMap;
    use crate::margin_engine_grpc::SwapProfileGrpcModel;
    use crate::server::APP_CTX;
    // ...
}

// ❌ WRONG — top-level imports cause warnings on web target
use std::collections::HashMap;
```

## 5) GET server functions — query parameters in route

`#[get]` endpoints pass parameters via query string. Declare each parameter with `?param` syntax:

```rust
// ✅ CORRECT — param in route
#[get("/api/accounts/get?account_id")]
pub async fn get_account(account_id: i64) -> Result<Option<AccountModel>, ServerFnError> { ... }

// ✅ Multiple params
#[get("/api/orders/search?account_id&status")]
pub async fn search_orders(account_id: i64, status: String) -> Result<Vec<OrderModel>, ServerFnError> { ... }

// ❌ WRONG — param in signature but NOT in route
#[get("/api/accounts/get")]
pub async fn get_account(account_id: i64) -> ... // account_id is always 0
```

`#[post]` sends parameters in request body — no `?param` needed.

## 6) Dialog template and form patterns — see common patterns

`dialog_template` / `dialog_template_ex` is a **common pattern** for all Dioxus projects. See **dioxus-design-patterns.md §9** for full documentation.

## 7) Form state management (admin projects)

Store form state in a signal-backed struct with validation methods:

```rust
let mut cs = use_signal(|| EditInstrumentState::from(item.as_ref()));
let cs_ra = cs.read();
let ok_is_disabled = !cs_ra.validation_ok();
```

Use `InputValue<T>` for string/numeric fields with `value_is_valid()` and `get_value()`.

## 8) Async actions in dialogs

Wrap network calls in `spawn`. Set boolean flags before/after the await:

```rust
spawn(async move {
    cs.write().is_checking = true;
    let resp = crate::api::items::check(id).await;
    let mut s = cs.write();
    s.is_checking = false;
    s.check_result = resp.ok();
});
```

## 9) Client-side "now" date/time

If "now" must be resolved on the **client side**: use `dioxus_utils::now_date_time()`.

## 10) Status messaging

Store transient statuses in form state. Clear stale statuses when the input they depend on changes.
