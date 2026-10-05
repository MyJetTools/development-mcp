# Transports, service-to-service auth, service-sdk features

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
- service-sdk always turns on `with-telemetry` of my-grpc-extensions, so every client method takes `&MyTelemetryContext` as second arg (`&MyTelemetryContext::Empty` where there is no context to pass on).

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
| `with-ring-tls` or `with-rust-tls` | One decision for the whole service — **ask the user**: no TLS, or TLS on one of the two. TLS is needed when the service connects to `wss://` (Binance, exchange feeds) or calls anything over `https://`. Without it a `wss://` connection panics in rustls and an `https://` request through FlUrl fails with `FlUrlError::UnsupportedScheme`. |
| `with-postgres-tls` | Postgres over TLS (only together with `postgres`). |
| `with-ssh` | gRPC / Postgres / MyNoSQL / HTTP over SSH tunnel. |
| `with-prometheus-metrics` | Service must expose `/metrics`. |

Telemetry needs no feature: service-sdk always turns on `with-telemetry` of my-grpc-extensions, my-service-bus, my-http-server and my-postgres. A gRPC server hands the context to its handlers with `with_telemetry: true` on `generate_server!`.
