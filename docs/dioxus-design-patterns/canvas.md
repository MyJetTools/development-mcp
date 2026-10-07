# Canvas — drawn by animation frames, not from state

Read this before writing or changing a component that draws on a `<canvas>` — the 2D context, WebGL or WebGPU. This topic is §19 of **dioxus-design-patterns** in full; the common Dioxus patterns (`ComponentState`, `DataState`, `NotifyChildComponent`) are in its index — `get_dioxus_design_patterns` without `topic` — and a `§N` below is a section of that index.

A write to a signal re-renders the component's HTML. The HTML of a canvas is one `<canvas>` tag, and it does not change. What does change is the data under the picture — new data arrived, the view was dragged, the pointer moved — and the answer to that is an **animation frame**, which redraws everything it needs from the data as it then stands.

So a component with a canvas has two halves, and nothing crosses the line between them:

| | The markup | The picture |
| --- | --- | --- |
| What it is | The card, the buttons, the figures around the canvas | Everything drawn on the canvas |
| Where it lives | `ComponentState` in a signal | An engine — `Rc<RefCell<Engine>>` outside every signal |
| What redraws it | A re-render, on a write to the signal | An animation frame, asked for by the engine |
| Files | `render.rs`, `state.rs`, `actions.rs` | `engine.rs`, `paint.rs` + a model with no framework in it |

```
views/chart/
├── mod.rs
├── render.rs   ← the markup: the <canvas> tag and the controls; canvas handlers call the engine
├── state.rs    ← ComponentState — only what the markup shows
├── actions.rs  ← the read; the controls that live on both sides
├── engine.rs   ← Engine + its handle: holds what the picture is drawn from, asks for frames
└── paint.rs    ← one frame: turns the model's answers into canvas calls
```

The rule is about the `<canvas>` element, not about the 2D context: a WebGL / WebGPU surface is the same tag that never changes. The engine, the single frame, the markup-only state, the size set by the frame and the model apply to it as written. The examples below use the 2D context.

## 1) What the picture is drawn from lives outside signals — in the engine

The data, the scroll, the zoom, the pointer: all of it is in an `Rc<RefCell<Engine>>` created once by `use_hook`. The handlers on the canvas — wheel, pointer, resize — call the engine and **write to no signal**.

```rust
// engine.rs
/// Everything one frame is drawn from
pub struct Engine {
    pub(super) canvas_id: &'static str,
    pub(super) model: ChartModel,         // the data, the scroll, the zoom, the pointer — see 8)
    pub(super) loc: Option<Localization>, // handed over by the component — see 4)
    pub(super) palette: Option<Palette>,  // read off the canvas's CSS — see 7)
    theme: Option<Theme>,
    frame_pending: bool,                  // see 2)
    dropped: bool,                        // see 4)
}

/// The component's handle on its engine — cheap to clone into each event handler
#[derive(Clone)]
pub struct ChartHandle(Rc<RefCell<Engine>>);
```

```rust
// ✅ CORRECT — render.rs
const CANVAS_ID: &str = "chart-canvas";

#[component]
pub fn Chart(instrument_id: String) -> Element {
    // Created once with the component, outside every signal
    let chart = use_hook(|| ChartHandle::new(CANVAS_ID));

    rsx! {
        canvas {
            id: CANVAS_ID,
            class: "chart__canvas",
            onresize: {
                let chart = chart.clone();
                move |e: Event<ResizeData>| {
                    if let Ok(size) = e.data().get_content_box_size() {
                        chart.resized(size.width, size.height);
                    }
                }
            },
            onwheel: {
                let chart = chart.clone();
                move |e: Event<WheelData>| {
                    let travel = e.data().delta().strip_units();
                    // `true` — the wheel was the chart's, and the page must not scroll with it
                    if chart.wheel(travel.x, travel.y, e.data().element_coordinates().x) {
                        e.prevent_default();
                    }
                }
            },
            onpointermove: {
                let chart = chart.clone();
                move |e: Event<PointerData>| {
                    let at = e.data().element_coordinates();
                    chart.pointer_move(at.x, at.y);
                }
            },
        }
    }
}
```

## 2) Every change asks for ONE `requestAnimationFrame`

Whatever changes the engine — data landing, a wheel, a drag, the pointer, a resize, the theme — changes it and asks for a frame. `frame_pending` makes any number of changes ride the same frame: a mouse reports its position more often than a screen refreshes.

```rust
// engine.rs
impl ChartHandle {
    pub fn request_frame(&self) {
        {
            let mut engine = self.0.borrow_mut();

            if engine.frame_pending || engine.dropped {
                return;
            }

            engine.frame_pending = true;
        }

        let engine = self.0.clone();

        // Freed by the browser's one call of it
        let frame = Closure::once_into_js(move || {
            let mut engine = engine.borrow_mut();
            engine.frame_pending = false;

            if engine.dropped {
                return;
            }

            paint::paint(&mut engine);
        });

        let asked = web_sys::window()
            .is_some_and(|window| window.request_animation_frame(frame.unchecked_ref()).is_ok());

        // No frame is coming: the next change has to be free to ask again
        if !asked {
            self.0.borrow_mut().frame_pending = false;
        }
    }

    /// Change the model, then draw
    fn change(&self, change: impl FnOnce(&mut ChartModel)) {
        change(&mut self.0.borrow_mut().model);
        self.request_frame();
    }

    /// Data arrived
    pub fn land(&self, series: CandleSeries) {
        self.change(|model| model.land(series));
    }

    pub fn pointer_move(&self, x: f64, y: f64) {
        self.change(|model| model.pointer_move(x, y));
    }

    /// The canvas was measured, or resized
    pub fn resized(&self, width: f64, height: f64) {
        self.change(|model| model.set_canvas(width, height));
    }

    /// A wheel over the canvas. A frame is asked for only when the chart took it
    pub fn wheel(&self, dx: f64, dy: f64, x: f64) -> bool {
        let taken = self.0.borrow_mut().model.wheel(dx, dy, x);

        if taken {
            self.request_frame();
        }

        taken
    }
}
```

- **The frame draws everything, every time**, from the engine as it stands. Nothing is kept from the frame before — a picture assembled from what was already on the canvas goes wrong the first time two things change at once.
- The `borrow_mut()` is released before `request_frame()` is called — a borrow held across it panics with `already borrowed`.
- **Tweens** (a value animated towards its target): the frame asks for the next frame while anything is still moving. It is still one live loop per canvas — a `loop_running` flag, never a second closure — and it stops when nothing is animating.

## 3) The component's signal holds only what the markup shows

Which button is pressed, what the read has come to. A write to it re-renders the markup around the canvas — that is all a write can do.

```rust
// ✅ CORRECT — state.rs
pub struct ComponentState {
    /// Which button of the switch is pressed
    size: CandleSize,
    /// What the read has come to: in flight, failed, answered. The candles themselves
    /// go to the engine, so the payload is nothing
    data: DataState<()>,
}
```

The read (dioxus-design-patterns §10) writes its outcome to the state and hands the data to the engine:

```rust
// actions.rs
spawn(async move {
    cs.write().data.set_loading();
    match crate::api::charts::get_candles(instrument_id).await {
        Ok(series) => {
            cs.write().data.set_loaded(());
            chart.land(series); // the engine asks for a frame
        }
        Err(err) => cs.write().data.set_error(err.to_string()),
    }
});
```

A control that is both a pressed button and something drawn is stated on both sides — one function moves both:

```rust
// actions.rs
pub fn pick_size(mut cs: Signal<ComponentState>, chart: &ChartHandle, size: CandleSize) {
    cs.write().pick_size(size); // the button that is pressed
    chart.pick_size(size);      // the candles that are drawn
}
```

```rust
// ❌ WRONG — the series, the scroll and the zoom in the state: every tick, every wheel notch
// and every pointer move re-renders markup that did not change
pub struct ComponentState {
    candles: DataState<Vec<Candle>>,
    scroll: f64,
    zoom_idx: usize,
    pointer: Option<(f64, f64)>,
}
```

## 4) A frame reads no signal and writes none

A frame is called by the browser, outside any Dioxus scope. Everything it needs from the app — the localization, the theme — is **handed to the engine beforehand** from a `use_effect`; `use_drop` raises `dropped`, so a frame already on its way draws nothing.

```rust
// ✅ CORRECT — render.rs
let app_state = consume_context::<Signal<AppState>>();

// Re-runs when the app's state changes; the engine asks for a frame only if the words
// or the theme really did
use_effect({
    let chart = chart.clone();
    move || {
        let app_ra = app_state.read();
        chart.set_look(app_ra.loc(), app_ra.theme);
    }
});

use_drop({
    let chart = chart.clone();
    move || chart.dropped()
});
```

```rust
// engine.rs
impl ChartHandle {
    pub fn set_look(&self, loc: &Localization, theme: Theme) {
        {
            let mut engine = self.0.borrow_mut();

            let same_words = engine.loc.as_ref().is_some_and(|held| held.id == loc.id);
            let same_theme = engine.theme == Some(theme);

            if same_words && same_theme {
                return;
            }

            if !same_words {
                engine.loc = Some(loc.clone());
            }

            if !same_theme {
                engine.theme = Some(theme);
                // The tokens the colours were read from have new values — see 7)
                engine.palette = None;
            }
        }

        self.request_frame();
    }

    /// The component left the screen
    pub fn dropped(&self) {
        self.0.borrow_mut().dropped = true;
    }
}
```

```rust
// ❌ WRONG — signals inside the frame: there is no Dioxus scope there
let frame = Closure::once_into_js(move || {
    let lang = app_state.read().lang;
    paint(&cs.read(), lang);
    cs.write().painted = true;
});
```

The frame finds the canvas by its id every time and never holds the element: a reference goes stale when the canvas re-mounts. No canvas with that id — the frame draws nothing.

## 5) Never render `width` / `height` on the canvas — the frame sets the size

Setting `width` or `height` clears the bitmap, and an attribute rendered from `rsx!` is applied by a re-render. The box is sized by CSS; the bitmap is sized by the frame: CSS pixels × `devicePixelRatio`.

```rust
// ❌ WRONG — a re-render applies the attributes and the picture is gone
canvas { id: CANVAS_ID, width: "{width}", height: "{height}" }

// ✅ CORRECT — the box is the stylesheet's; `onresize` tells the engine its size, see 1)
canvas { id: CANVAS_ID, class: "chart__canvas" }
```

```rust
// ✅ CORRECT — paint.rs: `width` / `height` are the CSS pixels the model was told by `resized()`
let ratio = window.device_pixel_ratio().max(1.0);
let (bitmap_width, bitmap_height) = (
    (width * ratio).round() as u32,
    (height * ratio).round() as u32,
);

// Only when it differs: setting it clears the canvas and re-allocates the bitmap
if canvas.width() != bitmap_width {
    canvas.set_width(bitmap_width);
}
if canvas.height() != bitmap_height {
    canvas.set_height(bitmap_height);
}

// Everything below is in CSS pixels
let _ = ctx.set_transform(ratio, 0.0, 0.0, ratio, 0.0, 0.0);
ctx.clear_rect(0.0, 0.0, width, height);
```

Not measured yet (`width <= 0.0`) — the frame draws nothing; the resize that measures the canvas asks for a frame of its own.

## 6) A canvas is not redrawn when its font arrives

HTML text set in a fallback face is redrawn by the browser the moment the real font loads. A canvas keeps the pixels it was given. So after the first frame that knows its font: `document.fonts.load(font)`, then one more frame.

```rust
// ✅ CORRECT — engine.rs: called ONCE (a `font_awaited` flag), after the first frame that knew its font
fn redraw_when_loaded(chart: ChartHandle, font: String) {
    let Some(fonts) = web_sys::window()
        .and_then(|window| window.document())
        .map(|document| document.fonts())
    else {
        return;
    };

    // `font` as a canvas spells one: "500 11px Inter"
    let loading = fonts.load(&font);

    wasm_bindgen_futures::spawn_local(async move {
        // Loaded or failed, the frame is the same one: drawn in the face the browser now has
        let _ = wasm_bindgen_futures::JsFuture::from(loading).await;
        chart.request_frame();
    });
}
```

```rust
// ❌ WRONG — the frame that drew the labels is the last one, and nothing waits for the font:
// on a quiet canvas they stand in the fallback face until something else asks for a frame
paint::paint(&mut engine);
```

## 7) Colours — a canvas does not take `var(--token)`

Declare the canvas's colours in CSS as aliases of the design tokens, read them back resolved with `getComputedStyle`, and read them again after the theme changes.

```css
/* ✅ CORRECT — declared once, on the card: the canvas reads them, the legend is styled from them */
.chart {
  --chart-up: var(--ok);
  --chart-down: var(--err);
  --chart-grid: var(--border);
}

.chart__canvas {
  display: block;
  width: 100%;
  height: 380px;
  /* The frame reads these two off the canvas: the face of its labels and the fallback ink */
  font-family: var(--font-mono);
  color: var(--text);
}
```

```rust
// ✅ CORRECT — paint.rs
impl Palette {
    fn read(window: &Window, canvas: &HtmlCanvasElement) -> Self {
        let style = window.get_computed_style(canvas).ok().flatten();

        let property = |name: &str| -> Option<String> {
            let value = style.as_ref()?.get_property_value(name).ok()?;
            let value = value.trim();

            (!value.is_empty()).then(|| value.to_string())
        };

        // A stylesheet that did not declare a chart colour still has a text colour
        let ink = property("color").unwrap_or_default();
        let token = |name: &str| property(name).unwrap_or_else(|| ink.clone());

        Self {
            up: token("--chart-up"),
            down: token("--chart-down"),
            grid: token("--chart-grid"),
            font_family: property("font-family").unwrap_or_else(|| "monospace".to_string()),
        }
    }
}

// In the frame: read the first time, and again after set_look() dropped it — see 4)
if engine.palette.is_none() {
    engine.palette = Some(Palette::read(&window, &canvas));
}
```

```rust
// ❌ WRONG — a canvas does not resolve it
ctx.set_stroke_style_str("var(--ok)");

// ❌ WRONG — a colour spelled in Rust: the legend and the theme go one way, the canvas another
ctx.set_stroke_style_str("#16a34a");
```

## 8) Everything the frame DECIDES lives in a model with no framework in it — with tests

Which items are on screen, where each one stands, the scales and the ticks, what a wheel or a drag does to the view. A canvas cannot be unit-tested, so none of this is decided in `paint.rs`: the model answers, the frame turns the answers into canvas calls.

```rust
// ✅ CORRECT — client-common/src/chart_model.rs: no dioxus, no web-sys, no clock
pub struct ChartModel {
    series: Option<CandleSeries>,
    view: View,                  // the candle width (the zoom) and the scroll
    canvas: (f64, f64),          // CSS pixels
    pointer: Option<Pointer>,
}

impl ChartModel {
    /// A wheel over the canvas. `true` = it was the chart's
    pub fn wheel(&mut self, dx: f64, dy: f64, x: f64) -> bool { /* ... */ }

    /// Which candles are on screen, and where
    pub fn slice(&self) -> VisibleSlice { /* visible_slice(...) */ }
}

/// The candles visible at `scroll`
pub fn visible_slice(total: usize, chart_width: f64, slot: f64, scroll: f64) -> VisibleSlice { /* ... */ }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_series_shows_every_candle() {
        let slice = visible_slice(10, 900.0, 9.0, 0.0);

        assert_eq!((slice.start, slice.end), (0, 10));
    }
}
```

```rust
// ✅ CORRECT — paint.rs: asks, and draws
let slice = model.slice();

for index in slice.start..slice.end {
    let x = slice.x_center(index, slot);
    // ... canvas calls for candle `index` at `x`
}
```

```rust
// ❌ WRONG — the frame decides what is on screen: nothing can test it
let first = ((scroll - width) / slot).floor() as usize;
for (i, candle) in candles.iter().enumerate().skip(first) {
    let x = i as f64 * slot - scroll;
    // ...
}
```

## The whole wrong picture

```rust
// ❌ WRONG — the scroll and the zoom in use_signal, the canvas painted from use_effect on every write
let mut cs = use_signal(ChartState::default);
let cs_ra = cs.read();

use_effect(move || paint(CANVAS_ID, &cs.read()));

rsx! {
    canvas {
        id: CANVAS_ID,
        width: "{cs_ra.width}",
        height: "{cs_ra.height}",
        onwheel: move |e| cs.write().zoom(&e),
        onpointermove: move |e| cs.write().set_pointer(&e),
    }
}
```

Each wheel notch and each pointer report is a write: a re-render of a tag that did not change, then a paint — as many times as events arrive, not once per screen refresh.

## A parent that changes the picture

A toolbar outside the canvas component notifies it through `NotifyChildComponent` (dioxus-design-patterns §14). `on_notify` hands the change to the engine, and the engine asks for a frame — nothing is painted inside the notification, and nothing is drawn from `cs.read()`.

## Cargo

`wasm-bindgen`, `wasm-bindgen-futures` and `web-sys` with the features `Window`, `Document`, `Element`, `HtmlCanvasElement`, `CanvasRenderingContext2d`, `CssStyleDeclaration` (the colours), `FontFaceSet` (`document.fonts`), `TextMetrics` (measuring a label).

## Checklist

1. Nothing the picture is drawn from is in a signal — `Rc<RefCell<Engine>>` from `use_hook`
2. The canvas handlers call the engine and write to no signal
3. Every change goes through `request_frame()` — one `requestAnimationFrame`, guarded by `frame_pending`
4. `ComponentState` holds only what the markup shows — the pressed button, `DataState<()>` of the read
5. The frame touches no signal — the localization and the theme are handed over from `use_effect`, `use_drop` raises `dropped`
6. No `width` / `height` on the `canvas` in `rsx!` — the frame sets CSS px × `devicePixelRatio`, only when it differs
7. `document.fonts.load(font)`, then one more frame
8. The colours are CSS aliases read with `getComputedStyle`, dropped when the theme changes
9. What the frame decides is in a model with no framework in it, and it is tested
