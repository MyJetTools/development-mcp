# The WebSocket — Dart's socket, the core's meaning

The twin of the Dioxus client's `ws/` (`my-web-sockets-wasm` + its `WsCallback`). Three files, the
same three jobs: `socket.dart` holds the connection, `handler.dart` carries the core's effects out
(topic `io`), `sender.dart` holds the user's actions that change what the server must send.

## socket.dart — the transport only

```dart
// lib/ws/socket.dart — what to send and what a frame means are the core's
final class TerminalSocket {
  TerminalSocket({required this.wsOrigin, required this.userAgent, required this.host});

  final String wsOrigin;   // the flavor's: wss://app.example.com
  final String userAgent;  // the flavor's constant — the handshake may be session-checked too
  final CoreHost host;

  WebSocket? _socket;

  /// `WsOpen` — connect, after the reconnect delay the core's transport numbers say
  void open(String pathAndQuery) { /* connectSocket(..), then listen with _onEvent */ }

  /// What arrives goes to the core as it is — text as text, binary (protobuf) as bytes
  void _onEvent(WebSocketEvent event) {
    switch (event) {
      case TextDataReceived(:final text):
        host.run(() => core_api.wsText(text: text));
      case BinaryDataReceived(:final data):
        host.run(() => core_api.wsBinary(data: data));
      case CloseReceived():
        _socket = null;
        host.run(core_api.wsDisconnected);
    }
  }

  void sendText(String text) => _socket?.sendText(text);

  /// `WsInitialized` — from now on the read timeout rules, not the start-up one
  void markInitialized() { /* … */ }

  void close() { /* … */ }
}

/// One interface, one platform class each — the handshake carries the User-Agent
Future<WebSocket> connectSocket(Uri url, String userAgent) async {
  if (Platform.isIOS) {
    final config = URLSessionConfiguration.ephemeralSessionConfiguration()
      ..httpAdditionalHeaders = {'User-Agent': userAgent};
    return CupertinoWebSocket.connect(url, config: config);
  }
  final socket = await io.WebSocket.connect(url.toString(), headers: {'User-Agent': userAgent});
  return IOWebSocket.fromWebSocket(socket);
}
```

- **The socket keeps `my-web-sockets-wasm`'s contract** (`get_my_web_sockets_wasm_readme`): 13 s
  to "initialized", then 10 s of silence at most, 3 s before a reconnect, no wait once after a
  connection that lived 20 s — with the numbers read from the core, so the web and the phone
  cannot disagree on them.
- **The server must send a frame at least every 10 seconds** — the web's rule, unchanged: a server
  that can stay silent longer sends a keep-alive frame of its own.
- **A reconnect is the core's decision, the delay the socket's.** `CloseReceived` goes to the core
  as `Disconnected`; while it still wants a feed it answers with `WsOpen`, and the socket waits the
  delay before connecting.
- **"initialized" is the core's to recognise** (`WsInitialized`) — the socket never parses a frame.
- **Verify the handshake on a device**: the iOS socket takes its User-Agent from the session
  configuration; a server that checks it refuses a socket that arrives without it.

## sender.dart — the actions that change what the server must send

In a Dioxus client each of these mutated the state AND re-subscribed; here the core does both in
one input:

```dart
// lib/ws/sender.dart
void selectInstrument(CoreHost host, String id) => host.run(() => feed_api.selectInstrument(id: id));
void setTab(CoreHost host, Tab tab) => host.run(() => feed_api.setTab(tab: tab));
```

## Reading the feed in widgets

`AppState` holds no copy of the feed. The core's state is read where it is, and `Changed` says when
to read it again — at most once per frame, however often the feed moves:

```dart
// lib/states/app_state.dart
class AppState extends StateBase {
  AppState({required Words words}) : _words = words;

  Words _words;
  FeedVersions _versions = const FeedVersions();
  int _pending = 0;
  bool _frameAsked = false;

  Words get words => _words;
  FeedVersions get versions => _versions;

  /// The feed's state, read through the bridge — never copied into Dart
  QuoteView? quote(String instrumentId) => feed_api.quote(id: instrumentId);

  /// The core's `Changed`
  void feedChanged(int slices) {
    _pending |= slices;
    if (_frameAsked) return;
    _frameAsked = true;
    SchedulerBinding.instance.scheduleFrameCallback((_) {
      _frameAsked = false;
      _versions = _versions.bump(_pending);
      _pending = 0;
      notifyListeners();
    });
  }
}
```

A widget that shows a slice selects that slice, so a tick of the quotes does not rebuild the
account card:

```dart
// a row of a quotes list — rebuilt when ITS quote changed, not on every tick of the feed
final quote = context.select<AppState, QuoteView?>((app) => app.quote(instrumentId));
```

- **Until the first "initialized"** the shell shows "connecting", once, on a cold start; after it,
  a reconnect keeps the screen up and a status dot carries the connection's state.
- **The status dot is not decoration** — on a phone a price that stopped arriving looks exactly
  like a price that is not moving.
