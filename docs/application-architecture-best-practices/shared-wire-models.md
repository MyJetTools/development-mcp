# Shared Wire-Models Crate — client-side SPA + standalone REST API

> **WHY:** When a WASM SPA and its REST-API service are **separate builds**, the request/response models are the contract between them. Redefine them on each side and the two drift silently — a renamed field or a changed type compiles everywhere and breaks only at runtime. One shared crate makes the wire contract a single source of truth the compiler checks on both ends.

**Archetype:** a **client-side SPA** (WASM — e.g. Dioxus `dioxus/web`) calling a **standalone REST-API service** over HTTP. This is distinct from Dioxus **fullstack** (`#[server]` functions in one binary), which has no wire boundary to share. See the `Dioxus Client-Side Bootstrap Guide` vs `Dioxus Fullstack Design Patterns` resources.

Lift the request/response models out of the SPA's local `src/models/` into a dedicated crate — `rest-api-shared` — depended on by **both** sides:

```
              rest-api-shared          ← wire models only, no behavior
             ╱               ╲
   client-side SPA        REST-API service
   (builds requests,      (parses input:  MyHttpInput,
    reads responses)       documents output: MyHttpObjectStructure)
```

- Request models derive `MyHttpInput`; response models derive `MyHttpObjectStructure` — the same structs the server's `#[http_route]` actions consume (see **HTTP Action Pattern**, topic `http`).
- **The crate is WASM-clean:** it depends on `my-http-utils`, and a `"server"` feature gates everything server-only. The default build compiles to WASM so the SPA can depend on it.

```rust
// rest-api-shared — pure data; server-only derives gated behind the "server" feature
#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "server", derive(MyHttpInput))]
pub struct CreateOrderRequest {
    pub symbol: String,
    pub qty: f64,
}
```

```toml
# rest-api-shared/Cargo.toml
[features]
default = []          # SPA build: serde only → stays WASM-clean
server  = []          # REST service build: enables MyHttpInput / MyHttpObjectStructure + server deps
```

**Models are pure data — behavior lives in extension traits on the consumer side.**

> **WHY:** A method on a shared model drags that consumer's dependencies and logic into the shared crate, breaking the WASM boundary and coupling client to server. Plain models plus per-side extension traits keep the shared crate a thin, dependency-light contract.

**Rules:**
- **ALWAYS** define shared request/response models once in `rest-api-shared` — never redefine them per side.
- **NEVER** add server-only dependencies to the default feature set — the SPA build must stay WASM-clean.
- **NEVER** put behavior (methods, I/O, mapping) on the shared models — add it via an extension trait on the consuming side.
- Same idiom as the **Service Bus Contract Pattern** (topic `service-bus`, `my-sb-contracts`): one shared crate = one wire contract, owned by neither side.
