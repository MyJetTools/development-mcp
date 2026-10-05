# AppContext Pattern

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

## In-memory State — always a dedicated struct

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

## Inner + Wrapper pattern (multiple related fields)

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
