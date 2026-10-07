# Canvas input — drags, the wheel, touch, and clicks on what is drawn

Read this when a canvas is dragged, zoomed by a wheel, touched, or clicked on what is drawn. It is built on the topic `canvas` of **dioxus-design-patterns**: the engine, its single frame and the model are as written there, and `canvas N)` below is a section of that topic.

What a gesture does to the view is decided in the model (`canvas` 8). A handler tells the engine what happened; the engine tells the model and asks for a frame. Every rule below was a bug first.

## 1) One box hears everything

`element_coordinates()` is `offsetX` / `offsetY`: measured from the corner of the element the event **hit**, not of the element the handler is on. On a lone `<canvas>` the two are the same element. Put anything over the canvas — a button, a popup — and its events bubble into the canvas's handlers with coordinates counted from the button's own corner: a click on the button lands as a click near the top left of the picture.

So markup laid over the canvas stops what is not meant for the picture:

```rust
// ✅ CORRECT — a popup over the canvas: nothing it hears is the canvas's
div {
    class: "chart__popup",
    onmousedown: move |e| e.stop_propagation(),
    onclick: move |e| e.stop_propagation(),
    ondoubleclick: move |e| e.stop_propagation(),
    onwheel: move |e| e.stop_propagation(),
    // ...
}
```

- **A mouse is heard with `onmouse*`, fingers with `ontouch*`** — the two families this topic uses. They are not mixed with `onpointer*` on the same box: a pointer event is sent for a mouse and for a finger alike, and the same move would reach the engine twice. The `onpointermove` of `canvas` 1) is enough for a hover; a canvas that is dragged or touched replaces it with the handlers below.
- **Keys** reach a box only while it holds focus, and a `div` or a `canvas` cannot hold it without `tabindex`: `tabindex: "-1"` — focused by a click, skipped by Tab.

## 2) A drag is a gesture in the model

A press is not a drag yet, a release is not always heard, and a click follows every release. The gesture is a small state machine with no canvas and no event in it — and each row below is one of its tests:

| What happens | What the gesture does |
| --- | --- |
| The button goes down | Arms. Nothing moves: a press is a click until the pointer has travelled a threshold (3px) |
| The pointer crosses the threshold | The drag begins from THAT point — the dead zone already travelled is not applied as a jump |
| The pointer moves on | The view moves by this move's own delta, from where the view is NOW — not by "press point to pointer": data that arrived mid-drag has moved the view, and the drag must not undo it |
| The view is against its edge | Nothing moved, so nothing else may change either — a setting the drag resets stays as it was |
| The pointer leaves with the button held, or passes over a child that swallows the moves | The drag stays armed; the next move it hears starts from where it comes back — the gap is not applied as one jump |
| The button comes up over the canvas | The drag ends. The browser sends `click` right after that `mouseup`: it is swallowed once — and only if the drag moved the view |
| The button comes up somewhere else | The canvas never hears it. Every move says which buttons are held, and a move with none ends the drag — with no click to swallow |

```rust
// ✅ CORRECT — render.rs; each handler takes its own `chart.clone()`, as in `canvas` 1)
use dioxus::html::input_data::MouseButton;

onmousedown: move |e: Event<MouseData>| {
    // The left button only: the others keep the context menu and the autoscroll
    if e.trigger_button() == Some(MouseButton::Primary) {
        let at = e.element_coordinates();
        chart.press(at.x, at.y);
    }
},
onmousemove: move |e: Event<MouseData>| {
    let at = e.element_coordinates();
    // A release that happened off the canvas never reached it — it is noticed here
    let held = e.held_buttons().contains(MouseButton::Primary);
    chart.mouse_move(at.x, at.y, held);
    // The cursor of the drag is markup — `canvas` 3)
    sync_shown(cs, &chart);
},
onmouseup: move |_| {
    chart.release();
    sync_shown(cs, &chart);
},
onmouseleave: move |_| chart.mouse_left(),
onclick: move |e: Event<MouseData>| {
    // The click that follows the release of a drag is the drag's own
    if chart.take_drag_click() {
        return;
    }

    let at = e.element_coordinates();
    chart.click(at.x, at.y);
},
```

```rust
// ✅ CORRECT — the model: the gesture decides, the engine applies what it answers
pub enum DragMove {
    /// Not a drag yet, the move it began on, or the move it came back on — nothing moves
    Idle,
    /// Against the edge: the view did not move, and nothing else is touched
    Held,
    /// The view goes here — clamped already
    To { scroll: f64 },
}

impl Drag {
    pub fn arm(&mut self, x: f64) { /* ... */ }

    /// `view` is the view as it is NOW — the move is added to it
    pub fn on_move(&mut self, x: f64, view: &View) -> DragMove { /* ... */ }

    /// The pointer left with the button held
    pub fn resync(&mut self) { /* ... */ }

    /// `released` — the button came up over the canvas, so a `click` follows.
    /// `false` — a move arrived with no button held
    pub fn end(&mut self, released: bool) { /* ... */ }

    /// The click after a drag that moved the view. Answers `true` once
    pub fn take_click(&mut self) -> bool { /* ... */ }
}
```

```rust
// ❌ WRONG — the view is put where the press was plus the whole way to the pointer: the first
// move jumps by the threshold, and a candle that formed during the drag is dragged back
let scroll = self.scroll_at_press + (x - self.pressed_at);
```

## 3) The wheel — travel is counted, not events

`canvas` 1) turns the delta into pixels. What those pixels do is the model's.

**A setting that moves in steps takes one step per stretch of travel**, not per event. A notch of a mouse wheel is about 100px in one event; a trackpad sends the same distance as dozens of small ones — a step per event runs the zoom to its limit in a fraction of a second.

```rust
// ✅ CORRECT — the model
/// Wheel travel that makes one step — about one notch of a mouse wheel
const WHEEL_STEP_PX: f64 = 100.0;

/// Whole steps travelled since the last one was taken. `left_over` is kept between events
fn take_steps(left_over: &mut f64, delta_px: f64) -> i32 {
    // A turn the other way starts over — it does not first undo what was left of the last one
    if *left_over != 0.0 && left_over.signum() != delta_px.signum() {
        *left_over = 0.0;
    }

    *left_over += delta_px;

    let steps = (*left_over / WHEEL_STEP_PX).trunc();
    *left_over -= steps * WHEEL_STEP_PX;
    steps as i32
}
```

```rust
// ❌ WRONG — a step per event: one swipe on a trackpad is dozens of them
self.zoom_level -= delta_px.signum() as i32;
```

**The unit does not say which device it was.** "Lines is a mouse, pixels is a trackpad" holds in Firefox only — Chromium and WebKit report a mouse wheel in pixels too, and a branch for the mouse never runs there. A wheel and two fingers on a trackpad do the same thing.

```rust
// ❌ WRONG — in Chromium and WebKit this is `false` for a mouse as well
let is_mouse = matches!(e.data().delta(), WheelDelta::Lines(_));
```

**What the canvas does not take stays the page's.** The model answers whether the wheel was its own, and only then is `prevent_default()` called (`canvas` 1) — a canvas that swallows every wheel traps the page's scroll under the pointer.

## 4) Touch

**Who owns a gesture is decided in CSS — `touch-action` on the box — never by `prevent_default()`.** A `touchmove` listener the browser has marked passive ignores the call, and which listeners those are is not the page's to decide.

```css
/* ✅ CORRECT — the canvas owns every gesture on it: one finger drags, two fingers zoom */
.chart__box {
  touch-action: none;
  /* A long press otherwise raises the text-selection callout over the picture */
  user-select: none;
  -webkit-user-select: none;
  -webkit-touch-callout: none;
}

/* ✅ CORRECT — a band inside a page that scrolls: the vertical swipe stays the page's.
   A canvas that swallowed it would trap the thumb — what lies below it could not be reached */
.chart__box.band {
  touch-action: pan-y;
  /* In px, not `dvh`: the browser's bars collapse while the page scrolls, and a height that
     follows them is a resize — and a full redraw — on every scroll frame */
  height: 320px;
}
```

```rust
// ❌ WRONG — ignored by a passive listener: the page scrolls under the finger anyway
ontouchmove: move |e: Event<TouchData>| {
    e.prevent_default();
    // ...
},
```

**Fingers call what the mouse calls.** One finger is the drag of a held button; two fingers are the zoom the wheel makes. The handlers only recognise the gesture — nothing about panning or zooming is written a second time, so a phone and a desktop cannot scroll to different places.

```rust
// ✅ CORRECT — render.rs
/// Client coordinates → coordinates inside the box. A touch carries nothing else: it has no
/// offset from an element, and with two fingers there is no single element to take one from
fn in_box(box_id: &str, client_x: f64, client_y: f64) -> Option<(f64, f64)> {
    let rect = web_sys::window()?
        .document()?
        .get_element_by_id(box_id)?
        .get_bounding_client_rect();

    Some((client_x - rect.left(), client_y - rect.top()))
}

/// How far apart the first two fingers are
fn spread(fingers: &[TouchPoint]) -> f64 {
    let (a, b) = (fingers[0].client_coordinates(), fingers[1].client_coordinates());
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// What the fingers are doing. Kept next to the handlers — `use_hook(|| Rc::new(Cell::new(..)))` —
/// and outside every signal: no markup shows it
#[derive(Clone, Copy, PartialEq)]
enum Touching {
    None,
    /// One finger: the drag of a held mouse button
    Drag,
    /// Two fingers. The distance between them at the LAST zoom step — not at the start
    Pinch(f64),
}

/// One zoom level per this much change of the distance between the fingers
const PINCH_STEP: f64 = 1.18;
```

```rust
// ✅ CORRECT — render.rs; each handler takes its own clones of `chart`, `touching` and `box_id`
ontouchstart: move |e: Event<TouchData>| {
    let fingers = e.touches();

    match fingers.len() {
        1 => {
            let at = fingers[0].client_coordinates();
            if let Some((x, y)) = in_box(&box_id, at.x, at.y) {
                chart.press(x, y);
                touching.set(Touching::Drag);
            }
        }
        count if count >= 2 => {
            // A second finger ends the drag where it is: the picture must not slide while it scales
            chart.touch_ended();
            touching.set(Touching::Pinch(spread(&fingers)));
        }
        _ => {}
    }
},
ontouchmove: move |e: Event<TouchData>| {
    let fingers = e.touches();

    match (touching.get(), fingers.len()) {
        (Touching::Drag, 1) => {
            let at = fingers[0].client_coordinates();
            if let Some((x, y)) = in_box(&box_id, at.x, at.y) {
                // The drag, and nothing else — a finger is not a hover
                chart.touch_move(x, y);
            }
        }
        (Touching::Pinch(last), count) if count >= 2 => {
            let now = spread(&fingers);
            if last <= 0.0 || now <= 0.0 {
                return;
            }

            // Apart — a level in, together — a level out
            let levels = if now / last >= PINCH_STEP {
                1
            } else if now / last <= 1.0 / PINCH_STEP {
                -1
            } else {
                return;
            };

            // The next level is measured from here
            touching.set(Touching::Pinch(now));
            chart.zoom_by(levels);
        }
        _ => {}
    }
},
ontouchend: move |_| {
    chart.touch_ended();
    touching.set(Touching::None);
},
// No `touchend` comes for a gesture that was taken away
ontouchcancel: move |_| {
    chart.touch_ended();
    touching.set(Touching::None);
},
```

- **`ontouchcancel` ends the gesture exactly as `ontouchend` does.** The OS takes a gesture away — a system swipe, an incoming call — and so does the browser, for the scroll that `pan-y` leaves it. No `touchend` follows, and a drag left armed makes the next tap jump.
- **One finger or two — never both.** A pinch that began as a drag ends the drag first. Dragging and scaling at once is what makes a picture fight the hand.
- **A pinch over a ladder of zoom levels re-anchors at every step**: the distance is compared with the one at the last step. Compared with the one at the start, the first level is easy and the tenth impossible.
- **A finger is not a hover.** It carries no crosshair along, and when it lifts there is no "leave" to take one away. `touch_move` runs the drag and nothing else.
- **No click follows a touch that moved** — so the click a drag swallows (2) is armed by a mouse release only. Armed by a lifted finger, it swallows the NEXT tap.
- **A tap sends the mouse events too** — `mousemove`, `mousedown`, `mouseup`, `click`, at the point of the tap — and no `mouseleave` after them. Whatever a hover shows appears under the tap and stays. A surface that is touched does not show it, until it has a gesture of its own for it.

## 5) A click on what is drawn

There are no elements to click — only pixels. A click is tested against **what the last frame drew**:

- **What the model can answer, the model answers** — with the values the frame was drawn with: a tween's `actual`, never its `target` (`canvas` 2). The same functions that told the frame where a thing stands tell the handler what stands there.
- **What only the canvas could measure — the width of a label — the frame hands back.** `paint()` returns the boxes it drew, the engine keeps them, and the handler tests against those.

```rust
// ✅ CORRECT — paint.rs: the frame reports what a click may be aimed at
pub struct Painted {
    /// The label of each level, in the order of `model.levels()` — as wide as its text measured
    pub level_labels: Vec<Rect>,
}

pub fn paint(engine: &mut Engine) -> Painted { /* ... */ }
```

```rust
// ✅ CORRECT — engine.rs
// in the frame
let painted = paint::paint(&mut engine);
engine.painted = painted;

impl ChartHandle {
    /// What stands at this point of the picture
    pub fn hit(&self, x: f64, y: f64) -> Option<Hit> {
        let engine = self.0.borrow();

        // Labels lie over everything else, so they are asked first
        if let Some(index) = engine.painted.level_labels.iter().position(|label| label.contains(x, y)) {
            return Some(Hit::LevelLabel(index));
        }

        engine.model.hit(x, y)
    }
}
```

```rust
// ❌ WRONG — the label's box is worked out again in the handler, from a width the frame
// measured and the handler guesses: the click lands beside the label
let label = Rect::new(x_right - 60.0, y - 8.0, 60.0, 16.0);
```

A thin line is hit within a few pixels of it (6px), not on its one pixel.

## 6) The cursor is markup

`grabbing` while a drag is on, `crosshair` while a drawing tool is armed: the cursor is a class on the box, taken from the copy of what the markup shows (`canvas` 3) and written after every handler that can change it. `cursor` is inherited, so the canvases inside the box follow.

```css
.chart__box.is-grabbing { cursor: grabbing; }
.chart__box.is-drawing  { cursor: crosshair; }
```

## Cargo

On top of `canvas`: `web-sys` with the feature `DomRect` (`get_bounding_client_rect`).

## Checklist

1. Markup over the canvas stops the events it does not mean for the picture — `element_coordinates()` are counted from the element that was hit
2. `onmouse*` and `ontouch*`, never mixed with `onpointer*` on the same box
3. The drag is a gesture in the model, with a test for the threshold, the first move past it, the edge, the way out and back, the release, and the release that was not heard
4. The click after a drag is swallowed once — only after a mouse release, and only if the view moved
5. A stepped setting moves a step per 100px of wheel travel, the remainder kept; the device is not guessed from the unit
6. `prevent_default()` on a wheel only when the model took it
7. `touch-action` on the box decides who owns a touch — `none`, or `pan-y` for a band in a page that scrolls
8. `ontouchcancel` does what `ontouchend` does
9. A touch is turned into box coordinates by the box's `get_bounding_client_rect()`
10. One finger is the mouse's drag, two fingers are the wheel's zoom — through the same methods of the engine
11. A click is tested against what the last frame drew; a box the canvas measured comes back from `paint()`
12. The cursor is a class on the box
