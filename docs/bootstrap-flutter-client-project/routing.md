# Routing — main.dart, the router, pre-auth and post-auth pages

The twin of the Dioxus client's `main.rs`: the panic hook first, then the start, then the router.

## main.dart

`main()` installs the error hooks, loads the Rust library, reads what the phone keeps, starts the
core with it, and only then runs the app — so the first frame is drawn from the core's state (the
twin of a state created from storage in `new()`).

```dart
// lib/main.dart — the twin of main.rs
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // First thing — the twin of dioxus_utils::set_panic_hook(): a failure at start-up otherwise
  // leaves nothing to read
  installErrorHooks();

  await RustLib.init();

  final prefs = Prefs(SharedPreferencesAsync());
  final sessionStore = SessionStore(FlutterSecureStorage(
    iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock_this_device),
  ));
  // The Keychain outlives an uninstall — topic `local-storage`
  await forgetTokenAfterReinstall(prefs, sessionStore);

  final stored = await prefs.load();
  final token = await sessionStore.read();

  final host = CoreHost.create(flavor: Flavor.current, prefs: prefs, sessionStore: sessionStore);

  // Resets the core (a hot restart keeps the Rust library) and hands it what the phone keeps;
  // with a session its first effect is the socket's WsOpen
  host.run(() => core_api.startCore(args: StartArgs(stored: stored, sessionToken: token)));

  runApp(App(host: host));
}

void installErrorHooks() {
  FlutterError.onError = FlutterError.presentError;
  PlatformDispatcher.instance.onError = (error, stack) {
    debugPrint('uncaught: $error\n$stack');
    return true;
  };
}
```

## The router — from the core's URL vocabulary

The web client and the phone app share one URL vocabulary — a link opens on whatever device is
in the reader's hand. It is the core's (`routes`), and the router is built from it, never from
literals of its own:

```dart
// lib/main.dart
final routes = routes_api.routes(); // every path and query name of the vocabulary

GoRouter buildRouter(SessionState session) => GoRouter(
      initialLocation: routes.terminal,
      // The session, NOT AppState: AppState moves with every tick of the feed, and the redirect
      // would run with it
      refreshListenable: session,
      redirect: (context, state) => redirectFor(session, state.matchedLocation),
      routes: [
        // ── pre-auth ──
        GoRoute(path: routes.login, builder: (_, _) => const RenderLogin()),
        GoRoute(
          path: routes.enterCode,
          builder: (_, state) => RenderEnterCode(email: state.uri.queryParameters[routes.queryEmail] ?? ''),
        ),
        GoRoute(path: routes.confirm, builder: (_, _) => const RenderConfirm()),
        GoRoute(path: routes.logout, builder: (_, _) => const RenderLogout()),
        // ── post-auth ──
        GoRoute(path: routes.terminal, builder: (_, _) => const RenderTerminal()),
      ],
    );
```

- **The guard is a test, both ways** — `test/routes_guard_test.dart` collects every
  `GoRoute.path` and compares them with `routes.all`: a path the app invented, and a path of the
  vocabulary the app does not declare, both fail. The second is the one that bites: the surface
  *sending* such a link works, the reader's device 404s. It is the twin of a Dioxus `build.rs`
  that checks every `#[route("…")]` against the shared vocabulary.
- **Tabs that the web shows at one address are state, not routes** — a route the web does not
  have is a link the web cannot answer.
- **Pre-auth vs post-auth is the redirect**: no session → the login path, except on the pre-auth
  paths. The core's `NeedsConfirm` goes to the confirm path (topic `io`), and that screen is the
  only one that asks the server.
- **Android's back button**: an open sheet closes first (`get_flutter_design_patterns` §9), then
  the router pops; at the app's root, back leaves the app.

## The start-up gate

A Dioxus client may refuse to render before a start-up read answers — e.g. a white-label
"which brand is this host" check. On the phone the gate sits under the router, in
`MaterialApp.router(builder: …)`: it needs the `MaterialApp` for text direction and the theme,
and nothing below it renders before it answers.

```dart
// lib/main.dart — the twin of a Bootstrap component above the Dioxus router
class StartupGate extends HookWidget {
  const StartupGate({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final http = context.read<CoreHost>().http;
    final attempt = useState(0);
    // use_resource: the read is memoized per attempt — Retry is a new attempt
    final gate = useFuture(useMemoized(() => branding_api.getBranding(http), [attempt.value]));

    if (gate.connectionState != ConnectionState.done) {
      return const RenderSplash();
    }

    return switch (gate.error) {
      null => child,
      BrandingError_DomainNotSupported() => const RenderDomainNotSupported(),
      // Any other failure. `error` is an Object? — an open domain, where a catch-all is right
      Object() => RenderCantReachServer(onRetry: () => attempt.value++),
    };
  }
}
```

## The providers

The shared states are created by `main()` and live as long as the process, so they are provided
with `.value`:

```dart
// lib/main.dart
class App extends StatelessWidget {
  const App({super.key, required this.host});

  final CoreHost host;

  @override
  Widget build(BuildContext context) => MultiProvider(
        providers: [
          Provider.value(value: host),
          ChangeNotifierProvider.value(value: host.app),
          ChangeNotifierProvider.value(value: host.session),
        ],
        child: const ThemedRouter(), // MaterialApp.router with the theme of the settings + StartupGate
      );
}
```

- **`AppState`** — view state (the open dialog) and the feed's versions; it holds no copy of the
  core's state (topic `websocket`).
- **`SessionState`** — whether a session exists; the router's `refreshListenable`.
- **`CoreHost`** — reached in actions with `context.read<CoreHost>()`.
