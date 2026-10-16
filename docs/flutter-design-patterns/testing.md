# Testing — what each level proves

The logic is Rust and is tested in Rust. Dart is tested for what Dart does: a state class changes and
notifies as its methods say, a screen shows each of its states and calls the right method, the host
carries the core's effects out, and the whole thing works on a phone. Every test sits at its own
level — **a widget test that asserts a price or a percent is a core test written in the wrong
language.**

## 1) The levels

| What is proved | Where | How |
|---|---|---|
| a rule — a number, a word, a URL, what a frame does, what is stored | the core | `cargo test`: inputs in, effects out (`get_flutter_client_bootstrap_guide`, topic `rust-core`) |
| a state class — its methods, its notifications, `disposed` | `test/` | `flutter test`, no widgets (section 3) |
| a screen — each `RenderState`, a tap reaching its method, a dialog opening | `test/` | `testWidgets` (section 4) |
| the host — what is sent, stored and navigated for an effect | `test/` | `flutter test` over the mocked bridge (`get_flutter_client_bootstrap_guide`, topic `bridge`) |
| accessibility | `test/` | the guidelines (topic `accessibility`) |
| the generated files are current, the routes, the label width | `test/` | the guards (`get_flutter_client_bootstrap_guide`, topic `ci`) |
| the real bridge, the native library, the platform's I/O | `integration_test/` | on a phone (section 7) |

Most tests are in the first row — that is where the decisions are.

## 2) Fakes at the interfaces, never the platform

`flutter test` runs on the developer's machine: no Keychain, no `URLSession`, no network — under
`testWidgets` every real `dart:io` HTTP request is answered with a 400. A test replaces each kind of
I/O at its interface (`get_flutter_client_bootstrap_guide`, topic `io`):

| I/O | In a test |
|---|---|
| the bridge | its mock (`get_flutter_client_bootstrap_guide`, topic `bridge`, section 11) |
| HTTP | `MockClient` from `package:http/testing.dart`, handed to `Http` |
| records | `SharedPreferencesAsyncPlatform.instance = InMemorySharedPreferencesAsync.empty()` — `shared_preferences_platform_interface`, a dev dependency at the version `shared_preferences` resolves |
| the session token | `FlutterSecureStorage.setMockInitialValues({})` |
| time | the fake clock of `testWidgets` — `tester.pump(duration)` |

**The one seam the app keeps for it**: `CoreHost.create` takes an optional `http.Client` and hands it
to `Http` (`get_flutter_client_bootstrap_guide`, topic `api-calls`). Production passes nothing and
gets the platform's client.

```dart
// test/helpers/test_host.dart — what main() builds, over fakes
const testFlavor = Flavor(
  origin: 'https://test.invalid', // a name that never resolves
  userAgentProduct: 'Test/1',
  appName: 'Test',
);

Future<CoreHost> testHost({http.Client? client}) async {
  SharedPreferencesAsyncPlatform.instance = InMemorySharedPreferencesAsync.empty();
  FlutterSecureStorage.setMockInitialValues({});

  final prefs = Prefs(SharedPreferencesAsync());
  final sessionStore = SessionStore(const FlutterSecureStorage());
  final host = CoreHost.create(
    flavor: testFlavor,
    prefs: prefs,
    sessionStore: sessionStore,
    httpClient: client ?? MockClient((_) async => http.Response('', 404)),
  );

  // main()'s start, with nothing stored and no session
  final stored = await prefs.load();
  host.run(() => core_api.startCore(args: StartArgs(stored: stored, sessionToken: null)));
  return host;
}
```

```dart
// test/helpers/test_app.dart — what main() puts above every screen
Widget testApp({required CoreHost host, required Widget child, MTokens tokens = MTokens.dark}) =>
    MultiProvider(
      providers: [
        Provider.value(value: host),
        ChangeNotifierProvider.value(value: host.app),
        ChangeNotifierProvider.value(value: host.session),
      ],
      // appTheme — the ThemeData builder of lib/theme/theme.dart, the app's own
      child: MaterialApp(theme: appTheme(tokens), home: Scaffold(body: child)),
    );
```

- **The same providers and the same theme as the app.** Without the `MTokens` extension
  `context.tokens` throws (it is a `!`), and without `AppState` every `select` does.
- **The bridge is mocked once per file, in `setUpAll`** (`get_flutter_client_bootstrap_guide`,
  topic `bridge`, section 11) — `testHost`, and every state class over an opaque object, assume it.
- **Each test builds its own host** — nothing static carries over from the test before.

## 3) State classes — no widgets

```dart
test('reset notifies once', () {
  final cs = PageState();
  var notified = 0;
  cs.addListener(() => notified++);

  cs.reset();

  expect(notified, 1); // §7: one method, one notification
});

test('a read that lands after dispose changes nothing', () async {
  final answer = Completer<http.Response>();
  final host = await testHost(client: MockClient((_) => answer.future));
  final cs = HistoryTabState(host.http, 42); // the read starts in the constructor (§10)

  cs.dispose();
  answer.complete(http.Response('[]', 200)); // the mocked bridge's reader turns it into a list
  await Future<void>.delayed(Duration.zero);

  expect(cs.data.asRef(), isA<RenderLoading<List<ClosedPosition>>>());
});
```

- **Every state class has these two**: a method notifies once, and an answer that arrives after
  `dispose()` is dropped — the twin of a `spawn` cancelled with its scope.

## 4) Widget tests — a screen and its states

```dart
testWidgets('the history tab shows the indicator, then the list', (tester) async {
  final answer = Completer<http.Response>();
  final host = await testHost(client: MockClient((_) => answer.future));

  await tester.pumpWidget(testApp(host: host, child: const RenderHistoryTab(accountId: 42)));
  expect(find.byType(RenderLoadingIndicator), findsOneWidget);

  answer.complete(http.Response('[]', 200)); // the mocked bridge's reader turns it into a list
  await tester.pump();

  expect(find.byType(RenderHistoryList), findsOneWidget);
});
```

- **Every `RenderState` branch is shown once** — none, loading, error, loaded.
- **Find by type or by key.** A text finder uses the words the test's core gave —
  `find.text(host.app.words.retry)` — never a literal copied from the screen.
- **Assert the state, not the pixels**: after a tap on "edit",
  `expect(host.app.dialog, isA<DialogEditTpSl>())`. The state is the contract; how the sheet looks
  is the theme's.
- **A tap is followed by a pump** — `await tester.tap(finder); await tester.pump();` — the rebuild
  happens on the frame.
- **`pumpAndSettle()` waits until no frame is scheduled.** On a screen with an endless animation —
  an indeterminate progress indicator, the feed asking for frames — it never settles and throws
  after its timeout; there, `pump(duration)` steps the clock by what the test needs.
- **A timer still pending when a test ends fails it.** A state class that starts a timer cancels it
  in `dispose()` — the failure is the leak it would be in the app.
- **Time inside `testWidgets` is fake** — never `sleep`, never a real wait;
  `tester.pump(const Duration(seconds: 3))` is three seconds. Real I/O, when a test truly needs it
  (a file), goes inside `tester.runAsync`.

## 5) Fonts — loaded before anything is measured

A test draws text in the test font (`FlutterTest`, whose glyphs are plain boxes) until the bundled
faces are loaded. Finders do not care; whatever is measured does — the label-width guard (topic
`canvas`, section 7), goldens, an overflow at a large text size. Load them once, for every test:

```dart
// test/flutter_test_config.dart — runs around every test file under test/
Future<void> testExecutable(FutureOr<void> Function() testMain) async {
  TestWidgetsFlutterBinding.ensureInitialized();
  await loadFont('Roboto Flex', 'assets/fonts/RobotoFlex.ttf');
  await loadFont('JetBrains Mono', 'assets/fonts/JetBrainsMono-Regular.ttf');
  await testMain();
}

Future<void> loadFont(String family, String asset) async {
  final loader = FontLoader(family)..addFont(rootBundle.load(asset));
  await loader.load();
}
```

The families and the files are the ones `pubspec.yaml` declares
(`get_flutter_client_bootstrap_guide`, topic `project-files`).

## 6) Goldens — few, on one platform

- **Only where the picture is the behaviour** — a painter, a theme recipe (a raised button,
  pressed). Never a whole screen: a screen golden fails on every unrelated change and gets
  regenerated without being looked at.
- **Made and compared on one OS — the CI's.** Text is rasterised differently on macOS and on
  Linux, and a golden made on a laptop fails on CI; `flutter test --update-goldens` runs where the
  goldens are compared.
- **Fonts loaded** (section 5); both themes.

## 7) On a phone — `integration_test/`

- **What only a phone has**: the native library through the real bridge, the platform's HTTP and
  WebSocket stacks, Keychain and Keystore, the lifecycle (`get_flutter_client_bootstrap_guide`,
  topics `bridge`, `io`, `lifecycle`).
- **Few tests, each a user's path** — sign-in, the feed arriving, a background → foreground
  resync, an order placed and an order refused.
- `IntegrationTestWidgetsFlutterBinding.ensureInitialized()` first, the real `RustLib.init()` in
  `setUpAll` (topic `bridge`).
- **On a real iPhone and a real Android phone** — `flutter test integration_test -d <device>` —
  against a staging host and a demo account, never production.

## 8) Rules for every test

- **One behaviour per test, named by it** — `'a read that lands after dispose changes nothing'`,
  not `'test 3'`.
- **A fixed bug starts with a failing test** — in the core when the bug was a rule, in Dart when it
  was the surface.
- **No test reaches the network or the real stores.** A test that passes only online is not a test.
- **Tests obey the zero-warnings rule** — `test/` and `integration_test/` are analysed like `lib/`.

## Checklist

1. Every rule is tested in the core; no Dart test asserts a number, a word or a URL the core decides
2. Every state class: a method notifies once; an answer after `dispose()` changes nothing
3. Every screen: each `RenderState` branch, each tap reaching its method — asserted on the state
4. Fakes at the interfaces: `MockClient`, in-memory records, mocked secure storage, the mocked bridge
5. `testApp` gives a screen the app's providers and theme; `testHost` is built per test
6. No `pumpAndSettle` on a screen that animates for ever; no real waiting; no timer left pending
7. The bundled fonts are loaded in `flutter_test_config.dart` before anything is measured
8. Goldens only for painters and theme recipes, made on the CI's OS
9. `integration_test/` passes on a real iPhone and a real Android phone before a release
