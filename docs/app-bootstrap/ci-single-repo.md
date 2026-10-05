# CI — single-repo (one service = one GitHub repo)

Use `ci-utils` in `build.rs` to auto-generate Dockerfile + workflow:

## Cargo.toml — build dependency

```toml
[build-dependencies]
ci-utils = { git = "https://github.com/MyJetTools/ci-utils.git", tag = "0.1.3" }
```

## build.rs — basic service

```rust
fn main() {
    CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_basic_service()
        .generate_github_ci_file()
        // .with_ci_test()  ← ONLY if project has #[test] somewhere
        .build();
}
```

## build.rs — with proto files

```rust
fn main() {
    CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_basic_service()
        .generate_github_ci_file()
        .build();

    ci_utils::ProtoFileBuilder::new("../proto-files/")
        .sync_and_build("MyService.proto");
}
```

## build.rs — Dioxus fullstack

```rust
fn main() {
    CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_dioxus_fullstack_service()
        .generate_github_ci_file()
        .build();
}
```

**Rules:**
- Always pass `env!("CARGO_PKG_NAME")` — never hardcode service name
- Only add `.with_ci_test()` if the project has at least one `#[test]`
- Never use `tonic_build` directly — always via `ci_utils::ProtoFileBuilder`
- Run `cargo build` once after creating `build.rs` — it generates `.github/workflows/release.yaml` and `Dockerfile`
- The pre-baked builder image described in topic `ci-monorepo` is **monorepo-only**. A
  single-repo service keeps the generated workflow as it is — do not hand-edit a `ci-utils`-generated
  file, it is overwritten on the next `cargo build`
