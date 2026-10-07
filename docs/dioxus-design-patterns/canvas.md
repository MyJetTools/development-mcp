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

Two things have a topic of their own, each built on this one — `get_dioxus_design_patterns` with that `topic`:

| Topic | Read it when |
| --- | --- |
| `canvas-input` | The canvas is dragged, zoomed by a wheel, touched, or clicked on what is drawn |
| `canvas-webgl` | The picture is drawn by WebGL |

## 1) What the picture is drawn from lives outside signals — in the engine

The data, the scroll, the zoom, the pointer: all of it is in an `Rc<RefCell<Engine>>` created once by `use_hook`. The handlers on the canvas — wheel, pointer, resize — call the engine and **write to no signal** (the one thing a handler may write is the copy of what the markup itself shows — see 3).

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
                    // In pixels, whatever unit the browser sent — see below
                    let (dx, dy) = wheel_px(e.data().delta());
                    // `true` — the wheel was the chart's, and the page must not scroll with it
                    if chart.wheel(dx, dy, e.data().element_coordinates().x) {
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

**A wheel does not always speak pixels.** `delta()` comes in pixels, lines or pages — Firefox reports a mouse wheel in lines — and `strip_units()` throws the unit away: three lines become three pixels, and the same notch moves the picture some thirty times less in one browser than in another. Convert it before the engine sees it:

```rust
// ✅ CORRECT — render.rs
use dioxus::html::geometry::WheelDelta;

/// Wheel travel in pixels. A line is taken as 40px and a page as 800px — what a browser
/// that reports pixels sends for the same notch
fn wheel_px(delta: WheelDelta) -> (f64, f64) {
    let unit_px = match delta {
        WheelDelta::Pixels(_) => 1.0,
        WheelDelta::Lines(_) => 40.0,
        WheelDelta::Pages(_) => 800.0,
    };

    let travel = delta.strip_units();
    (travel.x * unit_px, travel.y * unit_px)
}
```

```rust
// ❌ WRONG — the unit is thrown away: a notch that is about 100 in Chromium arrives as 3 in Firefox
let travel = e.data().delta().strip_units();
chart.wheel(travel.x, travel.y, e.data().element_coordinates().x);
```

**One id per instance.** The frame finds its canvas by id (see 4), so two canvases with one id are one canvas to it: both engines draw on whichever comes first in the document. A constant is right only for a canvas the page holds once. A component that can be on the page twice — two charts side by side, the same chart in a dialog over the page — numbers its instances:

```rust
// ✅ CORRECT — render.rs: a canvas that can be on the page more than once
let canvas_id: Rc<str> = use_hook(|| {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!("chart-{}", NEXT.fetch_add(1, Ordering::Relaxed)).into()
});
let chart = use_hook(|| ChartHandle::new(canvas_id.clone()));

rsx! {
    canvas { id: "{canvas_id}", class: "chart__canvas" }
}
```

The engine then keeps its id as an `Rc<str>` instead of a `&'static str`.

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
- **A frame with nothing to draw still clears.** The series was dropped, the read has not answered yet — the frame draws the empty chart. A frame that returns early leaves the last picture standing: unlike markup, a canvas does not go blank when its data does, and the previous instrument's candles stay under the new title until the new ones land.
- The `borrow_mut()` is released before `request_frame()` is called — a borrow held across it panics with `already borrowed`.
- **A change that forgot its frame is invisible while data keeps arriving** — the next tick repaints and hides it. Try every control with the feed stopped: a closed market, a canvas nobody touches. Whatever then shows up only "on the next tick" is a missing `request_frame()`.

### Tweens

A tween is a value animated towards its target. The frame that moved one asks for the next frame, and stops asking when nothing is moving. It is still ONE loop per canvas — never a second closure: `frame_pending` already refuses a second frame, so a change that arrives mid-tween rides the loop that is running.

A tween is stepped by the **time the browser hands the frame**, never by the count of frames — a 120Hz screen calls twice as often. And a step is capped: a tab in the background gets no frames at all, and comes back with minutes.

```rust
// ✅ CORRECT — engine.rs, in request_frame(): the frame takes its time from the browser
let chart = self.clone();

let frame = Closure::once_into_js(move |now_ms: f64| {
    let moving = {
        let mut engine = chart.0.borrow_mut();
        engine.frame_pending = false;

        if engine.dropped {
            return;
        }

        let moving = engine.model.animate(now_ms);
        paint::paint(&mut engine);
        moving
    };

    // Still on its way — the next frame. The borrow is released by now
    if moving {
        chart.request_frame();
    }
});
```

```rust
// ✅ CORRECT — the model: no clock of its own, the time comes in
const MAX_STEP_MS: f64 = 50.0;

pub struct Tween {
    pub actual: f64,  // what is drawn
    pub target: f64,  // where it is going
    duration_ms: f64, // how long a run takes
    elapsed_ms: f64,
    last_ms: Option<f64>,
}

impl Tween {
    pub fn set_target(&mut self, target: f64) {
        if target != self.target {
            self.target = target;
            self.elapsed_ms = 0.0;
        }
    }

    /// `true` while it has not arrived
    pub fn step(&mut self, now_ms: f64) -> bool {
        // The first step of a run covers no time, and no step covers more than MAX_STEP_MS
        let dt_ms = self.last_ms.map_or(0.0, |last| (now_ms - last).clamp(0.0, MAX_STEP_MS));
        let left_ms = self.duration_ms - self.elapsed_ms;

        if self.actual == self.target || dt_ms >= left_ms {
            self.actual = self.target;
            // The next run starts its own clock — not from the end of this one
            self.last_ms = None;
            return false;
        }

        // The share of what is left that this step is of the time left — a constant speed
        self.actual += (self.target - self.actual) * (dt_ms / left_ms);
        self.elapsed_ms += dt_ms;
        self.last_ms = Some(now_ms);
        true
    }
}

impl ChartModel {
    /// Steps every tween. `true` while any of them is still moving
    pub fn animate(&mut self, now_ms: f64) -> bool {
        // `|`, not `||`: a tween that is not stepped while another one moves stands still
        self.view.scroll.step(now_ms) | self.view.candle_width.step(now_ms)
    }
}
```

```rust
// ❌ WRONG — a fixed share per frame: twice as fast at 120Hz as at 60Hz, and it never arrives,
// so the loop never stops
self.actual += (self.target - self.actual) * 0.2;
```

A handler sets a `target`; what a frame draws is `actual`.

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

### What the markup takes back from the picture

Some markup follows the picture: the cursor of a drag, a "back to the newest" button that appears once the view is scrolled away. It is decided in the engine and stated on both sides, like the pressed button above: the state holds a **copy**, kept to exactly what the markup shows.

```rust
// ✅ CORRECT — state.rs
/// What the markup takes from the picture. Nothing the canvas alone needs: the pointer's
/// position here would re-render the markup on every move
#[derive(Clone, Copy, PartialEq, Default)]
pub struct Shown {
    pub cursor: Cursor,
    pub back_to_newest: bool,
}

pub struct ComponentState {
    size: CandleSize,
    data: DataState<()>,
    shown: Shown,
}
```

```rust
// ✅ CORRECT — actions.rs: called by a canvas handler, after its call into the engine
pub fn sync_shown(mut cs: Signal<ComponentState>, chart: &ChartHandle) {
    let shown = chart.shown();

    // `peek()`: a pointer move that changed nothing writes nothing
    if cs.peek().shown != shown {
        cs.write().set_shown(shown);
    }
}
```

```rust
// render.rs
onpointermove: {
    let chart = chart.clone();
    move |e: Event<PointerData>| {
        let at = e.data().element_coordinates();
        chart.pointer_move(at.x, at.y);
        sync_shown(cs, &chart);
    }
},
```

- It is written by the **handler**, never by the frame: a handler runs in a Dioxus scope, a frame does not (see 4).
- The engine answers it from where the picture is **going** — the targets of its tweens — so it is known when the handler returns, not when the tween ends.
- A value the parent keeps — the zoom a wheel reached, to be stored — goes up the same way: an `EventHandler` prop, called from the handler.

### The same component, another series

The read above is all a canvas needs when it shows one thing for as long as it is mounted. A component that is given another `instrument_id` is not mounted again: `use_hook` ran once, the engine still holds the previous series — and a canvas, unlike markup, goes on showing it. So every render tells the engine what the props name, and the engine answers when that is not the series it holds. That answer — not `RenderState::None` — is what starts the read, and `land()` takes the epoch the read was started under.

```rust
// ✅ CORRECT — render.rs: not a signal — a plain call in the render body
if let Some(epoch) = chart.show(&instrument_id) {
    read(cs, chart.clone(), instrument_id.clone(), epoch);
}
```

```rust
// ✅ CORRECT — engine.rs
impl ChartHandle {
    /// What the props name. `Some(epoch)` — it is not the series held: that one was dropped,
    /// a frame was asked for, and the read of the new one has to land under this epoch
    pub fn show(&self, series_id: &str) -> Option<u64> {
        let epoch = {
            let mut engine = self.0.borrow_mut();

            if engine.series_id.as_deref() == Some(series_id) {
                return None;
            }

            engine.series_id = Some(series_id.to_string());
            engine.epoch += 1;
            engine.model.clear();
            engine.epoch
        };

        // Draws the empty chart: the previous series must not stand until the new one lands
        self.request_frame();
        Some(epoch)
    }

    /// Data arrived. `false` — the props have named another series since: it is dropped
    pub fn land(&self, epoch: u64, series: CandleSeries) -> bool {
        if self.0.borrow().epoch != epoch {
            return false;
        }

        self.change(|model| model.land(series));
        true
    }

    pub fn shows(&self, epoch: u64) -> bool {
        self.0.borrow().epoch == epoch
    }
}
```

```rust
// ✅ CORRECT — actions.rs
pub fn read(mut cs: Signal<ComponentState>, chart: ChartHandle, instrument_id: String, epoch: u64) {
    spawn(async move {
        cs.write().data.set_loading();

        match crate::api::charts::get_candles(instrument_id).await {
            Ok(series) => {
                if chart.land(epoch, series) {
                    cs.write().data.set_loaded(());
                }
            }
            Err(err) => {
                if chart.shows(epoch) {
                    cs.write().data.set_error(err.to_string());
                }
            }
        }
    });
}
```

```rust
// ❌ WRONG — props that can name another series, and a read that lands whatever it brought:
// the answer for the previous instrument, arriving late, becomes the new chart
Ok(series) => {
    cs.write().data.set_loaded(());
    chart.land(series);
}
```

- **Every answer lands under the epoch it was asked with** — the first read, a retry, a page of older data. The engine outlives each of them.
- A read that failed is asked again by whoever shows the error — a button, or the read's own pause and retry — under the same epoch. No later render starts it.
- Mounting the component again instead — `key: "{instrument_id}"` where it is used — also gives the new series an empty canvas, at the price of a new canvas and a new engine for every change. A WebGL canvas is never given its data that way: topic `canvas-webgl`.

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

- **The data and the size arrive in either order.** Nothing that depends on the size is decided when the data lands — a first fit made then is made to a width of zero. It is made by the first frame that has both.
- **The context keeps its state from one frame to the next** — a dash, an alpha, a clip. Clearing does not reset it; assigning the size would, and that is what this section avoids. So whatever a frame sets, it puts back: `save()` / `restore()` around a clip, the dash and the alpha returned to their defaults by the code that changed them.

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

## Several canvases that draw the same thing

A level drawn on one chart belongs to every chart of that instrument on the page. The others are not children of the one that changed, and a signal cannot redraw them: a write re-renders markup, and the picture is not markup. So every live engine is listed, by its canvas id, for exactly as long as its component is mounted — and whoever changes what they share asks each of them for a frame.

```rust
// ✅ CORRECT — engine.rs
thread_local! {
    /// Every chart that is on the page, by its canvas id
    static LIVE: RefCell<HashMap<Rc<str>, ChartHandle>> = RefCell::new(HashMap::new());
}

impl ChartHandle {
    /// On the page from now on. Called once, from `use_hook`
    pub fn register(&self) {
        let canvas_id = self.0.borrow().canvas_id.clone();
        LIVE.with(|live| live.borrow_mut().insert(canvas_id, self.clone()));
    }

    /// The component left the screen
    pub fn dropped(&self) {
        let canvas_id = {
            let mut engine = self.0.borrow_mut();
            engine.dropped = true;
            engine.canvas_id.clone()
        };

        // The list never holds the engine of a canvas that is gone
        LIVE.with(|live| live.borrow_mut().remove(&canvas_id));
    }
}

/// What the charts share has changed — each of them draws it
pub fn request_frame_on_all() {
    // Taken out of the list first: no engine is called while the list is borrowed
    let live: Vec<ChartHandle> = LIVE.with(|live| live.borrow().values().cloned().collect());

    for chart in live {
        chart.request_frame();
    }
}

/// One chart, by its canvas id — for a listener set on the canvas element itself
pub fn request_frame_for(canvas_id: &str) {
    let chart = LIVE.with(|live| live.borrow().get(canvas_id).cloned());

    if let Some(chart) = chart {
        chart.request_frame();
    }
}
```

```rust
// render.rs
use_hook({
    let chart = chart.clone();
    move || chart.register()
});

use_drop({
    let chart = chart.clone();
    move || chart.dropped()
});
```

A listener the engine sets on its own canvas element finds the engine the same way — by the id. A closure the engine keeps must not hold the engine's own handle: the two would keep each other alive after the component is gone (topic `canvas-webgl`, the lost context).

## Cargo

`wasm-bindgen`, `wasm-bindgen-futures` and `web-sys` with the features `Window`, `Document`, `Element`, `HtmlCanvasElement`, `CanvasRenderingContext2d`, `CssStyleDeclaration` (the colours), `FontFaceSet` (`document.fonts`), `TextMetrics` (measuring a label).

## Checklist

1. Nothing the picture is drawn from is in a signal — `Rc<RefCell<Engine>>` from `use_hook`
2. The canvas handlers call the engine and write to no signal — but the copy of what the markup itself shows, and only when it differs (`peek()` first)
3. The canvas id is one per instance when the component can be on the page twice
4. A wheel delta is turned into pixels by its unit before the engine sees it
5. Every change goes through `request_frame()` — one `requestAnimationFrame`, guarded by `frame_pending` — and every control was tried with the feed stopped
6. A frame with nothing to draw still clears the canvas
7. A tween is stepped by the time the browser hands the frame, capped — never by the count of frames
8. `ComponentState` holds only what the markup shows — the pressed button, `DataState<()>` of the read
9. Props that can name another series: the engine is told on every render, and a read lands only under the epoch it was started with
10. The frame touches no signal — the localization and the theme are handed over from `use_effect`, `use_drop` raises `dropped`
11. No `width` / `height` on the `canvas` in `rsx!` — the frame sets CSS px × `devicePixelRatio`, only when it differs; what a frame sets on the context, it puts back
12. `document.fonts.load(font)`, then one more frame
13. The colours are CSS aliases read with `getComputedStyle`, dropped when the theme changes
14. What the frame decides is in a model with no framework in it, and it is tested
15. Canvases that draw the same thing are listed while they are mounted, and each is asked for its own frame
16. Drags, the steps of a wheel, touch and clicks on what is drawn — topic `canvas-input`; WebGL — topic `canvas-webgl`
