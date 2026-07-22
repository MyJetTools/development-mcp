---
alwaysApply: true
---
# Cargo Dependencies Guide

Use this guide when adding new dependencies to `Cargo.toml`. If no crate-specific instructions exist, add the dependency in the standard way using the `flurl` entry as the template.

## Standard pattern (Git + tag)

```toml
[dependencies]
flurl = { tag = "${last_tag}", git = "https://github.com/MyJetTools/fl-url.git" }
```

- Place the dependency under `[dependencies]`.
- Prefer pinned Git tags for internal crates to keep builds reproducible. Read the tag from the latest GitHub release of the crate (do not invent or use `main`).
- Keep existing style: `tag` then `git`, features in a separate `features = [...]` block when needed.
- Group related dependencies together and maintain the current ordering.

## Crates.io pattern

If the crate is published on crates.io and no Git pin is required, add it with an "*" version:

```toml
[dependencies]
serde = { version = "*", features = ["derive"] }
# or without extra features
anyhow = "*"
```

- If the library is a common/standard crates.io dependency, pin it as `version = "*"`, matching the local convention unless project-specific guidance says otherwise.
- Add features explicitly; keep the style consistent with existing entries.

## When a dependency fails to compile

If the build breaks inside one of the dependencies (not in your own code), **do not start debugging the library first**. Almost always it is a stale lock file / stale build cache after a tag bump, not a real bug in the crate.

Run this in the project root, in order, before any analysis:

```bash
cargo clean
cargo update
cargo build
```

- Only if the error still reproduces after a clean rebuild, start investigating the dependency itself (version/tag mismatch, feature flags, breaking API change).
- Never edit or patch a dependency, downgrade a tag, or report "the library is broken" until the `clean` → `update` → `build` cycle has been done and the error persists.

## When descriptions are missing

If the crate does not document a custom integration approach, default to the standard pattern above. Start by copying the `flurl` line and adjust the name, tag, Git URL, and optional features.