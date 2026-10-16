# Performance — rebuilds, lists, frames

The phone's budget is one frame: 16 ms at 60 Hz, 8 ms at 120 Hz, for the build, the layout and the
paint of everything that changed. The architecture already takes the heaviest work off Flutter: the
parsing and the computing are the core's, the feed notifies `AppState` at most once per frame
(`get_flutter_client_bootstrap_guide`, topic `websocket`), and a chart repaints without a rebuild
(topic `canvas`). What is left is Flutter's own cost: how much rebuilds on a notification, how much
a list builds, and what a frame repaints.

## 1) Measure — profile mode, a slow phone

- **`flutter run --profile` on the slowest supported Android**, and an older iPhone. Never debug
  mode (JIT and asserts — many times slower, it proves nothing) and never a simulator.
- **DevTools → Performance**: the frame chart splits the UI thread (build, layout) from the raster
  thread (paint), and its rebuild tracking shows which widgets were built in a frame.
- **A janky frame is fixed by finding what rebuilt or repainted**, not by guessing. A fix that
  measured nothing is a change, not a fix.

## 2) Rebuild the leaf, not the page

`context.watch<AppState>()` at the top of a page rebuilds the whole page on every notification —
and while the feed moves, that is every frame. **Watch where the value is shown**: a row selects
its own quote, the account card selects the account (topic `websocket` of the bootstrap guide).

```dart
// ❌ WRONG — the page rebuilds on every tick of every quote
@override
Widget build(BuildContext context) {
  final app = context.watch<AppState>();
  return Column(children: [RenderAccountCard(account: app.account()), RenderQuotes(app: app)]);
}

// ✅ CORRECT — the page watches nothing; each leaf selects its slice
@override
Widget build(BuildContext context) =>
    const Column(children: [RenderAccountCard(), Expanded(child: RenderQuotes())]);
```

- **`select` compares with deep equality** — `provider` uses `DeepCollectionEquality`: collections
  by their contents, everything else by `==`. Select a value: a number, a string, an enum, a bridge
  struct. **Never** a Dart object without `==` — a state class, a closure — it is a new value
  every time, and the widget rebuilds on every notification.
- **A selector runs on every notification, for every widget that selected** — keep it one query:
  one bridge call that returns one small value. Never a selector that brings a whole list across
  the bridge to pick one field out of it.

## 3) Widget classes, not helper methods; `const` wherever it can be

```dart
// ❌ WRONG — a method is part of its parent's build: rebuilt with it, never const
Widget _buildHeader(BuildContext context) => Row(children: [/* … */]);

// ✅ CORRECT — a widget is its own element: const when its arguments are, rebuilt only when they
// change or what it watches does
class RenderHeader extends StatelessWidget {
  const RenderHeader({super.key});
  // …
}
```

- **A `const` widget is the same instance on every build, and Flutter skips it.** Write `const`
  wherever it compiles, and keep constructors `const`-able — `final` fields, no logic in the
  initialiser list.
- **An animated subtree takes its static part as `child:`** — `ListenableBuilder` /
  `AnimatedBuilder` build that child once and hand it to every frame.
- **Small widgets** — each tab, list and row its own component (§11) — rebuild each for its own
  reasons and no one else's.

## 4) Lists are built lazily

A list whose length comes from data is a `ListView.builder` (or `SliverList.builder`): only the
rows on screen and in the cache extent are built — **and only those rows subscribe to the feed**, so
three hundred instruments cost what the fifteen visible ones cost.

```dart
// ❌ WRONG — builds and subscribes every row, visible or not
ListView(children: [for (final id in ids) RenderQuoteRow(instrumentId: id)])

// ✅ CORRECT — rows of one height: built on demand, laid out without measuring each
ListView.builder(
  itemCount: ids.length,
  itemExtent: context.tokens.quoteRowH,
  itemBuilder: (context, i) => RenderQuoteRow(key: ValueKey(ids[i]), instrumentId: ids[i]),
)
```

- **Rows of one height take `itemExtent`** (or `prototypeItem`): no row is measured, and a jump to
  an index is exact.
- **A row with state of its own (§11) in a list that inserts, removes or reorders has a
  `ValueKey(id)`** — without it the state follows the index, and a row shows its neighbour's
  state. A builder list that reorders also takes `findChildIndexCallback`, so a keyed row finds
  its new index instead of being rebuilt from scratch.
- **Never `shrinkWrap: true`** to fit a list into another scroll — it lays out every row. One
  scroll per screen (topic `theme`) is why it is never needed.
- **The order and the filter of a list are the core's** — a rule. `build` never sorts or filters a
  data list; it builds the rows it is given.
- A fixed short list — a settings menu, five tabs — may use `children`.

## 5) Paint less

- **What repaints on its own rhythm is its own layer** — `RepaintBoundary`. The chart already is
  (topic `canvas`), and `ListView` already wraps each row in one. Not everywhere: each boundary is
  a layer with its own memory — add one where DevTools (`debugRepaintRainbowEnabled`) shows a
  repaint spreading past the widget that changed.
- **Never animate an `Opacity`** — `FadeTransition` or `AnimatedOpacity`; for a plain shape, the
  alpha of its colour.
- **Keep layer-saving effects off what moves** — `ShaderMask`, `ColorFilter`, `BackdropFilter`,
  `Clip.antiAliasWithSaveLayer` each render into an offscreen layer every frame they are painted.

## 6) Nothing heavy on the UI thread — and nothing heavy in Dart

- **Every core call runs on the main isolate, synchronously** (`get_flutter_client_bootstrap_guide`,
  topic `rust-core`), so its time is spent inside the frame. Calls stay small: a query returns what
  one widget shows; the chart takes one call per frame (topic `canvas`).
- **Dart has no heavy work of its own** to move to an isolate — parsing, computing and formatting
  are the core's. **Never `compute` / `Isolate.run` around a core call**: another isolate is
  another thread, and the core is single-threaded.
- **Start-up**: what runs before `runApp` — reading the records, starting the core — delays the
  first frame. It is reads and one call; measure it in profile mode like any frame.
- **A ticker or a timer runs only while something moves** — the chart's tweens stop when they
  arrive (topic `canvas`).

## 7) Memory

- **What a hook creates, the hook disposes** — `useTextEditingController`,
  `useAnimationController`, `useComponentState`. A controller created in a state class is disposed
  in its `dispose()`.
- **An opaque bridge object is disposed by the state class that owns it** — the GC cannot see the
  native memory behind a small handle (`get_flutter_client_bootstrap_guide`, topic `bridge`).
- **A subscription is cancelled in the same scope that opened it** — `useEffect` returns the
  cancel; a state class cancels in `dispose()`.

## Checklist

1. Measured in `flutter run --profile` on the slowest supported phone, never in debug or a simulator
2. No `context.watch<AppState>()` above the leaves that show the value; leaves `select` their slice
3. A selector returns a value with `==` — never a state object or a closure — and makes one small query
4. Widget classes, not `_buildX()` methods; `const` constructors and instances wherever possible
5. Data lists are `.builder` lists; rows of one height take `itemExtent`; stateful rows are keyed by id
6. No `shrinkWrap: true`; no sorting or filtering in `build`
7. `RepaintBoundary` where a repaint is measured to spread; no animated `Opacity`; no layer-saving effects on what moves
8. No core call from another isolate; core calls small, one per frame for the chart
9. Everything created is disposed in the scope that created it
