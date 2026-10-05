# TLS Feature (required for `https://` and `wss://` connections)

TLS is **one decision for the whole service**, made on `service-sdk`, with three answers:

| Decision | `service-sdk` features | Trade-off |
|---|---|---|
| No TLS | neither | every url the service talks to must be `http://` / `ws://` |
| TLS on ring | `with-ring-tls` | mature, widely deployed; a bundled C/assembly build (needs a C toolchain) |
| TLS on pure Rust | `with-rust-tls` | no C toolchain; x86_64/aarch64 only, far less deployed than ring |

> **Always ask the user:** *"Does the service need TLS — and if so, `with-ring-tls` or `with-rust-tls`?"*
> Do not pick one yourself. Turn on at most one of the two; neither is part of `full`.

The feature covers the whole service: it turns TLS on in FlUrl and my-grpc-extensions, and `ServiceContext::new` installs its crypto provider for the process — `my-web-socket-client` uses that one too. Never turn TLS on per library.

The service needs TLS when it connects to external WebSocket endpoints over `wss://` (e.g. Binance, exchange feeds) or calls anything over `https://` — its Seq, telemetry and settings URLs included, they always go through FlUrl. Without a TLS feature no rustls `CryptoProvider` is installed, and TLS fails at the first connection: FlUrl answers an `https://` request with `FlUrlError::UnsupportedScheme`, and `my-web-socket-client` panics when it opens a `wss://` connection — rustls finds no provider to build its client config with.

```toml
service-sdk = { ..., features = ["with-ring-tls"] }   # or "with-rust-tls" — the one the user picked
```

**Rule:** any service that uses `my-web-socket-client` with `wss://` URLs, or whose Seq, telemetry or settings URL is `https://`, needs TLS → add the TLS feature the user picked to service-sdk features.
