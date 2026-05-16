---
alwaysApply: true
---
# Application Architecture Best Practices

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

## AppContext Pattern

```rust
pub struct AppContext {
    // 1. gRPC clients (alphabetical)
    pub chats_grpc_client: ChatsGrpcClient,
    pub users_grpc_client: UsersGrpcClient,

    // 2. DB repos
    pub likes_repo: LikesRepo,

    // 3. In-memory state
    pub ws_sockets: AppWebSockets,
    pub cover_etags: EtagCaches,

    // 4. Settings reader (last)
    pub settings_reader: Arc<SettingsReader>,
}

impl AppContext {
    pub async fn new(settings_reader: Arc<SettingsReader>, _service_ctx: &ServiceContext) -> Self {
        Self {
            users_grpc_client: UsersGrpcClient::new(settings_reader.clone()),
            likes_repo: LikesRepo::new(settings_reader.clone()).await,
            // ...
            settings_reader,
        }
    }
}
```

**Naming:**
- gRPC client field: `{service_name}_grpc_client`
- Repo field: `{entity_name}s` (plural) or `{entity}_repo`

**NEVER** put business logic in AppContext.
**NEVER** put per-request state in AppContext.
**NEVER** put raw `RwLock<HashMap<...>>` or other raw concurrent containers directly in AppContext — always wrap in a dedicated struct with its own module.

### In-memory State — always a dedicated struct

> **WHY:** A raw `RwLock<HashMap<K, V>>` in AppContext is hard to read and impossible to extend.
> A dedicated struct gives a clear name, encapsulates locking, and allows adding methods
> (e.g. `update`, `get_all`, `remove`) without touching AppContext.

```rust
// ❌ WRONG — raw container in AppContext
pub struct AppContext {
    pub bid_ask_cache: RwLock<HashMap<String, BidAskModel>>,
}

// ✅ CORRECT — dedicated struct in its own module
// src/bid_ask_cache/bid_ask_cache.rs
pub struct BidAskCache {
    data: RwLock<HashMap<String, BidAskModel>>,  // private field
}

impl BidAskCache {
    pub fn new() -> Self {
        Self { data: RwLock::new(HashMap::new()) }
    }

    pub async fn update(&self, key: String, value: BidAskModel) {
        self.data.write().await.insert(key, value);
    }

    pub async fn get_all(&self) -> Vec<BidAskModel> {
        self.data.read().await.values().cloned().collect()
    }
}

// AppContext just holds the struct
pub struct AppContext {
    pub bid_ask_cache: BidAskCache,
}
```

One struct = one module (`src/{name}/mod.rs` + `src/{name}/{name}.rs`).
The internal container (`RwLock`, `Mutex`, `DashMap`) is **always private** — callers use methods.

### Inner + Wrapper pattern (multiple related fields)

When a struct has **multiple related fields** behind a lock, use the Inner+Wrapper pattern:

- **Inner** — plain struct, all fields without locks. All logic via `&mut self` / `&self` — borrow checker guarantees consistency.
- **Wrapper** — single `RwLock<Inner>` field. Methods are thin async delegates: acquire lock → call Inner method → return.

> **WHY:** Separate locks per field cause race conditions between related data.
> A single lock on Inner guarantees atomic operations across all fields.
> Inner is testable synchronously without async runtime.

```rust
// src/order_book_subscribers/order_book_subscribers_inner.rs
pub(super) struct OrderBookSubscribersInner {
    subscriptions: HashMap<String, HashSet<i64>>,
    connection_instrument: HashMap<i64, String>,
}

impl OrderBookSubscribersInner {
    pub(super) fn subscribe(&mut self, connection_id: i64, instrument_id: String) {
        // All logic here — borrow checker enforces consistency
    }

    pub(super) fn unsubscribe_connection(&mut self, connection_id: i64) { ... }
    pub(super) fn get_subscribers(&self, instrument_id: &str) -> Vec<i64> { ... }
}
```

```rust
// src/order_book_subscribers/order_book_subscribers.rs
pub struct OrderBookSubscribers {
    inner: RwLock<OrderBookSubscribersInner>,
}

impl OrderBookSubscribers {
    pub async fn subscribe(&self, connection_id: i64, instrument_id: String) {
        self.inner.write().await.subscribe(connection_id, instrument_id);
    }

    pub async fn get_subscribers(&self, instrument_id: &str) -> Vec<i64> {
        self.inner.read().await.get_subscribers(instrument_id)
    }
}
```

```rust
// src/order_book_subscribers/mod.rs
mod order_book_subscribers;
mod order_book_subscribers_inner;

pub use order_book_subscribers::OrderBookSubscribers;
```

**Module structure — always a folder:**
```
order_book_subscribers/
├── mod.rs                            — mod + pub use
├── order_book_subscribers.rs         — Wrapper (pub struct)
└── order_book_subscribers_inner.rs   — Inner (all logic)
```

**Rules:**
- **NEVER** separate `RwLock`/`Mutex` per field — one lock for all related data
- **One lock acquisition** per operation — no dropping and re-acquiring between related writes
- Inner is `pub(super)` — invisible outside the module
- Wrapper contains **zero business logic** — only concurrency management

---

## Settings Pattern

```rust
// settings.rs
use service_sdk::macros::use_settings!();  // ← ALWAYS first

#[derive(
    my_settings_reader::SettingsModel,
    AutoGenerateSettingsTraits,
    SdkSettingsTraits,
    Serialize, Deserialize, Debug, Clone,
)]
pub struct SettingsModel {
    pub seq_conn_string: String,
    pub my_telemetry: Option<String>,
    pub postgres_conn_string: String,
    pub users_grpc_url: String,   // ← {service_name}_grpc_url for each client
}

// GrpcClientSettings impl — always in settings.rs
impl GrpcClientSettings for SettingsReader {
    async fn get_grpc_url(&self, name: &'static str) -> GrpcUrl {
        if name == UsersGrpcClient::get_service_name() {
            return self.use_settings(|s| s.users_grpc_url.clone().into()).await;
        }
        // one if per client
        panic!("Unknown grpc service name: {}", name)  // ← REQUIRED at end
    }
}
```

**NEVER** read settings directly from fields — only via `use_settings(|s| ...)`.
**NEVER** cache settings values — `SettingsReader` auto-refreshes every 30 seconds.
**ALWAYS** end `get_grpc_url` with `panic!` — catches missing client registrations.

---

## build.rs Pattern

Every service has `build.rs` that does two things:

```rust
fn main() {
    // 1. Generate Dockerfile + GitHub CI
    CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_basic_service()
        .generate_github_ci_file()
        // .with_ci_test()  ← ONLY if project has #[test] somewhere
        .build();

    // 2. Sync + compile proto files from shared repo
    ProtoFileBuilder::new("../proto-files/")
        .sync_and_build("Users.proto")
        .sync_and_build("Payments.proto");
}
```

**NEVER** add `.with_ci_test()` unless project has actual unit tests.
**ALWAYS** pass `env!("CARGO_PKG_NAME")` — never hardcode service name.
**NEVER** copy proto files manually into the service.
**NEVER** use `tonic_build` directly — always via `ci_utils::ProtoFileBuilder`.
Proto files live in a **shared repo** (`../proto-files/`), not in the service itself.

### Monorepo build.rs

In a monorepo (multiple services in one GitHub repo), **do NOT use `CiGenerator`**. Dockerfile and CI workflows are created manually per service. `build.rs` only syncs + compiles proto files:

```rust
fn main() {
    ci_utils::ProtoFileBuilder::new("../proto-files/")
        .sync_and_build("MyService.proto");
}
```

---

## gRPC Client Pattern

```rust
// grpc_client/users.rs
service_sdk::macros::use_grpc_client!();  // ← ALWAYS first line

#[generate_grpc_client(
    proto_file = "./proto/Users.proto",   // ← = not :
    crate_ns: "crate::users_grpc",        // ← : not =
    retries: 3,
    request_timeout_sec: 1,
    ping_timeout_sec: 1,
    ping_interval_sec: 3,
)]
pub struct UsersGrpcClient;
```

**NEVER** write gRPC client boilerplate manually.
**NEVER** add `impl` blocks on top of generated client.
**NEVER** manual imports — `use_grpc_client!()` handles everything.

`crate_ns` must exactly match the `mod` declaration in `main.rs`.

### Every proto file must be registered in main.rs

For every proto file used (client or server), add a module at the top of `main.rs`:

```rust
// main.rs
pub mod users_grpc {
    tonic::include_proto!("users");  // ← matches `package` in Users.proto
}

pub mod likes_grpc {
    tonic::include_proto!("likes");  // ← matches `package` in Likes.proto
}
```

The string in `include_proto!` must match the `package` name in the `.proto` file.
The `mod` name must match `crate_ns` in `#[generate_grpc_client]` / `generate_server!`.

```
Users.proto:          package users;
main.rs:              pub mod users_grpc { tonic::include_proto!("users"); }
grpc_client/users.rs: crate_ns: "crate::users_grpc"
```

---

## gRPC Server Pattern

```rust
// grpc_server/likes_grpc_service.rs
service_sdk::macros::use_grpc_server!();  // ← ALWAYS first

generate_server!(proto_file:"./proto/Likes.proto", crate_ns: "crate::likes_grpc",);

// 1. Simple request → response (trivial logic → inline in handler)
async fn get_amount(app: &Arc<AppContext>, request: GetAmountGrpcRequest) -> GetAmountGrpcResponse {
    let amount = app.likes_repo.get_count_by_object(request.tp, &request.object_id).await;
    GetAmountGrpcResponse { amount }
}

// 2. Streaming input → delegate to flow
async fn like_unlike(app: &Arc<AppContext>, request: StreamedRequestReader<LikeUnlikeGrpcRequest>) {
    crate::flows::like_unlike(app, request).await;
}

// 3. Streaming output → tokio::spawn + flow with producer
async fn get_user_likes(
    app: &Arc<AppContext>,
    request: GetUserLikesGrpcRequest,
) -> StreamedResponseWriter<GetUserLikesGrpcResponse> {
    let response_writer = StreamedResponseWriter::new(1024);
    let producer = response_writer.get_stream_producer();
    // ALWAYS tokio::spawn for streaming output — return stream handle immediately
    tokio::spawn(crate::flows::get_user_likes(app.clone(), request.tp, request.user_id, producer));
    response_writer  // ← return immediately, before spawn completes
}

// Flow with streaming — read from DB as stream, push to producer
pub async fn get_user_likes(
    app: Arc<AppContext>,
    tp: i32,
    user_id: String,
    mut producer: StreamedResponseProducer<GetUserLikesGrpcResponse>,
) {
    // ALWAYS query_rows_as_stream — never Vec for potentially large data
    let mut db_stream = app.likes_repo.get_likes_by_user(tp, &user_id).await;

    while let Some(dto) = db_stream.get_next().await {
        let response: GetUserLikesGrpcResponse = dto.into();
        producer.send(response).await;
    }
}
```

**Handlers are plain `async fn`**, not `impl` methods. First arg always `app: &Arc<AppContext>`.

When to delegate to flow vs inline:
- One repo/gRPC call + simple mapping → inline in handler
- Multiple calls, business logic, conditions → flow

**ALWAYS** `#[with_telemetry]` on every gRPC server handler.
**NEVER** `tokio::spawn` for non-streaming handlers (await directly).

### Error Handling in gRPC

> **WHY:** gRPC has no `HttpFailResult`. Business errors are returned via optional fields
> in the response (None = not found, empty list = empty result). Panicking on IO is correct —
> the gRPC server catches it, returns status INTERNAL, and logs it.

**Business errors → optional response fields**, not panic and not Result:
```rust
// ✅ CORRECT — None means "not found", the client understands this
async fn get_user(app: &Arc<AppContext>, request: GetUserGrpcRequest) -> GetUserGrpcResponse {
    let user = app.users_repo.get(&request.user_id).await;
    GetUserGrpcResponse {
        user: user.map(|u| u.into()),  // None if not found
    }
}

// ❌ WRONG — panicking on a business error
let user = app.users_repo.get(&request.user_id).await
    .expect("user must exist");  // business error, not IO
```

**IO errors → panic** (`.expect()`), the gRPC framework catches it and returns status INTERNAL.

**Logging in gRPC handlers** — via telemetry context, not directly through `my_logger::LOGGER`:
```rust
// ctx comes from #[with_telemetry] — logs are automatically bound to the request
async fn get_order(app: &Arc<AppContext>, request: GetOrderGrpcRequest, ctx: &MyTelemetryContext)
    -> GetOrderGrpcResponse {
    let result = app.orders_repo.get(&request.order_id, ctx).await;
    // ...
}
```

---

## Proto File Pattern

```protobuf
syntax = "proto3";
package likes;           // snake_case, matches filename lowercase

import "google/protobuf/empty.proto";

// Message naming: {Description}GrpcRequest / {Description}GrpcResponse
message LikeUnlikeGrpcRequest {
  int32 tp = 1;           // tp = type discriminator (one service, multiple entity types)
  string user_id = 2;
  string object_id = 3;
  bool like = 4;
}

service Likes {           // PascalCase, matches filename
  // Streaming input → Empty (fire and forget / batch)
  rpc LikeUnlike(stream LikeUnlikeGrpcRequest) returns (google.protobuf.Empty);

  // Unary
  rpc GetAmountByObjectId(GetAmountByObjectIdGrpcRequest) returns (GetAmountGrpcResponse);

  // Streaming output
  rpc GetUserLikes(GetUserLikesGrpcRequest) returns (stream GetUserLikesGrpcResponse);

  // REQUIRED in every service
  rpc Ping(google.protobuf.Empty) returns (google.protobuf.Empty);
}
```

**EVERY** proto service MUST have `Ping`.
**NEVER** create a custom empty message instead of `google.protobuf.Empty`.
**NEVER** change field numbers on existing fields (breaks compatibility).
**NEVER** skip field numbers without reason.

`tp` field pattern: one service handles multiple entity types distinguished by `int32 tp`.

---

## DB / Postgres Pattern

```rust
// postgres/likes_repo.rs
service_sdk::macros::use_my_postgres!();  // ← ALWAYS first

pub const TABLE_NAME: &str = "likes";
pub const PK_NAME: &str = "likes_pk";

pub struct LikesRepo {
    postgres: MyPostgres,  // ← private field
}

impl LikesRepo {
    pub async fn new(settings_reader: Arc<SettingsReader>) -> Self {
        let postgres = MyPostgres::from_settings(APP_NAME, settings_reader)
            .with_table_schema_verification::<LikeDto>(TABLE_NAME, Some(PK_NAME.into()))
            .build()
            .await;
        Self { postgres }
    }
}
```

### Write Strategy — CRITICAL

**DEFAULT:** `insert_or_update_db_entity` — for all create and update operations.
Reason: idempotent = safe retries.

**EXCEPTION:** `insert_db_entity_if_not_exists` — ONLY for registration/unique entity creation
where the business answer is "already exists or not".

**NEVER:** raw `insert_db_entity` for regular writes — no duplicate protection on retries.

> Both `insert_or_update_db_entity` and `bulk_insert_or_update_db_entity` take `UpdateConflictType` as the **second positional argument**. The standard value is `UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into())`. Import the type: `use service_sdk::my_postgres::UpdateConflictType;`.

### Update Pattern — Read → Modify → Write

```rust
// ✅ CORRECT
let mut entity = self.postgres
    .with_retries(3, Duration::from_secs(1))
    .query_single_row(TABLE_NAME, Some(&where_model), Some(ctx))
    .await
    .expect("entities: query_single_row get failed");
entity.status = NewStatus;
entity.updated_at = DateTimeAsMicroseconds::now();
self.postgres
    .with_retries(3, Duration::from_secs(1))
    .insert_or_update_db_entity(
        TABLE_NAME,
        UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()),
        &entity,
        Some(ctx),
    )
    .await
    .expect("entities: insert_or_update_db_entity failed");

// ❌ WRONG — partial update
// UPDATE SET field=value WHERE id=x  ← not retry-safe

// ❌ WRONG — no retries on write
self.postgres
    .insert_or_update_db_entity(
        TABLE_NAME,
        UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()),
        &entity,
        Some(ctx),
    )
    .await
    .expect("...");
```

### Retries — always use with_retries

```rust
// ✅ CORRECT — read
self.postgres
    .with_retries(3, Duration::from_secs(1))
    .query_single_row(TABLE_NAME, Some(&where_model), Some(ctx))
    .await
    .expect("chats: query_single_row get_by_participants failed");

// ✅ CORRECT — write
self.postgres
    .with_retries(3, Duration::from_secs(1))
    .insert_or_update_db_entity(
        TABLE_NAME,
        UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()),
        &entity,
        Some(ctx),
    )
    .await
    .expect("chats: insert_or_update_db_entity upsert failed");

// ✅ CORRECT — bulk write
self.postgres
    .with_retries(3, Duration::from_secs(1))
    .bulk_insert_or_update_db_entity(TABLE_NAME, UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()), items, Some(ctx))
    .await
    .expect("likes: bulk_insert_or_update_db_entity failed");

// ❌ WRONG — no retries on any operation
self.postgres
    .query_single_row(TABLE_NAME, Some(&where_model), Some(ctx))
    .await
    .expect("...");
```

**ALWAYS** `.with_retries(3, Duration::from_secs(1))` before every DB operation.
**ALWAYS** pass telemetry context `Some(ctx)` — everywhere, always.
**NEVER** pass `None` for telemetry context.

### Error Handling in DB / Postgres

> **WHY:** `MyHttpServer`, `GrpcServer`, `Postgres` — all implement retries internally.
> If an IO error reaches your code, all retry attempts are exhausted — there is nothing to do.
> Panicking is correct: the server logs it, returns 500/INTERNAL, and keeps running.
> Hiding the error with `?` or `unwrap_or` means continuing in an undefined state.

```rust
// ✅ CORRECT — read with retries + telemetry
self.postgres
    .with_retries(3, Duration::from_secs(1))
    .query_single_row(TABLE_NAME, Some(&where_model), Some(ctx))
    .await
    .expect("likes: query_single_row get_count_by_object failed");
    //       ↑ format: "{table}: {operation} failed"

// ✅ CORRECT — write with retries + telemetry
self.postgres
    .with_retries(3, Duration::from_secs(1))
    .insert_or_update_db_entity(
        TABLE_NAME,
        UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()),
        &entity,
        Some(ctx),
    )
    .await
    .expect("likes: insert_or_update_db_entity upsert failed");

// ❌ WRONG — propagating IO error up the call stack
let result = self.postgres.query_single_row(...).await?;

// ❌ WRONG — silently swallowing the error
let result = self.postgres.query_single_row(...).await.unwrap_or_default();

// ❌ WRONG — no retries on write
self.postgres.insert_or_update_db_entity(
    TABLE_NAME,
    UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()),
    &entity,
    Some(ctx),
).await.expect("...");
```

**NEVER** return `Result` from repo methods — only `.expect()`.
**ALWAYS** `.expect("{table}: {operation} failed")` — table name + method name in the message.

### Repo Methods — always `pub async fn`, never return Result

```rust
// ✅ CORRECT
pub async fn bulk_insert(&self, items: &[LikeDto], ctx: &MyTelemetryContext) {
    self.postgres
        .with_retries(3, Duration::from_secs(1))
        .bulk_insert_or_update_db_entity(
            TABLE_NAME,
            UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()),
            items,
            Some(ctx),
        )
        .await
        .expect("likes: bulk_insert_or_update_db_entity failed");
}

// ❌ WRONG — no retries, no ctx, returns Result
pub async fn bulk_insert(&self, items: &[LikeDto]) -> Result<(), Error>
```

### DTO Structure

```rust
// postgres/dto.rs
service_sdk::macros::use_my_postgres!();  // ← ALWAYS first, even in dto files

#[derive(SelectDbEntity, InsertDbEntity, UpdateDbEntity, Debug, TableSchema)]
pub struct LikeDto {
    #[primary_key(0)]
    #[generate_where_model("DeleteLikeWhereModel")]  // ← generate WhereModel from PK fields
    #[db_index(id:0, index_name:"like_by_object_id_idx", is_unique:true, order:"ASC")]
    pub tp: i32,

    #[primary_key(1)]
    #[generate_where_model("DeleteLikeWhereModel")]
    pub user_id: String,

    #[sql_type("timestamp")]
    pub moment: DateTimeAsMicroseconds,  // ← ALWAYS DateTimeAsMicroseconds, ALWAYS timestamp
}
```

Derive all four when table supports CRUD: `SelectDbEntity + InsertDbEntity + UpdateDbEntity + TableSchema`.

`#[generate_where_model]` on PK fields — preferred over manual WhereModel struct when types match.

### Where Models

Separate `WhereDbModel` struct when:
- Type differs from DTO (e.g. `Vec<T>` for IN queries)
- Synthetic fields not in table
- Operators (`>`, `<`, `LIKE`)
- Optional range filters

```rust
// Vec = IN ($1, $2, ...)
#[derive(WhereDbModel)]
pub struct GetByStatusesWhere {
    pub status: Vec<i32>,
}

// Use &'s str not String — zero-copy
#[derive(WhereDbModel)]
pub struct GetByUserIdWhereModel<'s> {
    pub tp: i32,
    pub user_id: &'s str,  // ← &str not String
}
```

**Naming:** `{Description}WhereModel`

### All DTOs and WhereModels in one dto.rs per repo

**NEVER** separate files for each DTO.
**ALWAYS** one `dto.rs` per repo file, all related structs inside.

### Datetime — always DateTimeAsMicroseconds

```rust
// ✅ CORRECT
#[sql_type("timestamp")]
pub created_at: DateTimeAsMicroseconds,

// ❌ WRONG
pub created_at: i64,  // chrono, SystemTime, etc.
```

---

## Flow Pattern

### Streaming Input: Collect → Process → Publish

> **WHY:** A gRPC stream is a batch of messages from one client in a single call.
> Collecting everything into a Vec → one bulk insert is far more efficient than N individual DB calls.
> Unlike streaming output, **no `tokio::spawn` needed** here — the handler just awaits the flow,
> and the gRPC framework keeps the connection open until the flow returns.

```rust
pub async fn like_unlike(
    app: &Arc<AppContext>,
    mut request: StreamedRequestReader<LikeUnlikeGrpcRequest>,
) {
    // Phase 1: collect — fix timestamp once for entire batch
    let now = DateTimeAsMicroseconds::now();  // ← ONE call before the loop — fixes timestamp for entire batch
    let mut to_insert = Vec::new();
    let mut to_delete = Vec::new();
    let mut to_publish = Vec::new();

    while let Some(item) = request.get_next().await {
        let item = item.unwrap();  // ← stream item: unwrap, not ? (stream errors are fatal)
        // distribute to vecs
    }

    // Phase 2: bulk DB — ALWAYS check len() > 0
    if !to_insert.is_empty() {
        app.likes_repo.bulk_insert(to_insert.as_slice()).await;
    }
    if !to_delete.is_empty() {
        app.likes_repo.bulk_delete(to_delete.as_slice()).await;
    }

    // Phase 3: publish (NOT a panic — log and continue)
    if !to_publish.is_empty() {
        if let Err(err) = app.publisher.publish_messages(to_publish.iter().map(|i| (i, None))).await {
            my_logger::LOGGER.write_error(
                "like_unlike",
                format!("{:?}", err),
                LogEventCtx::new().add("amount", to_publish.len().to_string()),
            );
        }
    }
}
```

**NEVER** call `DateTimeAsMicroseconds::now()` inside the loop — fix time once per batch.
**NEVER** return `Result` from a flow.
**NEVER** flow calls another flow.

---

## MyNoSql Pattern

MyNoSql is a distributed cache with a local copy. The client subscribes to a table and reads data **locally** — no network requests at read time. The central server is used only for synchronization.

### Entity Structure

```rust
service_sdk::macros::use_my_no_sql_entity!();  // ← ALWAYS first

#[my_no_sql_entity("bid-ask-snapshot")]  // ← table name
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BidAskSnapshotNoSqlEntity {
    // partition_key and row_key injected by macro — never declare them
    pub moment: i64,
    pub bid: f64,
    pub ask: f64,
}

impl BidAskSnapshotNoSqlEntity {
    pub fn generate_partition_key() -> &'static str { "s" }
    pub fn generate_row_key(instrument_id: &'static str) -> &'static str { instrument_id }
    pub fn get_instrument_id(&self) -> &str { &self.row_key }  // ← row_key injected by macro
}
```

### AppContext — NoSql Readers

```rust
pub struct AppContext {
    pub sessions_reader: Arc<MyNoSqlDataReaderTcp<SessionEntity>>,
    pub asset_pairs_dict: Arc<MyNoSqlDataReaderTcp<AssetPairMyNoSqlEntity>>,
}

impl AppContext {
    pub async fn new(settings_reader: Arc<SettingsReader>, service_context: &ServiceContext) -> Self {
        Self {
            // Generic type is inferred from the field type — no explicit annotation needed
            sessions_reader: service_context.get_ns_reader().await,
            asset_pairs_dict: service_context.get_ns_reader().await,
        }
    }
}
```

### ServiceContext — What Comes From Where

| What | Source |
|---|---|
| `MyServiceBusPublisher<T>` | `service_ctx.get_sb_publisher(true).await` |
| `MyNoSqlDataReaderTcp<T>` | `service_ctx.get_ns_reader().await` |
| gRPC client | `XxxGrpcClient::new(settings_reader.clone())` |
| DB repo | `XxxRepo::new(settings_reader.clone()).await` |

**NEVER** store `ServiceContext` in `AppContext`.
`service_ctx` lives in `main.rs` until `start_application()` is called — used to register SB subscribers after AppContext is created.

### Waiting for Initial Snapshot — CRITICAL

`MyNoSqlDataReaderTcp` connects via TCP and receives a table snapshot asynchronously. Between creating the reader and receiving the first snapshot, there is a delay. If you read data before the snapshot arrives — you get empty results.

Method `wait_until_first_data_arrives()` blocks until the first snapshot is received:

```rust
// ✅ CORRECT — wait for snapshot before reading
app.instruments_reader.wait_until_first_data_arrives().await;
let instruments = app.instruments_reader.get_by_partition_key("i").await;

// ❌ WRONG — reading immediately, data may be empty
let instruments = app.instruments_reader.get_by_partition_key("i").await;
```

**ALWAYS** call `wait_until_first_data_arrives()` before the first read from a reader (typically in `scripts/init.rs`).

---

## Mappers Pattern

Mappers live in `mappers/` — one file per domain entity.
One file = conversions for one type (e.g. `posts.rs` maps all post-related types).

### Always `impl Into<Target> for Source` — never standalone functions

```rust
// mappers/posts.rs
use crate::posts_grpc::*;
use crate::postgres::*;

impl Into<PostGrpcModel> for PostDto {
    fn into(self) -> PostGrpcModel {
        PostGrpcModel {
            creator_id: self.creator_id,
            id: self.id,
            publish_from: self.publish_from.unix_microseconds,  // ← DateTimeAsMicroseconds → i64
            likes_amount: self.likes_amount.unwrap_or_default(),  // ← Option → default
        }
    }
}
```

### Usage in flows/handlers:
```rust
// Clean — type system handles conversion
let grpc_model: PostGrpcModel = post_dto.into();
let models: Vec<PostGrpcModel> = dtos.into_iter().map(|x| x.into()).collect();
```

### Rules:
> **WHY:** `impl Into` is idiomatic Rust. The compiler applies the conversion automatically
> wherever the target type is expected. Standalone functions require explicit calls everywhere
> and pollute the namespace. `Into` keeps flows/handlers clean: `dto.into()` instead of `map_dto_to_grpc(dto)`.

- `impl Into<Target> for Source` — ALWAYS, never `fn map_post_to_grpc(dto: PostDto) -> PostGrpcModel`
- One mapper file per source entity
- `Option<T>` → `.unwrap_or_default()` in the mapper — when NULL in DB has clear business semantics (NULL likes = 0 likes). This is domain logic expressed in the mapper.
- `DateTimeAsMicroseconds` → `i64`: use `.unix_microseconds` field directly
- NEVER put mapping logic in flows, scripts, or handlers — always in `mappers/`

---

## Service Bus Contract Pattern

SB contracts live in a shared crate `my-sb-contracts` — used by both publisher and subscriber services.

```rust
// my-sb-contracts/src/like_unlike.rs
use service_sdk::my_service_bus;
use service_sdk::my_service_bus::macros::my_sb_entity_protobuf_model;

#[derive(Clone, PartialEq, ::prost::Message)]
#[my_sb_entity_protobuf_model(topic_id = "like-unlike")]  // ← topic name here
pub struct LikeUnlikeSbContract {
    #[prost(int32, tag = "1")]
    pub tp: i32,
    #[prost(string, tag = "2")]
    pub user_id: String,
    #[prost(string, tag = "3")]
    pub object_id: String,
    #[prost(bool, tag = "4")]
    pub like: bool,
}
```

### Rules:
- SB contracts live in a **shared crate** (`my-sb-contracts`) — never define them in the service itself
- Serialization: **protobuf** via `prost` — `#[derive(prost::Message)]` + field tags
- Topic ID defined in macro: `#[my_sb_entity_protobuf_model(topic_id = "topic-name")]`
- Field tags: sequential `1, 2, 3...` — **NEVER change existing tags** (breaks deserialization)
- **NEVER add `serde`** to SB contracts — protobuf only
- Naming: `{Action}SbContract` — e.g. `LikeUnlikeSbContract`, `UserCreatedSbContract`

---

## Service Bus Subscriber Pattern

```rust
// sb_subscribers/likes_sb_subscriber.rs
use service_sdk::my_service_bus::prelude::*;

pub struct LikesSbSubscriber {
    app: Arc<AppContext>,
}

impl LikesSbSubscriber {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl SubscriberCallback<LikeUnlikeSbContract> for LikesSbSubscriber {
    async fn handle_messages(
        &self,
        messages_reader: &MessagesReader<LikeUnlikeSbContract>,
    ) -> Result<(), MySbSubscriberHandleError> {
        while let Some(mut next_message) = messages_reader.get_next_message().await {
            let sb_msg = next_message.take_message();

            // Engage telemetry BEFORE calling script
            let ctx = next_message.engage_telemetry().await;

            crate::scripts::handle_event(&self.app, sb_msg, &ctx).await;
        }
        Ok(())
    }
}
```

### Registration in main.rs — after AppContext::new()

```rust
let app = Arc::new(AppContext::new(settings_reader, &service_context).await);

// Register AFTER app is created, BEFORE start_application.
//
// `register_sb_subscribe` is SYNCHRONOUS (returns `&Self`, not a future) and takes
// three positional arguments: (callback, delete_on_no_subscribers: bool, single_connection: bool).
// There is no `TopicQueueType` argument despite the old enum still existing internally
// in `my-service-bus-abstractions`.
service_context.register_sb_subscribe(
    Arc::new(LikesSbSubscriber::new(app.clone())),
    /* delete_on_no_subscribers */ true,
    /* single_connection         */ false,
);

service_context.start_application().await;
```

### Queue semantics — pick the right (delete_on_no_subscribers, single_connection) pair:

| Intent | Flags | Old enum equivalent |
|---|---|---|
| Queue deleted when no subscribers — for events where only freshness matters (presence, notifications) | `(true, false)` | `DeleteOnDisconnect` |
| Queue persistent, multi-connection | `(false, false)` | `Permanent` |
| Queue persistent, exclusive connection — for critical events that must not be lost on a reconnect | `(false, true)` | `PermanentWithSingleConnection` |

### Rules:
- `next_message.engage_telemetry().await` — ALWAYS before calling scripts/flows
- Subscriber delegates to `scripts/` (not `flows/`) — there is no HTTP/gRPC context here
- NEVER business logic directly in `handle_messages` — routing and telemetry only
- `while let Some(mut next_message)` — always `mut`, required for `take_message()` and `engage_telemetry()`
- Filter by type (`sb_msg.tp`) in subscriber — different types → different script calls

### Error Handling in Service Bus

> **WHY:** An SB subscriber cannot return an error to a client — there is no client.
> IO errors (DB, gRPC) → panic, same as everywhere. But SB publish failures inside a handler
> → log and continue, because the data is already written to DB.

**IO errors (DB, gRPC calls) → panic** — same as everywhere:
```rust
// ✅ CORRECT
app.posts_repo.update(post).await.expect("posts: update failed");
```

**SB publish failure inside handler → log and continue**:
```rust
// ✅ CORRECT — data is already in DB, losing an SB message is non-critical
if let Err(err) = app.publisher.publish_messages(items).await {
    my_logger::LOGGER.write_error(
        "recalculate_likes",
        format!("{:?}", err),
        LogEventCtx::new().add("amount", items.len().to_string()),
    );
}
// continue — do NOT panic, do NOT return error

// ❌ WRONG
app.publisher.publish_messages(items).await.expect("publish failed");  // kills already-completed work
```

**Logging** — always with telemetry context from `engage_telemetry()`:
```rust
let ctx = next_message.engage_telemetry().await;
// pass ctx to all script calls — it routes logs through the telemetry pipeline
crate::scripts::recalculate(&self.app, sb_msg, &ctx).await;
```

---

## HTTP Action Pattern

Read `HTTP Actions Design Guide` from MCP before writing HTTP actions.

```rust
// http_server/controllers/{group}/{action_name}_action.rs
use service_sdk::macros::use_my_http_server!();

#[http_route(
    method: "POST",
    route: "/api/likes/v1/like-unlike",
    controller: "Likes",
    summary: "Like or unlike an object",
    description: "...",
    input_data: "LikeUnlikeInputModel",
    authorized: Yes,
    result: [
        {status_code: 200, description: "Ok"},
        {status_code: 401, description: "Unauthorized"},
    ]
)]
pub struct LikeUnlikeAction {
    app: Arc<AppContext>,
}

async fn handle_request(
    action: &LikeUnlikeAction,
    input_data: LikeUnlikeInputModel,
    _ctx: &HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    crate::flows::like_unlike(&action.app, input_data).await;
    HttpOutput::Empty.into_ok_result(true).into()
}
```

**Controllers are thin** — one line delegating to flow.
**NEVER** business logic directly in `handle_request`.

### Error Handling in HTTP

> **WHY:** The HTTP response is seen by the client — the message should be clear but without internal IDs or PII.
> Logs are seen by developers — full context via `LogEventCtx`, no PII.
> `write_log` controls log noise: do not log what abusive traffic can generate at scale.

**HTTP response message** — short, no internal IDs, no PII:
```rust
// ✅ CORRECT
HttpFailResult::as_not_found("Order not found", true).into_err()
HttpFailResult::as_bad_request("Invalid email format", false).into_err()
HttpFailResult::as_fatal_error("Payment processing failed").into_err()

// ❌ WRONG — exposing internal IDs to the client
HttpFailResult::as_not_found(format!("Order {} not found for client {}", order_id, client_id), true).into_err()
```

**write_log:**
- `true` — authenticated request, unexpected error, signals a real system problem
- `false` — missing/invalid token, validation errors on public endpoints, 404s from bots/scrapers
- `as_fatal_error` — always logs (implicit)

**Logger context** — full context with IDs, no PII:
```rust
// ✅ CORRECT
my_logger::LOGGER.write_error(
    "get_order",
    "Order not found",
    LogEventCtx::new()
        .add("order_id", order_id.to_string())
        .add("client_id", client_id.to_string()),
);

// ❌ WRONG — PII in log
LogEventCtx::new()
    .add("email", email)    // PII — never log
    .add("phone", phone)    // PII — never log
```

**Centralize domain → HTTP error mapping** in `src/http_server/errors.rs`.
Controllers use `?` only — no inline `map_err`:
```rust
// src/http_server/errors.rs
impl From<OrderError> for HttpFailResult {
    fn from(err: OrderError) -> Self {
        match err {
            OrderError::NotFound    => HttpFailResult::as_not_found("Order not found", true),
            OrderError::InvalidData(msg) => HttpFailResult::as_bad_request(msg, false),
        }
    }
}

// Controller — ? only
let order = get_order(id).await?;
Ok(HttpOutput::as_json(order).into_ok_result(true)?)
```
