# The I/O stack — HTTP, WebSocket, storage, timers, all in Dart

On the phone every request, the socket, every record and every timer is Dart's, on the platform's
own stacks. None of it decides anything: what to send, what an answer or a frame means, what to
store — the core (topic `rust-core`). Dart sends, holds, stores and reports back.

## 1. One interface each, one platform class each

| I/O | iOS | Android | Package |
|---|---|---|---|
| HTTP | `CupertinoClient` — `URLSession` | `IOClient` — `dart:io` | `package:http` 1.6 + `cupertino_http` 3.1 |
| WebSocket | `CupertinoWebSocket` — `URLSession` | `IOWebSocket` over `dart:io`'s `WebSocket` | `package:web_socket` 1.0 + `cupertino_http` |
| records (the browser's `localStorage`) | `NSUserDefaults` | DataStore | `shared_preferences` 2.5 — `SharedPreferencesAsync` |
| the session token | Keychain | Keystore-backed | `flutter_secure_storage` 11.2 |
| timers | `dart:async` `Timer` | | |
| the lifecycle | `useOnAppLifecycleStateChange` | | `flutter_hooks` |

- **TLS and the trusted certificates are the phone's.** `URLSession` is the OS's own stack, and
  `dart:io` takes the trusted roots from the platform's store on Android and iOS (Dart's
  `SecurityContext` docs) — the same certificates the phone's browser trusts, updated with the
  OS. Nothing in the app carries a certificate list of its own, and nothing in it needs a TLS
  provider decision.
- **Code above `lib/native/` and `lib/ws/` sees the interfaces** — `http.Client`, `web_socket`'s
  `WebSocket` — never a platform class.
- **Not now**: `cronet_http` (HTTP/2–3 on Android, at the price of Google Play Services or
  +14 MB embedded) — a measured need first; `ok_http` (an Android WebSocket on OkHttp) — it is
  experimental.

## 2. The core's host — `lib/native/core_host.dart`

One object holds everything an effect needs, and every call that can change the core goes through
its `run`, which takes the effects right after the call:

```dart
// lib/native/core_host.dart — the core's host on the phone
final class CoreHost {
  late final Http http;             // lib/native/http.dart — topic `api-calls`
  late final TerminalSocket socket; // lib/ws/socket.dart — topic `websocket`
  late final Prefs prefs;           // lib/native/storage/prefs.dart — topic `local-storage`
  late final SessionStore sessionStore;
  late final AppState app;
  late final SessionState session;
  late final GoRouter router;

  /// Every call that can change the core goes through here: the call, then its effects
  T run<T>(T Function() call) {
    final value = call();
    for (final effect in core_api.takeEffects()) {
      onCoreEffect(effect, this);
    }
    return value;
  }
}
```

- **A call that skips `run` leaves effects waiting** — a request nobody sends, a record nobody
  stores — until the next `run` takes them late.
- **A query needs no `run`**: it changes nothing.
- **Carrying an effect out never calls the core back synchronously.** What answers — an HTTP
  result, a frame, a closed socket — arrives later, through `run` again.
- `CoreHost` is provided above the router (`Provider.value`), so actions reach it with
  `context.read<CoreHost>()`.

## 3. The effects, carried out — `lib/ws/handler.dart`

The twin of the Dioxus client's `ws/handler.rs`, which used to apply frames AND do things; here it
only does things, and every variant is named:

```dart
// lib/ws/handler.dart
void onCoreEffect(CoreEffect effect, CoreHost host) {
  switch (effect) {
    case CoreEffect_Http(:final id, :final request):
      unawaited(host.http.sendForCore(id, request));      // answers with core_api.httpDone
    case CoreEffect_WsOpen(:final pathAndQuery):
      host.socket.open(pathAndQuery);
    case CoreEffect_WsSend(:final text):
      host.socket.sendText(text);
    case CoreEffect_WsInitialized():
      host.socket.markInitialized();
    case CoreEffect_WsClose():
      host.socket.close();
    case CoreEffect_Store(:final key, :final value):
      unawaited(host.prefs.setString(key, value));
    case CoreEffect_Forget(:final key):
      unawaited(host.prefs.remove(key));
    case CoreEffect_PersistSessionToken(:final token):
      unawaited(host.sessionStore.write(token));
      host.session.setHasToken(token != null);
    case CoreEffect_Changed(:final slices):
      host.app.feedChanged(slices);                       // one notification per frame
    case CoreEffect_NeedsConfirm():
      host.router.go(routes.confirm);
    case CoreEffect_LanguageChanged(:final lang):
      host.app.setWords(lang);
    case CoreEffect_SettingsChanged():
      host.app.settingsChanged();
  }
}
```

A new effect in Rust does not compile here until it is handled — the exhaustive-`switch` rule.

## 4. Timers

The core keeps no clock-driven work of its own; a delay is the host's:

- **A request's retry** — the policy is in the request the core built (`retry`); the HTTP client
  waits and replays (topic `api-calls`).
- **The socket's timeouts and reconnect delay** — the numbers are the core's, the timers the
  socket's (topic `websocket`).
- **Coalescing `Changed`** — one notification per frame,
  `SchedulerBinding.instance.scheduleFrameCallback` (topic `websocket`).

Reading the clock is not I/O: the core reads it itself (`SystemTime` natively, `js_sys::Date` on
the web).
