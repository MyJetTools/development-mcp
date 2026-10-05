# Service Bus rules

## Publisher: queue-backed, infallible

The queue-backed publisher is `PublisherWithInternalQueue` (`service_context.get_sb_publisher_with_internal_queue()`) — it wraps an internal in-memory queue. From the caller's perspective, `publish_and_forget()` **never fails** on delivery — it only enqueues, and its `Result<_, PublishError>` is `Err` only when the message cannot be serialized; no retries at the call site, no fallback. Business logic stays linear and unconcerned. `MyServiceBusPublisher` (`get_sb_publisher(do_retries)`) is **not** queue-backed — its `publish().await` returns `Result<_, PublishError>`.

> **Accepted tech-debt:** if the service restarts before the queue drains, unsent events are lost. Track this on a project's tech-debt log; do not solve it per-feature with bespoke retry — fixes belong in shared publisher infrastructure.

## Subscriber: drain-batch-persist-apply

Drain `MessagesReader` into a `Vec`, persist the batch first, then apply to in-memory state. Don't apply per-message-then-persist — recovery semantics get muddled.

## Subscriber queue-type — explicit decision

When designing any subscriber, the architect MUST consciously pick `TopicQueueType` and document why. This is not a default — it changes durability, fan-out, and replay semantics.

`TopicQueueType` is the kind of queue the Service Bus server keeps for the subscriber. In code it is not an argument: `register_sb_subscribe(callback, delete_on_no_subscribers, single_connection)` takes two flags, and the client turns them into the queue type (`TopicQueueType::from_flags`):

| `TopicQueueType` | `delete_on_no_subscribers` | `single_connection` |
|---|---|---|
| `Permanent` | `false` | `false` |
| `PermanentWithSingleConnection` | `false` | `true` |
| `DeleteOnDisconnect` | `true` | `false` |
| `DeleteOnDisconnectWithSingleConnection` | `true` | `true` |

**Step 0 — answer the deployment-cardinality question first** (topic `deployment-cardinality`). The same `TopicQueueType` value behaves differently depending on whether the service runs in one copy or several. Without knowing cardinality, the choice is a guess.

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

## Read-model SB queue type — always durable

A read-model subscriber is always durable (`Permanent` / `PermanentWithSingleConnection`), never `DeleteOnDisconnect`. On restart, queued events drain and the projection catches up — the queue itself is the recovery mechanism. No silent data loss; no separate gRPC backfill from the domain-owner.

Cardinality picks the specific durable variant:
- **Single-instance read-model** — `PermanentWithSingleConnection` (fast-reconnect kick-out, see type table).
- **Multi-instance work-sharing read-model** — `Permanent`.
- **Multi-instance broadcast read-model** (per-replica cache hydration) — per-replica queue via `<service-name>-<ENV_INFO>`, each `Permanent` (or `PermanentWithSingleConnection` since each suffixed queue has only one connection).

`DeleteOnDisconnect` is never appropriate for read-models — even with the 20s grace, a read-model rebuild may take longer than that.

## Queue naming: `<service-name>[-<ENV_INFO>]`

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

## Contracts live in the contracts crate

SB models go in the shared contracts crate (e.g. `my-sb-contracts`). Never inline. Never duplicated.

## SB schema evolution: backward-compatible only

Any model in the SB contract crate is backward-compatible by construction.

1. **Adding a field** is allowed; the field **must have a default value**. "Field absent in payload" deserialises to that default. Old producers and old consumers keep working without code change. Default must be safe — must not silently change behavior of old messages.
2. **Removing a field** is forbidden. The desire to remove a field from a domain model usually signals a deeper architecture problem; investigate that, don't break the contract.
3. **Incompatible change** (rare, last resort): create a **new topic** with a new contract. Old topic + old contract live in parallel until consumers migrate. **Never** version a contract in-place on the same topic.

Design hint: when first defining an event, plan default values for fields likely to be added later — affects both producer-side conventions and how consumers reason about missing fields.

(This rule applies specifically to SB contracts. gRPC follows protobuf evolution; Postgres uses migrations; HTTP/REST has its own versioning story.)
