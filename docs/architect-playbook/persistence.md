# Persistence, caching, the hot path, audit-log

## Persistence layers

### Postgres
Durable, structured, service-owned state.

- `with_table_schema_verification::<Dto>("table", Some("pk_name"))` at startup. Auto-creates tables/columns/PKs/indexes; **cannot** change column types or tighten NULL→NOT NULL.
- `#[primary_key(N)]` for composite keys; `#[db_index(id, index_name, is_unique, order)]` for multi-column indexes.
- `#[e_tag]` + `concurrent_insert_or_update_single_entity` for concurrent updates.
- `jsonb` arrays via `#[json]` + `#[sql_type("jsonb")]`.
- Single shared connection unless throughput requires a pool. TLS auto via `sslmode=require`. SSH tunnel via `ssh=user@host:port` in conn-string.

### MyNoSQL

Replicated reference and cache state read by many services.

Hard rules:
- Entity defs in the shared entities crate only. **Never** duplicate across crates.
- Writer always via `.with_retries(N).method()`. Direct writer calls are an anti-pattern.
- Reader reads are **sync** (no `.await`). Only `wait_until_first_data_arrives` is async.
- Do not call `wait_until_first_data_arrives` yourself — `service_context.start_application()` waits for the first snapshot of every reader from `get_ns_reader` before it starts timers and the queues, events loops and background executors created through `service_context`, then Service Bus, HTTP and gRPC. A service whose MyNoSQL server is unreachable does not start (no `/api/isalive`).
- `reader.get_by_partition_key` returns `Option<BTreeMap<String, Arc<T>>>` (key = row_key). Use `_as_vec` for just values.
- Reader callbacks: **full reload pattern**, always `tokio::spawn` inside the callback. Never incremental. Such a cache is filled **after** the reader — until it has been loaded once, read from the reader (`get_my_no_sql_entity_patterns`, «The cache is filled after the reader»).

### In-memory + persist queue (hot path)

For state mutated on every request that must not block on DB writes.

Pattern:
1. Single `Mutex` over all in-memory state.
2. Separate `QueueToSaveWithId<Key, Payload>` per persist target.
3. Background handler drains the queue and batches writes to Postgres / NoSQL.
4. Client request returns as soon as state is mutated and key is enqueued. No blocking I/O under the lock.

Wiring:
- Create the queue with `service_context.create_queue_to_save_with_id(...)` and keep the returned `Arc<QueueToSaveWithId<Key, Payload>>` in `AppContext`.
- Register the handler (`register_events_handler`) before `start_application()` — a queue without one panics when it is started.
- Do not start the queue yourself — `start_application()` does it, and a second `start()` panics.

## Cache policy — pick one of four

| Pattern | When |
|---|---|
| Mutex + persist queue | Write-frequent local state; source of truth in memory; persistence is best-effort durability. |
| In-memory hydrated from MyNoSQL on startup (readers are loaded before `start_application()` starts anything else; a cache filled from reader callbacks lags behind them — read from the reader until it is loaded once) | Service consumes a stream and must apply on top of persisted history. |
| Read-through (no local cache) | Rarely-read state where staleness is unacceptable. |
| Write-through (`.with_retries(3).insert_or_replace_entity`) | Service rarely reads but must publish state visible to others immediately. |

## Hot path constraints

Inviolate rules for any quote/order/trade hot path:

- Do not `await` Postgres/NoSQL writes inside the Mutex lock.
- Do not perform external I/O (HTTP, gRPC) while holding the Mutex.
- Persist via queues, not inline.
- All in-memory state behind one Mutex (one lock acquisition per event).
- For cross-domain reads in hot path: read a **read-model's MyNoSQL projection** via TCP reader, not the domain-owner's gRPC.

## Audit-log vs Logger separation

A **business-critical audit channel** and an **operational log channel** are not the same thing and must not share infrastructure.

| | Logger | Audit-log |
|---|---|---|
| Content | Technical messages: errors, traces, request flows, debug | Business / security-relevant events: who changed what, when, on whose behalf |
| Volume | Can grow **explosively** under failure (bug → every operation logs error → channel saturates) | **Controlled** — bounded by business activity, not by code health |
| Durability | Best-effort; under load, drops are acceptable | Guaranteed; never lose entries |
| Storage | Logger stack (e.g. `my-logger`) | Dedicated audit service / Postgres |

**Antipattern:** "log everything to the logger; we'll filter for audit later." During an incident the logger overflows with technical traces, and the very business events you need are lost.

**Boundary cases** (errors during business operations): emit twice — technical detail to logger, business essence (what / on whose object / who did it) to audit.

## Microsecond timestamps everywhere

All timestamps in services on this stack use **microsecond precision** (`DateTimeAsMicroseconds` from `rust-extensions`). In Postgres, store as either `bigint` (epoch microseconds) or `timestamp` via `#[sql_type("timestamp")]`. Do not default to second/millisecond precision — it's worse for ordering, deduplication, and any composite key that includes a time component.
