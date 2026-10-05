---
alwaysApply: true
---
# App Bootstrap Guide

How a service on `service-sdk` is wired up: CI / GitHub Actions, the `service-sdk` features, the MyNoSql reader and writer, TLS.

Read a topic: `resource://app-bootstrap/{topic}`, or `get_app_bootstrap_guide` with `topic`.

## Two decisions that belong to the user

> **CI — always ask the user:** *"Should I create a CI workflow for this service?"*
> The approach depends on whether the project is a single-repo or a monorepo: topic `ci-single-repo` or `ci-monorepo`.

> **TLS — always ask the user:** *"Does the service need TLS — and if so, `with-ring-tls` or `with-rust-tls`?"*
> Do not pick one yourself: topic `tls`.

## Topics

| Topic | What is inside |
| --- | --- |
| [`ci-single-repo`](ci-single-repo.md) | One service = one GitHub repo: `ci-utils` `CiGenerator` in `build.rs` generates the Dockerfile and `release.yaml` — basic service, proto files, Dioxus fullstack |
| [`ci-monorepo`](ci-monorepo.md) | Several services in one repo: hand-written runtime Dockerfile, `release-{service-name}.yaml` and the pre-baked builder image `build-{service-name}-docker.yaml` (~2 min releases instead of ~10), its load-bearing rules, compile-time secrets, adopting it for a service |
| [`service-sdk-features`](service-sdk-features.md) | Every `service-sdk` Cargo feature and what it enables; there is no `http-server` feature |
| [`my-no-sql`](my-no-sql.md) | MyNoSql reader and writer wiring: features, settings, `AppContext` fields, reads and writes through `with_retries`, the entity macro, module paths, feature names |
| [`tls`](tls.md) | TLS is one decision for the whole service — no TLS, `with-ring-tls` or `with-rust-tls` — and what fails without it |
