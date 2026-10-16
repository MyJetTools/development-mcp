---
alwaysApply: false
---
# Bootstrap a Flutter Client (iOS / Android) next to a Dioxus Client

This document describes how to bootstrap a **Flutter app for iOS and Android** that is built in
parallel with a Dioxus client-side web app (`get_dioxus_client_side_bootstrap_guide`) and shares
its logic. Flutter here is for phones only — no Flutter web, no Flutter desktop: the browser stays
Dioxus.

For component patterns (state, `DataState`, dialogs, the canvas, the theme) see
**flutter-design-patterns** (`get_flutter_design_patterns`) — the twin of
`get_dioxus_design_patterns`, with the same section numbers.

The guide follows the Dioxus client-side bootstrap section for section, and the Dioxus client
file for file: a Rust developer finds every Dart file where the Rust one would be, and porting a
component is a move, not a redesign.

## The architecture

- **The logic is Rust — one implementation.** Everything the two surfaces must agree on — the
  words, the URLs, the session, what a server message does to the state, what is stored where,
  every number the user acts on — lives in a shared crate with no UI framework (here
  `client-common`). Dioxus compiles it to wasm; the phone app embeds the same Rust through
  flutter_rust_bridge (FRB). Flutter gets the Rust, never a Dart copy of it.
- **The core does no I/O.** An input goes in — a tap, a server frame, an HTTP answer — the state
  changes, and effects come out: send this request, send this frame, store this record, go to
  this page. The core builds every request and reads every answer; it never sends, waits or
  stores (topic `rust-core`).
- **On the phone all I/O is Dart's** — HTTP, the WebSocket, storage, timers, the lifecycle — on
  the platform's own stacks: `URLSession` on iOS, `dart:io` on Android. TLS, the trusted
  certificates, proxies and VPNs are the phone's (topic `io`).

```
                         client-common   (Rust — logic only, no I/O)
      words · URLs · session · what a frame does to the state · what to store
      requests built and answers read · rules · formatting · the chart model
                 │                                            │
        + the web host (feature `web`)                nothing more — no network stack
        FlUrl over fetch, localStorage,                           │
        my-web-sockets-wasm                                       │
                 │                                            │
     the Dioxus client (wasm)                        the bridge crate (FRB) — synchronous calls,
     render.rs · state.rs · actions.rs · mod.rs      effects back
     CSS tokens                                      the Flutter app (Dart)
                                                     render.dart · state.dart · actions.dart · mod.dart
                                                     HTTP, WebSocket, storage on the platform's stacks
                                                     tokens generated from the same CSS
```

## When to use

| Use case | Project |
|---|---|
| A web client (terminal, cabinet, admin) | Dioxus — `get_dioxus_client_side_bootstrap_guide` |
| A phone app with the same product logic as a Dioxus client | **this guide** |
| A phone app with no Dioxus twin yet | this guide too — the core crate is then the app's own, ready for a web twin |

## How a feature is built, in parallel

1. **Dioxus first.** The feature is built and tried in the Dioxus client. It is the reference:
   what the screen shows, what each control does, what is stored where.
2. **What it decides goes into the core** — every number the user acts on, every word, every
   URL, every stored key, every rule about the feed — with tests (`cargo test`). The Dioxus code
   calls it from there. A rule left in the Dioxus client is a rule Flutter has to retype, and a
   retyped rule drifts.
3. **The bridge grows by what the screen needs** — an input, a query, a request / response pair,
   an opaque per-screen struct (topic `bridge`); the bindings are regenerated, never edited.
4. **The Flutter screen is a port of the UI only**, file by file: `render.rs` → `render.dart`,
   `state.rs` → `state.dart`, `actions.rs` → `actions.dart`, `mod.rs` → `mod.dart`. If the port
   needs an `if` that decides something about money, words or the feed, step 2 was not
   finished — go back, not forward.
5. **Both sides pass their guards** before the commit: `dx build` for Dioxus; `cargo test` /
   `clippy` for the core and the bridge; `flutter analyze` and `flutter test` for Dart (topic
   `ci`).

A feature may land in Dioxus days before its Flutter twin. What may not happen is the twin
landing with its own logic.

## What goes where

| Kind | Examples | Lives in | Dioxus holds it as | Flutter holds it as |
|---|---|---|---|---|
| App-wide domain state | the feed (quotes, positions, orders), the selection, the subscriptions, the session, KV, settings | the core | a `Signal` its web host writes | nothing of its own: `AppState` reads the core when `Changed` says it moved |
| Requests and answers | what a call sends, what its answer means, a 401 | the core — a builder and a reader per call | composed with FlUrl in `src/api/` | composed with the platform HTTP client in `lib/api/` |
| Per-screen state with rules | an order ticket, a TP/SL editor, a chart engine | a struct in the core | `use_signal(Ticket::default)` | an opaque bridge object inside the screen's state class |
| Pure rules | price formatting, `%` → price, the timeframe list, the words, the URLs | functions in the core | a direct call | a synchronous bridge call |
| I/O | sending, the socket, storage, timers, the lifecycle | the surface | Rust — FlUrl, `my-web-sockets-wasm`, `dioxus-utils` storage | Dart — `package:http`, `package:web_socket`, `shared_preferences`, Keychain / Keystore |
| View state | which dialog is open, a pressed button, scroll, focus, an animation | the surface | `ComponentState` / `AppState` view fields | a state class (`ChangeNotifier`) / hooks |
| Look and feel | layout, tokens, gestures, sheets, painters, haptics | the surface | CSS + `rsx!` | widgets + generated tokens |

The last two rows may differ between the web and the phone — a native sheet, the back button,
haptics. The others may not: a number, a word, a colour token, a URL, a stored key or the shape of
a stored settings struct that differs between two surfaces of one product is a bug, whichever side
is "right".

## Project Structure

Every folder has the same name and the same job as in the Dioxus client. The two that cannot keep
their name say why.

```
project-root/                          ← the Dioxus client's project-root/
├── CLAUDE.md                          ← (new) the agent's entry: the docs to read first, the hard rules — topic `project-files`
├── .mcp.json                          ← (new) the documentation MCP server; its URL from the environment
├── pubspec.yaml                       ← Cargo.toml + Dioxus.toml (name, version, deps, assets, fonts)
├── analysis_options.yaml              ← #![deny(warnings)] + clippy — the zero-warnings policy
├── flutter_rust_bridge.yaml           ← (new) the bridge's codegen config — topic `bridge`
├── build.sh                           ← checks the Flutter version, builds the release — topic `ci`
├── flavors/                           ← (new) one JSON per brand: origin, User-Agent token, name
├── hook/
│   └── build.dart                     ← (new) the native-assets hook: builds rust/ inside `flutter build`
├── tool/                              ← build.rs — generators run before a build
│   └── gen_tokens.dart                ←   the Dioxus client's css tokens → lib/theme/tokens.g.dart
├── assets/                            ← public/
│   ├── fonts/                         ←   the web's faces, bundled
│   └── icons/
├── rust/                              ← (new) the bridge crate — the core's door
│   ├── Cargo.toml
│   ├── rust-toolchain.toml            ←   the pinned toolchain + the six targets
│   └── src/api/                       ←   what Dart can call — all of it synchronous
├── android/  ios/                     ← the shells: ids, icons, entitlements, deep links
├── test/                              ← build.rs's guards (routes, tokens, bindings) + state and widget tests
├── integration_test/                  ← on a device, with the real Rust library
└── lib/                               ← src/
    ├── main.dart                      ← main.rs: error hooks, RustLib.init, the core's start, the router
    ├── api/                           ← api/ — the calls: the core builds and reads them, native/http.dart sends them
    ├── bridge/                        ← (generated) the Dart side of rust/src/api — never edited
    ├── components/                    ← components/
    ├── dialogs/                       ← dialogs/ — dialog_state.dart, render.dart, a folder per dialog
    ├── icons/                         ← icons/
    ├── models/                        ← models/ — client view-state; extensions on bridge models
    ├── states/                        ← states/ — app_state.dart, session_state.dart
    ├── templates/                     ← templates/ — full_screen_form.dart, the app's shell
    ├── theme/                         ← css/ — tokens.g.dart (GENERATED), theme.dart
    ├── utils/                         ← dioxus-utils — DataState, DialogValue, NotifyChildComponent, hooks
    ├── views/                         ← views/ — one folder per page
    ├── native/                        ← web/ — the host: core_host.dart, http.dart, storage/, lifecycle.dart
    └── ws/                            ← ws/ — socket.dart, handler.dart (the core's effects carried out),
                                              sender.dart (the actions that change subscriptions)
```

- **`web/` → `native/`.** The folder is "what touches the host": the browser there, the phone's
  OS here — the HTTP client, the stores, the lifecycle, and `core_host.dart`, the one object that
  carries the core's effects out. A `lib/web/` in a Flutter app would read as Flutter web.
- **`css/` → `theme/`.** There are no stylesheets; there are tokens generated from the Dioxus
  client's CSS and the few recipes built on them (`get_flutter_design_patterns`, topic `theme`).
- **`api/` keeps its job — the calls.** In the Dioxus client a call in `src/api/` builds a request
  from the shared model and sends it with FlUrl. Here the core builds it and reads its answer, and
  `lib/api/` is the three lines between: builder → `native/http.dart` → reader. `lib/bridge/` is
  the codegen's.
- **`ws/` holds the socket**: `socket.dart` is the connection (the web's `my-web-sockets-wasm`),
  `handler.dart` what is done with what the core says, `sender.dart` the actions that change
  subscriptions.
- **`utils/` is the twin of `dioxus-utils`**, under the same names (`get_flutter_design_patterns`).
  It moves into a package of its own when a second Flutter app needs it.

### A component is a folder with the same four files

```
views/terminal/trade/          ← src/views/terminal/trade/ of the Dioxus client
├── mod.dart                   ← mod.rs — exports only, no code
├── render.dart                ← render.rs — the widget: draws, wires handlers, nothing else
├── state.dart                 ← state.rs — the component's state class and its methods
└── actions.dart               ← actions.rs — functions: reads of AppState, async actions
```

```dart
// views/terminal/trade/mod.dart — the `mod x; pub use x::*;` rule
export 'actions.dart';
export 'render.dart';
export 'state.dart';
```

The same file names in every folder is unusual for Dart and deliberate: a port of
`views/terminal/trade/render.rs` is `views/terminal/trade/render.dart`, with no naming decision in
between. A simple component is `render.dart` alone, as in Rust.

## Build and Dev Commands

```bash
# after any change in rust/src/api
flutter_rust_bridge_codegen generate

# after any change in the Dioxus client's css tokens
dart run tool/gen_tokens.dart

# development — a device or a simulator; the native-assets hook builds rust/.
# A change in rust/ needs a full re-run: hot reload and hot restart keep the old library
flutter run

# the checks — all of them, before every commit (the full list: topic `ci`)
flutter analyze --fatal-infos --fatal-warnings
dart format --output=none --set-exit-if-changed .
flutter test
(cd rust && cargo test && cargo clippy -- -D warnings)
```

**NEVER** edit `lib/bridge/` or `lib/theme/tokens.g.dart` — both are regenerated and a hand edit is
lost; the guards fail while either is stale.

## Critical Reminders

1. **No logic Dart could get from the core** — and none the core should have: a number, a word, a
   URL, a stored key or a feed rule written in Dart is a second implementation.
2. **All I/O is Dart's, and none of it decides anything.** What to send, what an answer or a frame
   means, what to store — the core. Sending, holding the socket, storing — Dart.
3. **Every call that can change the core goes through `host.run(..)`**, which takes the core's
   effects right after it (topic `io`).
4. **The core is called from the main isolate only** — another isolate is another thread, and the
   core's state is single-threaded like a browser's (topic `rust-core`).
5. **`lib/bridge/` is generated** — change `rust/src/api/`, regenerate, never edit.
6. **Error hooks are the first lines of `main()`**, before `RustLib.init()` — the twin of
   `dioxus_utils::set_panic_hook()`.
7. **The core is started before `runApp`**, from the records the phone keeps, so the first frame
   is drawn right — the twin of a state created from storage in `new()`.
8. **The origin and the User-Agent are the flavor's**, used only by `native/http.dart` and
   `ws/socket.dart`; the core builds rooted paths, no call site builds a URL or sets a header.
9. **The session token is in Keychain / Keystore**, and `lib/native/storage/session.dart` is the
   one file that touches it.
10. **Pre-auth vs post-auth are routes**; without a session the core asks for no socket — the twin
    of `get_url()` answering `None`.
11. **Wire models are Rust** and reach Dart as generated bridge classes; `lib/models/` holds
    client view-state and Dart `extension`s on those classes, never a hand-written copy of a wire
    model.
12. **Status codes are read in one place — the core's readers.** No screen checks one, and a 401
    is never a screen's business.
13. **Hooks are called unconditionally, in the same order, every build** — the rule of Dioxus
    hooks, word for word.
14. **Every `switch` over an `enum` or a `sealed` class is exhaustive with no `default:` and no
    `_`.**
15. **Tokens are generated** from the Dioxus client's CSS; **fonts are bundled**. A colour literal
    in a widget is a bug.
16. **Zero warnings** — `flutter analyze --fatal-infos --fatal-warnings` and
    `cargo clippy -- -D warnings`.

## Topics

Every part of the skeleton is a topic. Bootstrapping a new project means going through all of
them in the order of the table; a change to one part needs only its topic. Read a topic:
`resource://flutter-client-bootstrap/{topic}`, or `get_flutter_client_bootstrap_guide` with
`topic`.

| Topic | What is inside |
| --- | --- |
| [`project-files`](project-files.md) | `pubspec.yaml`, `analysis_options.yaml`, `flutter_rust_bridge.yaml`, `rust-toolchain.toml` and the pinned versions of everything; `CLAUDE.md` and `.mcp.json` — how an agent finds these docs |
| [`rust-core`](rust-core.md) | The shared core without I/O: what goes in, inputs and effects, requests built and read in the core, the socket and storage held by the host, one thread, features, tests, moving an existing Dioxus client's logic in |
| [`bridge`](bridge.md) | The FRB bridge crate: layout, everything synchronous, types, opaque per-screen structs, effects taken after every call, errors and panics, start-up, hot restart, codegen |
| [`io`](io.md) | The I/O stack — HTTP, WebSocket, storage, timers, one platform class each — the core's host `core_host.dart` and the effect dispatcher |
| [`routing`](routing.md) | `main.dart`: error hooks, `RustLib.init`, the core's start from the stored records, the router from the core's URL vocabulary and its guard test, the start-up gate, providers |
| [`websocket`](websocket.md) | The socket in Dart with `my-web-sockets-wasm`'s contract; frames to the core as they are; `WsInitialized`; reconnects; `sender.dart`; reading the feed in widgets |
| [`api-calls`](api-calls.md) | `lib/api/`: the core's request builder, `native/http.dart`, the core's reader; status handling in the core; a constant User-Agent; what a screen shows for a failed call |
| [`shared-wire-models`](shared-wire-models.md) | Wire models stay Rust: generated bridge classes, Dart extensions, no Dart copy of a wire model |
| [`local-storage`](local-storage.md) | Records in `shared_preferences` under the web's keys, the token in Keychain / Keystore, two absences, the Keychain after a reinstall, logout |
| [`templates-and-dialogs`](templates-and-dialogs.md) | `full_screen_form`, the app's shell, the `DialogState` scaffold |
| [`lifecycle`](lifecycle.md) | Background and foreground: the socket closed in the background and reopened with a full resync |
| [`deep-links`](deep-links.md) | Universal Links / App Links for the route vocabulary; the one-time-login link |
| [`ci`](ci.md) | The pinned toolchain, the guards before every commit, versions and tags, flavors, signing and release |
