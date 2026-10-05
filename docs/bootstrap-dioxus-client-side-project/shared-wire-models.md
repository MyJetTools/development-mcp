# Shared Wire Models (`rest-api-shared`)

Every model that crosses the wire lives in a **separate crate `rest-api-shared`** and is reused
**verbatim** by both the REST-API server and the wasm client. One definition — both ends. This kills
request/response drift: the client cannot serialize a shape the server does not accept.

- **Request models** derive `MyHttpInput`:

  ```rust
  // rest-api-shared/src/auth.rs
  use my_http_utils::macros::MyHttpInput;

  #[derive(MyHttpInput)]
  pub struct SendCodeRequest {
      #[http_body(name = "email", description = "")]
      pub email: String,
  }

  #[derive(MyHttpInput)]
  pub struct VerifyCodeRequest {
      #[http_body(name = "email", description = "")]
      pub email: String,
      #[http_body(name = "code", description = "")]
      pub code: String,
  }
  ```

- **Response models** derive `Serialize, Deserialize, MyHttpObjectStructure, Clone, Debug, PartialEq`:

  ```rust
  use serde::{Deserialize, Serialize};
  use my_http_utils::macros::MyHttpObjectStructure;

  #[derive(Serialize, Deserialize, MyHttpObjectStructure, Clone, Debug, PartialEq)]
  pub struct SessionTokenResponse {
      pub token: String,
      #[serde(rename = "refreshToken")]
      pub refresh_token: String,
  }

  #[derive(Serialize, Deserialize, MyHttpObjectStructure, Clone, Debug, PartialEq)]
  pub struct UserInfoResponse {
      pub email: String,
      pub first_name: String,
      pub last_name: String,
  }
  ```

## The crate is wasm-clean

`rest-api-shared` depends **only on `my-http-utils`** (never on `my-http-server`), so it compiles to
`wasm32`. A `server` feature turns on the server-side request parsing that only the REST-API service
needs:

```toml
# rest-api-shared/Cargo.toml
[dependencies]
my-http-utils = { tag = "0.1.0", git = "https://github.com/MyJetTools/my-http-utils.git" }
serde = { version = "*", features = ["derive"] }

[features]
server = ["my-http-utils/server"]
```

- **Server** depends on it **with** `features = ["server"]` — it needs to parse the incoming request.
- **Client** depends on it **without** any feature — it only needs the schema plus the FlUrl request
  builder (the `THttpRequestBuilder` impl used by `execute_request`).

## Models stay pure data

Shared types carry **no methods and no behaviour** — only fields. Presentational or derived helpers
belong to the **consumer** as an extension trait, so the wire contract never picks up client-only logic:

```rust
// client: src/models/user_info_ext.rs
use rest_api_shared::auth::UserInfoResponse as UserInfo;

pub trait UserInfoExt {
    fn full_name(&self) -> String;
}

impl UserInfoExt for UserInfo {
    fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}
```

When a shared type reads better under a local name, re-export it:

```rust
pub use rest_api_shared::auth::UserInfoResponse as UserInfo;
```

> **NEVER** put `///` doc-comments on the fields of a struct that derives `MyHttpInput` or
> `MyHttpObjectStructure` — the proc-macro panics. Document the struct itself instead, or use the
> `description = "..."` attribute argument on the field.

## What stays in `src/models/`

Only **client view-state** that never crosses the wire — dialog state, form buffers, UI toggles,
the `RequestError` type, WS message enums. Anything that goes to or comes from the REST API belongs in
`rest-api-shared`, not here.
