# Flow Pattern

## Streaming Input: Collect → Process → Publish

> **WHY:** A gRPC stream is a batch of messages from one client in a single call.
> Collecting everything into a Vec → one bulk insert is far more efficient than N individual DB calls.
> Unlike streaming output, **no `tokio::spawn` needed** here — the handler just awaits the flow,
> and the gRPC framework keeps the connection open until the flow returns.

```rust
pub async fn like_unlike(
    app: &Arc<AppContext>,
    mut request: StreamedRequestReader<LikeUnlikeGrpcRequest>,
) {
    // Phase 1: collect — fix timestamp once for entire batch
    let now = DateTimeAsMicroseconds::now();  // ← ONE call before the loop — fixes timestamp for entire batch
    let mut to_insert = Vec::new();
    let mut to_delete = Vec::new();
    let mut to_publish = Vec::new();

    while let Some(item) = request.get_next().await {
        let item = item.unwrap();  // ← stream item: unwrap, not ? (stream errors are fatal)
        // distribute to vecs
    }

    // Phase 2: bulk DB — ALWAYS check len() > 0
    if !to_insert.is_empty() {
        app.likes_repo.bulk_insert(to_insert.as_slice()).await;
    }
    if !to_delete.is_empty() {
        app.likes_repo.bulk_delete(to_delete.as_slice()).await;
    }

    // Phase 3: publish (NOT a panic — log and continue)
    if !to_publish.is_empty() {
        if let Err(err) = app.publisher.publish_messages(to_publish.iter().map(|i| (i, None))).await {
            my_logger::LOGGER.write_error(
                "like_unlike",
                format!("{:?}", err),
                LogEventCtx::new().add("amount", to_publish.len().to_string()),
            );
        }
    }
}
```

**NEVER** call `DateTimeAsMicroseconds::now()` inside the loop — fix time once per batch.
**NEVER** return `Result` from a flow.
**NEVER** flow calls another flow.
