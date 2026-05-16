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

### Monorepo (multiple services in one GitHub repo)

Do **NOT** use `ci-utils` / `build.rs`. Create Dockerfile and workflow manually per service.

Each service gets its own tag pattern `{service-name}-*` and its own workflow file.

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
  DIR: {service-dir}

jobs:
  build:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v6.0.2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

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

      # Add this step ONLY if the service uses proto files:
      # - name: Install Protoc
      #   uses: arduino/setup-protoc@v1

      - run: |
          export GIT_HUB_TOKEN="${{ secrets.PUBLISH_TOKEN }}"
          cd ${DIR}
          cargo build --release

      - name: Docker login
        run: |
          echo "${{ secrets.PUBLISH_TOKEN }}" | docker login https://ghcr.io -u "${{ github.actor }}" --password-stdin

      - name: Docker Build and Publish
        run: |
          cd ${DIR}
          docker build -t ${IMAGE_NAME}:${{ steps.get_version.outputs.VERSION }} .
          docker push ${IMAGE_NAME}:${{ steps.get_version.outputs.VERSION }}
```

**Placeholders to replace:**
- `{service-name}` — crate name from `Cargo.toml` (e.g. `price-feed-binance`)
- `{service-dir}` — directory name (usually same as service-name)
- `{repo-org}` — GitHub org or repo path for docker image (e.g. `my-margin-trading`)

**Release:** `git tag {service-name}-0.1.0 && git push --tags`

**Rules:**
- One workflow file per service: `release-{service-name}.yaml`
- Tag pattern: `{service-name}-*` — version is extracted after the last `-`
- Add `Install Protoc` step only if the service compiles `.proto` files
- Dockerfile lives inside the service directory, not at repo root

---

## `service-sdk` feature flags

`service-sdk` exposes the following Cargo features. Pick the smallest set the service actually needs.

| Feature | What it enables |
|---|---|
| `default` | HTTP server, settings reader, logger, telemetry — the baseline every service needs. **Do not opt out.** |
| `full` | Everything below. Convenient for prototyping, avoid in production crates. |
| `macros` | Brings in the `use_settings!()`, `use_grpc_server!()`, `use_grpc_client!()`, `use_my_postgres!()`, `use_my_http_server!()`, `use_my_no_sql_entity!()` macros. **Almost always needed.** |
| `grpc` | gRPC server + client stack. |
| `my-service-bus` | SB publisher / subscriber. |
| `postgres` | `my-postgres` re-exports + `PostgresSettings` trait auto-impl via `SdkSettingsTraits`. |
| `my-nosql-data-reader-sdk` | TCP reader for MyNoSql. |
| `my-nosql-data-writer-sdk` | HTTP writer for MyNoSql. |
| `my-nosql-sdk` | Entity macros only (no reader, no writer). |
| `websockets` | Server-side WebSocket on top of the HTTP server. |
| `http-static-files` | Mount a static-files directory through the HTTP server. |
| `signal-r` | SignalR server (uncommon). |
| `with-tls` | Required to make `wss://` outbound connections work — initialises the rustls crypto provider at startup. |
| `with-ssh` | SSH tunnel support for Postgres / other TCP clients. |
| `rustls` | Lower-level TLS plumbing (usually pulled in transitively by `with-tls`). |

### Common gotcha

There is **no `http-server` feature**. The HTTP server is part of `default`. A typical REST-only service should be:

```toml
service-sdk = { tag = "0.4.2", git = "https://github.com/MyJetTools/service-sdk.git", features = [
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

`SdkSettingsTraits` derive auto-generates the trait impl for `ServiceContext`.

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
            instruments_reader: service_context.get_ns_reader().await,
            // ...
        }
    }
}
```

### Reading data

```rust
// Get all in partition → Option<Vec<(String, Arc<T>)>>
// Tuple: (row_key, entity)
let items = app.instruments_reader
    .get_by_partition_key(InstrumentEntity::PARTITION_KEY)
    .await;

if let Some(entities) = items {
    for (row_key, entity) in entities {
        // entity: Arc<InstrumentEntity>
    }
}

// Get single entity → Option<Arc<T>>
let entity = app.instruments_reader
    .get_entity("partition_key", "row_key")
    .await;
```

**CRITICAL:** `get_by_partition_key` returns `Option<Vec<(String, Arc<T>)>>` — tuple with row_key, NOT `Vec<Arc<T>>`.

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
service_sdk::my_no_sql_sdk::abstractions  → DataSynchronizationPeriod
service_sdk::my_no_sql_sdk::core          → MyNoSqlEntity trait
```

---

## service-sdk MyNoSql Feature Names

| Feature | What it enables |
|---|---|
| `my-nosql-sdk` | Entity macros only. No reader, no writer. |
| `my-nosql-data-reader-sdk` | Entity macros + TCP reader |
| `my-nosql-data-writer-sdk` | Entity macros + HTTP writer |

---

## TLS Feature (required for `wss://` WebSocket connections)

If the service connects to external WebSocket endpoints over `wss://` (e.g. Binance, exchange feeds), the `with-tls` feature **must** be enabled in `service-sdk`. Without it, the rustls `CryptoProvider` is not initialized and the service will panic at runtime.

```toml
service-sdk = { ..., features = ["with-tls"] }
```

**Rule:** any service that uses `my-web-socket-client` with `wss://` URLs → add `"with-tls"` to service-sdk features.
