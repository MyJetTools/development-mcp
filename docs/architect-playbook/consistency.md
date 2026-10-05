# Consistency — upserts, idempotent retries, the retry mindset

## 99% upsert assumption

When designing a service, assume that almost every state change is an `upsert` (insert-or-replace whole object). This shapes:

- gRPC method shape: methods take a whole object (or enough fields to upsert), not a delta.
- Persist queue (`QueueToSaveWithId<Key, Payload>`) is upsert-by-key by design.
- Idempotency comes for free; retries are safe.
- Concurrency via `e_tag`: load → mutate → upsert with e_tag check; conflict surfaces immediately.
- Postgres schemas built around `INSERT ... ON CONFLICT ... DO UPDATE`.

`load → modify → upsert` is the natural **internal** implementation of any "semantic" gRPC method (`UpdatePassword`, `IncrementBalance`). The semantic methods exist on the gRPC API for a clean call-site; they're implemented internally as load + modify + upsert.

**Exceptions** (declared explicitly): append-only services — audit logs, event histories, balance histories — write `INSERT`, never upsert. Justify append-only at design time; do not let it spread into routine state.

## Cross-domain consistency via idempotent retries (not sagas)

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

## Retry mindset: sudden-disconnect, not bugs

Retries exist to recover from **transient transport failures** — TCP drop, brief unavailability, network blip. The mental model: connection vanished — next request after reconnect resumes cleanly because the server is idempotent.

What retries are **not** for:
- Bugs in business logic. Retry won't fix a wrong calculation.
- Persistent outages (minutes/hours). That's an incident; alert, don't loop.
- Sustained service unavailability. Don't engineer around it; surface it.

What this implies: simple bounded retry policies, no exponential back-off-for-an-hour, no circuit breakers / bulkheads / fallback ladders.
