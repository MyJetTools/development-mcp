# MyNoSql Pattern

MyNoSql is a distributed cache with a local copy. The client subscribes to a table and reads data **locally** — no network requests at read time. The central server is used only for synchronization.

## Entity Structure

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

## AppContext — NoSql Readers

```rust
pub struct AppContext {
    pub sessions_reader: Arc<MyNoSqlDataReaderTcp<SessionEntity>>,
    pub asset_pairs_dict: Arc<MyNoSqlDataReaderTcp<AssetPairMyNoSqlEntity>>,
}

impl AppContext {
    pub async fn new(settings_reader: Arc<SettingsReader>, service_context: &ServiceContext) -> Self {
        Self {
            // Generic type is inferred from the field type — no explicit annotation needed
            sessions_reader: service_context.get_ns_reader(),
            asset_pairs_dict: service_context.get_ns_reader(),
        }
    }
}
```

## Initial Snapshot — service-sdk waits for it

`MyNoSqlDataReaderTcp` connects via TCP and receives a table snapshot asynchronously. Between creating the reader and receiving the first snapshot, there is a delay. If you read data before the snapshot arrives — you get empty results.

`service_context.start_application()` closes that gap for you. It starts the MyNoSql connection first and waits until every reader handed out by `get_ns_reader` has received its first snapshot. Only then is the app marked as initialized and everything else started — background timers and the queues, events loops and background executors created through `service_context`, then the Service Bus client and the HTTP and gRPC servers. So no request, message, timer tick or handler of those queues, events loops and background executors ever runs against a table that is not loaded yet.

```rust
// ✅ CORRECT — gRPC/HTTP handler, SB subscriber, timer tick: the snapshot is already there
let instruments = app.instruments_reader.get_by_partition_key("i");

// ❌ WRONG — redundant, service-sdk has already waited for this reader
app.instruments_reader.wait_until_first_data_arrives().await;
let instruments = app.instruments_reader.get_by_partition_key("i");
```

**NEVER** call `wait_until_first_data_arrives()` on a reader obtained through `get_ns_reader`.

What follows from that wait:

- **Before `start_application()` the readers are empty** — the connection is not started yet. Nothing can be read from a reader inside `AppContext::new()` or anywhere in `main.rs` above `start_application()`. `start_application()` returns only on shutdown, so the first read is possible inside a handler, a subscriber or a timer tick — not on the line after `start_application().await`. Calling `wait_until_first_data_arrives()` before `start_application()` hangs forever.
- While the wait lasts the HTTP port is closed, `/api/isalive` included. A service whose MyNoSql server is unreachable never becomes alive.
- There is no timeout. Every 5 seconds the console prints which table is still being waited for: `MyNoSql readers are not initialized: table '<table>' has no data yet - start of application is delayed`.
- An empty or not yet created table does not block the start — the server answers the subscription with an empty snapshot.
- Only readers obtained through `get_ns_reader` are waited for. A reader taken directly from `service_context.my_no_sql_connection` is not — that is the one case where you call `wait_until_first_data_arrives()` yourself, in the handler, subscriber or timer tick that reads it first.
- A cache filled from a reader callback is not covered either. The wait ends when the reader's own copy of the table is loaded; the callbacks registered with `assign_callback` are delivered after that, one at a time, from the reader's own events loop, and the full-reload pattern adds a `tokio::spawn` on top. So the first request, message or timer tick can find such a cache empty. Until the cache has been loaded once, read from the reader — it holds the data by then (`get_my_no_sql_entity_patterns`, «The cache is filled after the reader»).
