# API calls — the core builds and reads, Dart sends

The twin of the Dioxus client's `src/api/`. There a call builds a request from the shared
`MyHttpInput` model, sends it with FlUrl and decodes the answer with the three centralised
helpers. Here the halves are the core's (topic `rust-core`, section 3) and the sending is Dart's.

## lib/native/http.dart — the ONE place an HTTP request is sent

```dart
// lib/native/http.dart
final class Http {
  Http({required this.origin, required this.userAgent, required this.host})
      : _client = platformClient(userAgent);

  final String origin;    // the flavor's: https://app.example.com
  final String userAgent; // the flavor's constant
  final CoreHost host;
  final http.Client _client;

  /// What the core built, sent; replayed as its retry policy says — idempotent methods only,
  /// on a 5xx or no answer. No answer at all is a result too (`status: null`), never an
  /// exception upwards
  Future<HttpResult> send(HttpRequestSpec request) async { /* … */ }

  /// A screen's call: the core reads the answer — through host.run, because a 401 leaves
  /// NeedsConfirm behind
  Future<T> call<T>(HttpRequestSpec request, T Function(HttpResult result) read) async {
    final result = await send(request);
    return host.run(() => read(result));
  }

  /// A request the core asked for itself (`CoreEffect.http`): the answer goes back to it
  Future<void> sendForCore(int id, HttpRequestSpec request) async {
    final result = await send(request);
    host.run(() => core_api.httpDone(id: id, result: result));
  }
}

/// One interface, one platform class each
http.Client platformClient(String userAgent) {
  if (Platform.isIOS) {
    final config = URLSessionConfiguration.ephemeralSessionConfiguration()
      ..httpAdditionalHeaders = {'User-Agent': userAgent};
    return CupertinoClient.fromSessionConfiguration(config);
  }
  return IOClient(HttpClient()..userAgent = userAgent);
}
```

- **The origin is put in front of the core's rooted path here, and nowhere else.** On the web
  FlUrl resolves `/api/...` against the page; a phone app has no page, so the origin is the
  flavor's (topic `ci`).
- **The User-Agent is a constant, set here and on the socket.** A server that ties a session to
  its User-Agent byte for byte (the MT terminal's does) answers a changed one with a 401 — so the
  User-Agent carries no app version, OS version or device model, which would change with an
  update and send every user to re-confirmation the day it ships. `ProductName/1 (iOS)`; bumping
  the `/1` is a deliberate re-confirmation of everyone.

## lib/api/ — a call is three lines

```dart
// lib/api/trading.dart — the twin of src/api/trading.rs
Future<TradingResponse> setOrder(Http http, SetOrderParams params) => http.call(
      trading_bridge.setOrderRequest(params: params),
      (result) => trading_bridge.setOrderResponse(result: result),
    );
```

- **A module is always imported with its prefix** — `trading_api.setOrder(http, ..)`, the twin of
  `crate::api::trading::set_order(..)` (`get_flutter_design_patterns` §12).
- **Never a request around `lib/api/`.** A request sent straight through `package:http` has no
  core reader: no 401 handling, no typed error, a status code checked in a screen.

## Status handling — once, in the core's readers

The twin of `handle_http_response` / `handle_http_empty` / `handle_http_response_opt`: every reader
maps the answer the same way —

| Answer | Reader returns |
|---|---|
| 2xx | the shared response model |
| 401 | `RequestError::Unauthorized` — and `NeedsConfirm` left for the host |
| any other status | `RequestError::Refused { message }` — the server's reason |
| no answer | `RequestError::Transport { message }` — the core's words, in the user's language |

and one Dart extension decides what a screen shows for a failed call:

```dart
// lib/models/request_error_text.dart
extension RequestErrorText on RequestError {
  /// What a screen shows — `null` = nothing: the router is already on its way to confirmation
  String? get text => switch (this) {
        RequestError_Unauthorized() || RequestError_NoSession() => null,
        RequestError_Refused(:final message) || RequestError_Transport(:final message) => message,
      };
}
```

No screen checks a status code, and no screen reacts to a 401 — one confirmation screen asks,
not a dozen requests each reacting to their own 401.
