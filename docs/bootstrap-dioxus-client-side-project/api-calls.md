# API calls — FlUrl, centralized response handling, client models

## API Calls

API calls use **FlUrl**, which compiles to the browser `fetch` API under wasm. Two rules:

1. **Use relative URLs.** Pass `"/api/..."` straight into `FlUrl::new(...)` — the wasm backend resolves
   it against the current page origin for you. **Do not** compute a base URL; there is no
   `get_base_url()`. (`GlobalAppSettings::get_origin()` is still needed for the **WebSocket** URL only —
   the browser WebSocket API requires an absolute `ws`/`wss` URL.)
2. **Build the request from a shared `MyHttpInput` model** via `.execute_request(HttpVerb::X, model)`.
   Never hand-assemble a JSON body. For a request that has no parameters, pass `EmptyRequestModel`
   instead of declaring an empty model.

### Centralized response handling (`src/api/mod.rs`)

Status/error handling lives in **three helpers**, each taking the raw `Result<FlUrlResponse, FlUrlError>`
straight off FlUrl and decoding status + error **once**. Every API function forwards FlUrl's `Result`
into a helper — there is no repeated `if !is_success(...) { return Err(...) }` in each method.

```rust
// api/mod.rs
mod auth;
pub use auth::*;

use flurl::{FlUrl, FlUrlError, FlUrlResponse, HttpVerb};
use my_http_utils::THttpRequestBuilder;
use serde::de::DeserializeOwned;

use crate::models::RequestError;

fn is_success(status: u16) -> bool {
    (200..300).contains(&status)
}

async fn read_error_body(response: &mut FlUrlResponse) -> RequestError {
    let message = response
        .get_body_as_str()
        .await
        .map(|body| body.to_string())
        .unwrap_or_else(|err| err.to_string());

    RequestError { message }
}

/// 2xx → deserialize the body into `T`; any other status → `Err` carrying the response body.
pub async fn handle_http_response<T: DeserializeOwned>(
    response: Result<FlUrlResponse, FlUrlError>,
) -> Result<T, RequestError> {
    let mut response = response?;

    if is_success(response.get_status_code()) {
        return Ok(response.get_json().await?);
    }

    Err(read_error_body(&mut response).await)
}

/// Endpoints with no response body: 2xx → `Ok(())`, otherwise `Err` carrying the response body.
pub async fn handle_http_empty(
    response: Result<FlUrlResponse, FlUrlError>,
) -> Result<(), RequestError> {
    let mut response = response?;

    if is_success(response.get_status_code()) {
        return Ok(());
    }

    Err(read_error_body(&mut response).await)
}

/// Like `handle_http_response`, but `401`/`403` map to `Ok(None)` (not logged in / no rights).
pub async fn handle_http_response_opt<T: DeserializeOwned>(
    response: Result<FlUrlResponse, FlUrlError>,
) -> Result<Option<T>, RequestError> {
    let mut response = response?;

    let status = response.get_status_code();

    if status == 401 || status == 403 {
        return Ok(None);
    }

    if is_success(status) {
        return Ok(Some(response.get_json().await?));
    }

    Err(read_error_body(&mut response).await)
}

/// Authenticated POST: attaches the session token, then executes any shared `MyHttpInput` model.
/// The `THttpRequestBuilder` bound (from `my-http-utils`) is what lets the model drive the request.
async fn authed_post<TModel: THttpRequestBuilder>(
    url: &str,
    model: TModel,
) -> Result<FlUrlResponse, FlUrlError> {
    let token = crate::web::storage::session::get_session_token().unwrap_or_default();

    FlUrl::new(url)
        .with_header("Authorization", format!("Bearer {}", token))
        .execute_request(HttpVerb::Post, model)
        .await
}
```

### Example: `api/auth.rs`

Each call builds a shared model, hands FlUrl's `Result` to a helper, and returns. No status checks,
no manual JSON, no base URL.

```rust
// api/auth.rs
use flurl::{EmptyRequestModel, FlUrl, HttpVerb};
use rest_api_shared::auth::*;

use crate::models::RequestError;

use super::{authed_post, handle_http_empty, handle_http_response, handle_http_response_opt};

// POST with a body, no response payload.
pub async fn send_code(email: &str) -> Result<(), RequestError> {
    let response = FlUrl::new("/api/auth/v1/SendCode")
        .execute_request(HttpVerb::Post, SendCodeRequest { email: email.to_string() })
        .await;

    handle_http_empty(response).await
}

// POST with a body and a typed response payload.
pub async fn verify_code(email: &str, code: &str) -> Result<SessionTokenResponse, RequestError> {
    let request = VerifyCodeRequest {
        email: email.to_string(),
        code: code.to_string(),
    };

    let response = FlUrl::new("/api/auth/v1/VerifyCode")
        .execute_request(HttpVerb::Post, request)
        .await;

    handle_http_response(response).await
}

// Authenticated GET with no parameters — `EmptyRequestModel`, 401/403 → `Ok(None)`.
pub async fn get_user_info() -> Result<Option<UserInfoResponse>, RequestError> {
    let token = crate::web::storage::session::get_session_token().unwrap_or_default();

    let response = FlUrl::new("/api/auth/v1/UserInfo")
        .with_header("Authorization", format!("Bearer {}", token))
        .execute_request(HttpVerb::Get, EmptyRequestModel)
        .await;

    handle_http_response_opt(response).await
}

// Authenticated POST with no parameters — reuses the generic helper + `EmptyRequestModel`.
pub async fn logout() -> Result<(), RequestError> {
    let response = authed_post("/api/auth/v1/Logout", EmptyRequestModel).await;

    handle_http_empty(response).await
}
```

## Models

`src/models/` holds **only client-side view-state** — no wire models (those live in `rest-api-shared`,
see topic `shared-wire-models`). One file per type; `mod.rs` uses the
standard `mod x; pub use x::*;` pattern.

`RequestError` is the client-side error type every API function returns. It converts from both FlUrl
transport errors and `serde_json` errors, so the helpers in `api/mod.rs` can use `?` freely.

```rust
// models/request_error.rs
use std::fmt;

pub struct RequestError {
    pub message: String,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<flurl::FlUrlError> for RequestError {
    fn from(err: flurl::FlUrlError) -> Self {
        Self { message: err.to_string() }
    }
}

// get_json() / try_as_json() surface serde_json::Error, so RequestError converts from it too.
impl From<serde_json::Error> for RequestError {
    fn from(err: serde_json::Error) -> Self {
        Self { message: err.to_string() }
    }
}
```

Client view-state — e.g. a form buffer — is a plain struct, and unlike the pure shared wire models it
may carry behaviour:

```rust
// models/login_form.rs
#[derive(Default, Clone)]
pub struct LoginForm {
    pub email: String,
    pub code: String,
}

impl LoginForm {
    pub fn is_email_valid(&self) -> bool {
        self.email.contains('@')
    }
}
```
