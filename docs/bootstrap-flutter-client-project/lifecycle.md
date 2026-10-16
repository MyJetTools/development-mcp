# Lifecycle — the background and the foreground

New on the phone. A browser tab in the background is throttled; a phone app in the background is
suspended, and its socket dies under it. So the core is told, and it answers with the socket's
effects.

```dart
// lib/native/lifecycle.dart — a hook, used once, by the app's shell
void useForegroundSync(CoreHost host) {
  useOnAppLifecycleStateChange((previous, current) {
    switch (current) {
      case AppLifecycleState.resumed:
        host.run(() => core_api.foreground(on: true));
      case AppLifecycleState.paused:
        host.run(() => core_api.foreground(on: false));
      case AppLifecycleState.inactive || AppLifecycleState.hidden || AppLifecycleState.detached:
        break;
    }
  });
}
```

- **Into the background: `WsClose`.** Nothing streams to a phone in a pocket, and a socket the OS
  kills silently is a socket that looks open.
- **Back in the foreground: `WsOpen` and a full resync** — the same path as any reconnect: what a
  reconnect clears is cleared (state that a fresh snapshot replaces — open positions, orders),
  what it keeps is kept (prices and catalogues that the first frames replace anyway), every
  subscription is replayed, a chart reloads the gap. Never "carry on" with what was on screen.
- **No background modes.** The app does not keep a socket alive behind the user's back.
- **The decision is the core's**: what a return to the foreground clears, re-reads and re-sends is
  a rule both surfaces share — the web's "the tab became visible" path and the phone's "resumed"
  are one input.
