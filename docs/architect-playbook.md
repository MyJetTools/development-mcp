---
name: architect
description: Universal architectural playbook for Rust microservice systems built on the service-sdk stack (gRPC + Service Bus + MyNoSQL + Postgres + Dioxus). Use this skill whenever the user asks about sketching a new service, choosing a transport, deciding where data lives, designing an event/queue contract, partitioning data, classifying a service archetype, or any cross-cutting architectural decision. The skill is decision-only — implementation details live in the best-practices MCP (see "When to drill down" at the bottom). Project-specific decisions for any concrete codebase live in a sibling `architectual-considerations.md` (or equivalent), not here.
---

# Architect — universal patterns

Decision-focused playbook for high-level design choices in Rust microservice systems built on the service-sdk stack. Pick the *what* and *where*; pull the *how* from the implementation MCP. Project-specific facts (service inventory, host topology, named services, ID conventions) belong in a sibling project document, not this skill.

## Decomposition philosophy

Two background commitments shape every other rule in this skill:

1. **Always Rust.** Every service in the system is a Rust binary on the `service-sdk` stack. No mixed-language toolchains, no per-service language choice. This skill assumes Rust + Tokio + service-sdk; if you find yourself reaching for "but in Python this would be...", you're in the wrong skill.

2. **Don't economize on microservices.** Spinning up another Rust service is cheap — one crate, one Dockerfile, one deployment workflow, one SB queue. Extracting concerns into separate services is the **default disposition**. The cost of mashing two concerns into one service is permanent (entangled APIs, blurred ownership, harder evolution); the cost of one extra service is operational and recoverable.

   **The only exception** is the [performance-driven domain co-location](#performance-driven-domain-co-location-deliberate-exception) pattern — when the latency budget is measured in microseconds and gRPC overhead would blow it, multiple domains co-locate in one process. This is the **only** justification accepted for merging concerns; "felt like it" / "didn't want a new service" are not.

The rest of this skill is just a careful working-out of these two commitments.

## Service archetypes

Every service in such a system fits one of five archetypes. The archetype drives transport, deployment cardinality, exposure rules, auth model, and what shape the service's API takes.

| Archetype | Audience | Exposure | Inbound | Notes |
|---|---|---|---|---|
| **rest-api** | Browsers / API clients (external) | Internet | HTTP | Public contract stability matters. OpenAPI surface, TLS termination, rate-limit, DDoS defence. Usually thin gateway over `grpc-flows`. |
| **grpc-flows** | Internal services | Internal only | gRPC | Owns or fronts a domain object. Splits further into **domain-owner** and **read-model** (see below). |
| **background-worker** | — | Internal only | Driven by SB subscription / timers / external feeds — no inbound API | No `MyHttpServer`. Failure mode is "data not flowing", not "user gets 5xx". |
| **admin-ui** | Employees | Internal | HTTP (Dioxus fullstack) | Server functions are part of the deployable. Full trust by default; auth is identity, not adversarial. |
| **client-ui** | End-users (external) | Internet | None on backend — browser bundle | Dioxus **CSR-only**; backend is a separate `rest-api` service. **Not a server in backend sense** — just a static bundle + hosting. Don't put server functions here. |

**Implication of the archetype choice:** once it's picked, most other decisions are constrained. Trying to combine archetypes in one service ("a `background-worker` that's also a `rest-api`") is almost always a sign of two separate concerns being collapsed.

### grpc-flows split: domain-owner vs read-model

`grpc-flows` always falls into one of two sub-archetypes:

| Sub-archetype | Owns? | Mutates? | Persistence | Examples of role |
|---|---|---|---|---|
| **domain-owner** | Authoritative state for some domain | Yes (write API + events) | Postgres + e_tag for concurrency; in-memory + persist queue for hot-path | Holds positions, accounts, credentials, instruments |
| **read-model** | Derivative view of one or more domains | No (read-only to consumers) | MyNoSQL (if bounded) or Postgres (if unbounded) | Aggregations, projections, joins across domains, top-N, summaries |

Hard rules that follow from this split are below.

## Domain boundary heuristic — separate by responsibility, not by entity

When deciding whether two pieces of functionality belong in the same domain or in separate ones, ask **"what does this service _do_?"** — not **"what entity does it operate on?"**.

The same entity (user, instrument, account, trade) can legitimately appear in multiple domains. Sharing an entity is **not** a reason to merge domains. Sharing a **responsibility** is.

**Filter question at design time:** *"If responsibility A breaks, can responsibility B keep working?"*
- **Yes** — these are different domains; split them.
- **No** — same domain; keep together.

**Examples of the heuristic in action:**
- A `credentials` service ("authenticate this user") and an `accounts` service ("what trading accounts does this user have") both touch the same `User` entity. They are different domains: identity verification can be down while account inventory is queryable, and vice versa. Split.
- A trading engine that holds `positions`, `orders`, and `balance` for an account. These are three sub-models, but together they form one responsibility: "current trading state of this account". One stops working, the others are meaningless in isolation. Same domain (and possibly co-located if hot-path latency demands).

**Antipattern: god-domain by entity.** Collapsing every concern that touches `User` into a single `users-service` produces a tangled mix of identity / accounts / preferences / sessions / payments / billing — each of which is its own responsibility. The same antipattern shows up with `instruments-service`, `trades-service`, etc. The fix: ask the filter question; if responsibilities are independent, split.

**Boundary cases:**
- "We share a Postgres table between two services." That's almost always a sign the split is wrong (or that one of the two needs to consume from the other via gRPC / events, not via shared storage). Domains don't share schemas.
- "Both services need this entity's basic fields." Embed denormalised copies via events — don't merge the services.

## Hard rules

### Domain-owners do not read each other

Domain isolation is the load-bearing principle. A domain-owner never reaches across to another domain-owner's data. Cross-domain interactions go through one of:

- **`rest-api` orchestrates** user-facing operations that touch multiple domains. The gateway calls each domain-owner separately and assembles the response.
- **Read-models** subscribe to events from one or more domains and assemble views by id. Read-models are the only services allowed to cross domain boundaries.
- **Hot-path co-location** (see "Performance-driven co-location" below) is the controlled exception.

A domain-owner asking another domain-owner directly is the failure mode this rule prevents.

### Domain-owner ≠ read-model in the same service

A service is either authoritative for state or a derivative view, never both. The pattern: domain-owner publishes domain events to SB; read-models subscribe and build their projections. Mixing the two in one service collapses the very separation the read-model was created to provide.

**Exception:** performance-driven co-location (next rule).

### Performance-driven domain co-location (deliberate exception)

The architect can deliberately co-locate multiple domains in one process when the latency budget for a request is tighter than gRPC overhead allows (microsecond-level hot path). This is an **architectural trade-off**, not a default.

| Trade-off | What you give up | What you gain |
|---|---|---|
| | Independent deploy/scale; clean event-shaped contracts between the merged domains; per-domain schema isolation | Microsecond-level latency; in-process atomicity across domains; zero (de)serialization overhead |

Apply this **only** when there is a measured latency budget that cannot otherwise be met. "We also want it fast" is not justification.

### 99% upsert assumption

When designing a service, assume that almost every state change is an `upsert` (insert-or-replace whole object). This shapes:

- gRPC method shape: methods take a whole object (or enough fields to upsert), not a delta.
- Persist queue (`QueueToSaveWithId<Key, Payload>`) is upsert-by-key by design.
- Idempotency comes for free; retries are safe.
- Concurrency via `e_tag`: load → mutate → upsert with e_tag check; conflict surfaces immediately.
- Postgres schemas built around `INSERT ... ON CONFLICT ... DO UPDATE`.

`load → modify → upsert` is the natural **internal** implementation of any "semantic" gRPC method (`UpdatePassword`, `IncrementBalance`). The semantic methods exist on the gRPC API for a clean call-site; they're implemented internally as load + modify + upsert.

**Exceptions** (declared explicitly): append-only services — audit logs, event histories, balance histories — write `INSERT`, never upsert. Justify append-only at design time; do not let it spread into routine state.

### Cross-domain consistency via idempotent retries (not sagas)

Distributed transactions, two-phase commit, saga frameworks, compensating actions — **none** of these are the model. The model is:

- Each cross-service write is **idempotent**: the gRPC server stores a recent journal of `retry_id → response`, and on duplicate request returns the prior result without re-executing.
- The caller sequences operations as `write A → write B → write C`, retrying each until success.
- If A succeeds, B fails, and retries don't recover — that's an operational incident, not an architectural concern. Alert and recover manually.

What we don't build: compensating actions, distributed lock managers, two-phase commit, cross-service orchestrators of transactions, automatic rollback machinery.

**Edge-case process at design time.** When sketching a cross-domain operation, evaluate the **probability** that step A succeeds and step B fails to the point of needing recovery:

- **Low probability** — accept "operational issue, manual recovery" as the answer. No additional infrastructure.
- **Medium / high probability** — file an entry on the project's **tech-debt board** (don't build the mitigation upfront, just record it). The mitigation will be designed when the debt is being addressed, not at original feature time.

**Cleanup cadence.** After main feature work ships, walk the tech-debt board in **severity order**. Edge cases that caused real incidents move up; ones that never materialised get deprioritised or closed.

**Terminal states of a tech-debt entry.** Every entry on the board resolves into one of two outcomes:

1. **Implementation** — the mitigation is built; the debt is paid off; the entry closes. This is the case when severity / actual incidents justify the engineering investment.
2. **Documented as a known limitation** — analysis concludes the probability is acceptably low or the cost-of-fix outweighs the cost-of-incident. The entry closes with an **explicit note in the project documentation**: "this race / edge / failure mode is possible but low-probability — accepted by design at <date> on the basis of <reasoning>."

Both are valid completions. Closing as a documented limitation **is not avoidance** — it's an explicit, traceable decision to live with the residual risk. What's not allowed: leaving an entry on the board for ever without resolution one way or the other. Each entry walks toward one of the two terminal states.

The architectural stance: **the cost of over-engineering preemptively (sagas, orchestrators, compensating-action frameworks) is higher than the cost of fixing the first real incident**, given that incidents are rare and operationally recoverable. Tech-debt-driven prioritisation gives a feedback loop that real-world data drives the response, not anticipated edge cases.

The same lifecycle (implementation OR documented-limitation) applies to other tech-debt entries this skill produces — idempotency-journal S3 flush for critical services, the 1000-record fallback for bounded sizing, capacity-updater discovery, and so on. None of them sit on the board indefinitely; each gets resolved.

**Idempotency key naming.** The field is named `retry_id` in gRPC requests across the codebase — chosen because the **interface-level purpose** is "support retry on disconnect". Naming it `retry_id` makes that purpose obvious at the call site; `request_id` would conflict with generic per-request identifiers; `idempotency_key` is technically accurate but doesn't convey intent.

**Don't conflate** `retry_id` with `correlation_id`. They live on the same request but mean different things:
- `retry_id` — idempotency key (server uses it to dedupe retries).
- `correlation_id` — tracing identifier (links a chain of cross-service calls in distributed traces, observability tool consumes it).

Each role gets one stable name; don't have one service call the idempotency key `retry_id` and another call it `idempotency_key`.

**Idempotency journal storage — default is in-memory.**

The journal is `retry_id → response` with TTL retention (typically minutes-to-hours, sized to cover realistic retry bursts). Default storage: in-process hash map. Restart drops the journal; the accepted risk is that any retry whose duplicate landed before the restart will re-execute on the new process.

Two tiers of treatment:

1. **Non-critical / non-money-handling services** — in-memory, **document in the service spec**: "idempotency journal in-memory; service is non-critical, restart-loss accepted." No further mitigation. This is the default for analytics, metrics, read-models, monitors, etc.

2. **Critical / money-handling / state-mutating services** — also in-memory, but **logged as tech debt** with a planned mitigation:
   - On **graceful shutdown** — flush recent journal entries to durable blob storage (S3 / equivalent).
   - On **startup** — restore the journal from the last snapshot before serving traffic.
   - Graceful restart preserves idempotency; **hard crashes still lose the journal** — residual tech-debt acknowledged.
   - This mitigation is built **after** the service's main feature work is shipped, not gating MVP. Tracked on the project's tech-debt board.

What we don't do at the universal level: per-service Postgres tables for the journal (overkill — Postgres roundtrip on every write to record idempotency hurts throughput), shared central idempotency service (introduces a coupling and SPOF), or "exactly-once" infrastructure (philosophically not buying it; "effectively-once via idempotent retries" is the model).

### Retry mindset: sudden-disconnect, not bugs

Retries exist to recover from **transient transport failures** — TCP drop, brief unavailability, network blip. The mental model: connection vanished — next request after reconnect resumes cleanly because the server is idempotent.

What retries are **not** for:
- Bugs in business logic. Retry won't fix a wrong calculation.
- Persistent outages (minutes/hours). That's an incident; alert, don't loop.
- Sustained service unavailability. Don't engineer around it; surface it.

What this implies: simple bounded retry policies, no exponential back-off-for-an-hour, no circuit breakers / bulkheads / fallback ladders.

### SB schema evolution: backward-compatible only

Any model in the SB contract crate is backward-compatible by construction.

1. **Adding a field** is allowed; the field **must have a default value**. "Field absent in payload" deserialises to that default. Old producers and old consumers keep working without code change. Default must be safe — must not silently change behavior of old messages.
2. **Removing a field** is forbidden. The desire to remove a field from a domain model usually signals a deeper architecture problem; investigate that, don't break the contract.
3. **Incompatible change** (rare, last resort): create a **new topic** with a new contract. Old topic + old contract live in parallel until consumers migrate. **Never** version a contract in-place on the same topic.

Design hint: when first defining an event, plan default values for fields likely to be added later — affects both producer-side conventions and how consumers reason about missing fields.

(This rule applies specifically to SB contracts. gRPC follows protobuf evolution; Postgres uses migrations; HTTP/REST has its own versioning story.)

### Microsecond timestamps everywhere

All timestamps in services on this stack use **microsecond precision** (`DateTimeAsMicroseconds` from `rust-extensions`). In Postgres, store as either `bigint` (epoch microseconds) or `timestamp` via `#[sql_type("timestamp")]`. Do not default to second/millisecond precision — it's worse for ordering, deduplication, and any composite key that includes a time component.

## Deployment cardinality

The first axis is **stateless vs stateful**; the second is **environment** (prod vs dev).

| Service shape | Production default | Dev default |
|---|---|---|
| **rest-api** (stateless gateway) | Multi-instance OK | Single |
| **grpc-flows: domain-owner** (holds authoritative state in memory + persists) | **Single instance** | Single |
| **grpc-flows: read-model** publishing to MyNoSQL (state lives in MyNoSQL, service is stateless) | Multi OK if needed | Single |
| **grpc-flows: read-model** with local Postgres projection (state in service-owned Postgres) | **Single instance** | Single |
| **background-worker**, stateless (pure mapper / pass-through producer) | Multi OK | Single |
| **background-worker**, stateful (in-memory cache, position-tracking subscriber) | **Single instance** | Single |
| **admin-ui** (Dioxus fullstack, stateless server functions) | Multi OK | Single |

**Master rule:** **state-bearing services run as a single instance.** "State-bearing" = in-memory cache that the service treats as source-of-truth between persists; positional / sequential processing where ordering matters; any per-message state the service can't reconstruct from external storage on demand.

**Dev default = single regardless.** Even services that *could* run multi-instance in production are deployed single on dev. Halves operational complexity, matches local-dev assumptions, makes log-tracing trivial. Multi-instance on dev only when actively testing scale-out behaviour.

**Sharding** — multiple instances each owning a partition of the data — is **bespoke design**, never a default. When throughput or state size forces it, design routing / rebalancing / cross-shard queries from scratch and document in the service's design notes. Not covered by this skill at the default level.

**Connection to SB subscriber Step 0.** The cardinality decision is exactly what drives `TopicQueueType` choice for any subscriber the service has — single-instance + `Permanent`, vs multi-instance work-share + `Permanent`, vs multi-instance broadcast + queue-per-replica. Always pick cardinality first; SB subscriber type follows.

## Service Bus rules

### Publisher: queue-backed, infallible

Every publisher wraps an internal in-memory queue. From the caller's perspective, `publish()` **never fails** — no `Result<_, PublishError>`, no retries at the call site, no fallback. Business logic stays linear and unconcerned.

> **Accepted tech-debt:** if the service restarts before the queue drains, unsent events are lost. Track this on a project's tech-debt log; do not solve it per-feature with bespoke retry — fixes belong in shared publisher infrastructure.

### Subscriber: drain-batch-persist-apply

Drain `MessagesReader` into a `Vec`, persist the batch first, then apply to in-memory state. Don't apply per-message-then-persist — recovery semantics get muddled.

### Subscriber queue-type — explicit decision

When designing any subscriber, the architect MUST consciously pick `TopicQueueType` and document why. This is not a default — it changes durability, fan-out, and replay semantics.

**Step 0 — answer the deployment-cardinality question first.** The same `TopicQueueType` value behaves differently depending on whether the service runs in one copy or several. Without knowing cardinality, the choice is a guess.

| Deployment | Default safe pick | What goes wrong with the alternatives |
|---|---|---|
| Single instance | **`PermanentWithSingleConnection`** (preferred over plain `Permanent` — gets in-order delivery and fast-reconnect kick-out for free) | `DeleteOnDisconnect` — restart longer than 20s loses messages; plain `Permanent` — stale-connection delays on reconnect, no order guarantee |
| Multi-instance, work-sharing | `Permanent` (consumers share the queue) | `PermanentWithSingleConnection` — only one replica works, others sit idle |
| Multi-instance, every replica must see every message (broadcast / per-replica cache hydration) | One queue per replica via `<service-name>-<ENV_INFO>` (durability per replica's needs) | A single shared `Permanent` — only one replica gets each message; per-replica caches break |

**Step 1 — pick the type:**

| Type | Behaviour | Pick when |
|---|---|---|
| `Permanent` | Durable, survives restarts, retains until ack. Multiple consumers share work. | Subscriber must not lose events across restarts (audit, ledger, position state). Default for stateful services. |
| `PermanentWithSingleConnection` | Durable, at most one connected consumer. Two properties this gives you for free: (1) **Strict message order is preserved** — only one consumer at a time, no inter-consumer reordering. (2) **Fast-reconnect kick-out** — on reconnect, any pre-existing connection (which the server may still believe is alive due to TCP/heartbeat lag) is immediately evicted along with all its pending ack-waits; pending messages flip to the new connection without waiting for the server to detect the dead socket. | **Default for single-instance services.** Strictly better than plain `Permanent` for that case: order preserved, fast reconnect after blink, no operational downside. Mandatory for cases where order matters and parallel processing is unsafe (account-state mutator, ledger applier, anything with causal dependencies between events). |
| `DeleteOnDisconnect` | **Not immediately auto-deleted** — the queue lingers ~20 seconds after the consumer disconnects, accumulating messages during the grace period. Deleted only after 20s with no consumer. | Multi-replica API broadcast (per-replica WebSocket / streaming forwarders) where each replica has a suffixed queue and a normal restart/deploy fits within the 20s grace — zero message loss across restart, automatic cleanup if the replica goes away permanently. Also: transient consumers (debug/admin views, ephemeral monitors). Never for state-bearing read-model logic — restart longer than 20s loses events. |

If the chosen type isn't `Permanent`, justify in code or in the service's design doc. If cardinality might change later, document the migration.

### Read-model SB queue type — always durable

A read-model subscriber is always durable (`Permanent` / `PermanentWithSingleConnection`), never `DeleteOnDisconnect`. On restart, queued events drain and the projection catches up — the queue itself is the recovery mechanism. No silent data loss; no separate gRPC backfill from the domain-owner.

Cardinality picks the specific durable variant:
- **Single-instance read-model** — `PermanentWithSingleConnection` (fast-reconnect kick-out, see type table).
- **Multi-instance work-sharing read-model** — `Permanent`.
- **Multi-instance broadcast read-model** (per-replica cache hydration) — per-replica queue via `<service-name>-<ENV_INFO>`, each `Permanent` (or `PermanentWithSingleConnection` since each suffixed queue has only one connection).

`DeleteOnDisconnect` is never appropriate for read-models — even with the 20s grace, a read-model rebuild may take longer than that.

### Queue naming: `<service-name>[-<ENV_INFO>]`

**Format.** SB queue names follow the convention `<service-name>` (no suffix) or `<service-name>-<ENV_INFO>` (suffixed). `ENV_INFO` is a value assigned **per machine / per replica**.

**No suffix — single queue per service name.** Default for:
- Single-instance services.
- Multi-instance work-sharing (each event processed exactly once across replicas; consumers compete for messages from one shared `Permanent` queue).

**With `ENV_INFO` suffix — per-replica queue.** Default for **multi-instance broadcast** — when every replica must independently consume the full event stream.

**Canonical use case for the suffix.** A REST API service runs in N replicas, each holding a set of connected clients (WebSocket / SSE / streaming push). When the upstream domain publishes a change, every replica must receive it, because the change must be forwarded to **its own** connected clients. A shared queue would distribute events round-robin — one replica gets the event, that replica forwards to its clients, but clients connected to other replicas miss the update. With suffixed queues, each replica creates its own `<service-name>-<ENV_INFO>` queue, consumes everything, fans out to its local clients.

**Pair this with `DeleteOnDisconnect`.** For the multi-replica forwarder case, the queue type is `DeleteOnDisconnect` (auto-delete) — **not** `Permanent`:
- The queue lingers ~20 seconds after consumer disconnect (`DeleteOnDisconnect` is **not** instant). A normal restart/deploy fits within that grace window, so reconnecting replica picks the queue up — **zero message loss** across normal restarts.
- If a replica is removed permanently (scale-down, host retired), the queue auto-cleans after 20s. No orphan queues lingering forever per ex-machine.

For per-replica cache hydration / per-replica counters / read-model whose state lives in-process per replica — the durability requirement is different, and `Permanent` is the right call. The forwarder case is special because the state is "currently connected clients", which itself doesn't survive a restart anyway.

**Implication for cardinality / Step 0.** The "multi-instance broadcast" row in the SB subscriber table is implemented precisely via this naming. Step 0 says "decide cardinality first"; if the answer is multi-instance broadcast, the queue-per-replica mechanism is `ENV_INFO`-suffixed names.

### Contracts live in the contracts crate

SB models go in the shared contracts crate (e.g. `my-sb-contracts`). Never inline. Never duplicated.

## Read-model design

### When to extract a read-model — eagerly, because they're cheap to throw away

**Default: almost always extract.** Whenever the requirement involves aggregation, a different shape than the domain object, a cross-domain view, or any read-side workload that doesn't trivially fit a domain-owner's CRUD — make it its own read-model service.

**Why eagerly:**
- Read-models are designed to be **deletable**. When a use case disappears (UI is removed, product pivots, metrics are retired), the read-model service is stopped, its tables dropped, its SB queue deleted, and **it's as if it never existed**. State is never canonical there — the source of truth is the domain-owner, the read-model is just a derived projection.
- The cost of mixing read-shaped methods into a domain-owner is **permanent**. The domain-owner's gRPC API inflates with view methods; its codebase mixes write logic with view logic; future evolution is harder. That coupling doesn't go away when the use case does.
- Adding a small service is operationally cheap: one repo entry, one Dockerfile, one deployment pipeline, one `Permanent` SB queue.

**The rare merge case:** if a read truly is just CRUD on the domain object, with no aggregation, projection, or cross-domain join — it isn't a "read-model"; it's a domain-owner CRUD method. Keep it on the domain-owner.

**Decision filter:**
- *Does this read have a different shape than the domain object?* (top-N, summary, joined view, time-windowed, projected) — **extract**.
- *Does this read combine multiple domains?* — **extract** (and only a read-model is allowed to do that; see Domain isolation rule).
- *Is it just `Get` / `List` of the domain object as-is?* — **method on the domain-owner**.

### Output mode is driven by size

> **Bounded read-model + fits in memory** — publish to **MyNoSQL**; consumers read via TCP `MyNoSqlReader` (sync local).
> **Unbounded read-model** — store in **Postgres**; consumers read via **gRPC** at the read-model service.

| | Output | Consumer access | Recovery model |
|---|---|---|---|
| **Bounded** (fits in memory of every consumer) | MyNoSQL replicated cache | TCP reader, sync local, lock-free | Consumers always see last-known-good even if read-model service is down |
| **Unbounded** (doesn't fit) | Postgres in the read-model service | gRPC with filters/pagination | Consumers depend on the read-model service availability |

Architect must **size the projection first** before any other design step on a read-model. Output mode determines storage choice, consumer pattern, recovery semantics, deployment cardinality.

If a projection can be compressed into a bounded view (top-N, last-day, summary), prefer that — bounded output gives lower latency and less operational coupling.

### Recovery / cold-start with history

Three approaches, in preference order:

1. **SB replay from event 0.** If the topic retains full history, the new read-model creates its queue with "start from beginning" semantics, consumes all events, builds state. Cleanest when applicable.
   *Works when:* topic retains full history. *Doesn't work when:* topic has TTL, was recreated, or the volume is unworkable.

2. **Prod-first + post-start init script.** Deploy and start the production read-model — it begins consuming SB live, building forward state. Then run a one-shot init (Rust binary, Python script, `cargo run` — whatever's convenient) that backfills historical data from the source of truth (domain-owner's Postgres). Init filters by time (`WHERE created_at < <service_start_time>`) so init data and live data don't collide. Init is throwaway, not part of permanent infrastructure.
   *Works when:* SB doesn't retain enough history but a queryable historical source exists.

3. **Accept "from now on".** New read-model just starts; no history. Fine for current-state projections (live open positions, current instrument map). Not fine for audit-shaped views.

What we **don't** do: build a permanent `GetAllEvents` / `StreamHistorical` gRPC on the domain-owner (clutters API for one rare use case); standing snapshot infrastructure (over-engineered for rare cold-starts).

### Domain-owner gRPC API

A domain-owner's gRPC API is **primarily a set of semantic methods** with expressive names that convey intent. CRUD primitives exist underneath but are not the primary call surface.

- **Semantic methods** are the default: `OpenPosition`, `CancelOrder`, `UpdatePassword`, `ChangeEmail`, `IncrementBalance`, `SuspendAccount`, etc.
- **Reading guideline:** a developer should understand what a method does **from its name alone**, without diving into the implementation. If the name doesn't carry the intent, rename.
- **CRUD primitives** (`Get`, `List`, `Upsert`, `Delete`) exist for read-side queries and special administrative paths (backfill, recovery, debug). Generic `Upsert` is **not** the standard path for production callers — they call semantic methods.
- **Internally,** each semantic method usually implements `load → modify → upsert` against the persistence layer. The semantic name lives at the API boundary; load-modify-upsert lives inside.

Read-models are introduced **only** when there's an actual reason: aggregation, cross-domain view, projection of different shape, or scaling reads. "Plain reads of own state" don't justify a separate read-model service.

### Business invariants live in semantic methods

Domain integrity rules — "balance can't go below maintenance margin", "order transitions from `Pending` to `Filled / Cancelled / Rejected` only", "≤ N open orders per account", "can't open a position on a non-tradable instrument" — are enforced **inside the semantic gRPC method on the domain-owner**, never in the caller and never via diff-validation on a generic Upsert.

Why semantic-method enforcement, not the alternatives:

- **Not caller-side** — even with strict architecture, callers eventually have bugs, mistakes, or new entry points that bypass the rules. The domain-owner is the only place the rule can be guaranteed.
- **Not diff-based on a generic Upsert** — the validation is indirect (you'd have to compute a diff to see what changed and pattern-match against per-field rules). Adding a new invariant means editing a giant diff-validator instead of editing the method that semantically owns the rule. Easy to miss cases when adding new fields.

The semantic-method approach has a known cost: invariants shared across multiple methods (e.g., "balance ≥ maintenance margin" applies to `OpenPosition`, `IncreasePosition`, `Withdraw`) risk copy-paste. The mitigation is to extract such invariants into shared validator functions in a domain-internal module — but the **call site stays in the semantic method**, not in upsert middleware.

## Data ownership patterns

### Step 1 — who owns the data?

| Ownership | Identity | What this gives you |
|---|---|---|
| **System-owned** | 1 ID (`ObjectId`) | Reference data, system events. PK by ObjectId; per-user fields are indexed values, not identity. |
| **User-owned** | 2 IDs (`UserId + ObjectId`) | Multi-tenant data per user. `partition_key = UserId, row_key = ObjectId` in MyNoSQL; composite PK in Postgres. Authorization checks (requestor = owner) are mandatory. |

**Implications:**
- System-owned: reads can be optimized for any access pattern via indexes; per-user scans cost an index lookup.
- User-owned: per-user scan is the natural primary access (`get_by_partition_key(user_id)`); cross-user scan is fan-out and unusual.
- Persist-queue keys for user-owned data must include the UserId — otherwise enqueued mutations from different users can clobber each other.

### Step 2 — is the data size-bounded?

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

### Storage decision matrix

|   | Bounded | Unbounded |
|---|---|---|
| **System-owned (1 ID)** | MyNoSQL replicated cache + TCP reader. Every consumer holds the full set. | Postgres with single PK; access via gRPC, paginated/filtered. |
| **User-owned (2 IDs)** | MyNoSQL with `partition_key=UserId, row_key=ObjectId`. Per-user load small, replicates or reads-by-partition. Per-user cap enforced at write-path. | Postgres with composite PK `(user_id, object_id)`, indexed by `user_id`. Hot-state via in-memory + persist queue. |

Read-models very often translate unbounded domain data into bounded views (top-N, last-day, summary) — that's exactly what makes them cheap to consume.

### Pagination-aware caching for history data

When a read-model represents history (logically unbounded), design the **base case as DB-backed**. Then layer caching on top:

- **First page** in pagination terms is bounded by definition — N records (typically "two average screen-fulls" of UI). Fits the bounded → MyNoSQL pattern. Serves the common case "user opens the screen" from memory.
- **Subsequent pages** (rare access — user scrolling back through history) — served directly from DB via gRPC.

**Architectural shape:** one read-model service with **two storage backends** (MyNoSQL for hot window + Postgres for full history) is a **native shape** for this pattern, not a violation of single-responsibility. The service "owns history of X" and knows to serve the first page fast and the rest from DB.

**Sizing decision:** "how much in cache?" = typical screen size × 2 (one screen of headroom). Not a fixed record count — driven by UI, not by storage capacity.

### MyNoSQL JSON cost — factor it in

MyNoSQL serialises records as JSON. For datasets of "millions of small records" with hot-path read patterns, JSON serialise/deserialise overhead becomes significant. MyNoSQL is a good fit for "moderate-cardinality, moderate-record-size, frequently read" data; not for "huge counts of micro-records". When the profile leans towards micro-records at high volume, evaluate a **custom binary cache** as an alternative. This is a heavy hammer; reach for it only when JSON cost is measured to dominate.

### MyNoSQL caching capacity

#### Client-context partitions

When a read-model or cache is partitioned by client/user id and published to MyNoSQL:

```
max_partitions_amount = max(2 × peak_concurrent_clients_per_hour, 1000)
```

- **2×** — buffer for churn (clients disconnect and return; their partition must survive that window).
- **floor 1000** — sanity baseline. On low-traffic / dev environments the `2× hourly` count can be tiny; 1000 keeps things robust against sudden spikes. MyNoSQL partitions are cheap.
- **Re-evaluate** when usage profile shifts (active base grows, new client classes appear). Not set-and-forget.

`max_partitions_amount` can be updated after table creation — capacity adjustment is dynamic, not a recreate-table operation.

#### System-owned reference (admin-managed)

For system-owned bounded reference data (instruments, swap profiles, exchange configs, any admin-managed catalog):

```
max_partitions_amount = None
```

**Reasoning:** this class of data is generated by **employees** through admin-ui, not by clients through public APIs. The input rate is bounded by human admin throughput, orders of magnitude below any client RPS — no LRU eviction is needed. Setting a limit on admin-managed data is an antipattern: an admin action could evict another admin-managed entry, breaking the integrity of system reference state.

#### `max_rows_per_partition_amount`

No fixed default — **decide by data nature.** Architect estimates expected per-partition row count, growth profile, and whether eviction is acceptable. Common settings:
- `None` for partitions with bounded-by-design row count (e.g., one record per `partition_key`).
- A specific number when row count grows but eviction of oldest is acceptable (e.g., last-N pattern).

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
- `reader.get_by_partition_key` returns `Option<BTreeMap<String, Arc<T>>>` (key = row_key). Use `_as_vec` for just values.
- Reader callbacks: **full reload pattern**, always `tokio::spawn` inside the callback. Never incremental.

### In-memory + persist queue (hot path)

For state mutated on every request that must not block on DB writes.

Pattern:
1. Single `Mutex` over all in-memory state.
2. Separate `QueueToSaveWithId<Key, Payload>` per persist target.
3. Background handler drains the queue and batches writes to Postgres / NoSQL.
4. Client request returns as soon as state is mutated and key is enqueued. No blocking I/O under the lock.

## Cache policy — pick one of four

| Pattern | When |
|---|---|
| Mutex + persist queue | Write-frequent local state; source of truth in memory; persistence is best-effort durability. |
| In-memory hydrated from MyNoSQL on startup (`wait_until_first_data_arrives`) | Service consumes a stream and must apply on top of persisted history. |
| Read-through (no local cache) | Rarely-read state where staleness is unacceptable. |
| Write-through (`.with_retries(3).insert_or_replace`) | Service rarely reads but must publish state visible to others immediately. |

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

## Service-SDK feature matrix

When sketching a new service's dependencies:

| Feature | Add when |
|---|---|
| `macros` | Always (settings, entity macros). |
| `grpc` | Service exposes or calls gRPC. |
| `my-service-bus` | Service publishes or subscribes. |
| `postgres` | Uses my-postgres. |
| `my-nosql-sdk` | Only entity macros (no I/O). |
| `my-nosql-data-reader-sdk` | Service reads MyNoSQL state via TCP reader. |
| `my-nosql-data-writer-sdk` | Service writes MyNoSQL state via HTTP writer. |
| `with-tls` | Service connects to `wss://` (Binance, exchange feeds). Without it: rustls panic at runtime. |
| `with-telemetry` | Propagate telemetry context across gRPC. |
| `with-ssh` | gRPC over SSH tunnel. |

## Transport decisions

| Use case | Pick | Why |
|---|---|---|
| Public-facing API for browsers / external clients | HTTP | OpenAPI surface, browser-friendly |
| Internal request/response between services | gRPC | Type-safe, proto-versioned, retry/ping built into client macro |
| Real-time stream from one producer to many consumers | Service Bus | Decoupling, durable buffer, fan-out |
| Cross-service read of slowly-changing reference data | MyNoSQL replicated cache | Sync local reads, no N+1 RPCs |
| External LP / exchange feed | Adapter service → SB publisher | Translate LP-specific protocol into SB contracts |

### gRPC sub-rules

- Streaming output — handler returns `StreamedResponseWriter<T>`, must `tokio::spawn` the producer task.
- Streaming input — collect via `request.into_vec().await` (no spawn).
- Non-streaming — await directly (no spawn).
- Clients via `#[generate_grpc_client]` with `retries`, `request_timeout_sec`, `ping_*`. Per-method overrides allowed.
- When service-sdk pulls `with-telemetry`, every client method takes `&MyTelemetryContext` as second arg.

## Service-to-service auth

**Default model: trust the network.** Internal gRPC calls between services in the monorepo do not carry auth tokens, do not use mTLS, do not verify caller identity at the application layer. All services live in a protected perimeter (private network / VPN / firewall isolation). Network-level isolation is the single line of defence for service-to-service traffic.

**What this means:**
- gRPC servers don't validate caller identity on internal endpoints.
- No JWT propagation between services for the call itself (auth tokens for **end-user** identity may flow through to satisfy business rules — that's a different concern; service-to-service trust is unconditional).
- Adding mTLS / per-service signing keys / token validation is **not** a baseline architectural concern.

**When to reconsider:**
- Multi-tenant deployments where services don't all belong to the same trust boundary.
- Regulatory / compliance requirement for end-to-end auth (e.g., explicit policy that every gRPC call be authenticated).
- Network perimeter assumptions break (services exposed beyond the controlled network).

If any of these apply in a concrete project, the auth model needs explicit design — not handled by this skill.

## Anti-patterns (universal)

- Calling MyNoSQL writer methods directly (`writer.insert_or_replace`). Always via `.with_retries(N)`.
- Returning `Result<_, PublishError>` from a `publish()` call. Publishers are queue-backed and infallible from the caller's POV — propagating a fake error type pollutes business code.
- Wiring up a SB subscriber without explicitly choosing `TopicQueueType`. Defaulting to `DeleteOnDisconnect` for state-bearing logic = silent data loss on reconnect.
- Awaiting MyNoSQL reader read methods. They are sync.
- Duplicating entity structs across services. Always shared crate.
- Incremental cache update in a NoSQL reader callback. Always full reload + `tokio::spawn`.
- `start_up.rs` or manual `MyHttpServer` instantiation. SDK owns lifecycle.
- Top-level `use` imports in Dioxus server functions (warns on web target). Imports inside the function body.
- Mixing audit data into the logger or operational logs into audit-log.
- Building distributed transactions / sagas / two-phase commit / cross-service rollback. Use idempotent retries instead.
- Removing fields from SB contracts. Add only.
- Combining domain-owner and read-model in one service (except deliberate hot-path co-location with measured latency budget).

## Open questions (track until resolved)

These intentionally don't have universal answers — they need a decision per system / per project, but the architect should be aware they exist:

- **Bounded read-model + strict read-after-write requirement.** Edge case where bounded → MyNoSQL pattern doesn't fit because of replication lag; the recorded escape hatch is "custom binary cache with stronger semantics" but that's a heavy hammer — invoke only with measured justification.
- **Cross-user data shape** (transfers / messages / fills with two participants) — partition by sender / receiver / two records each. Pattern not chosen at universal level; default until a real case appears: two records, each in the partition of its owner.
- **B2B / OrgId hierarchy** above user. Not addressed at universal level; if a system needs it, add `OrgId` as the top level of identity and revisit this skill.

## When to drill down

This skill is decision-only. For implementation API surface, fetch from the `best-practices` MCP:

| Topic | Tool |
|---|---|
| Project bootstrap, Dockerfile/CI templates, NoSQL reader/writer wiring, TLS rules | `get_app_bootstrap_guide` |
| HTTP action structure, input/output models, errors, cookies, IP, file uploads | `get_http_actions_design_guide` |
| gRPC server macros, client macros, streaming patterns, telemetry | `get_my_grpc_extensions_readme` |
| MyNoSQL entity design, expirations, reader callbacks, anti-patterns | `get_my_no_sql_entity_patterns` |
| Postgres macros, table schema, where models, e_tag | `get_my_postgres_readme` |
| Dioxus fullstack: shared models, mappers, server functions, GET params | `get_dioxus_fullstack_design_patterns` |
| FIX protocol library | `get_rust_fix_readme` |
| TCP sockets, SSH, FlUrl | corresponding `get_*_readme` tools |

Don't memorize their content here.

---

**Project-specific decisions** (service inventory, named services, host topology, settings field-name conventions, ID rules, integration shapes) live in a sibling `architectual-considerations.md` (or equivalent) for each concrete system. This file stays universal.
