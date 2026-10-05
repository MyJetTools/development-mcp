# Data ownership patterns

## Step 1 — who owns the data?

| Ownership | Identity | What this gives you |
|---|---|---|
| **System-owned** | 1 ID (`ObjectId`) | Reference data, system events. PK by ObjectId; per-user fields are indexed values, not identity. |
| **User-owned** | 2 IDs (`UserId + ObjectId`) | Multi-tenant data per user. `partition_key = UserId, row_key = ObjectId` in MyNoSQL; composite PK in Postgres. Authorization checks (requestor = owner) are mandatory. |

**Implications:**
- System-owned: reads can be optimized for any access pattern via indexes; per-user scans cost an index lookup.
- User-owned: per-user scan is the natural primary access (`get_by_partition_key(user_id)`); cross-user scan is fan-out and unusual.
- Persist-queue keys for user-owned data must include the UserId — otherwise enqueued mutations from different users can clobber each other.

## Step 2 — is the data size-bounded?

| | Description |
|---|---|
| **Bounded** | Cardinality and total volume capped; full set fits in memory of every potential consumer; safe to replicate via TCP reader. |
| **Unbounded** | Grows with time / volume / users; cannot be held in memory of consumers; needs queryable backend. |

**How to decide bounded vs unbounded:**

1. **First — assess the nature of the data.** Is the maximum cardinality intrinsically capped by business logic? List of tradable instruments = tens-to-hundreds (bounded by nature). Audit log = grows forever (unbounded by nature). Active sessions = capped by online clients (bounded). Order history per account = grows over time (unbounded).
2. **If nature isn't obvious — fallback threshold is 1000 records.** Comparable to or below 1000 — treat as bounded. Substantially above — unbounded.
3. **Whenever the 1000 fallback is invoked**, file a **mandatory tech-debt entry** to revisit once usage data accumulates. The fallback is a placeholder, not a final answer.

**Time-decaying data with retention** (candles for N hours, logs for N days) — no special rule. Apply the same heuristic. Retention alone doesn't make data automatically bounded; estimate `retention × write_rate` and compare against the 1000 threshold and the consumer memory budget.

**User-owned + per-user bounded by enforcement.** When a user-owned domain could in principle grow per user (orders, watchlists, alerts), the architect should impose an **explicit application-level cap** at design time — e.g., "max 100 open orders per account". This guarantees the per-user partition stays small regardless of usage pattern, and the data fits the bounded → MyNoSQL path. Designing for "what if a user generates 100K records" is a sign the cap wasn't agreed with product/business — limits go in explicitly.

## Storage decision matrix

|   | Bounded | Unbounded |
|---|---|---|
| **System-owned (1 ID)** | MyNoSQL replicated cache + TCP reader. Every consumer holds the full set. | Postgres with single PK; access via gRPC, paginated/filtered. |
| **User-owned (2 IDs)** | MyNoSQL with `partition_key=UserId, row_key=ObjectId`. Per-user load small, replicates or reads-by-partition. Per-user cap enforced at write-path. | Postgres with composite PK `(user_id, object_id)`, indexed by `user_id`. Hot-state via in-memory + persist queue. |

Read-models very often translate unbounded domain data into bounded views (top-N, last-day, summary) — that's exactly what makes them cheap to consume.

## Pagination-aware caching for history data

When a read-model represents history (logically unbounded), design the **base case as DB-backed**. Then layer caching on top:

- **First page** in pagination terms is bounded by definition — N records (typically "two average screen-fulls" of UI). Fits the bounded → MyNoSQL pattern. Serves the common case "user opens the screen" from memory.
- **Subsequent pages** (rare access — user scrolling back through history) — served directly from DB via gRPC.

**Architectural shape:** one read-model service with **two storage backends** (MyNoSQL for hot window + Postgres for full history) is a **native shape** for this pattern, not a violation of single-responsibility. The service "owns history of X" and knows to serve the first page fast and the rest from DB.

**Sizing decision:** "how much in cache?" = typical screen size × 2 (one screen of headroom). Not a fixed record count — driven by UI, not by storage capacity.

## MyNoSQL JSON cost — factor it in

MyNoSQL serialises records as JSON. For datasets of "millions of small records" with hot-path read patterns, JSON serialise/deserialise overhead becomes significant. MyNoSQL is a good fit for "moderate-cardinality, moderate-record-size, frequently read" data; not for "huge counts of micro-records". When the profile leans towards micro-records at high volume, evaluate a **custom binary cache** as an alternative. This is a heavy hammer; reach for it only when JSON cost is measured to dominate.

## MyNoSQL caching capacity

### Client-context partitions

When a read-model or cache is partitioned by client/user id and published to MyNoSQL:

```
max_partitions_amount = max(2 × peak_concurrent_clients_per_hour, 1000)
```

- **2×** — buffer for churn (clients disconnect and return; their partition must survive that window).
- **floor 1000** — sanity baseline. On low-traffic / dev environments the `2× hourly` count can be tiny; 1000 keeps things robust against sudden spikes. MyNoSQL partitions are cheap.
- **Re-evaluate** when usage profile shifts (active base grows, new client classes appear). Not set-and-forget.

`max_partitions_amount` can be updated after table creation — capacity adjustment is dynamic, not a recreate-table operation.

### System-owned reference (admin-managed)

For system-owned bounded reference data (instruments, swap profiles, exchange configs, any admin-managed catalog):

```
max_partitions_amount = None
```

**Reasoning:** this class of data is generated by **employees** through admin-ui, not by clients through public APIs. The input rate is bounded by human admin throughput, orders of magnitude below any client RPS — no LRU eviction is needed. Setting a limit on admin-managed data is an antipattern: an admin action could evict another admin-managed entry, breaking the integrity of system reference state.

### `max_rows_per_partition_amount`

No fixed default — **decide by data nature.** Architect estimates expected per-partition row count, growth profile, and whether eviction is acceptable. Common settings:
- `None` for partitions with bounded-by-design row count (e.g., one record per `partition_key`).
- A specific number when row count grows but eviction of oldest is acceptable (e.g., last-N pattern).
