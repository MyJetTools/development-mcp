# build.rs Pattern

Every service has `build.rs` that does two things:

```rust
fn main() {
    // 1. Generate Dockerfile + GitHub CI
    CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_basic_service()
        .generate_github_ci_file()
        // .with_ci_test()  ← ONLY if project has #[test] somewhere
        .build();

    // 2. Sync + compile proto files from shared repo
    ProtoFileBuilder::new("../proto-files/")
        .sync_and_build("Users.proto")
        .sync_and_build("Payments.proto");
}
```

**NEVER** add `.with_ci_test()` unless project has actual unit tests.
**ALWAYS** pass `env!("CARGO_PKG_NAME")` — never hardcode service name.
**NEVER** copy proto files manually into the service.
**NEVER** use `tonic_build` directly — always via `ci_utils::ProtoFileBuilder`.
Proto files live in a **shared repo** (`../proto-files/`), not in the service itself.

## Monorepo build.rs

In a monorepo (multiple services in one GitHub repo), **do NOT use `CiGenerator`**. Dockerfile and CI workflows are created manually per service. `build.rs` only syncs + compiles proto files:

```rust
fn main() {
    ci_utils::ProtoFileBuilder::new("../proto-files/")
        .sync_and_build("MyService.proto");
}
```

Each monorepo service owns **two** hand-written workflow files, not one:

- `.github/workflows/release-{service-name}.yaml` — triggers on tag `{service-name}-*`, builds
  inside the pre-baked builder image and pushes the runtime image (~2 min)
- `.github/workflows/build-{service-name}-docker.yaml` — `workflow_dispatch` only, bakes the
  dependency graph into `ghcr.io/{org}/{service-name}-build-docker:latest` (~10 min, run by hand
  whenever the graph moves)

`Cargo.lock` must be committed for a monorepo service — un-ignore it in `.gitignore`. Without a
committed lock, CI resolves fresh every run and the builder image stops matching.

**Never hand-write these two files from memory** — fetch the exact templates and the load-bearing
rules with `get_app_bootstrap_guide`, topic `ci-monorepo`. Details like `CARGO_HOME` living
outside `/src` and the builder base matching the runtime base are what make the difference between a
2-minute build and a 10-minute one, and getting them wrong fails silently.
