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

## When descriptions are missing

If the crate does not document a custom integration approach, default to the standard pattern above. Start by copying the `flurl` line and adjust the name, tag, Git URL, and optional features.