# gRPC — client, server, proto files

## gRPC Client Pattern

```rust
// grpc_client/users.rs
service_sdk::macros::use_grpc_client!();  // ← ALWAYS first line

#[generate_grpc_client(
    proto_file = "./proto/Users.proto",   // ← = not :
    crate_ns: "crate::users_grpc",        // ← : not =
    retries: 3,
    request_timeout_sec: 1,
    ping_timeout_sec: 1,
    ping_interval_sec: 3,
)]
pub struct UsersGrpcClient;
```

**NEVER** write gRPC client boilerplate manually.
**NEVER** add `impl` blocks on top of generated client.
**NEVER** manual imports — `use_grpc_client!()` handles everything.

`crate_ns` must exactly match the `mod` declaration in `main.rs`.

### Every proto file must be registered in main.rs

For every proto file used (client or server), add a module at the top of `main.rs`:

```rust
// main.rs
pub mod users_grpc {
    tonic::include_proto!("users");  // ← matches `package` in Users.proto
}

pub mod likes_grpc {
    tonic::include_proto!("likes");  // ← matches `package` in Likes.proto
}
```

The string in `include_proto!` must match the `package` name in the `.proto` file.
The `mod` name must match `crate_ns` in `#[generate_grpc_client]` / `generate_server!`.

```
Users.proto:          package users;
main.rs:              pub mod users_grpc { tonic::include_proto!("users"); }
grpc_client/users.rs: crate_ns: "crate::users_grpc"
```

## gRPC Server Pattern

```rust
// grpc_server/likes_grpc_service.rs
service_sdk::macros::use_grpc_server!();  // ← ALWAYS first

generate_server!(
    proto_file: "./proto/Likes.proto",
    crate_ns: "crate::likes_grpc",
    with_telemetry: true  // every handler gets `ctx: &MyTelemetryContext` as its last argument
);

// 1. Simple request → response (trivial logic → inline in handler)
async fn get_amount(
    app: &Arc<AppContext>,
    request: GetAmountGrpcRequest,
    ctx: &my_telemetry::MyTelemetryContext,
) -> GetAmountGrpcResponse {
    let amount = app.likes_repo.get_count_by_object(request.tp, &request.object_id, ctx).await;
    GetAmountGrpcResponse { amount }
}

// 2. Streaming input → delegate to flow
async fn like_unlike(
    app: &Arc<AppContext>,
    request: StreamedRequestReader<LikeUnlikeGrpcRequest>,
    ctx: &my_telemetry::MyTelemetryContext,
) {
    crate::flows::like_unlike(app, request, ctx).await;
}

// 3. Streaming output → tokio::spawn + flow with producer
async fn get_user_likes(
    app: &Arc<AppContext>,
    request: GetUserLikesGrpcRequest,
    _ctx: &my_telemetry::MyTelemetryContext,
) -> StreamedResponseWriter<GetUserLikesGrpcResponse> {
    let response_writer = StreamedResponseWriter::new(1024);
    let producer = response_writer.get_stream_producer();
    // ALWAYS tokio::spawn for streaming output — return stream handle immediately
    tokio::spawn(crate::flows::get_user_likes(app.clone(), request.tp, request.user_id, producer));
    response_writer  // ← return immediately, before spawn completes
}

// Flow with streaming — read from DB as stream, push to producer
pub async fn get_user_likes(
    app: Arc<AppContext>,
    tp: i32,
    user_id: String,
    mut producer: StreamedResponseProducer<GetUserLikesGrpcResponse>,
) {
    // ALWAYS query_rows_as_stream — never Vec for potentially large data
    let mut db_stream = app.likes_repo.get_likes_by_user(tp, &user_id).await;

    while let Some(dto) = db_stream.get_next().await {
        let response: GetUserLikesGrpcResponse = dto.into();
        producer.send(response).await;
    }
}
```

**Handlers are plain `async fn`**, not `impl` methods. First arg always `app: &Arc<AppContext>`.

**Registration** — `src/grpc_server/mod.rs` has `service_sdk::macros::generate_grpc_service!(crate::app::AppContext);` (it defines the `SdkGrpcService` that `generate_server!` implements the service for), and `main.rs` registers it before `start_application()`:

```rust
service_context.configure_grpc_server(|builder| {
    builder.add_grpc_service(LikesServer::new(SdkGrpcService::new(app.clone())));
});
```

Details: `get_service_sdk_readme`, «GRPC Server».

When to delegate to flow vs inline:
- One repo/gRPC call + simple mapping → inline in handler
- Multiple calls, business logic, conditions → flow

**ALWAYS** `with_telemetry: true` on `generate_server!` — it reads the telemetry context from the request and passes it to every handler as the last argument (`ctx: &MyTelemetryContext`); pass it on to repos and gRPC clients. The `#[with_telemetry]` attribute is for handwritten tonic handlers only, and it does not compile on a `generate_server!` handler (it looks for `let request = request.into_inner()` in the body) — see `get_my_grpc_extensions_readme`.
**NEVER** `tokio::spawn` for non-streaming handlers (await directly).

### Error Handling in gRPC

> **WHY:** gRPC has no `HttpFailResult`. Business errors are returned via optional fields
> in the response (None = not found, empty list = empty result). Panicking on IO is correct —
> the gRPC server catches it, returns status INTERNAL, and logs it.

**Business errors → optional response fields**, not panic and not Result (unless the service declares `with_error: true` on `generate_server!` — then unary handlers return `Result<T, GrpcError>` and the error reaches the client as a gRPC status; see `get_my_grpc_extensions_readme`):
```rust
// ✅ CORRECT — None means "not found", the client understands this
async fn get_user(app: &Arc<AppContext>, request: GetUserGrpcRequest) -> GetUserGrpcResponse {
    let user = app.users_repo.get(&request.user_id).await;
    GetUserGrpcResponse {
        user: user.map(|u| u.into()),  // None if not found
    }
}

// ❌ WRONG — panicking on a business error
let user = app.users_repo.get(&request.user_id).await
    .expect("user must exist");  // business error, not IO
```

**IO errors → panic** (`.expect()`), the gRPC framework catches it and returns status INTERNAL.

**Logging in gRPC handlers** — via telemetry context, not directly through `my_logger::LOGGER`:
```rust
// ctx comes from generate_server!(…, with_telemetry: true) — logs are automatically bound to the request
async fn get_order(app: &Arc<AppContext>, request: GetOrderGrpcRequest, ctx: &MyTelemetryContext)
    -> GetOrderGrpcResponse {
    let result = app.orders_repo.get(&request.order_id, ctx).await;
    // ...
}
```

## Proto File Pattern

```protobuf
syntax = "proto3";
package likes;           // snake_case, matches filename lowercase

import "google/protobuf/empty.proto";

// Message naming: {Description}GrpcRequest / {Description}GrpcResponse
message LikeUnlikeGrpcRequest {
  int32 tp = 1;           // tp = type discriminator (one service, multiple entity types)
  string user_id = 2;
  string object_id = 3;
  bool like = 4;
}

service Likes {           // PascalCase, matches filename
  // Streaming input → Empty (fire and forget / batch)
  rpc LikeUnlike(stream LikeUnlikeGrpcRequest) returns (google.protobuf.Empty);

  // Unary
  rpc GetAmountByObjectId(GetAmountByObjectIdGrpcRequest) returns (GetAmountGrpcResponse);

  // Streaming output
  rpc GetUserLikes(GetUserLikesGrpcRequest) returns (stream GetUserLikesGrpcResponse);

  // REQUIRED in every service
  rpc Ping(google.protobuf.Empty) returns (google.protobuf.Empty);
}
```

**EVERY** proto service MUST have `Ping`.
**NEVER** create a custom empty message instead of `google.protobuf.Empty`.
**NEVER** change field numbers on existing fields (breaks compatibility).
**NEVER** skip field numbers without reason.

`tp` field pattern: one service handles multiple entity types distinguished by `int32 tp`.
