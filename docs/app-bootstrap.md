---
alwaysApply: true
---
## CI / GitHub Actions

> **Always ask the user:** *"Should I create a CI workflow for this service?"*
> The approach depends on whether the project is a single-repo or a monorepo.

### Single-repo (one service = one GitHub repo)

Use `ci-utils` in `build.rs` to auto-generate Dockerfile + workflow:

#### Cargo.toml — build dependency

```toml
[build-dependencies]
ci-utils = { git = "https://github.com/MyJetTools/ci-utils.git", tag = "0.1.3" }
```

#### build.rs — basic service

```rust
fn main() {
    CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_basic_service()
        .generate_github_ci_file()
        // .with_ci_test()  ← ONLY if project has #[test] somewhere
        .build();
}
```

#### build.rs — with proto files

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

#### build.rs — Dioxus fullstack

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
- The pre-baked builder image described in the monorepo section below is **monorepo-only**. A
  single-repo service keeps the generated workflow as it is — do not hand-edit a `ci-utils`-generated
  file, it is overwritten on the next `cargo build`

### Monorepo (multiple services in one GitHub repo)

Do **NOT** use `ci-utils` / `build.rs` for CI. Create the Dockerfile and the workflows manually per
service. (`build.rs` is still used in a monorepo for proto files and CSS — just never with
`CiGenerator`.)

Each service gets its own tag pattern `{service-name}-*` and **two** workflow files:

| File | Trigger | What it does |
|---|---|---|
| `release-{service-name}.yaml` | tag `{service-name}-*` | builds the service and pushes the runtime image — ~2 min |
| `build-{service-name}-docker.yaml` | `workflow_dispatch` | bakes the dependency graph into a builder image — ~10 min, run by hand |

Generate both. A release workflow without its builder image works, it is just five times slower.

#### Dockerfile — `{service-dir}/Dockerfile`

```dockerfile
FROM ubuntu:22.04
COPY ./target/release/{service-name} ./target/release/{service-name}
ENTRYPOINT ["./target/release/{service-name}"]
```

#### Workflow — `.github/workflows/release-{service-name}.yaml`

```yaml
name: Release App
on:
  push:
    tags:
      - "{service-name}-*"

env:
  IMAGE_NAME: ghcr.io/{repo-org}/{service-name}
  BUILD_IMAGE: ghcr.io/{repo-org}/{service-name}-build-docker
  DIR: {service-dir}

jobs:
  build:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v6.0.2

      - name: Get the version
        id: get_version
        run: |
          TAG="${{ github.ref_name }}"
          VERSION="${TAG##*-}"
          echo "VERSION=$VERSION" >> "$GITHUB_OUTPUT"

      - name: Updating version
        run: |
          cd ${DIR}
          sed -i -e 's/^version = .*/version = "${{ steps.get_version.outputs.VERSION }}"/' Cargo.toml

      - name: Docker login
        run: |
          echo "${{ secrets.PUBLISH_TOKEN }}" | docker login https://ghcr.io -u "${{ github.actor }}" --password-stdin

      - name: Pull the builder image
        id: builder
        continue-on-error: true
        run: docker pull ${BUILD_IMAGE}:latest

      - name: Build (warm, in the builder image)
        if: steps.builder.outcome == 'success'
        run: |
          docker run --rm \
            -v "${PWD}:/src" \
            -w /src/${DIR} \
            ${BUILD_IMAGE}:latest \
            bash -c "cargo build --release \
              && mkdir -p target/release \
              && cp /build/target/release/${DIR} target/release/${DIR}"

      - uses: actions-rust-lang/setup-rust-toolchain@v1.15.2
        if: steps.builder.outcome != 'success'
        with:
          toolchain: stable
          rustflags: ""

      - name: Install Protoc
        if: steps.builder.outcome != 'success'
        run: sudo apt-get update && sudo apt-get install -y protobuf-compiler

      - name: Build (cold, on the runner)
        if: steps.builder.outcome != 'success'
        run: |
          export GIT_HUB_TOKEN="${{ secrets.PUBLISH_TOKEN }}"
          cd ${DIR}
          cargo build --release

      - name: Docker Build and Publish
        run: |
          cd ${DIR}
          docker build -t ${IMAGE_NAME}:${{ steps.get_version.outputs.VERSION }} .
          docker push ${IMAGE_NAME}:${{ steps.get_version.outputs.VERSION }}
```

`${DIR}` is expanded by the **runner** before `docker run`, which is why the inner `bash -c` is in
double quotes — the container has no `DIR`.

The `cp` line uses `${DIR}` as the **binary** name, and the runtime Dockerfile copies
`./target/release/{service-name}`. That only lines up while `{service-dir}` and `{service-name}` are
the same string, which is the normal case. If they differ, add a separate `BIN: {service-name}` to
the workflow `env:` and use `${BIN}` for the binary in both the `cp` and the Dockerfile — otherwise
the warm build succeeds and the `docker build` fails on a missing file.

#### Builder image — `.github/workflows/build-{service-name}-docker.yaml`

A release used to spend **thirteen of its fourteen minutes** rebuilding a dependency graph that had
not changed. This workflow bakes that graph into a Docker image once, by hand; every release then
mounts fresh sources over it and compiles only the delta. Measured on `accumulator-grpc`,
`ubuntu-22.04`: release **9 m 55 s → 2 m 27 s** (pull 41 s, compile 83 s, image 9 s); baking the
image itself takes ~9 m 36 s and is *not* run per release.

`workflow_dispatch` **only** — there is no Docker daemon on the dev machine, so a builder image can
only be tested where it is built, and manual dispatch lets you iterate on it without burning release
tags. It writes the builder Dockerfile inline with a heredoc, over the runtime `Dockerfile`, in the
runner's throwaway checkout — so the service folder keeps exactly one Dockerfile, the runtime one.

```yaml
name: Build {service-name}-build-docker

on:
  workflow_dispatch:

env:
  IMAGE_NAME: ghcr.io/{repo-org}/{service-name}-build-docker
  DIR: {service-dir}

jobs:
  build:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v6.0.2

      - name: Docker login
        run: |
          echo "${{ secrets.PUBLISH_TOKEN }}" | docker login https://ghcr.io -u "${{ github.actor }}" --password-stdin

      # Quoted heredoc (<<'DOCKERFILE') — unquoted, the runner's shell would expand $PATH in
      # `ENV PATH=...:$PATH` and bake the RUNNER's PATH into the image.
      - name: Generate the builder Dockerfile
        run: |
          cat > ${DIR}/Dockerfile <<'DOCKERFILE'
          FROM ubuntu:22.04

          ENV DEBIAN_FRONTEND=noninteractive
          ENV CARGO_HOME=/usr/local/cargo
          ENV CARGO_TARGET_DIR=/build/target
          ENV PATH=/usr/local/cargo/bin:$PATH

          RUN apt-get update && apt-get install -y --no-install-recommends \
                  build-essential \
                  ca-certificates \
                  curl \
                  git \
                  libprotobuf-dev \
                  libssl-dev \
                  pkg-config \
                  protobuf-compiler \
              && rm -rf /var/lib/apt/lists/*

          RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
              | sh -s -- -y --default-toolchain stable --profile minimal

          COPY . /src

          RUN cd /src/{service-dir} && cargo build --release

          # Emptied on purpose: an image run WITHOUT the release's mount would otherwise compile the
          # sources baked at build time and look like it worked.
          RUN rm -rf /src && mkdir -p /src

          WORKDIR /src/{service-dir}
          DOCKERFILE
          cat ${DIR}/Dockerfile

      - name: Build and push the builder image
        run: |
          docker build -f ${DIR}/Dockerfile -t ${IMAGE_NAME}:latest .
          docker push ${IMAGE_NAME}:latest
```

**Load-bearing rules.** Each of these silently turns a warm build into a cold one — no error, just
the old fourteen minutes:

1. **`CARGO_HOME` and `CARGO_TARGET_DIR` must live OUTSIDE `/src`.** The release bind-mounts its
   checkout over `/src`, and a bind mount *hides* whatever the image had underneath. A target dir
   inside `/src` disappears at exactly the moment it is supposed to be reused.
2. **Bake at the same absolute path the release mounts** (`/src`). Cargo fingerprints record
   absolute paths; the same sources at another path are a cold build wearing a warm image's name.
3. **The base must match the runtime image's base** (`ubuntu:22.04`). The binary is copied into the
   runtime image, not rebuilt there — a builder on a newer base links it against a newer glibc and
   the container dies at start-up on a symbol lookup.
4. **`Cargo.lock` must be committed** — un-ignore it per service in `.gitignore`. With the lock
   ignored CI resolves fresh and takes the newest semver-compatible release of every transitive
   crate, so one patch published anywhere in a ~400-crate graph invalidates the whole image. Warm
   hits become a lottery you cannot even measure. Bonus: a broken transitive dependency now
   reproduces locally instead of only in CI.
5. **Build context is the repo root**, not the service folder — sibling crates arrive as path
   dependencies and `build.rs` reads `../proto-files/`.
6. **Pre-install what the graph needs**, and it is more than protoc: `build-essential` (jemalloc is
   a configure+make build), `libprotobuf-dev` (protoc without the well-known types is not protoc),
   `pkg-config`, `libssl-dev`, `git`, `ca-certificates`.

**Do not pass a build secret to the builder image.** All MyJetTools / my-ai-utils git dependencies
are public. A token passed as `--build-arg` is written into the image history, where anyone who can
pull the image can read it back. If a private dependency ever appears, use a BuildKit secret mount
— never `--build-arg`.

**Compile-time secrets need `-e` on the warm step, or they vanish silently.** If the service bakes
values into the binary (`ci_utils::bake_compile_time_secret("X")`, or `add_compile_time_secret` on a
`CiGenerator` build), note that the cold step `export`s them into the runner's shell while
`docker run` starts a container with none of the runner's environment. `build.rs` then sees nothing
— and it cannot even complain, because the guard that fails a release build keys off
`GITHUB_ACTIONS=true`, which is also absent inside the container. The result is a green workflow
shipping a binary whose `option_env!` returned `None`. Pass each secret explicitly:

```yaml
          docker run --rm \
            -e ENCRYPTION_KEY="${{ secrets.ENCRYPTION_KEY }}" \
            -e GITHUB_ACTIONS=true \
            -v "${PWD}:/src" \
```

`-e` sets the environment of one container run — it is not written into any image layer, so this is
not the `--build-arg` mistake above. Keep `-e GITHUB_ACTIONS=true` so a missing secret still turns
the step red instead of shipping quietly.

Only the service crate and its path-dependency siblings recompile each release: `actions/checkout`
stamps every file with the checkout time, so cargo considers all of them dirty regardless. That is
fine and expected — they are small. The dependency sources keep their baked mtimes inside the
image's `CARGO_HOME`, and that graph is the thirteen minutes.

**Do not use the GitHub Actions cache for this.** It is scoped **by ref**: a run reads caches
written by its own ref or by the default branch, and every release runs on a fresh `{service-name}-*`
tag — so each build writes hundreds of megabytes under a ref nothing will ever read from again. With
~28 services sharing one 10 GB repo quota they also evict each other. `cache-workspaces` on
`setup-rust-toolchain` looks like it is working and is not: the tell is a cache step that finishes in
three seconds. A registry image has no ref scoping — written once, read by every tag afterwards.

#### Adopting the builder image for a service

1. Add both workflow files (copy from an adopted service, substitute the name — the files differ by
   nothing else).
2. Un-ignore its lock: `!{service-dir}/Cargo.lock` in `.gitignore`, then commit the lock. Verify
   with `cargo check --locked` first — a lock that does not satisfy the manifests fails the build.
3. Run `build-{service-name}-docker.yaml` by hand and let it finish.
4. Release as usual. Confirm in the run that *Build (warm, …)* ran and the cold steps were skipped.

**Not for Dioxus WASM builds** (`dashboards-ui` and friends) — own toolchain, own `dx build`, built
inside the `ghcr.io/my-jet-tools/dioxus-docker` container instead. Different problem, different fix: see the
Dioxus client-side bootstrap guide for that workflow.

**Placeholders to replace:**
- `{service-name}` — crate name from `Cargo.toml` (e.g. `price-feed-binance`)
- `{service-dir}` — directory name; keep it identical to `{service-name}` unless there is a reason
  not to, otherwise add a `BIN` env var as described above
- `{repo-org}` — GitHub org or repo path for docker image (e.g. `my-margin-trading`)

**Release:** `gh release create {service-name}-0.1.0 --title "{service-name}-0.1.0" --notes ""` —
this creates both the release and the tag. Full flow, re-deploys and troubleshooting: the release
guide (`get_release_guide`).

**Rules:**
- One workflow file per service: `release-{service-name}.yaml`, plus one
  `build-{service-name}-docker.yaml` for its builder image
- Tag pattern: `{service-name}-*` — version is extracted after the last `-`
- Dockerfile lives inside the service directory, not at repo root — and it is the **runtime** one;
  the builder Dockerfile is generated inline by the dispatch workflow and never committed
- A release never breaks because of the builder: the pull step is `continue-on-error` and the cold
  steps behind the `if:` are the exact build used before the image existed. The worst outcome of a
  bad image is the time we used to pay anyway
- Re-bake the builder image by hand when the graph moves — a MyJetTools tag bumped, a dependency
  added, `Cargo.lock` updated. Nothing else needs it

---

## `service-sdk` feature flags

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
| `with-rust-tls` | Same TLS with the pure-Rust provider (`rustls-graviola`): no C toolchain, but x86_64/aarch64 only and far less deployed than ring. No TLS / `with-ring-tls` / `with-rust-tls` is one decision for the whole service — **ask the user**, see "TLS Feature" below. |
| `with-postgres-tls` | TLS for Postgres (openssl — a separate stack from the two above). No-op unless `postgres` is on. |
| `with-ssh` | SSH tunnels for gRPC, FlUrl, MyNoSql and Postgres. |
| `with-prometheus-metrics` | The `/metrics` endpoint plus HTTP / gRPC request metrics. Without it nothing is reported to Prometheus. |

### Common gotcha

There is **no `http-server` feature**. The HTTP server is part of `default`. A typical REST-only service should be:

```toml
service-sdk = { tag = "0.5.0", git = "https://github.com/MyJetTools/service-sdk.git", features = [
    "macros",
] }
```

Add `postgres`, `my-service-bus`, `my-nosql-data-reader-sdk`, etc. as needed. The HTTP server is already wired in by `default`.

---

## MyNoSql Reader (add only when the service needs to read from MyNoSql)

### service-sdk feature

```toml
service-sdk = { ..., features = ["my-nosql-data-reader-sdk"] }
```

### Settings — add field

```rust
pub struct SettingsModel {
    pub my_no_sql_tcp_reader: String,   // ← REQUIRED for NoSql reader
    // ... other fields
}
```

The `AutoGenerateSettingsTraits` derive implements `MyNoSqlTcpConnectionSettings` for `SettingsReader` from this field. (`SdkSettingsTraits` generates only `ServiceInfo`.)

### AppContext — add reader field

```rust
use my_no_sql_entities::InstrumentEntity;

pub struct AppContext {
    pub instruments_reader:
        Arc<service_sdk::my_no_sql_sdk::reader::MyNoSqlDataReaderTcp<InstrumentEntity>>,
    // ... other fields
}

impl AppContext {
    pub async fn new(
        settings_reader: Arc<SettingsReader>,
        service_context: &ServiceContext,   // ← NOT underscore — needed for get_ns_reader
    ) -> Self {
        Self {
            // Generic type inferred from field type
            instruments_reader: service_context.get_ns_reader(),
            // ...
        }
    }
}
```

### Reading data

```rust
// Get all in partition → Option<BTreeMap<String, Arc<T>>>
// Key: row_key, value: entity
let items = app.instruments_reader
    .get_by_partition_key(InstrumentEntity::PARTITION_KEY);

if let Some(entities) = items {
    for (row_key, entity) in entities {
        // entity: Arc<InstrumentEntity>
    }
}

// Get single entity → Option<Arc<T>>
let entity = app.instruments_reader
    .get_entity("partition_key", "row_key");
```

**CRITICAL:** `get_by_partition_key` returns `Option<BTreeMap<String, Arc<T>>>` — keyed by row_key, NOT `Vec<Arc<T>>`.

### First snapshot — no manual wait

`service_context.start_application()` starts the MyNoSql connection first and waits until every reader obtained through `get_ns_reader` has received its first snapshot; only then it starts timers and the queues, events loops and background executors created through `service_context`, then Service Bus, HTTP and gRPC. Do **not** call `wait_until_first_data_arrives()` on these readers. Before `start_application()` a reader is empty. Details: `get_application_architecture_best_practices` → "Initial Snapshot — service-sdk waits for it".

---

## MyNoSql Writer (add only when the service needs to write to MyNoSql)

### service-sdk feature

```toml
service-sdk = { ..., features = ["my-nosql-data-writer-sdk"] }
```

### Settings — implement MyNoSqlWriterSettings

```rust
use service_sdk::my_no_sql_sdk::data_writer::MyNoSqlWriterSettings;

pub struct Settings {
    pub my_no_sql_writer_url: String,
    // ... other fields
}

#[async_trait::async_trait]
impl MyNoSqlWriterSettings for AppSettingsReader {
    async fn get_url(&self) -> String {
        self.settings_reader
            .get(|s| s.my_no_sql_writer_url.clone())
            .await
    }

    fn get_app_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn get_app_version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}
```

### AppContext — add writer field

```rust
use my_no_sql_entities::InstrumentEntity;
use service_sdk::my_no_sql_sdk::{
    abstractions::DataSynchronizationPeriod,
    data_writer::{CreateTableParams, MyNoSqlDataWriter, MyNoSqlDataWriterWithRetries},
};

pub struct AppContext {
    instruments: MyNoSqlDataWriter<InstrumentEntity>,
    // ... other fields
}

impl AppContext {
    pub async fn new(settings_reader: Arc<AppSettingsReader>) -> Self {
        Self {
            instruments: MyNoSqlDataWriter::new(
                settings_reader.clone(),
                Some(CreateTableParams {
                    persist: true,
                    max_partitions_amount: None,
                    max_rows_per_partition_amount: None,
                }),
                DataSynchronizationPeriod::Immediately,
            ),
            // ...
        }
    }

    // ALWAYS expose via with_retries
    pub fn get_instruments(&self) -> MyNoSqlDataWriterWithRetries<InstrumentEntity> {
        self.instruments.with_retries(3)
    }
}
```

### Writing / reading data

```rust
let w = app_ctx.get_instruments();

// Insert or replace
w.insert_or_replace_entity(&entity).await.unwrap();

// Bulk insert or replace
w.bulk_insert_or_replace(&entities).await.unwrap();

// Get entity → Result<Option<T>>
let entity = w.get_entity("pk", "rk", None).await.unwrap();

// Get all in partition → Result<Option<Vec<T>>>
let items = w.get_by_partition_key("pk", None).await.unwrap().unwrap_or_default();

// Delete
w.delete_row("pk", "rk").await.unwrap();
```

**NEVER** call writer methods directly — always through `.with_retries(N)`.

---

## MyNoSql Entity Macro

service-sdk provides `use_my_no_sql_entity!()` to import entity macros.
Available with any of: `my-nosql-sdk`, `my-nosql-data-reader-sdk`, `my-nosql-data-writer-sdk`.

```rust
service_sdk::macros::use_my_no_sql_entity!();

#[my_no_sql_entity("instruments")]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct InstrumentEntity {
    pub name: String,
}
```

**CRITICAL:** `use_my_no_sql_entity!()` does NOT import serde.
Always use `serde::Serialize`, `serde::Deserialize` (fully qualified).

---

## service-sdk Module Paths for MyNoSql

```
service_sdk::my_no_sql_sdk::reader        → MyNoSqlDataReaderTcp<T>
service_sdk::my_no_sql_sdk::data_writer   → MyNoSqlDataWriter<T>, MyNoSqlDataWriterWithRetries<T>,
                                             MyNoSqlWriterSettings, CreateTableParams
service_sdk::my_no_sql_sdk::abstractions  → DataSynchronizationPeriod, MyNoSqlEntity trait
```

---

## service-sdk MyNoSql Feature Names

| Feature | What it enables |
|---|---|
| `my-nosql-sdk` | Entity macros only. No reader, no writer. |
| `my-nosql-data-reader-sdk` | Entity macros + TCP reader |
| `my-nosql-data-writer-sdk` | Entity macros + HTTP writer |

---

## TLS Feature (required for `https://` and `wss://` connections)

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
