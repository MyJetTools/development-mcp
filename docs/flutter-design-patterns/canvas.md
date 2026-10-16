# Canvas — a chart decided by the core, painted by Flutter

Read this before writing or changing a widget that draws on a canvas — a candle chart, a depth
chart. It is the twin of the `canvas` and `canvas-input` topics of `get_dioxus_design_patterns`,
and keeps their numbering where a rule carries over; a `§N` below is a section of the
flutter-design-patterns index.

The web's chart engine already has one rule that makes another painter affordable: **several
painters, one model — nothing is decided in a painter** (`canvas-webgl`). The phone does not port
the engine. It takes the engine — the core's, an opaque object through the bridge — and adds one
more painter, Flutter's `Canvas`. Everything a painter would have to decide is already the
engine's.

| | The markup | The picture |
|---|---|---|
| What it is | the timeframe row, the style switch, "back to the newest" | everything on the chart |
| Where it lives | the component's state — a `ChangeNotifier` (`state.dart`) | the chart engine — an opaque Rust object the widget holds, outside every notifier |
| What redraws it | `notifyListeners()` → a rebuild of the markup | `CustomPainter.paint`, asked for by the engine's frame notifier — no rebuild |
| Files | `render.dart`, `state.dart`, `actions.dart` | the engine (Rust, shared) + `painter.dart` (Dart, draws a frame plan) |

## 1) The engine lives outside every notifier

What the picture is drawn from — the series, the scroll, the zoom, the pointer — is in the engine,
created once with the widget and dropped with it. On the web that is `Rc<RefCell<Engine>>` from
`use_hook`; here it is the bridge's opaque engine from a hook:

```dart
// render.dart
class RenderChartPanel extends HookWidget {
  const RenderChartPanel({super.key, required this.instrumentId});

  final String instrumentId;

  @override
  Widget build(BuildContext context) {
    final cs = useComponentState(ChartPanelState.new);
    // Created once, disposed on unmount — never in a notifier, never rebuilt
    final chart = useChartEngine();

    // …the markup reads `cs`; the canvas reads only `chart`
    return RepaintBoundary(
      child: CustomPaint(painter: ChartPainter(chart), isComplex: true, willChange: true),
    );
  }
}
```

- **A gesture handler calls the engine and writes to no notifier** — except the copy of what the
  markup itself shows, and only when it differs (section 3).
- **One engine per chart on screen.** Nothing global is keyed by a canvas id: there is no DOM to
  find a canvas in.
- **The chart is its own layer** — `RepaintBoundary`: a frame of the chart repaints the chart, not
  the screen around it, and the screen's rebuilds do not repaint the chart.

## 2) Every change asks for ONE frame

On the web the engine asks for one `requestAnimationFrame`, guarded by `frame_pending`. Flutter
already paints at most once per vsync, so the guard is the framework's; what stays is the shape: **a
change asks for a frame, the frame draws.**

```dart
// The engine handle carries a Listenable it notifies after every call that changed the picture.
// CustomPainter repaints on it WITHOUT rebuilding any widget
class ChartPainter extends CustomPainter {
  ChartPainter(this.chart) : super(repaint: chart.frames);

  final ChartEngineHandle chart;

  @override
  void paint(Canvas canvas, Size size) {
    // ONE synchronous call: the whole frame, decided by the engine
    final plan = chart.engine.frame(width: size.width, height: size.height);
    paintPlan(canvas, plan, chart.look);
  }

  @override
  bool shouldRepaint(ChartPainter old) => old.chart != chart;
}
```

- **The frame draws everything, every time**, from the plan as it stands. Nothing is kept from the
  previous frame.
- **A frame with nothing to draw still clears** — a Flutter canvas starts empty on every paint, so
  this one is free. Section 5's epoch rule is not: the engine has to drop the previous series
  itself.
- **A change that forgot its frame is invisible while the feed ticks.** Try every control with the
  feed stopped — a closed market, a chart nobody touches.

## 3) Tweens — stepped by the time Flutter hands the frame

```dart
// The chart's ticker: runs ONLY while something moves. `chart.wake()` — called by every engine
// call that set a target — starts it when it is stopped
Ticker useChartTicker(ChartEngineHandle chart) {
  final vsync = useSingleTickerProvider();
  final ticker = useMemoized(() {
    late final Ticker ticker;
    ticker = vsync.createTicker((_) {
      // The frame's own timestamp — what requestAnimationFrame hands the web's frame
      final nowMs = SchedulerBinding.instance.currentFrameTimeStamp.inMicroseconds / 1000.0;
      final moving = chart.engine.animate(nowMs: nowMs);
      chart.frames.notify();
      if (!moving) {
        ticker.stop();
      }
    });
    return ticker;
  });
  useEffect(() => ticker.dispose, [ticker]);
  return ticker;
}
```

- **By time, never by frame count** — a 120 Hz screen calls twice as often. The step is capped in
  the engine, the same code as the web's: it is shared.
- **One ticker per chart**, running only while `animate` says something moves. A handler sets a
  target; a frame draws `actual`.

### The component's state holds only what the markup shows

The pressed timeframe, `DataState<void>` of the read — the candles themselves go to the engine.
What the markup takes back from the picture ("back to the newest") is a copy, written after a
gesture handler, and only when it differs:

```dart
// actions.dart
void syncShown(ChartPanelState cs, ChartEngineHandle chart) {
  final shown = chart.engine.shown();
  if (cs.shown != shown) {
    cs.setShown(shown);
  }
}
```

## 4) A frame touches no notifier — the look is handed over

The palette and the words are handed to the engine when they change, never read in the frame:

```dart
final tokens = context.tokens;
final words = context.watch<AppState>().words;
useEffect(() {
  chart.engine.setLook(palette: ChartPalette.fromTokens(tokens), words: words);
  chart.frames.notify();
  return null;
}, [tokens, words]);
```

- **An effect may do this** although effects run inside `build`: it changes the engine and asks
  for a repaint — `frames` is listened to by the painter, not by a widget, so nothing is rebuilt.
- **Colours cross as numbers, once per theme** (`canvas-webgl` §6): `ChartPalette.fromTokens` turns
  the generated tokens into ARGB integers; the painter maps the plan's palette indices onto `Paint`s
  built once. A colour typed into `painter.dart` is two colours the first time one is edited.

## 5) The same component, another series

A widget given another instrument is not created again — its engine still holds the previous
series. So every build tells the engine what the props name; a new series empties the chart and
moves the engine to a new epoch, and the read of the new series is a state object created for that
epoch:

```dart
// render.dart — a plain call in build: it changes the ENGINE, which no widget listens to, so it is
// not a write in build (§16)
chart.engine.show(seriesId: instrumentId);

// One read per epoch, started in its state's constructor — never from build or an effect (§10)
final read = useComponentState(
  () => SeriesReadState(http, chart, instrumentId, epoch: chart.engine.epoch),
  [chart.engine.epoch],
);
```

- **Every answer lands under the epoch it was asked with** (`land(epoch, series)` refuses a stale
  one). The epoch logic is the engine's — shared, not re-written in Dart.
- **The live data — the forming candle, the bid, the positions — reaches the engine from the core's
  state** (`chart.engine.syncFromFeed()` on a `Changed` that touches them). Dart never copies the
  feed to hand it back.
- **The history read** goes through `lib/api/` like any other call
  (`get_flutter_client_bootstrap_guide`, topic `api-calls`).

## 6) Size — logical pixels, and nothing decided without one

Flutter's canvas is in logical pixels, so the web's bitmap rules (CSS px × `devicePixelRatio`,
never render `width` / `height`) have no twin. What remains:

- **The engine is told the size by the frame** (`frame(width:, height:)`), and it decides nothing
  size-dependent until it has one. The data and the size arrive in either order; a first fit made
  when data lands is a fit to a width of zero.
- Lines are 1 logical pixel, as they are 1 CSS pixel on the web.

## 7) Text — the engine's character width must be true

An engine that lays labels out without measuring them takes a fixed width per character (the MT
terminal's takes `AXIS_CHAR_WIDTH` = 6.7 px). That holds only for the font and size the web draws
labels in, so:

- **Labels are drawn in the web's label face at the engine's size** — a monospaced face, bundled
  (topic `theme`).
- **A widget test pins it**: lay out a run of digits with `TextPainter` in that style and assert
  the advance per character equals the engine's width within half a pixel. A font update that moves
  it is caught before an axis clips its labels.
- **`TextPainter` layouts are cached** per string and style — the plan repeats most labels from frame
  to frame — and a painter evicted from the cache is `dispose()`d.
- **What only a painter can measure** (a plate's real width) comes back to the engine
  (`canvas-input` §5): the engine tests a tap against what the last frame drew.
- Bundled fonts are there before the first frame, so the web's "redraw when the font loads" has no
  twin.

## 8) Gestures — `canvas-input` on Flutter

**Fingers call what the mouse calls.** One finger is the drag of a held button; two fingers are the
wheel's zoom ladder. The handlers only recognise the gesture and call the same engine methods the
web's touch handlers call.

```dart
GestureDetector(
  onScaleStart: (d) {
    if (d.pointerCount == 1) {
      chart.engine.press(x: d.localFocalPoint.dx, y: d.localFocalPoint.dy);
    } else {
      // A second finger ends the drag where it is, then the pinch begins
      chart.engine.pinchStart();
    }
  },
  onScaleUpdate: (d) {
    // `scale` is relative to the start of the gesture; the engine turns it into levels of the zoom
    // ladder, re-anchored at every level — the web's rule, in the engine
    final redraw = d.pointerCount == 1
        ? chart.engine.touchMove(x: d.localFocalPoint.dx, y: d.localFocalPoint.dy)
        : chart.engine.pinchTo(scale: d.scale);
    if (redraw) {
      chart.frames.notify();
    }
  },
  // A gesture the system takes away ends here too — the `touchcancel` rule
  onScaleEnd: (_) => chart.engine.touchEnded(),
  child: CustomPaint(painter: ChartPainter(chart)),
)
```

- **`onScale*` only.** Scale is a superset of pan: a `GestureDetector` given both `onPan*` and
  `onScale*` throws. One finger arrives as a scale gesture with `pointerCount == 1`.
- **The pinch's step lives in the engine**, so the web and the phone zoom by the same ladder.

**Who owns a gesture is decided by the gesture arena — never by swallowing events.** It is the twin
of `touch-action`:

| Where the chart is | Web | Flutter |
|---|---|---|
| full screen | `touch-action: none` | the chart's recognizers alone — nothing above competes |
| a band inside a page that scrolls | `touch-action: pan-y` | a horizontal-drag recognizer for the pan and a two-finger scale for the zoom; a vertical swipe is left to the page's `ListView` |

- **A finger is not a hover.** No crosshair follows a finger, and none is left behind when it lifts.
- **One finger or two — never both.**
- Try every gesture on a device — a simulator's two-finger gesture is not a hand.

## 9) The frame plan across the bridge

- **One synchronous call per frame returns everything** — never a call per candle.
- **Geometry as flat arrays** — `Float32List` positions and colour indices — drawn as one
  `Vertices` mesh (`canvas.drawVertices`) and line lists (`drawRawPoints` with `PointMode.lines`):
  the web WebGL painter's "one mesh, one draw call", on Flutter.
- **Dashes are segments the engine emits** — a Flutter `Paint` has no line dash.
- **Clipping is done while the plan is built**, not with `canvas.clipRect` per group.
- **The budget**: the call plus the paint well inside a frame at 120 Hz — measured in
  `flutter run --profile` on the slowest supported Android, with the indicators on.

## Checklist

1. The engine is an opaque object from a hook, outside every notifier; the chart is its own `RepaintBoundary`
2. Gesture handlers call the engine; the only notifier they write is the copy of what the markup shows, and only when it differs
3. Every engine call that changed the picture notifies the frame `Listenable`; `CustomPainter(repaint:)` repaints without a rebuild
4. Every control was tried with the feed stopped
5. Tweens are stepped by the frame's timestamp, capped in the engine; the ticker runs only while something moves
6. The component's state holds only the pressed buttons and `DataState<void>`
7. Every build tells the engine what the props name; the read's state is created per epoch, and a read lands only under its epoch
8. Live data reaches the engine from the core's state, not through Dart
9. The palette crosses as numbers, once per theme change; no colour literal in `painter.dart`
10. The engine decides nothing size-dependent without a size
11. Labels are in the web's face at the engine's size; a test pins the character width
12. One finger = the drag, two = the zoom ladder, through the engine's own methods; `onScale*` only
13. The arena decides ownership: full screen takes everything, a band leaves vertical swipes to the page
14. One synchronous call per frame; geometry as flat arrays, one mesh
