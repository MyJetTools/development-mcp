# ServiceContext — What Comes From Where

| What | Source |
|---|---|
| `MyServiceBusPublisher<T>` | `service_ctx.get_sb_publisher(true)` |
| `Arc<MyNoSqlDataReaderTcp<T>>` | `service_ctx.get_ns_reader()` |
| gRPC client | `XxxGrpcClient::new(settings_reader.clone())` |
| DB repo | `XxxRepo::new(settings_reader.clone()).await` |
| Timer (`MyTimer`) | `service_ctx.register_timer(duration, \|timer\| timer.register_timer("MyTick", Arc::new(MyTick::new(app.clone()))))` — the closure is `Fn`: build the tick inside it |
| Exact timer (`MyExactTimer`, wall-clock marks) | `service_ctx.register_exact_timer(ExactTimerInterval::Every1Minute, \|timer\| …)` |
| `Arc<EventsLoop<T>>` | `service_ctx.create_events_loop("name")` |
| `Arc<BackgroundExecutor>` / `Arc<BackgroundExecutorWithMultiThreads<Id>>` | `service_ctx.create_background_executor("name")` / `create_background_executor_with_multi_threads("name")` |
| `Arc<QueueToSave<T>>`, `…AsBulk<T>`, `…WithId<Id, T>`, `…OrDeleteWithId<Id, T>` | `service_ctx.create_queue_to_save("name")`, `create_queue_to_save_as_bulk`, `create_queue_to_save_with_id`, `create_queue_to_save_or_delete_with_id` |
| Any of the above with non-default settings, or a `Startable` of your own | `service_ctx.register_startable(component)` → `Arc<component>` |

**Timers, events loops, background executors and queues are created on the `ServiceContext` and started by `start_application()` — never by hand.** They are started after the MyNoSql readers are loaded and the app is marked initialized, in the order they were registered, before the Service Bus client and the HTTP and gRPC servers. Rules:

- Register the handler / tick on the component before `start_application()`: a component without one panics when it is started, and registering on a component which is already started panics too.
- Never call `start()` yourself — `start_application()` does, and a second `start()` panics. `trigger()` on a background executor before it is started panics.
- The rust-extensions README builds these components with `X::new(...)` and calls `.start()` — that is the library used on its own. In a service, take them from the `ServiceContext`. Started by hand, timers, background executors and queues work from `start()` on — before the MyNoSql readers are loaded; only `EventsLoop` waits for the application states to be initialized.

The full method table with examples is in `get_service_sdk_readme` («Background timers», «Queues, events loops and background executors»), together with how HTTP actions and gRPC servers are registered («HTTP Server», «GRPC Server»).

**NEVER** store `ServiceContext` in `AppContext`.
`service_ctx` lives in `main.rs` until `start_application()` is called — used to register SB subscribers after AppContext is created.
