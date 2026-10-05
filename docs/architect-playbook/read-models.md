# Read-model design

## When to extract a read-model — eagerly, because they're cheap to throw away

**Default: almost always extract.** Whenever the requirement involves aggregation, a different shape than the domain object, a cross-domain view, or any read-side workload that doesn't trivially fit a domain-owner's CRUD — make it its own read-model service.

**Why eagerly:**
- Read-models are designed to be **deletable**. When a use case disappears (UI is removed, product pivots, metrics are retired), the read-model service is stopped, its tables dropped, its SB queue deleted, and **it's as if it never existed**. State is never canonical there — the source of truth is the domain-owner, the read-model is just a derived projection.
- The cost of mixing read-shaped methods into a domain-owner is **permanent**. The domain-owner's gRPC API inflates with view methods; its codebase mixes write logic with view logic; future evolution is harder. That coupling doesn't go away when the use case does.
- Adding a small service is operationally cheap: one repo entry, one Dockerfile, one deployment pipeline, one `Permanent` SB queue.

**The rare merge case:** if a read truly is just CRUD on the domain object, with no aggregation, projection, or cross-domain join — it isn't a "read-model"; it's a domain-owner CRUD method. Keep it on the domain-owner.

**Decision filter:**
- *Does this read have a different shape than the domain object?* (top-N, summary, joined view, time-windowed, projected) — **extract**.
- *Does this read combine multiple domains?* — **extract** (and only a read-model is allowed to do that; see the domain isolation rule, topic `domain-boundaries`).
- *Is it just `Get` / `List` of the domain object as-is?* — **method on the domain-owner**.

## Output mode is driven by size

> **Bounded read-model + fits in memory** — publish to **MyNoSQL**; consumers read via TCP `MyNoSqlReader` (sync local).
> **Unbounded read-model** — store in **Postgres**; consumers read via **gRPC** at the read-model service.

| | Output | Consumer access | Recovery model |
|---|---|---|---|
| **Bounded** (fits in memory of every consumer) | MyNoSQL replicated cache | TCP reader, sync local read (a short `parking_lot` mutex on the in-memory copy, no I/O) | Consumers always see last-known-good even if read-model service is down |
| **Unbounded** (doesn't fit) | Postgres in the read-model service | gRPC with filters/pagination | Consumers depend on the read-model service availability |

Architect must **size the projection first** before any other design step on a read-model. Output mode determines storage choice, consumer pattern, recovery semantics, deployment cardinality.

If a projection can be compressed into a bounded view (top-N, last-day, summary), prefer that — bounded output gives lower latency and less operational coupling.

## Recovery / cold-start with history

Three approaches, in preference order:

1. **SB replay from event 0.** If the topic retains full history, the new read-model creates its queue with "start from beginning" semantics, consumes all events, builds state. Cleanest when applicable.
   *Works when:* topic retains full history. *Doesn't work when:* topic has TTL, was recreated, or the volume is unworkable.

2. **Prod-first + post-start init script.** Deploy and start the production read-model — it begins consuming SB live, building forward state. Then run a one-shot init (Rust binary, Python script, `cargo run` — whatever's convenient) that backfills historical data from the source of truth (domain-owner's Postgres). Init filters by time (`WHERE created_at < <service_start_time>`) so init data and live data don't collide. Init is throwaway, not part of permanent infrastructure.
   *Works when:* SB doesn't retain enough history but a queryable historical source exists.

3. **Accept "from now on".** New read-model just starts; no history. Fine for current-state projections (live open positions, current instrument map). Not fine for audit-shaped views.

What we **don't** do: build a permanent `GetAllEvents` / `StreamHistorical` gRPC on the domain-owner (clutters API for one rare use case); standing snapshot infrastructure (over-engineered for rare cold-starts).

## Domain-owner gRPC API

A domain-owner's gRPC API is **primarily a set of semantic methods** with expressive names that convey intent. CRUD primitives exist underneath but are not the primary call surface.

- **Semantic methods** are the default: `OpenPosition`, `CancelOrder`, `UpdatePassword`, `ChangeEmail`, `IncrementBalance`, `SuspendAccount`, etc.
- **Reading guideline:** a developer should understand what a method does **from its name alone**, without diving into the implementation. If the name doesn't carry the intent, rename.
- **CRUD primitives** (`Get`, `List`, `Upsert`, `Delete`) exist for read-side queries and special administrative paths (backfill, recovery, debug). Generic `Upsert` is **not** the standard path for production callers — they call semantic methods.
- **Internally,** each semantic method usually implements `load → modify → upsert` against the persistence layer. The semantic name lives at the API boundary; load-modify-upsert lives inside.

Read-models are introduced **only** when there's an actual reason: aggregation, cross-domain view, projection of different shape, or scaling reads. "Plain reads of own state" don't justify a separate read-model service.

## Business invariants live in semantic methods

Domain integrity rules — "balance can't go below maintenance margin", "order transitions from `Pending` to `Filled / Cancelled / Rejected` only", "≤ N open orders per account", "can't open a position on a non-tradable instrument" — are enforced **inside the semantic gRPC method on the domain-owner**, never in the caller and never via diff-validation on a generic Upsert.

Why semantic-method enforcement, not the alternatives:

- **Not caller-side** — even with strict architecture, callers eventually have bugs, mistakes, or new entry points that bypass the rules. The domain-owner is the only place the rule can be guaranteed.
- **Not diff-based on a generic Upsert** — the validation is indirect (you'd have to compute a diff to see what changed and pattern-match against per-field rules). Adding a new invariant means editing a giant diff-validator instead of editing the method that semantically owns the rule. Easy to miss cases when adding new fields.

The semantic-method approach has a known cost: invariants shared across multiple methods (e.g., "balance ≥ maintenance margin" applies to `OpenPosition`, `IncreasePosition`, `Withdraw`) risk copy-paste. The mitigation is to extract such invariants into shared validator functions in a domain-internal module — but the **call site stays in the semantic method**, not in upsert middleware.
