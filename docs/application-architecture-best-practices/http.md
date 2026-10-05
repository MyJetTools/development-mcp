# HTTP Action Pattern

Read `HTTP Actions Design Guide` from MCP before writing HTTP actions. Its sections «Controller Registration» and «Server Startup» (`ControllersMiddleware`, `MyHttpServer`, `start_up.rs`) describe my-http-server used on its own — **not** how a service on service-sdk does it. In a service the SDK owns the server: actions are registered on the `HttpServerBuilder` given to `configure_http_server`, by value, before `start_application()`:

```rust
// main.rs
service_context.configure_http_server(|http| {
    crate::http_server::build_controllers(&app, http);
});

// http_server/build_controllers.rs
pub fn build_controllers(app: &Arc<AppContext>, http: &mut service_sdk::HttpServerBuilder) {
    http.register_post_action(super::controllers::likes::LikeAction::new(app.clone()));
}
```

Details: `get_service_sdk_readme`, «HTTP Server». A crate with `MyHttpInput` / `MyHttpObjectStructure` models needs `my-http-utils` as a direct dependency.

```rust
// http_server/controllers/{group}/{action_name}_action.rs
service_sdk::macros::use_my_http_server!();

#[http_route(
    method: "POST",
    route: "/api/likes/v1/like-unlike",
    controller: "Likes",
    summary: "Like or unlike an object",
    description: "...",
    input_data: "LikeUnlikeInputModel",
    authorized: Yes,
    result: [
        {status_code: 200, description: "Ok"},
        {status_code: 401, description: "Unauthorized"},
    ]
)]
pub struct LikeUnlikeAction {
    app: Arc<AppContext>,
}

async fn handle_request(
    action: &LikeUnlikeAction,
    input_data: LikeUnlikeInputModel,
    _ctx: &HttpContext,
) -> Result<HttpOkResult, HttpFailResult> {
    crate::flows::like_unlike(&action.app, input_data).await;
    HttpOutput::Empty.into_ok_result(true).into()
}
```

**Controllers are thin** — one line delegating to flow.
**NEVER** business logic directly in `handle_request`.

## Error Handling in HTTP

> **WHY:** The HTTP response is seen by the client — the message should be clear but without internal IDs or PII.
> Logs are seen by developers — full context via `LogEventCtx`, no PII.
> `write_log` controls log noise: do not log what abusive traffic can generate at scale.

**HTTP response message** — short, no internal IDs, no PII:
```rust
// ✅ CORRECT
HttpFailResult::as_not_found("Order not found", true).into_err()
HttpFailResult::as_bad_request("Invalid email format", false).into_err()
HttpFailResult::as_fatal_error("Payment processing failed").into_err()

// ❌ WRONG — exposing internal IDs to the client
HttpFailResult::as_not_found(format!("Order {} not found for client {}", order_id, client_id), true).into_err()
```

**write_log:**
- `true` — authenticated request, unexpected error, signals a real system problem
- `false` — missing/invalid token, validation errors on public endpoints, 404s from bots/scrapers
- `as_fatal_error` — always logs (implicit)

**Logger context** — full context with IDs, no PII:
```rust
// ✅ CORRECT
my_logger::LOGGER.write_error(
    "get_order",
    "Order not found",
    LogEventCtx::new()
        .add("order_id", order_id.to_string())
        .add("client_id", client_id.to_string()),
);

// ❌ WRONG — PII in log
LogEventCtx::new()
    .add("email", email)    // PII — never log
    .add("phone", phone)    // PII — never log
```

**Centralize domain → HTTP error mapping** in `src/http_server/errors.rs`.
Controllers use `?` only — no inline `map_err`:
```rust
// src/http_server/errors.rs
impl From<OrderError> for HttpFailResult {
    fn from(err: OrderError) -> Self {
        match err {
            OrderError::NotFound    => HttpFailResult::as_not_found("Order not found", true),
            OrderError::InvalidData(msg) => HttpFailResult::as_bad_request(msg, false),
        }
    }
}

// Controller — ? only
let order = get_order(id).await?;
Ok(HttpOutput::as_json(order).into_ok_result(true)?)
```
