# The bridge — flutter_rust_bridge between the core and Dart

The phone app reaches the shared core through [flutter_rust_bridge](https://cjycode.com/flutter_rust_bridge/)
(FRB): Rust functions in one crate, Dart bindings generated from them. The core does no I/O (topic
`rust-core`), so the bridge is small and **entirely synchronous** — Dart tells the core what
happened, asks it what is, and carries out what it says to do.

Facts below are FRB 2.13.0 (checked 2026-10-16 against its docs and source) on Flutter 3.47.7.

## 1. One version, four places

FRB's Dart package, runtime crate, codegen and macros must be **the same version** — the
generated code checks it at start-up. `=2.13.0` in `rust/Cargo.toml`, `2.13.0` (no caret) in
`pubspec.yaml` for both `flutter_rust_bridge` and `flutter_rust_bridge_hooks`, and:

```bash
cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked
```

An upgrade moves all of them in one commit.

## 2. Layout — a thin bridge crate inside the app

```
project-root/
├── flutter_rust_bridge.yaml
├── hook/build.dart             ← the native-assets build hook: builds rust/ inside `flutter build`
├── lib/bridge/                 ← GENERATED — the Dart side of rust/src/api, never edited
│   ├── frb_generated.dart
│   └── api/                    ←   one file per Rust module
└── rust/
    ├── Cargo.toml
    ├── rust-toolchain.toml
    └── src/
        ├── lib.rs              ← mod api; mod frb_generated;
        ├── frb_generated.rs    ← GENERATED
        └── api/                ← EVERYTHING Dart can call
            ├── core.rs         ←   start, take_effects, what the socket and the lifecycle report
            ├── session.rs      ←   sign-in and its steps: requests + readers
            ├── feed.rs         ←   the user's actions on the feed + its queries
            ├── trading.rs      ←   requests + readers
            ├── ticket.rs       ←   an opaque per-screen struct
            ├── chart.rs        ←   the chart engine — opaque
            ├── words.rs        ←   GENERATED from the core's localization (section 4)
            ├── rules.rs        ←   formatting, trading arithmetic — pure functions
            └── routes.rs       ←   the URL vocabulary
```

**A crate of its own, not FRB pointed at the shared crate**: the codegen writes
`frb_generated.rs` and `mod frb_generated;` into the crate it is pointed at, which would put FRB
inside the crate the wasm bundle compiles. The bridge crate depends on the shared crate by path
(`features = ["chart"]`, no `web`) and adds nothing to it. `Cargo.toml`, `rust-toolchain.toml` and
`flutter_rust_bridge.yaml`: topic `project-files`.

### Setting it up — `integrate`, with the native-assets backend

```bash
cd project-root
flutter_rust_bridge_codegen integrate \
  --rust-crate-name client_bridge \
  --integration-backend native-assets \
  --platforms android,ios \
  --no-write-lib
```

then point `dart_output` at `lib/bridge` and replace the template's `simple.rs` with the modules
above.

**Native assets, not Cargokit.** FRB's default is still Cargokit, but since Flutter 3.44 iOS
builds use Swift Package Manager, Cargokit's podspec falls back to CocoaPods with a "will become
an error" warning, and the CocoaPods registry becomes read-only on 2026-12-02. The native-assets
backend (`hook/build.dart` + `FlutterRustBridgeNativeAssetsBuilder`) is FRB's answer to that and,
in its own docs, "possibly the preferred way in the future". It needs Dart ≥ 3.10, a pinned
`rust-toolchain.toml` with the targets, and NDK r27+ (r28+ for Google Play's 16 KB page-size
check).

## 3. What may cross — everything is synchronous

`rust/src/api/` is the whole surface: everything `pub` there gets a Dart twin. A function in
`api/` forwards to the core and nothing more. **Logic in `api/` is logic the web client does not
have** — it belongs in the core.

| Kind | Rust | Dart |
|---|---|---|
| **Input** — something happened: a tap, a frame, an answer, the lifecycle | `#[frb(sync)] pub fn select_instrument(id: String)` | `selectInstrument(id: …)` — through `host.run` (topic `io`) |
| **Query** — what is | `#[frb(sync)] pub fn quote(id: String) -> Option<QuoteView>` | a plain call, in `build` too |
| **Request / reader** — one call of the server, split in two | `#[frb(sync)] pub fn set_order_request(params: SetOrderParams) -> HttpRequestSpec` · `#[frb(sync)] pub fn set_order_response(result: HttpResult) -> Result<TradingResponse, RequestError>` | composed in `lib/api/` with the HTTP client (topic `api-calls`) |
| **Rule** — a pure function | `#[frb(sync)] pub fn format_price(value: f64, accuracy: i32) -> String` | a plain call |

- **`#[frb(sync)]` on everything.** FRB runs a plain function on its thread pool and an `async`
  one on its tokio runtime — other threads, where a browser-shaped core's `thread_local!` state is
  a different, empty map. A sync function runs on the caller's thread; called from the Dart main
  isolate, that is always the same thread. No `async fn`, no `StreamSink`, no thread in the
  bridge.
- **Never from another isolate** — another isolate is another thread.
- **Small and quick.** A sync call runs on the platform's main thread (since Flutter 3.29 the UI
  and platform threads are one): it parses a frame, applies an answer, reads the state, and never
  waits on anything.
- **One call per screen section, never one per row** — a query returns what a list shows.
- Rust `snake_case` arrives as Dart `camelCase`; parameters become named.

## 4. Types

| Rust | Dart | Rule |
|---|---|---|
| a struct whose fields all translate | a class with `final` fields, a `const` constructor, `==` / `hashCode` | data that crosses as a value — `HttpRequestSpec`, `HttpResult`, views |
| a unit enum | a Dart `enum` | |
| an enum with data | a `@freezed sealed class` | `CoreEffect`, `RequestError`; the codegen runs `build_runner`. Switched exhaustively, no `_` |
| `Option<T>` / `Vec<T>` | `T?` / `List<T>` | |
| `Vec<u8>` / `Vec<f32>` / `Vec<f64>` | `Uint8List` / `Float32List` / `Float64List` | bodies, binary frames, a chart frame plan |
| `i64` | `int` | ids, versions, request ids, unix microseconds |
| `u64` / `usize` | **`BigInt`** | **never in the API** — an `i64` or an `i32` instead |
| `#[frb(opaque)]` struct | a handle with methods (section 5) | |

- **Wire models are not retyped in Dart.** A shared model crosses as a mirror —
  `#[frb(mirror(Type))]` on a `pub use`d type whose fields are all `pub` (enums with struct
  variants cannot be mirrored) — or as a bridge struct with a `From` impl (topic
  `shared-wire-models`).
- **A binary frame crosses as bytes** and is decoded by the core's prost — Dart never parses
  protobuf or the wire's JSON.
- **Moments are `i64` unix microseconds**, the wire's unit, printed by the core's formatter — no
  `DateTime` arithmetic in Dart: the offset of a moment is the one it had then, and only the core
  knows the user's clock.
- **The words are generated.** A localization struct of `&'static str` fields does not translate;
  `rust/src/api/words.rs` — a `Words` struct of `String` fields and its `From` — is generated from
  the core's localization, and a test fails when it is stale. A field added in Rust reaches Dart
  by regeneration, never by hand.

## 5. Per-screen structs — opaque

What a screen owns and Rust decides — an order ticket, a TP/SL editor, a chart engine — crosses as
an **opaque** object: a handle (`RustAutoOpaque`, an `Arc<RwLock<T>>`) with methods, freed by
`dispose()` or by Dart's GC.

```rust
// rust/src/api/ticket.rs
use flutter_rust_bridge::frb;

/// The order ticket's rules — the core's struct, owned by the screen
#[frb(opaque)]
pub struct Ticket(client_common::ticket::Ticket);

impl Ticket {
    /// Dart: Ticket()
    #[frb(sync)]
    pub fn new() -> Self {
        Self(client_common::ticket::Ticket::default())
    }

    /// Dart: ticket.quantity
    #[frb(sync, getter)]
    pub fn quantity(&self) -> String {
        self.0.quantity.clone()
    }

    /// Turning TP/SL off clears both fields — the rule is the core's
    #[frb(sync)]
    pub fn set_tp_sl(&mut self, on: bool) {
        self.0.set_tp_sl(on);
    }
}
```

```dart
// lib/views/terminal/trade/state.dart
class TradeState extends StateBase {
  final Ticket _ticket = Ticket();

  String get quantity => _ticket.quantity;

  void setTpSl(bool on) {
    _ticket.setTpSl(on: on);
    notifyListeners();
  }

  @override
  void dispose() {
    _ticket.dispose(); // freed with the screen — not whenever the GC gets to it
    super.dispose();
  }
}
```

- **A wrapper, not the shared struct itself** — the bridge chooses names; the shared crate stays
  free of FRB.
- **Owned and disposed by the Dart state class** that holds it.
- **Never app-wide state.** An opaque struct belongs to one screen; the feed, the session and KV
  are the core's, reached through inputs and queries.

## 6. Effects — taken after every call

The core cannot do anything itself, so what it wants done waits in its outbox until Dart takes
it. Every input and every reader is called through one function that takes the effects right
after the call (topic `io`):

```rust
// rust/src/api/core.rs
/// What the core wants done since the last time it was asked
#[flutter_rust_bridge::frb(sync)]
pub fn take_effects() -> Vec<CoreEffect> {
    crate::core::take_effects()
}
```

- **No stream, no callbacks into Dart.** Nothing in Rust runs unless Dart called it, so there is
  nothing for the core to push.
- **A query needs no `run`** — it changes nothing.

## 7. Errors and panics

- **A reader fails with a typed error**: `Result<T, RequestError>` in Rust, a thrown
  `RequestError` (an `FrbException`) in Dart, caught with `on RequestError catch (err)`. Not
  `anyhow` in the API — an `AnyhowException` is a string, and a screen cannot `switch` on a
  string.

```rust
pub enum RequestError {
    /// The core has already queued NeedsConfirm — a screen shows nothing for it
    Unauthorized,
    NoSession,
    Refused { message: String },
    /// No answer — the message is the core's, in the user's language
    Transport { message: String },
}
```

- **A panic crosses as a `PanicException` — only because the bridge crate unwinds**
  (`panic = "unwind"` in its release profile). The error hooks of `main()` report it.
- **`unwrap()` is not an answer in `api/`.** A function that cannot answer returns its error.

## 8. Start-up, hot reload, hot restart

```rust
// rust/src/api/core.rs — runs inside RustLib.init(), and again after a hot restart
#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Rust logs to logcat / os_log, backtraces on
    flutter_rust_bridge::setup_default_user_utils();
}

/// The records the platform keeps for the core, the token from the secure store.
/// Resets everything — see below
#[flutter_rust_bridge::frb(sync)]
pub fn start_core(args: StartArgs) {
    crate::core::start(args);
}
```

- **Neither hot reload nor hot restart reloads Rust.** After a change in `rust/` or a
  regeneration: stop, then `flutter run` again.
- **A hot restart keeps the Rust library and its state** while Dart starts from `main()`.
  `start_core` resets the core, so the new Dart side — its new socket, its new pending requests —
  starts from a clean state.

## 9. Codegen

- `flutter_rust_bridge_codegen generate` after any change in `rust/src/api/`. Its output —
  `rust/src/frb_generated.rs`, `lib/bridge/**` — is committed and never edited.
- **The bindings are checked**: `flutter_rust_bridge_codegen generate && git diff --exit-code --
  lib/bridge rust/src/frb_generated.rs` fails when someone changed `api/` and did not regenerate.
- `lib/bridge/**` and `**/*.freezed.dart` are excluded from analysis.

## 10. Memory — two managers in one process

- **Rust keeps its own heap**: the global allocator (the system's by default), memory freed the
  moment its owner is dropped — no garbage collector.
- **Dart keeps the VM's heap**, freed by its garbage collector — the ordinary Flutter scheme.
- **Values are copied at the bridge.** A `String`, a list or a struct that crosses arrives as a
  copy owned by the other side; no object is shared between the two heaps.
- **An opaque object is the exception**: its memory is Rust's, Dart holds a handle. It is freed by
  `dispose()` or when Dart's GC finalizes the handle — and the GC cannot see how much native
  memory sits behind a small handle, so it may come late. **The state class that owns an opaque
  object disposes it in its own `dispose()`** (section 5).
- **The core's state lives as long as the process** — and survives a hot restart, which is why
  `start_core` resets it.
- **A leak on the Rust side is an `Rc` / `Arc` cycle or a handle never disposed** — Rust has no
  collector to find it. A closure kept by the engine must not hold the engine's own handle.

## 11. Tests

- **The logic is tested in Rust** — `cargo test` in the core: inputs in, effects out.
- **Dart's host is tested against a mock of the bridge** — `RustLib.init(api: MockRustLibApi())`:
  given these effects, does the host send, store and navigate as it should.
- **On a device** — `integration_test/`, with `setUpAll(() async => RustLib.init())`.
