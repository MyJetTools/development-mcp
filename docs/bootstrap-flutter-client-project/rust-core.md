# The shared core — logic without I/O

The core (`client-common` here) is the one place the web client and the phone app agree. It holds
everything the user can see as a number, a word, a URL or a stored key, and every rule about what
the server's messages do. **It does no I/O**: it never sends a request, never holds a socket, never
reads or writes storage, never starts a timer. An input goes in, the state changes, effects come
out — and the host (the Dioxus web host in the browser, Dart on the phone) carries the effects out
and feeds the results back in as the next input.

## 1. What goes into the core — and what does not

| In the core | Not in the core — the surface's |
|---|---|
| the words (`localization`), the URL vocabulary (`routes`) | sending a request, the socket, storage, timers, the lifecycle |
| the session: the token in memory, what a 401 means | the token's secure storage |
| the feed: what each server message does to the state, what to subscribe to, what a reconnect clears | which dialog is open, focus, scroll, animations |
| every request — built from the shared input model — and every answer — read into the shared response model | layout, tokens, gestures, painters, haptics |
| the stored records' keys and shapes, write-through, a KV store's dirty-write tracking | resolving "system" into a theme |
| per-screen rules: an order ticket, a TP/SL editor, a chart engine | |
| formatting, trading arithmetic, an order book's price grid, the chart model | |

**The test of where a line goes:** if the web and the phone could disagree on it, and the user
would see the difference as a different number, word, URL, stored value or server traffic, it is
the core's.

## 2. An input in, the state and the effects out

```rust
// client-common/src/core/reduce.rs
pub enum CoreInput {
    /// The host's socket opened. `first` = the first connection of this session
    Connected { first: bool },
    Disconnected,
    /// A frame as it arrived — text as text, binary (protobuf) as bytes
    Frame(WsContract),
    /// The answer to a request the core asked for with `Http { id, .. }`
    HttpDone { id: i64, result: HttpResult },
    /// The app went to the background / came back
    Foreground(bool),
    // … the user's actions: SelectInstrument(String), SetTab(Tab), …
}

#[derive(Debug, PartialEq)]
pub enum CoreEffect {
    // ── I/O, carried out by the host ──
    /// Send this request; answer with `HttpDone { id, .. }`
    Http { id: i64, request: HttpRequestSpec },
    /// Open the socket on this rooted path (`/ws?token=…`); the host puts its origin in front
    /// and adds its User-Agent
    WsOpen { path_and_query: String },
    /// A text frame for the socket — subscriptions go out this way
    WsSend(String),
    /// The server's "initialized" frame arrived: the socket's start-up timeout becomes its
    /// read timeout — what `conn.mark_initialized()` is on the web
    WsInitialized,
    WsClose,
    /// Keep this record / drop it (the browser's localStorage, the phone's preferences)
    Store { key: String, value: String },
    Forget { key: String },
    /// The secure store must now hold this token (`None` = delete it)
    PersistSessionToken(Option<String>),

    // ── what the UI does ──
    /// The state moved; `slices` says which parts
    Changed { slices: u32 },
    /// A 401: the session needs confirming — one screen asks, nothing else reacts
    NeedsConfirm,
    LanguageChanged(LangId),
    SettingsChanged,
}

impl Core {
    pub fn apply(&mut self, input: CoreInput) -> Vec<CoreEffect> { /* … */ }
}
```

What this buys:

- **A rule cannot be half-applied.** An action that changes what the server must send *returns*
  the frames; a surface cannot change the selection without being handed the subscription that
  goes with it. (In the MT terminal's Dioxus client a `build.rs` scan enforced "every such
  mutation goes through the wrapper that re-subscribes" — the core makes it a type.)
- **Every rule is a test of inputs in and effects out** — no server, no socket, no browser:

```rust
#[test]
fn a_reconnect_drops_positions_but_keeps_quotes() { /* … */ }

#[test]
fn a_delta_without_a_snapshot_is_dropped() { /* … */ }

#[test]
fn a_kv_write_whose_answer_failed_is_sent_again_with_the_next_snapshot() { /* … */ }
```

- **Both hosts are thin.** The Dioxus host turns `WsCallback` events into inputs and carries the
  effects out with Rust (`WsSend` → the socket, `Http` → FlUrl, `NeedsConfirm` →
  `navigator().push`). The phone's Dart does the same with its own means (topic `io`).

## 3. Requests — built and read in the core, sent by the host

Every call of the server is split in two, and neither half sends anything:

```rust
// client-common/src/api/trading.rs
/// What to send — built from the shared input model (the one the server parses) through
/// my-http-utils' THttpRequestBuilder: fill_url / fill_headers / get_body
pub fn set_order_request(session: &Session, params: SetOrderParams) -> HttpRequestSpec { /* … */ }

/// What the answer means: a 2xx → the model, a 401 → RequestError::Unauthorized (and the session
/// marked for confirmation), anything else → RequestError::Refused with the reason
pub fn set_order_response(session: &mut Session, result: HttpResult)
    -> Result<TradingResponse, RequestError> { /* … */ }
```

```rust
// client-common/src/http.rs — transport-agnostic, the same on both targets
pub struct HttpRequestSpec {
    pub method: HttpMethod,
    /// A rooted path with its query — the host puts its origin in front
    pub path_and_query: String,
    /// Authorization (the core's session) included; the User-Agent is the host's
    pub headers: Vec<HttpHeader>,
    pub body: Vec<u8>,
    /// Replay on a 5xx or no answer, `delay_ms` apart — idempotent methods only, the rule of
    /// FlUrl's `with_retry`. `None` = exactly once
    pub retry: Option<RetryPolicy>,
}

pub struct HttpResult {
    /// `None` — no answer at all (timeout, no network)
    pub status: Option<u16>,
    pub body: Vec<u8>,
}
```

- **The shared models stay the contract.** my-http-utils turns a `MyHttpInput` model into URL,
  headers and body with no transport (`get_my_http_utils_readme`: "a thin adapter turns
  `UrlBuilder` + headers + `HttpRequestBody` into a real request"). The core is the model's user;
  the adapter is the host's.
- **Status handling is centralised in the readers** — the twin of `handle_http_response` & co in
  the Dioxus client: no host checks a status code, no screen reacts to a 401.
- **An idempotency key is minted by the builder**, so a retry carries the same key.
- **Two shapes of call:**
  - *The core's own requests* — a KV write, sign-in steps, a settings save: the core returns
    `Http { id, request }` and keeps what the id was for; the host answers with
    `HttpDone { id, result }`. That is how a "confirm the dirty mark when the server answered"
    rule lives without a `spawn_local` in the core.
  - *A screen's read* — a history list, closed positions, candles: the screen's action takes the
    request from the builder, has the host send it, hands the answer to the reader.
- **The web host composes the same halves with FlUrl** — a `crate::api::trading::set_order` of
  three lines: builder → `FlUrl` → reader.

## 4. The socket — held by the host, understood by the core

- **The host owns the connection**: it opens on `WsOpen`, sends on `WsSend`, closes on `WsClose`,
  and reports `Connected` / `Disconnected` / every frame back.
- **The core owns its meaning**: the path and query (the token, the options), parsing (the shared
  WS contracts crate: `try_from_str` for text, `from_proto_payload` for binary), what a frame
  changes, what to send, what a reconnect clears.
- **The transport's timing is one set of numbers**, in the core: `my-web-sockets-wasm`'s contract
  (13 s to "initialized", then 10 s of silence at most, 3 s before a reconnect, no wait once after
  a connection that lived 20 s). Both hosts' sockets read them from there.
- **The "initialized" frame is an effect** (`WsInitialized`), not a convention — forgetting it is
  the web's famous footgun: a socket that reconnects every 13 seconds forever.
- **Without a session there is no `WsOpen`** — the twin of `get_url()` answering `None`.

## 5. Storage — held by the host, decided by the core

- **At start the host hands the core every record it keeps** — the keys are the core's
  (`stored_keys()`), the values whatever the host's store had: `localStorage` on the web, the
  platform's preferences on the phone. Same keys on both.
- **A change goes out as `Store` / `Forget`** — the same write-through as on the web: the state
  method that changes a stored value produces the effect in the same call, after comparing.
- **`sessionStorage` is the core's memory** on the phone — a visit is the life of the process.
- **The session token is never a `Store`** — it goes out as `PersistSessionToken` into the secure
  store.

## 6. Threads — none

The core runs no thread, no runtime and no timer. **Every call into it is synchronous**, and on the
phone every call comes from the Dart main isolate — one thread, as a browser has one.

- **That keeps a browser-shaped core sound.** A core written for wasm keeps state in
  `thread_local!` and `Rc<RefCell<…>>`. FRB would run an `async` function on its tokio pool and a
  plain one on its thread pool, where a `thread_local!` is a different, empty map; a
  `#[frb(sync)]` function runs on the caller's thread — the main isolate's — every time.
- **Never call the core from another isolate.** A second isolate is a second thread.
- **The work is the browser's work** — parsing a frame, applying a delta, a chart frame plan —
  done on the thread a browser does it on. Measured on the slowest supported Android in a profile
  build, not assumed.
- **Target**: the core's state as fields of one `Core` value the host owns, instead of
  `thread_local!`s.

## 7. Features and Cargo

```toml
# client-common/Cargo.toml
[features]
# no feature: the pure core — what the phone gets. No FlUrl, no tokio, no TLS
default = []
# the web host: FlUrl over fetch, my-web-sockets-wasm, storage through dioxus-utils
web = ["dep:flurl", "dep:dioxus-utils", "dep:web-sys", "dep:js-sys", "dep:wasm-bindgen-futures"]
# the chart's model — every target
chart = []
# the chart's painters and the Dioxus component — the browser only
chart-web = ["chart", "web", "dep:dioxus", "dep:wasm-bindgen"]
```

- The Dioxus client takes `features = ["chart-web"]`; the bridge crate takes `["chart"]` and
  nothing else.
- **The phone's Rust has no network stack** — no FlUrl, no tokio, no rustls. TLS is the phone's
  (topic `io`).
- `web_sys`, `js_sys`, `dioxus_utils` and FlUrl are optional dependencies switched on by `web`
  alone: in the phone's build they do not exist, and a stray call is a compile error.
  `cargo build --features chart --target aarch64-linux-android` is that guard.
- Wire-model crates (`rest-api-shared`: my-http-utils + serde; WS contracts: serde + prost) carry
  no I/O and compile for the phone as they are.

## 8. Tests

- **Everything in the core is tested natively** — `cargo test`. One suite protects every surface.
- **A test per rule of the reducer** — the rules a Dioxus client keeps in comments of its WS
  handler are exactly the ones to pin when they move.
- **Every request builder against its server model**, every reader against a recorded answer, a
  401 and a dead network.

## 9. Moving an existing Dioxus client's logic in

A Dioxus client that predates this split keeps logic in its own crate (feed state, the WS handler,
subscriptions, form rules, copies of API calls) and I/O inside the shared crate (FlUrl calls,
`localStorage`, `spawn_local`). The order that keeps the web client working at every step — `dx
build` after each:

1. **Split `api::*` into builders and readers**, FlUrl only in the `web` composition.
2. **Move the pure pieces** — formatting, API calls duplicated between clients, per-screen rule
   structs.
3. **Move the feed** — the market state, the domain half of `AppState`, the WS handler as
   `Core::apply` + effects, the subscribe frames, the stored selection. The client's `ws/` shrinks
   to a host.
4. **Take I/O out of the stateful modules** — a KV store's push becomes an `Http` effect and its
   confirmation an `HttpDone`; storage modules take records in and give `Store` / `Forget` out.
5. **Split the chart** — the model (`chart`) from the painters and the component (`chart-web`);
   no framework type (a Dioxus `Signal`, a `wasm_bindgen` closure) left in the model.
6. **The bridge crate**, the cross-builds for iOS and Android, then the first Flutter screens.

In the MT terminal this is `mt-client-common` taking over `mt-client-mobile`'s `states/`, `ws/`,
`api/trading.rs`, `models/fmt.rs` and the trade screen's `TradeState`.
