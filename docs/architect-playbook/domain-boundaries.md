# Domain boundaries and domain isolation

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
