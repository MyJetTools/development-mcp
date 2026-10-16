---
alwaysApply: false
---
# Flutter Design Patterns

The conventions every component of a Flutter client is written by, when the client is the phone
twin of a Dioxus client (`get_flutter_client_bootstrap_guide`). **The section numbers are the
numbers of `dioxus-design-patterns`** (`get_dioxus_design_patterns`), and each section says what its
Dioxus rule becomes in Flutter — so a Dioxus component and its Flutter twin are written and reviewed
against the same list, rule by rule.

The split is fixed: **the business logic is Rust** (the shared core), **rendering and I/O are
Dart's**. A pattern here never decides a number, a word, a URL or a stored value — that is the
core's (`get_flutter_client_bootstrap_guide`, topic `rust-core`).

A pattern that needs a document of its own is a topic, listed at the end. Read a topic:
`resource://flutter-design-patterns/{topic}`, or `get_flutter_design_patterns` with `topic`.

## The shape that keeps the two close

| Dioxus | Flutter |
|---|---|
| `#[component] fn RenderX(props) -> Element` | `class RenderX extends HookWidget` — `flutter_hooks` |
| `use_signal(\|\| ComponentState::new(..))` | `useComponentState(() => XState(..))` — `lib/utils/hooks.dart` |
| `use_hook(\|\| value)` | `useMemoized(() => value)` |
| `use_effect(..)` · `use_drop(..)` | `useEffect(() { ..; return dispose; }, keys)` |
| `consume_context::<Signal<AppState>>()` | `context.watch<AppState>()` in build · `context.read<AppState>()` in a handler — `provider` |
| `use_context_provider(\|\| ..)` | `ChangeNotifierProvider` above the router |
| a `struct` with methods in a signal | a `ChangeNotifier` subclass with methods |
| `dioxus-utils` | `lib/utils/` — the same names: `DataState<T>` / `RenderState<T>`, `DialogValue<T>`, `NotifyChildComponent<T>`, `EventHandler<T>` |

Packages: `flutter_hooks` 0.21.3+1, `provider` 6.1.5+1 (checked 2026-10-16).

**Why `ChangeNotifier` and not a signals library.** A Dioxus write guard notifies when it is
dropped, whatever was written: *mutate in place, always notify*. `ChangeNotifier` is exactly that.
`signals.dart` (7.x) notifies only when the new value is not `==` the old one, so a struct mutated
in place needs `set(value, force: true)` at every write — the Dioxus shape fought at every line.

## 1) Naming conventions

- **`cs`** — the component's state object: `final cs = useComponentState(() => TradeState(order));`
- **`app`** — `AppState`: `context.watch<AppState>()` in `build`, `context.read<AppState>()` in a
  handler.
- There is no `cs_ra` / `cs_wa`: Dart has no borrow guards. **The rule they enforced stays: `build`
  reads `cs`; only event handlers and the continuations of async actions call `cs`'s methods.**

```dart
class RenderItem extends HookWidget {
  const RenderItem({super.key, required this.id});

  final int id;

  @override
  Widget build(BuildContext context) {
    final cs = useComponentState(ItemState.new);

    return FilledButton(
      onPressed: () => cs.select(id), // event — a method call
      child: Text(cs.title),          // build — a read
    );
  }
}

// async — the method is called after the await, and only if the component is still there
Future<void> save(Http http, ItemState cs, int id) async {
  final result = await items_api.save(http, id);
  if (cs.disposed) return;
  cs.saveFinished(result);
}

// ❌ WRONG — a state method in build: it notifies, and the widget asks to rebuild while it is
// building (§16)
@override
Widget build(BuildContext context) {
  final cs = useComponentState(ItemState.new);
  cs.select(id);
  // …
}
```

- **A state class is named after its component** — `TradeState` for `RenderTradeScreen`. Dioxus
  examples say `ComponentState` in every folder; Dart refuses two exported classes of one name
  through one `mod.dart`.
- **Widgets keep the Rust names** (`RenderTradeScreen`, `RenderQuotesTab`). A component whose name
  Flutter already uses takes the `M` prefix of the stylesheet's `.m-` classes — `MAppBar`,
  `MTabBar`, `MBottomSheet` — never a `hide` on the Material import.

## 2) One state class, one notifier

**NEVER** several `useState` / `ValueNotifier`s for the fields of one component. One class, one
notifier:

```dart
// ❌ WRONG — separate notifiers
final submitting = useState(false);
final candleType = useState(0);

// ✅ CORRECT — one state class
final cs = useComponentState(() => TradeState(order, accuracy));
```

**Exception**: a child widget whose API takes a `ValueListenable<T>` — keep that one value
separate, as Dioxus keeps a `Signal<T>` a child requires as a prop.

`useComponentState` is the twin of `use_signal`:

```dart
// lib/utils/hooks.dart
/// `use_signal(|| ComponentState::new(..))`: created on the first build (and again when `keys`
/// change — Dioxus's `key:`), rebuilds the widget whenever it notifies, disposed with the widget
T useComponentState<T extends ChangeNotifier>(
  T Function() create, [
  List<Object?> keys = const [],
]) {
  final cs = useMemoized(create, keys);
  useEffect(() => cs.dispose, [cs]);
  return useListenable(cs);
}
```

- **`create` sees the props of the first build** — as Dioxus's init closure does. A prop the state
  is built from goes into `keys`; when it changes, a new state is created and the old one disposed
  at the end of that build.
- **`useComponentState` is the first line of `build`.** A hot reload recreates every hook from the
  first changed line onwards; a state declared first survives edits below it.
- **A state class never overrides `==`.** The dispose effect is keyed by the instance; two
  instances that compare equal would leave the new one never disposed.

Every state class extends `StateBase`, which knows it was disposed — what lets an async action stop
where a `dioxus::spawn` would have been cancelled with its scope:

```dart
// lib/utils/state_base.dart
abstract class StateBase extends ChangeNotifier {
  bool _disposed = false;

  bool get disposed => _disposed;

  /// After dispose a notification has no one to reach — debug builds would assert; this makes it
  /// the no-op it is in release. The action still checks `disposed` after its await
  @override
  void notifyListeners() {
    if (!_disposed) {
      super.notifyListeners();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}
```

## 3) Component folder structure — render / state / actions / mod

Every non-trivial component lives in its own folder, with the Rust file names:

```
dialogs/edit_tp_sl/
├── mod.dart       ← export 'actions.dart'; export 'render.dart'; export 'state.dart';
├── render.dart    ← the widget — rendering only, minimal logic
├── state.dart     ← the state class + its mutation methods
└── actions.dart   ← functions: conversions, AppState readers, async actions
```

- **render.dart** — draws. Reads `cs` and `app`, calls functions from `state.dart` / `actions.dart`,
  builds widgets.
- **state.dart** — one `StateBase` subclass. Methods for coupled changes, each notifying once.
- **actions.dart** — top-level functions. A function that reads external state takes it as a
  parameter. Async actions live here, not in the widget.
- **mod.dart** — `export` lines only, the `mod x; pub use x::*;` rule: callers import the folder,
  never a file inside it.

A simple component — no state, no actions — is `render.dart` alone, as in Rust.

## 4) Pass models to components — not individual props

```dart
// ❌ WRONG — fifteen fields unpacked from a model
const EditDialog({super.key, required this.accountId, required this.orderId, required this.openPrice /* … */});

// ✅ CORRECT — the model
const EditTpSlDialog({super.key, required this.order, required this.instrumentName, required this.accuracy});

final EditTpSl order;
final String instrumentName;
final int accuracy;
```

A model is immutable: a generated bridge class, an opaque handle, or a Dart class with `final`
fields. A widget never mutates a model it was given. Flutter rebuilds a child whenever its parent
rebuilds (Dioxus skips equal props) — `const` constructors and small widgets keep that cheap.

## 5) External state reads — functions with the state as a parameter

```dart
// ❌ WRONG — a lookup inline in build
final volume = isActive ? app.positions.firstWhere((p) => p.id == id).volume : 0.0;

// ✅ CORRECT — a function in actions.dart
final volume = positionVolume(app, id, isActive: isActive);
```

If such a function DECIDES something — a number the user acts on — it is a core function reached
through the bridge, and `actions.dart` only calls it.

## 6) Controlled inputs — the state holds the text, the controller follows it

Dioxus renders `input { value: "{cs_ra.tp_price}", oninput: … }`: the state holds the string the user
typed (including a bare `1.` mid-entry). Flutter's `TextField` keeps its own text in a controller,
so a hook keeps the controller in step with the state:

```dart
// lib/utils/hooks.dart
/// The `value: "{…}"` of a controlled input. What the user types reaches the state through
/// `onChanged`; a value the STATE changed (the other field recomputed) is copied in, with the
/// cursor at its end
TextEditingController useBoundController(String text) {
  final controller = useTextEditingController(text: text);
  if (controller.text != text) {
    controller.value = TextEditingValue(
      text: text,
      selection: TextSelection.collapsed(offset: text.length),
    );
  }
  return controller;
}
```

```dart
final tpPrice = useBoundController(cs.tpPrice);
final tpPct = useBoundController(cs.tpPct);

TextField(controller: tpPrice, onChanged: (v) => cs.setTpPrice(v));
TextField(controller: tpPct, onChanged: (v) => cs.setTpPct(v));
```

- Typing in the price field: `setTpPrice` → the state recomputes the percent → rebuild → the price
  controller already holds that text, the percent controller takes the new percent.
- The controller lives in a hook, never in the state class. The write in `build` goes to a
  **child's** controller — Flutter allows marking a descendant dirty while an ancestor builds; it
  is never a write to a notifier this widget listens to (§16).

## 7) State methods over inline handler logic

Domain logic is a method; a handler is one call. When the rule is the core's, the state class wraps
the core's per-screen struct (`get_flutter_client_bootstrap_guide`, topic `bridge`):

```dart
// state.dart — the TP/SL editor's rules are the core's
class TpSlState extends StateBase {
  TpSlState(EditTpSl order) : _editor = TpSlEditor(order: order);

  final TpSlEditor _editor; // opaque — the core's struct, through the bridge

  String get tpPrice => _editor.tpPrice;
  String get tpPct => _editor.tpPct;

  void setTpPrice(String value) {
    _editor.setTpPrice(value: value); // price → percent, decided in Rust
    notifyListeners();
  }

  @override
  void dispose() {
    _editor.dispose(); // the opaque object goes with the screen
    super.dispose();
  }
}

// render.dart — one line
onChanged: (v) => cs.setTpPrice(v),
```

Several notifications in a row are the antipattern `cs.write()` × 3 is:

```dart
// ❌ WRONG — three rebuilds, two of them of a half-changed state
cs.setLoading(true);
cs.setError(null);
cs.setTab(Tab.active);

// ✅ CORRECT — one method, one notification
void reset() {
  _loading = true;
  _error = null;
  _tab = Tab.active;
  notifyListeners();
}
```

- **Fields are private, with getters; changes go through methods.** Dart has no write guard to
  spot a one-off `cs.field = x` by, so there are no public setters at all.
- **State classes are tested without widgets** — `flutter test` on the class alone, as the Rust
  structs are tested with `cargo test`.

## 8) Classes with many fields — the required ones in the constructor, defaults for the rest

```dart
class ChartViewState extends StateBase {
  ChartViewState({required this.instrumentId, required int candleType}) : _candleType = candleType;

  final String instrumentId;
  int _candleType;
  CandleStyle _candleStyle = CandleStyle.candles;
  // … every other field — a default here
}
```

When a child must change the parent's class (a toolbar switching the candle style), pass **the
state object**, not a callback per field — the `Signal<Struct>` rule.

## 9) Dialogs: lifecycle, rendering, and template

On a phone a dialog is a bottom sheet; everything else of §9 holds as written.

### `DialogState` — always part of `AppState`

```dart
// lib/dialogs/dialog_state.dart
sealed class DialogState {
  const DialogState();
}

final class DialogNone extends DialogState {
  const DialogNone();
}

final class DialogEditTpSl extends DialogState {
  const DialogEditTpSl({required this.order, required this.instrumentName, required this.accuracy});

  final EditTpSl order;
  final String instrumentName;
  final int accuracy;
}
```

```dart
// lib/states/app_state.dart
class AppState extends StateBase {
  DialogState _dialog = const DialogNone();

  DialogState get dialog => _dialog;

  void openEditTpSl(EditTpSl order, String instrumentName, int accuracy) {
    _dialog = DialogEditTpSl(order: order, instrumentName: instrumentName, accuracy: accuracy);
    notifyListeners();
  }

  void closeDialog() {
    _dialog = const DialogNone();
    notifyListeners();
  }
}
```

A `sealed` class is Dart's enum with data: a `switch` over it is exhaustive, so a new dialog fails
to compile everywhere it is not handled.

### `RenderDialog` — the router

```dart
// lib/dialogs/render.dart
class RenderDialog extends StatelessWidget {
  const RenderDialog({super.key});

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();

    return switch (app.dialog) {
      DialogNone() => const SizedBox.shrink(),
      DialogEditTpSl(:final order, :final instrumentName, :final accuracy) => dialogTemplate(
          context,
          title: instrumentName,
          content: EditTpSlDialog(
            order: order,
            accuracy: accuracy,
            onSubmit: (data) => onTpSlSubmit(context.read<CoreHost>(), order, data),
          ),
        ),
    };
  }
}
```

The app's shell puts it over the screen in a sheet host: the backdrop, the slide in and out, the
drag, and Android's back closing the sheet first (topic `theme`). A dialog is opened by changing the
state: `context.read<AppState>().openEditTpSl(…)`.

### Dialog results — always through an `EventHandler`

A dialog **never** sends a request or changes `AppState`. It collects input and hands the result to
its parent:

```dart
// lib/utils/event_handler.dart — the name the Dioxus code uses
typedef EventHandler<T> = void Function(T value);
```

```dart
// ✅ CORRECT — the dialog returns data
FilledButton(onPressed: () => onSubmit(cs.submitData()), child: Text(words.save))

// the parent's action — the request and the state
Future<void> onTpSlSubmit(CoreHost host, EditTpSl order, TpSlSubmitData data) async {
  try {
    await trading_api.setTpSl(host.http, data.toRequest(order));
    host.app.closeDialog();
  } on RequestError catch (err) {
    if (err.text case final text?) {
      host.app.showToast(text, ToastType.error);
    }
  }
}
```

```dart
// ❌ WRONG — the dialog sends the request and changes the state
onPressed: () async {
  await trading_api.setTpSl(context.read<CoreHost>().http, cs.request());
  context.read<AppState>().closeDialog();
},
```

### `dialogTemplate` — the standard wrapper

Every sheet is built by `dialogTemplate(context, title:, content:, okButton:)` — the grip, the
title, the body (the one scrollable), an optional primary button. **No close cross and no Cancel
button**: a sheet goes by a swipe, a tap outside, or back. While data loads, the loading or error
widget is the `content`.

## 10) `DataState` + reading — for async data

Every piece of async data is a `DataState<T>` — the twin of `dioxus_utils::DataState`, same names:

```dart
// lib/utils/data_state.dart
sealed class RenderState<T> {
  const RenderState();
}

final class RenderNone<T> extends RenderState<T> {
  const RenderNone();
}

final class RenderLoading<T> extends RenderState<T> {
  const RenderLoading();
}

final class RenderLoaded<T> extends RenderState<T> {
  const RenderLoaded(this.value);

  final T value;
}

final class RenderError<T> extends RenderState<T> {
  const RenderError(this.message);

  final String message;
}

final class DataState<T> {
  RenderState<T> _state = RenderNone<T>();

  RenderState<T> asRef() => _state;

  void setLoading() => _state = RenderLoading<T>();

  void setLoaded(T value) => _state = RenderLoaded<T>(value);

  void setError(String message) => _state = RenderError<T>(message);

  /// Back to `RenderNone` — the next read starts from scratch
  void reset() => _state = RenderNone<T>();

  bool hasValue() => _state is RenderLoaded<T>;

  T? tryUnwrapAsLoaded() => switch (_state) {
        RenderLoaded<T>(:final value) => value,
        RenderNone<T>() || RenderLoading<T>() || RenderError<T>() => null,
      };
}
```

**One difference from Dioxus, on purpose: the read starts with the state, not in build.** Dioxus
starts it from render (`RenderState::None` → `spawn`), and `spawn` defers the write. Flutter has no
deferring `spawn` in `build`, and a notification during `build` is §16. So the state class starts
its read when it is created and when it is reset; `build` only switches:

```dart
// state.dart
class HistoryTabState extends StateBase {
  HistoryTabState(this._http, this.accountId) {
    unawaited(_read());
  }

  final Http _http;
  final int accountId;
  final data = DataState<List<ClosedPosition>>();

  /// After a change on the server — the twin of `.reset()` then a re-render
  void reload() => unawaited(_read());

  Future<void> _read() async {
    data.setLoading();
    notifyListeners();

    try {
      final items = await history_api.closedPositions(_http, accountId);
      if (disposed) return;
      data.setLoaded(items);
    } on RequestError catch (err) {
      if (disposed) return;
      // `text` is null for a 401 — the router is already on its way to confirmation
      data.setError(err.text ?? '');
    }

    notifyListeners();
  }
}
```

```dart
// render.dart — another account is another state: `keys` recreate it
final http = context.read<CoreHost>().http;
final cs = useComponentState(() => HistoryTabState(http, accountId), [accountId]);

return switch (cs.data.asRef()) {
  RenderNone() || RenderLoading() => const RenderLoadingIndicator(),
  RenderError(:final message) => RenderLoadingError(message: message),
  RenderLoaded(:final value) => RenderHistoryList(items: value),
};
```

- The `notifyListeners()` in the constructor finds no listener yet — the hook subscribes after
  `create` returns — so it costs nothing and requests nothing.
- **Never start a read from `useEffect`.** Flutter's `useEffect` runs synchronously inside `build`
  (Dioxus's `use_effect` runs after render). A read starts in the constructor, in a method a
  handler calls, or — for work that must follow the frame — from
  `WidgetsBinding.instance.addPostFrameCallback`.

## 11) Tabs and lists — each is its own component with its own state

- Each tab is a widget that receives an id and owns its `DataState`.
- Lists inside a tab are widgets with their own state too.
- The page's state holds only UI — the current tab, the input, the selected item — **never data
  arrays**.

```
RenderPage                   ← PageState: input, selected item, tab
└── RenderContent
    ├── RenderTabA { id }    ← own state + DataState
    │   ├── RenderListOne    ← own state + DataState
    │   └── RenderListTwo    ← own state + DataState
    └── RenderTabB { id, cs } ← the parent's state when the parent must reset it
```

A live feed is not a `DataState`: it is the core's state, read where it is when the core says it
moved (`get_flutter_client_bootstrap_guide`, topic `websocket`).

## 12) API calls — always through the module's prefix

A call of the server is a function in `lib/api/` — the core's request builder, sent by
`lib/native/http.dart`, read by the core's reader (`get_flutter_client_bootstrap_guide`, topic
`api-calls`). Each file is imported with the module's name as a prefix, so a call reads as a call to
the server:

```dart
// ✅ CORRECT — visibly an API call, like crate::api::trading::set_order(..)
import 'package:client_flutter/api/trading.dart' as trading_api;

await trading_api.setOrder(http, params);
```

```dart
// ❌ WRONG — looks like a local function
import 'package:client_flutter/api/trading.dart';

await setOrder(http, params);
```

```dart
// ❌ WRONG — a request sent around lib/api: no core reader, so no 401 handling, no typed error
final response = await http.post(Uri.parse('$origin/api/trading/v1/SetOrder'), body: json);
```

## 13) Hook handling tips

- **Hooks are called unconditionally, in the same order, on every build, before any early
  `return`** — never under an `if`, never in a loop, never in a callback. The rule of Dioxus hooks,
  word for word. A different hook at the same position throws `Type mismatch between hooks`.
- **A custom hook is a function named `useX`** that calls other hooks; it is called only from a
  `HookWidget.build` or another hook.
- **`useEffect` runs inside `build`, synchronously** — not after the frame, as Dioxus's
  `use_effect` does. It never calls a method that notifies a widget's state (§10, §16); work that
  must follow the frame goes to `WidgetsBinding.instance.addPostFrameCallback`.
- **`useEffect` lists what it reads in its keys.** Dioxus tracks the signals an effect reads;
  Flutter does not — a value missing from the keys is a change the effect never sees. `null` keys
  run it on every build; `const []` once.
- **The Dioxus hooks, side by side**: `use_hook(|| v)` → `useMemoized(() => v)`; `use_drop(f)` →
  `useEffect(() => f, const [])`; `use_resource` → `useFuture(useMemoized(read, [key]))`; a callback
  without a rebuild → `useOnListenableChange`; a rebuild on part of a notifier →
  `useListenableSelector`.
- **`context.watch` / `context.select` in build, `context.read` in handlers.** A `watch` in a
  handler throws in debug, and so does one inside an effect's first run; a `read` in build is a
  widget that misses changes.
- **After an `await`, `context.mounted` before touching `context`** — the
  `use_build_context_synchronously` lint enforces it. `useIsMounted` is deprecated.
- **An object captured by a closure is a reference** — like a `Signal`, captured once, never cloned.
- **After changing a state class, hot restart** — hot reload keeps the instance the hook memoized,
  with the old fields.

## 14) `NotifyChildComponent<T>` — parent-to-child notification

When a parent's action must make a child update something it owns (a reload, a chart's style), and
the child keeps its own state:

```dart
// parent — created once with the parent, passed down like any other prop
final repaintNotify = useNotifyChildComponent<void>();

FilledButton(
  onPressed: () {
    chartView.setCandleStyle(CandleStyle.line);
    repaintNotify.notifyOtherComponents(null);
  },
  child: Text(words.line),
),
RenderCanvasChart(view: chartView, repaintNotify: repaintNotify),
```

```dart
// child — subscribes with a hook, at the top level of build
useOnNotify(repaintNotify, (_) => chart.setStyle(view.candleStyle));
```

```dart
// lib/utils/notify_child_component.dart
final class NotifyChildComponent<T> extends ChangeNotifier {
  T? _last;

  T? get last => _last;

  void notifyOtherComponents(T value) {
    _last = value;
    notifyListeners();
  }
}

// lib/utils/hooks.dart
NotifyChildComponent<T> useNotifyChildComponent<T>() {
  final notify = useMemoized(NotifyChildComponent<T>.new);
  useEffect(() => notify.dispose, [notify]);
  return notify;
}

/// The child's `on_notify`: a callback, no rebuild of its own
void useOnNotify<T>(NotifyChildComponent<T> notify, void Function(T value) onNotify) {
  useOnListenableChange(notify, () => onNotify(notify.last as T));
}
```

- `useOnNotify` is a hook — unconditional, at the top of `build`.
- The callback runs inside the parent's handler, not in a build — it may call the child's state
  methods.
- For a canvas: the callback hands the change to the engine, and the engine asks for a frame — it
  never paints (topic `canvas`).

## 15) Styles — tokens, not stylesheets

There is no CSS. Colours, sizes, radii and fonts come from `context.tokens`, generated from the
Dioxus client's CSS tokens file. **NEVER** edit the generated `tokens.g.dart`, and never write a
colour or a size literal in a widget. The tokens, the theme and the phone's layout rules — safe
areas, tap targets, one scroll per screen, the keyboard, sheets — are the topic `theme`.

## 16) NEVER change a notifier during build

A state method in `build` notifies; the widget is told to rebuild while it is building. Debug
builds stop on the assertion (`setState() or markNeedsBuild() called during build`); release
builds rebuild again next frame — the Dioxus rule's infinite loop, at frame rate.

```dart
// ❌ WRONG
@override
Widget build(BuildContext context) {
  final chartView = useComponentState(ChartViewState.new);
  chartView.setPositions(positionOverlays(app, accountId)); // a write in build
  return RenderCanvasChart(view: chartView);
}

// ✅ CORRECT — compute a local, pass it as a prop
@override
Widget build(BuildContext context) {
  final chartView = useComponentState(ChartViewState.new);
  final positions = positionOverlays(app, accountId);
  return RenderCanvasChart(view: chartView, positions: positions);
}
```

**Rule:** only event handlers (`onPressed`, `onChanged`, `onTap`, …) and the continuations of async
actions call state methods. `build` reads and builds.

## 17) Single access per operation

One operation is one method and one notification (§7). In `build`, read what is needed into locals
once at the top — `final quote = app.quote(id);` — rather than asking the same question in every
child.

## 18) Stored state initialises the state — the screen is never drawn from storage

On the web, browser storage is read once in `new()` so the first render is already right. On the
phone storage is Dart's I/O and its meaning is the core's (`get_flutter_client_bootstrap_guide`,
topic `local-storage`), and the same rules hold:

- **Dart reads the phone's records before `runApp` and starts the core with them** — the first
  frame is right: the selection, the settings, the theme. Nothing is drawn wrong and then
  corrected.
- **A component state is created from stored values in its constructor** — through the core's
  queries — never from a `useEffect` after the first frame.
- **Write-through.** The state method that changes a stored value sends the core its input in the
  same method; the core answers with `Store` (and the server KV request, when there is one), keeps
  the dirty mark until the server confirms, and Dart carries both out — a sheet that closes on
  save does not lose the write, because nothing of it belonged to the sheet.
- **Navigating with data** — the destination's record is written (a core input), then
  `context.go(..)`. The route carries only what the server needs — an id, a token — never a figure
  the screen states as a fact.
- **Above the router** — what the shell draws outside any screen (a toast, the words) is provided
  above `MaterialApp.router`, born from the core at start and changed by the same input that
  stores it: the twin of the `GlobalSignal` born from storage.

## 19) A canvas is drawn by frames, not from state

The engine lives outside every notifier — in the core, as an opaque object; a change asks for a
frame; the frame draws everything from the engine. **The full pattern is the topic `canvas` — read
it before writing or changing anything that paints.**

## Rules for every line of Dart

The twins of the rules every line of Rust follows (`get_application_architecture_best_practices`).

### Zero warnings

`flutter analyze --fatal-infos --fatal-warnings` MUST pass with `very_good_analysis` and the
analyzer's strict modes (`get_flutter_client_bootstrap_guide`, topic `project-files`).
**NEVER** `// ignore:` as a solution — the `#[allow(clippy::…)]` rule. Optionally
`flutter_hooks_lint` (a `custom_lint` plugin) adds `hooks_avoid_nesting`, which catches a hook under
a condition before it throws at run time.

### Named classes over records

**NEVER** a record — `(double, double)` — in a field, a parameter or a return type of an API. A
record is fine inside one function.

```dart
// ❌ WRONG — which double is which?
(double, double)? crosshair(Offset pointer);

// ✅ CORRECT
final class ScreenPoint {
  const ScreenPoint({required this.x, required this.y});

  final double x;
  final double y;
}

ScreenPoint? crosshair(Offset pointer);
```

### Exhaustive `switch` — no `default:`, no `_` on an `enum` or a `sealed` class

```dart
// ❌ WRONG — a new dialog compiles and is silently ignored
switch (app.dialog) {
  case DialogEditTpSl():
    return const RenderEditTpSl();
  default:
    return const SizedBox.shrink();
}

// ✅ CORRECT — every variant named; a new one does not compile until it is handled
return switch (app.dialog) {
  DialogNone() => const SizedBox.shrink(),
  DialogEditTpSl() => const RenderEditTpSl(),
};
```

A wildcard is fine on an open domain — a `String`, an `int`, an unknown id from the server, an
`Object?` error.

### Logging

No `Info` / `Debug` noise by default. `debugPrint` is the twin of `console_log` — for what a
developer reads at a cable; anything the user must know is on screen.

### Module exports

`mod.dart` holds `export` lines and nothing else; one file = one class or one job; callers import a
folder's `mod.dart`, never a file inside it.

### Performance, accessibility, tests

- **Watch at the leaf, build lists lazily** — `select` where the value is shown, `.builder` lists,
  widget classes over helper methods, `const` wherever it compiles. Measured in profile mode on a
  slow phone. Topic `performance`.
- **Every control is labelled with the core's words, and the layout survives the system's text
  size** — a painted chart has a description, a sheet hides the screen behind it. Topic
  `accessibility`.
- **Rules are tested in the core; Dart tests what Dart does** — state classes without widgets,
  every `RenderState` of a screen, fakes at the interfaces, real phones for the bridge. Topic
  `testing`.

## Topics

Read a topic: `resource://flutter-design-patterns/{topic}`, or `get_flutter_design_patterns` with
`topic`.

| Topic | What is inside |
| --- | --- |
| [`canvas`](canvas.md) | A widget that draws on a canvas — a chart: the engine in the core as an opaque object, one frame per change through `CustomPainter(repaint:)`, tweens stepped by the frame's time, the frame plan across the bridge in one call, text and the engine's character width, gestures and who owns them in the gesture arena |
| [`theme`](theme.md) | Tokens generated from the Dioxus client's CSS, the theme and the status bar, safe areas, tap targets, one scroll per screen, the keyboard, bottom sheets, haptics, what may differ between the web and the phone |
| [`performance`](performance.md) | Rebuilds, lists and frames: measuring in profile mode on a slow phone, `select` at the leaf, widget classes and `const`, lazy `.builder` lists with keys and `itemExtent`, repaint boundaries and the effects that cost a layer, nothing heavy on the UI thread, disposing what was created |
| [`accessibility`](accessibility.md) | Screen readers, text size, motion, contrast: labels from the core's words, one `MergeSemantics` stop per row, a described chart, sheets behind a `ModalBarrier`, text scaling and local clamps, Reduce Motion on both platforms, tap targets, the guideline tests |
| [`testing`](testing.md) | What each level proves: rules in the core, state classes without widgets, widget tests over `testHost` / `testApp`, fakes at the interfaces, fake time, fonts in tests, goldens on one OS, `integration_test` on real phones |
