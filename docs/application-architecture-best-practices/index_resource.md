---
alwaysApply: true
---
# Application Architecture Best Practices

The rules on this page apply to every line of code in every service. The patterns of each area — AppContext, settings, gRPC, Postgres, MyNoSql, Service Bus, HTTP and the rest — are topics, listed at the end. Before writing code in an area, read its topic: `resource://application-architecture-best-practices/{topic}`, or `get_application_architecture_best_practices` with `topic`.

## Zero Warnings Policy

`cargo clippy -- -D warnings` MUST pass before any code is considered complete.
`cargo fmt --check` MUST pass.

```toml
# .cargo/config.toml
[build]
rustflags = ["-D", "warnings"]
```

```rust
// lib.rs / main.rs
#![deny(warnings)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
```

**NEVER** use `#[allow(clippy::...)]` as a solution.
The ONLY exception: `#[allow(dead_code)]` in tests.

When clippy warns → fix the root cause, not the warning.

```rust
// ❌ WRONG — suppressing the warning
#[allow(clippy::too_many_arguments)]
fn do_something(a: u64, b: u64, c: u64, d: u64, ...) {}

// ✅ CORRECT — fix the root cause
struct DoSomethingParams { a: u64, b: u64, c: u64, d: u64 }
fn do_something(params: DoSomethingParams) {}
```

---

## Named Structs Over Multi-Field Tuples

**NEVER** use multi-field tuples (`(T, T)`, `(T, T, T, T)`) in public APIs — struct fields, function signatures, return types.
**ALWAYS** introduce a named struct with self-documenting field names.

> **WHY:** Positional unpacking `(a, b) = foo()` forces the reader to remember the order and meaning. `Option<(f64, f64)>` could be a point, a range, a pair of before/after values — the name is lost. Named fields (`ScreenPoint { x, y }`, `YRange { min, max }`, `ScreenRect { x, y, w, h }`) document themselves at every use site.

```rust
// ❌ WRONG — readers must remember the tuple order
pub fn draw_crosshair(mouse: (f64, f64)) -> (Option<(f64, f64)>, Option<f64>) { ... }

pub struct ChartState {
    pub quick_order_btn_bounds: Option<(f64, f64, f64, f64)>,
    pub y_range_override: Option<(f64, f64)>,
}

// ✅ CORRECT — named structs, intent is obvious
pub struct ScreenPoint { pub x: f64, pub y: f64 }
pub struct ScreenRect  { pub x: f64, pub y: f64, pub w: f64, pub h: f64 }
pub struct YRange      { pub min: f64, pub max: f64 }

pub struct CrosshairLabels {
    pub ohlc: Option<ScreenPoint>,
    pub date_label_x: Option<f64>,
}

pub fn draw_crosshair(mouse: ScreenPoint) -> CrosshairLabels { ... }

pub struct ChartState {
    pub quick_order_btn_bounds: Option<ScreenRect>,
    pub y_range_override: Option<YRange>,
}
```

**Reuse across modules:** when the same shape appears in several places (points, rects, ranges), lift the struct into a shared module (e.g. `types.rs` / `geom.rs`) rather than redefining per-caller.

**Exceptions — tuples are fine for:**
- Local destructuring: `let (start, end, shift) = compute_slice(...);` inside one function.
- Standard-library / ecosystem pairs: `HashMap::iter() -> (K, V)`, `Result<T, E>`, `Option<(T, U)>` from a zip/split helper.
- Single-purpose internal helpers where the tuple never escapes the function.

**When refactoring:** introduce the struct, update the signature, let the compiler drive every call-site migration — each conversion becomes `SomeStruct { field_a: x, field_b: y }` which is self-documenting at the usage site too.

---

## Match Exhaustiveness — No `_ => {}` on Enums

**NEVER** use `_ => {}` (or `_ => ...` wildcard) when matching on an **enum**.
Always enumerate every variant explicitly.

> **WHY:** A wildcard arm silently swallows variants that don't exist yet. The moment someone adds a new variant to the enum, every `match` with `_ =>` compiles without warning and the new case is ignored at runtime — a whole class of bugs where a message / state / event is dropped on the floor with no trace. Listing every variant forces the compiler to flag every site that must be updated.

```rust
pub enum OrderEvent {
    Placed,
    Filled,
    Cancelled,
}

// ❌ WRONG — adding `Rejected` tomorrow compiles silently
match event {
    OrderEvent::Placed => handle_placed(),
    OrderEvent::Filled => handle_filled(),
    _ => {}
}

// ✅ CORRECT — compiler forces every call-site to handle the new variant
match event {
    OrderEvent::Placed    => handle_placed(),
    OrderEvent::Filled    => handle_filled(),
    OrderEvent::Cancelled => {}
}
```

**When genuinely stuck** — you've tried at least two approaches (e.g. grouping variants with `|`, an explicit `ignore_remaining!` helper, pulling the ignored set into a method on the enum) and still cannot express the intent without a wildcard — **stop and ask the user** before adding `_ =>`. A wildcard on an enum should be a deliberate, discussed exception, never a default.

**Wildcard is fine on open domains.** `&str`, numeric types, bytes, any unknown external tag — the value space is infinite by definition, so a catch-all is required:

```rust
// ✅ Storage key parser — unknown keys are forward-compat noise, skip them
match key {
    "renderer"     => renderer = value.to_string(),
    "candle_type"  => candle_type = value.to_string(),
    _ => {}
}

// ✅ WebSocket msg_id — unknown ids from future server versions
match msg_id {
    "bid_ask"     => BidAsk::parse(payload),
    "instruments" => Instruments::parse(payload),
    _ => Unknown(msg_id.to_string()),
}
```

**Also fine:**
- Matching on `#[non_exhaustive]` enums from external crates — the compiler requires a wildcard; add one with a clear `// _ => {} // non_exhaustive upstream` comment.
- Matching on `Result<T, E>` where `E` is erased / boxed — use `Ok(_) | Err(_)` style rather than `_ =>`.

---

## Logging Levels

Use `my_logger::LOGGER` with the correct level. Choosing the wrong level is a bug.

| Level | Method | When to use |
|---|---|---|
| `FatalError` | `write_fatal_error` | Emitted by **libraries only** — e.g. cannot connect to DB, infrastructure is down. The service cannot function at all. **Never call this from business logic.** |
| `Error` | `write_error` | Business logic failure — something went wrong that should not have, and we need to investigate |
| `Warning` | `write_warning` | Known technical debt — we know it's bad, we know why, it's tolerated for now. Documents intentional shortcuts. |
| `Info` / `Debug` | — | **Never use.** Only added on explicit instruction when debugging a specific issue, then removed. |

```rust
// ✅ Error — SB publish failed, this is unexpected and needs attention
my_logger::LOGGER.write_error(
    "like_unlike",
    format!("{:?}", err),
    LogEventCtx::new().add("amount", items.len().to_string()),
);

// ✅ Warning — known limitation, tracked, tolerated
my_logger::LOGGER.write_warning(
    "recalculate_likes",
    "Skipping duplicate event — idempotency not yet implemented",
    LogEventCtx::new().add("object_id", object_id),
);

// ❌ WRONG — Info/Debug: never add unless explicitly asked for a debugging session
my_logger::LOGGER.write_info("get_user", "Fetching user from DB", LogEventCtx::new());
```

**NEVER** add `Info`/`Debug` logging as a default practice.
**NEVER** call `write_fatal_error` from your own code — it is reserved for infrastructure libraries.

---

## Project Architecture

### Directory Structure

```
src/
├── main.rs
├── app/
│   └── app_ctx.rs
├── flows/          ← entry points from HTTP/gRPC, never call other flows
├── scripts/        ← reusable functions, called from flows/scripts/background
├── postgres/       ← repos + dto (in gRPC services)
├── db/             ← repos + dto (in public-api, legacy)
├── grpc_client/    ← one file per gRPC client
├── grpc_server/    ← gRPC handlers
├── http_server/    ← HTTP controllers, errors.rs
├── settings/
├── background/     ← timers
├── sb_subscribers/ ← service bus subscribers
├── mappers/
└── models/
```

### Business Logic Layers — CRITICAL

> **WHY:** `flows` are the entry points from the API — they know the HTTP/gRPC context.
> They must never call each other, otherwise the boundary of what constitutes an entry point blurs.
> `scripts` are reusable building blocks with no knowledge of transport.
> This separation allows the same logic to be called from HTTP, gRPC, timers, and SB subscribers.

```
HTTP/gRPC handlers
      ↓
   flows/          ← entry points from API. ONE level deep. NEVER call other flows.
      ↓
  scripts/         ← reusable small functions. Can call other scripts.
      ↓
  postgres/ (db/)  ← data access only
```

**NEVER:** business logic directly in HTTP/gRPC handlers.
**NEVER:** flows calling other flows.
**NEVER:** scripts calling flows.
**NEVER:** DB accessed directly from handlers or flows (only via scripts or simple repo calls).

### Public API Has NO DB

> **WHY:** Public API is a BFF (Backend for Frontend) on top of microservices.
> Direct DB access blurs service boundaries and makes independent scaling impossible.
> Each domain (likes, posts, users) must own its data through its own dedicated gRPC service.

Public API (HTTP/gRPC gateway) MUST NOT have direct DB access.
DB lives ONLY in dedicated CRUD gRPC microservices.

```
public-api
    └── flows/ → calls gRPC clients only → dedicated gRPC services
                                               └── postgres/ (Postgres repos)
```

**DO NOT** add new DB repos to public-api AppContext.
**DO NOT** write new flows that call DB directly in public-api.
Any new data access → create/extend a gRPC service first.

---

## Module Export Pattern — Universal

> **WHY:** `pub use x::*` means callers don't need to know which file a struct lives in.
> Files can be renamed or split — the module's public API stays the same.
> `pub use x::SpecificStruct` creates a fragile dependency on the internal file structure.

**Every** `mod.rs` in the project follows this exact pattern:

```rust
mod file_name;
pub use file_name::*;
```

**NEVER:**
```rust
pub use file_name::SpecificStruct;  // ❌
pub mod file_name;                  // ❌ without pub use
```

One `.rs` file = one struct/functionality.
`mod.rs` contains ONLY re-exports, no logic.
Applies to: `grpc_client/`, `postgres/`, `flows/`, `scripts/`, `models/`, `mappers/` — everywhere.

---

## Topics

| Topic | What is inside |
| --- | --- |
| [`app-context`](app-context.md) | `AppContext` field order and naming; in-memory state always behind a dedicated struct; the Inner + Wrapper pattern for related fields under one lock |
| [`settings`](settings.md) | `SettingsModel` with `use_settings!()`, the `GrpcClientSettings` impl ending in `panic!`, never caching settings values |
| [`build-rs`](build-rs.md) | `build.rs` with `CiGenerator` + `ProtoFileBuilder`; the monorepo `build.rs` (proto only) and its two hand-written workflows |
| [`grpc`](grpc.md) | `#[generate_grpc_client]`, registering proto modules in `main.rs`, `generate_server!` handlers (unary, streaming input, streaming output), registration, error handling, proto file conventions with `Ping` |
| [`postgres`](postgres.md) | Repo setup, write strategy (`insert_or_update_db_entity`), read → modify → write, `with_retries` on every call, `.expect("{table}: {operation} failed")`, DTOs, where models, `DateTimeAsMicroseconds` |
| [`flows`](flows.md) | Streaming input flow: collect → bulk write → publish; one timestamp per batch; flows never return `Result` and never call each other |
| [`my-no-sql`](my-no-sql.md) | MyNoSql entity structure, readers in `AppContext`, the initial snapshot `start_application()` waits for — never `wait_until_first_data_arrives()` |
| [`service-context`](service-context.md) | What comes from the `ServiceContext`: publishers, readers, timers, events loops, background executors, queues to save; who starts them and when |
| [`mappers`](mappers.md) | `impl Into<Target> for Source` in `mappers/`, one file per source entity, never standalone mapping functions |
| [`service-bus`](service-bus.md) | Protobuf contracts in `my-sb-contracts`; subscribers with `engage_telemetry`, registration flags, queue semantics, error handling |
| [`http`](http.md) | HTTP actions on service-sdk (`configure_http_server`), thin controllers, response messages without IDs or PII, `write_log`, central domain → HTTP error mapping |
| [`shared-wire-models`](shared-wire-models.md) | The `rest-api-shared` crate for a client-side SPA + standalone REST API: WASM-clean, `server` feature, pure-data models with extension traits |
