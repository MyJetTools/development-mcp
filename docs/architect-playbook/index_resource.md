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

   **The only exception** is the performance-driven domain co-location pattern (topic `domain-boundaries`) — when the latency budget is measured in microseconds and gRPC overhead would blow it, multiple domains co-locate in one process. This is the **only** justification accepted for merging concerns; "felt like it" / "didn't want a new service" are not.

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

## Hard rules — one line each

Every rule is binding at the moment of the decision. The reasoning, the edge cases and the trade-offs are in the topic named next to it.

- **Domain-owners do not read each other.** Cross-domain work goes through `rest-api` orchestration, through read-models, or through deliberate hot-path co-location — `domain-boundaries`.
- **A service is a domain-owner or a read-model, never both** — `domain-boundaries`.
- **Domains co-locate in one process only for a measured microsecond-level latency budget.** "We also want it fast" is not a justification — `domain-boundaries`.
- **99% upsert.** A state change is an upsert of the whole object; append-only is a declared exception — `consistency`.
- **Cross-domain consistency via idempotent retries (`retry_id` journal), never sagas, two-phase commit or compensating actions** — `consistency`.
- **Retries recover from a sudden disconnect, not from bugs or outages** — simple bounded policies only — `consistency`.
- **State-bearing services run as a single instance; decide the cardinality before any Service Bus queue type** — `deployment-cardinality`.
- **Every Service Bus subscriber gets a consciously chosen `TopicQueueType`; read-models are always durable** — `service-bus`.
- **Service Bus contracts are backward-compatible only:** add a field with a default, never remove one; an incompatible change is a new topic — `service-bus`.
- **Size a read-model first:** bounded → MyNoSQL, unbounded → Postgres behind gRPC — `read-models`, `data-ownership`.
- **No `await` on I/O and no external calls under the hot-path mutex; persist through queues** — `persistence`.
- **Microsecond timestamps everywhere** (`DateTimeAsMicroseconds`) — `persistence`.

## Topics

Read a topic: `resource://architect-skill/{topic}`, or `get_architect_playbook` with `topic`.

| Topic | What is inside |
| --- | --- |
| [`domain-boundaries`](domain-boundaries.md) | Separate domains by responsibility, not by entity; domain-owners do not read each other; domain-owner ≠ read-model; performance-driven co-location as the one exception |
| [`consistency`](consistency.md) | 99% upsert assumption; cross-domain consistency via idempotent retries instead of sagas; the tech-debt lifecycle of edge cases; `retry_id` vs `correlation_id`; the idempotency journal; the retry mindset |
| [`deployment-cardinality`](deployment-cardinality.md) | Single instance vs multi-instance per service shape, prod vs dev; state-bearing services are single; sharding is bespoke |
| [`service-bus`](service-bus.md) | Queue-backed publisher; drain-batch-persist-apply subscriber; choosing `TopicQueueType` (Step 0 cardinality, Step 1 type); read-model queues; queue naming with `ENV_INFO`; contracts crate; backward-compatible contract evolution |
| [`read-models`](read-models.md) | When to extract a read-model (eagerly); output mode driven by size; cold-start with history; semantic methods of the domain-owner gRPC API; where business invariants live |
| [`data-ownership`](data-ownership.md) | System-owned vs user-owned data; bounded vs unbounded (the 1000-record fallback); storage decision matrix; pagination-aware caching; MyNoSQL JSON cost and capacity settings |
| [`persistence`](persistence.md) | Postgres, MyNoSQL and in-memory + persist queue layers; the four cache policies; hot-path constraints; audit-log vs logger; microsecond timestamps |
| [`transports`](transports.md) | HTTP vs gRPC vs Service Bus vs MyNoSQL per use case; gRPC sub-rules; service-to-service auth (trust the network); the service-sdk feature matrix |

## Anti-patterns (universal)

- Calling MyNoSQL writer methods directly (`writer.insert_or_replace_entity`). Always via `.with_retries(N)`.
- Propagating `Result<_, PublishError>` from a publish call into business code. `PublisherWithInternalQueue::publish_and_forget()` only enqueues and can fail solely on serialization; `MyServiceBusPublisher::publish()` does return `Result<_, PublishError>` — log it and continue, do not propagate it.
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
| `ServiceContext`: startup order, timers, queues, events loops, background executors, HTTP / gRPC / NoSQL / Service Bus / Postgres wiring | `get_service_sdk_readme` |
| Project bootstrap, Dockerfile, CI / GitHub Actions workflows (single-repo `ci-utils`, monorepo release + pre-baked builder image), NoSQL reader/writer wiring, TLS rules | `get_app_bootstrap_guide` |
| Cutting a release: tags, `gh` commands, re-deploy, re-baking the builder image, troubleshooting a run | `get_release_guide` |
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
