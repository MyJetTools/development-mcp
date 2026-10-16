# Accessibility — screen readers, text size, motion, contrast

On a phone the accessibility settings are part of the platform the app runs on: VoiceOver and
TalkBack read the semantics tree, the system text size scales every `Text`, Reduce Motion and Remove
animations ask for less movement. Flutter builds most of the semantics tree from the widgets by
itself — a `Text` is read, a `FilledButton` is a button with its label. What is left is what only
the app knows: what an icon means, what a painted chart shows, what belongs together, and what must
not be read at all.

## 1) Every control says what it does

```dart
final words = context.select((AppState app) => app.words);

// ✅ CORRECT — an icon-only button: the tooltip is its label
IconButton(tooltip: words.closePosition, icon: const Icon(Icons.close), onPressed: onClose)

// ✅ CORRECT — a tappable that is not a button widget
Semantics(
  button: true,
  label: words.openChart,
  child: GestureDetector(onTap: onOpenChart, child: const RenderMiniChart()),
)

// ❌ WRONG — read as "button", with nothing after it
IconButton(icon: const Icon(Icons.close), onPressed: onClose)
```

- **Labels are words** — from the core, like every word on the screen, never a Dart literal: the
  screen reader of a German phone reads German.
- **A decoration says nothing.** An `Icon` without `semanticLabel` is already silent; a glyph drawn
  as text (`▲`, `•`) or a decorative image is wrapped in `ExcludeSemantics`.
- **Colour is never the only carrier.** A change shows its sign (`+0.12%`, `−0.40%`, as the core
  formats it), so a screen reader reads it and a colour-blind user sees it; Buy and Sell are
  labelled, not only green and red.

## 2) What belongs together is read together

```dart
// a quote row — one stop for the screen reader: "EURUSD, 1.0842, 1.0844, +0.12%"
MergeSemantics(
  child: InkWell(
    onTap: onSelect,
    child: Row(children: [/* the name, the bid, the ask, the change */]),
  ),
)
```

- **One row, one stop.** Without `MergeSemantics` a list of three hundred instruments is twelve
  hundred swipes.
- **A screen's title is a header** — a hand-made app bar (`MAppBar`) wraps its title in
  `Semantics(header: true)`, and the screen reader jumps from header to header.

## 3) A painted picture has a description

A `CustomPaint` is silent; the chart says nothing unless the app says it:

```dart
Semantics(
  // a query of the core — its words and its formatting: "EURUSD, 1 hour, 1.0842, +0.12% today"
  label: chartSummary,
  child: RepaintBoundary(child: CustomPaint(painter: ChartPainter(chart))),
)
```

- **The summary is read at build, not per frame** — the chart repaints without rebuilding
  (topic `canvas`); the screen reader reads the summary when it reaches the chart and does not need
  every tick.
- **Never `liveRegion` on a value that ticks** — a live region is announced every time it changes.
  `Semantics(liveRegion: true)` belongs on what appears once and matters: an order refused, the
  connection lost.
- **What the chart's gestures change — the timeframe, the style — is also a labelled control**
  around it.

## 4) Sheets — the screen behind is not read

`ModalBarrier` drops the semantics of everything painted before it (it is a `BlockSemantics`), so
with the shell's order — the screen, the barrier, the sheet — the screen reader reads only the
sheet. A sheet painted without the barrier leaves the screen behind it readable and tappable.

A screen-reader user cannot swipe a sheet down; the barrier is their close:

```dart
// the shell's sheet host (topic `theme`, section 7)
ModalBarrier(
  color: context.tokens.sheetBackdrop,
  // without it the barrier calls Navigator.maybePop — and pops the page under the sheet
  onDismiss: () => context.read<AppState>().closeDialog(),
  // read and activated by VoiceOver and TalkBack; nothing is drawn — still no close cross
  semanticsLabel: words.close,
)
```

## 5) Text size is the system's

- **Every `Text` scales by `MediaQuery.textScalerOf(context)`.** The app never sets
  `TextScaler.noScaling` at its root — the twin of a web page that disables zoom.
- **A layout survives twice the text size**: no fixed height on a box that holds text — a minimum
  instead (`BoxConstraints(minHeight: context.tokens.tapMin)`); a line of text wraps or ellipsizes
  (`maxLines: 1, overflow: TextOverflow.ellipsis`) and never overflows.
- **A dense table that cannot reflow** — an order book, a price ladder — may clamp locally:
  `MediaQuery.withClampedTextScaling(maxScaleFactor: 1.3, child: …)` around that table, never
  around the app.
- **Chart labels do not scale** — the engine lays them out at the character width it assumes
  (topic `canvas`, section 7). The chart's description (section 3) and the price block above it are
  what a user with large text reads.

## 6) Motion — two settings, one rule

| Setting | Reaches Flutter as | Applied by Flutter itself |
|---|---|---|
| Android — Remove animations | `MediaQuery.disableAnimationsOf(context)` | `AnimationController`s are shortened (`AnimationBehavior.normal`) |
| iOS — Reduce Motion | `accessibilityFeatures.reduceMotion` of the platform dispatcher — iOS only | nothing |

```dart
// lib/utils/motion.dart — read when a motion starts, never cached
bool reduceMotion(BuildContext context) =>
    MediaQuery.disableAnimationsOf(context) ||
    View.of(context).platformDispatcher.accessibilityFeatures.reduceMotion;
```

- **When it is on**, sheets and tabs switch without the slide, and the chart jumps to its target
  instead of tweening — the flag goes to the engine with the rest of the look (topic `canvas`).
- **Feedback that is not movement stays** — the pressed state, haptics (topic `theme`).

## 7) Contrast and targets

- **Contrast is a property of the tokens**, and the tokens are the web's (topic `theme`). A pair
  that fails is fixed in the Dioxus client's CSS tokens, so both surfaces get the fix — never a
  Dart override for one widget.
- `textContrastGuideline` is WCAG AA: 4.5 : 1 for text, 3 : 1 for large text (18 and up, or 14
  bold). It is checked in both themes.
- **`tokens.tapMin` (44) is the floor for a target** (topic `theme`), checked with
  `iOSTapTargetGuideline` (44 × 44) on both platforms. `androidTapTargetGuideline` asks 48 × 48 —
  Material's own floor — and fails the product's 44 by design; it is run only if the tokens move to
  48.

## 8) The checks

```dart
// test/accessibility_test.dart — every screen, in both themes (testApp: topic `testing`)
testWidgets('the trade screen meets the guidelines', (tester) async {
  final handle = tester.ensureSemantics();
  final host = await testHost();
  await tester.pumpWidget(testApp(host: host, child: const RenderTradeScreen()));

  await expectLater(tester, meetsGuideline(labeledTapTargetGuideline));
  await expectLater(tester, meetsGuideline(iOSTapTargetGuideline));
  await expectLater(tester, meetsGuideline(textContrastGuideline));

  handle.dispose();
});

testWidgets('the trade screen holds at twice the text size', (tester) async {
  tester.platformDispatcher.textScaleFactorTestValue = 2;
  addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
  final host = await testHost();

  await tester.pumpWidget(testApp(host: host, child: const RenderTradeScreen()));

  expect(tester.takeException(), isNull); // a RenderFlex overflow is an exception here
});
```

- **The checks find a missing label, not a wrong one.** Before a release: one pass with VoiceOver
  and one with TalkBack through sign-in, the quotes, an order and a sheet, and one pass at the
  phone's largest text size.

## Checklist

1. Every icon-only control has a `tooltip` or a `Semantics` label, and every label is the core's words
2. Decorations are silent; colour never carries a meaning alone
3. A row is one `MergeSemantics` stop; a screen's title is a header
4. A painted chart has a `Semantics` label from the core; no `liveRegion` on a ticking value
5. The barrier sits between the screen and the sheet, with `onDismiss` and a `semanticsLabel`
6. No `TextScaler.noScaling` at the root, no fixed height around text; a clamp only on a dense table
7. Reduce Motion is read from both settings; the sheets and the chart jump instead of moving when it is on
8. Contrast is fixed in the CSS tokens; every target is at least `tokens.tapMin`
9. The guideline tests pass for every screen in both themes; VoiceOver and TalkBack are tried before a release
