# Service Bus — contracts and subscribers

## Service Bus Contract Pattern

SB contracts live in a shared crate `my-sb-contracts` — used by both publisher and subscriber services.

```rust
// my-sb-contracts/src/like_unlike.rs
use service_sdk::my_service_bus;
use service_sdk::my_service_bus::macros::my_sb_entity_protobuf_model;

#[derive(Clone, PartialEq, ::prost::Message)]
#[my_sb_entity_protobuf_model(topic_id = "like-unlike")]  // ← topic name here
pub struct LikeUnlikeSbContract {
    #[prost(int32, tag = "1")]
    pub tp: i32,
    #[prost(string, tag = "2")]
    pub user_id: String,
    #[prost(string, tag = "3")]
    pub object_id: String,
    #[prost(bool, tag = "4")]
    pub like: bool,
}
```

### Rules:
- SB contracts live in a **shared crate** (`my-sb-contracts`) — never define them in the service itself
- Serialization: **protobuf** via `prost` — `#[derive(prost::Message)]` + field tags
- Topic ID defined in macro: `#[my_sb_entity_protobuf_model(topic_id = "topic-name")]`
- Field tags: sequential `1, 2, 3...` — **NEVER change existing tags** (breaks deserialization)
- **NEVER add `serde`** to SB contracts — protobuf only
- Naming: `{Action}SbContract` — e.g. `LikeUnlikeSbContract`, `UserCreatedSbContract`

## Service Bus Subscriber Pattern

```rust
// sb_subscribers/likes_sb_subscriber.rs
use service_sdk::my_service_bus::prelude::*;

pub struct LikesSbSubscriber {
    app: Arc<AppContext>,
}

impl LikesSbSubscriber {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl SubscriberCallback<LikeUnlikeSbContract> for LikesSbSubscriber {
    async fn handle_messages(
        &self,
        messages_reader: &MessagesReader<LikeUnlikeSbContract>,
    ) -> Result<(), MySbSubscriberHandleError> {
        while let Some(mut next_message) = messages_reader.get_next_message().await {
            let sb_msg = next_message.take_message();

            // Engage telemetry BEFORE calling script
            let ctx = next_message.engage_telemetry().await;

            crate::scripts::handle_event(&self.app, sb_msg, &ctx).await;
        }
        Ok(())
    }
}
```

### Registration in main.rs — after AppContext::new()

```rust
let app = Arc::new(AppContext::new(settings_reader, &service_context).await);

// Register AFTER app is created, BEFORE start_application.
//
// `register_sb_subscribe` is SYNCHRONOUS (returns `&Self`, not a future) and takes
// three positional arguments: (callback, delete_on_no_subscribers: bool, single_connection: bool).
// There is no `TopicQueueType` argument despite the old enum still existing internally
// in `my-service-bus-abstractions`.
service_context.register_sb_subscribe(
    Arc::new(LikesSbSubscriber::new(app.clone())),
    /* delete_on_no_subscribers */ true,
    /* single_connection         */ false,
);

service_context.start_application().await;
```

### Queue semantics — pick the right (delete_on_no_subscribers, single_connection) pair:

| Intent | Flags | Old enum equivalent |
|---|---|---|
| Queue deleted when no subscribers — for events where only freshness matters (presence, notifications) | `(true, false)` | `DeleteOnDisconnect` |
| Queue persistent, multi-connection | `(false, false)` | `Permanent` |
| Queue persistent, exclusive connection — for critical events that must not be lost on a reconnect | `(false, true)` | `PermanentWithSingleConnection` |

### Rules:
- `next_message.engage_telemetry().await` — ALWAYS before calling scripts/flows
- Subscriber delegates to `scripts/` (not `flows/`) — there is no HTTP/gRPC context here
- NEVER business logic directly in `handle_messages` — routing and telemetry only
- `while let Some(mut next_message)` — always `mut`, required for `take_message()`
- Filter by type (`sb_msg.tp`) in subscriber — different types → different script calls

### Error Handling in Service Bus

> **WHY:** An SB subscriber cannot return an error to a client — there is no client.
> IO errors (DB, gRPC) → panic, same as everywhere. But SB publish failures inside a handler
> → log and continue, because the data is already written to DB.

**IO errors (DB, gRPC calls) → panic** — same as everywhere:
```rust
// ✅ CORRECT
app.posts_repo.update(post).await.expect("posts: update failed");
```

**SB publish failure inside handler → log and continue**:
```rust
// ✅ CORRECT — data is already in DB, losing an SB message is non-critical
if let Err(err) = app.publisher.publish_messages(items.iter().map(|i| (i, None))).await {
    my_logger::LOGGER.write_error(
        "recalculate_likes",
        format!("{:?}", err),
        LogEventCtx::new().add("amount", items.len().to_string()),
    );
}
// continue — do NOT panic, do NOT return error

// ❌ WRONG
app.publisher.publish_messages(items.iter().map(|i| (i, None))).await.expect("publish failed");  // kills already-completed work
```

**Logging** — always with telemetry context from `engage_telemetry()`:
```rust
let ctx = next_message.engage_telemetry().await;
// pass ctx to all script calls — it routes logs through the telemetry pipeline
crate::scripts::recalculate(&self.app, sb_msg, &ctx).await;
```
