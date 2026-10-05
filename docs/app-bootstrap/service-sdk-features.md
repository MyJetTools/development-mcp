# `service-sdk` feature flags

`service-sdk` exposes the following Cargo features. Pick the smallest set the service actually needs.

| Feature | What it enables |
|---|---|
| `default` | HTTP server, settings reader, logger, telemetry — the baseline every service needs. **Do not opt out.** |
| `full` | `macros`, `grpc`, `my-service-bus`, `postgres` and the three `my-nosql-*` features. Convenient for prototyping, avoid in production crates. **Never** includes the TLS, SSH or metrics features — those are always named explicitly. |
| `macros` | Brings in the `use_settings!()`, `use_grpc_server!()`, `use_grpc_client!()`, `use_my_postgres!()`, `use_my_http_server!()`, `use_my_no_sql_entity!()` macros. **Almost always needed.** |
| `grpc` | gRPC server + client stack. |
| `my-service-bus` | SB publisher / subscriber. |
| `postgres` | `my-postgres` re-exports + `PostgresSettings` trait auto-impl for `SettingsReader` via `AutoGenerateSettingsTraits` (field `postgres_conn_string`). |
| `my-nosql-data-reader-sdk` | TCP reader for MyNoSql. |
| `my-nosql-data-writer-sdk` | HTTP writer for MyNoSql. |
| `my-nosql-sdk` | Entity macros only (no reader, no writer). |
| `websockets` | Server-side WebSocket on top of the HTTP server. |
| `http-static-files` | Mount a static-files directory through the HTTP server. |
| `signal-r` | SignalR server (uncommon). |
| `with-ring-tls` | rustls TLS with the **ring** crypto provider — installs the provider at startup. This or `with-rust-tls` is required for `https://` through FlUrl, `wss://` outbound connections and TLS gRPC. Mature and widely deployed; a bundled C/assembly build. |
| `with-rust-tls` | Same TLS with the pure-Rust provider (`rustls-graviola`): no C toolchain, but x86_64/aarch64 only and far less deployed than ring. No TLS / `with-ring-tls` / `with-rust-tls` is one decision for the whole service — **ask the user**, see topic `tls`. |
| `with-postgres-tls` | TLS for Postgres (openssl — a separate stack from the two above). No-op unless `postgres` is on. |
| `with-ssh` | SSH tunnels for gRPC, FlUrl, MyNoSql and Postgres. |
| `with-prometheus-metrics` | The `/metrics` endpoint plus HTTP / gRPC request metrics. Without it nothing is reported to Prometheus. |

## Common gotcha

There is **no `http-server` feature**. The HTTP server is part of `default`. A typical REST-only service should be:

```toml
service-sdk = { tag = "0.5.0", git = "https://github.com/MyJetTools/service-sdk.git", features = [
    "macros",
] }
```

Add `postgres`, `my-service-bus`, `my-nosql-data-reader-sdk`, etc. as needed. The HTTP server is already wired in by `default`.
