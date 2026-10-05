# Deployment cardinality

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

**Connection to SB subscriber Step 0** (topic `service-bus`). The cardinality decision is exactly what drives `TopicQueueType` choice for any subscriber the service has — single-instance + `Permanent`, vs multi-instance work-share + `Permanent`, vs multi-instance broadcast + queue-per-replica. Always pick cardinality first; SB subscriber type follows.
