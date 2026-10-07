# Canvas with WebGL — two layers, a context that can be lost, a frame that stays opaque

Read this when the picture is drawn by WebGL. It is built on the topic `canvas` of **dioxus-design-patterns**: the engine, its single frame, the markup-only state and the model are as written there, and `canvas N)` below is a section of that topic.

WebGL draws triangles, fast, and nothing else: no text, no dashes. And it is lent by the browser, which can take it back. This is what that changes.

## 1) Two canvases — the mesh below, the text above

Labels, plates and dashed lines are drawn on a 2D canvas laid over the WebGL one. The box around them hears the events and is what gets measured; the canvases only show.

```rust
// ✅ CORRECT — render.rs
rsx! {
    div {
        id: "{box_id}",
        class: "chart__box",
        // onresize, onwheel, ... — the handlers of `canvas` 1), on the box
        canvas { id: "{gl_id}" }
        canvas { id: "{text_id}", style: "pointer-events: none;" }
    }
}
```

```css
.chart__box {
  position: relative;
  overflow: hidden;
}

.chart__box canvas {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
}
```

- **The upper canvas takes no pointer events.** Every event then hits ONE element, and `element_coordinates()` are always counted from the same corner (topic `canvas-input` 1).
- **Both bitmaps are sized by the same frame** (`canvas` 5), and the text layer is cleared at the start of every frame like any 2D canvas.
- **What has to cover what is drawn on ONE layer.** Everything on the text layer lies over everything in the mesh. A plate drawn in the mesh under a label drawn as text cannot hide the text of the label behind it — that text is on the layer above, and shows through. A label and its plate go on the text layer together.

## 2) A canvas gives out ONE kind of context, for life

After `get_context("webgl2")` the same canvas answers `None` to `get_context("2d")` — and the other way round. The picture drawn without WebGL (3) therefore cannot go on the WebGL canvas: it goes, whole, on the text canvas — its background included, because the canvas below is empty.

```rust
// ❌ WRONG — `None`: this canvas has been a WebGL canvas since its first frame
let ctx = gl_canvas.get_context("2d");
```

## 3) WebGL may not be there, and may go away — keep a 2D painter of the same picture

`get_context("webgl2")` fails on some machines. A context that was given is taken away when the page holds too many of them, or when the driver resets. A lost context **accepts every call and draws nothing** — and says nothing: the mesh vanishes, the text layer stays.

So there are two painters and ONE model. Both are handed the same answers of the model (`canvas` 8): the slice, the scales, where each thing stands. That is what makes the second painter affordable — and why nothing may be decided inside a painter: decided twice, it drifts.

```rust
// ✅ CORRECT — paint.rs: which painter draws this frame
if engine.gl.is_none() && !engine.gl_gone {
    match GlRenderer::new(&gl_canvas) {
        Some(gl) => {
            engine.gl = Some(gl);

            // The frame that would notice a lost context is the next one — on a quiet canvas,
            // minutes away. So the loss itself asks for it
            let canvas_id = engine.canvas_id.clone();
            let on_lost = Closure::<dyn FnMut()>::new(move || request_frame_for(&canvas_id));
            let _ = gl_canvas
                .add_event_listener_with_callback("webglcontextlost", on_lost.as_ref().unchecked_ref());

            // Kept by the engine: it lives exactly as long as the chart does
            engine.on_context_lost = Some(on_lost);
        }
        // No WebGL2 here: 2D from the first frame
        None => engine.gl_gone = true,
    }
}

// Taken away. It does not come back by itself — this canvas is drawn in 2D from here on
if engine.gl.as_ref().is_some_and(|gl| gl.is_context_lost()) {
    engine.gl = None;
    engine.gl_gone = true;
}

match engine.gl.as_ref() {
    Some(gl) => paint_gl(gl, &text_ctx, &engine.model),
    None => paint_2d(&text_ctx, &engine.model),
}
```

```rust
// ❌ WRONG — the listener holds the engine's own handle, and the engine holds the listener:
// neither is ever freed
let chart = self.clone();
let on_lost = Closure::<dyn FnMut()>::new(move || chart.request_frame());
self.0.borrow_mut().on_context_lost = Some(on_lost);
```

`request_frame_for` finds the engine by its canvas id in the list of live engines — `canvas`, "Several canvases that draw the same thing".

## 4) The frame stays opaque — blend the colour channels only

The browser lays the WebGL canvas over the page by the canvas's own alpha. With one blend function for all four channels, every translucent fill leaves the frame's alpha below 1 — and the page shows through exactly where a zone or a band was drawn: over a white page the same 0.30 fill comes out much fainter than the 2D painter draws it.

```rust
// ✅ CORRECT — the alpha channel keeps what the clear put there: 1.0
gl.enable(WebGl2RenderingContext::BLEND);
gl.blend_func_separate(
    WebGl2RenderingContext::SRC_ALPHA,
    WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
    WebGl2RenderingContext::ONE,
    WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
);
```

```rust
// ❌ WRONG — one function for all four channels: the frame goes translucent under every fill
gl.blend_func(
    WebGl2RenderingContext::SRC_ALPHA,
    WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
);
```

## 5) One frame: the viewport, the clear, one mesh, one draw call

```rust
// ✅ CORRECT — the frame of the WebGL layer. `width` / `height` are CSS pixels,
// `bitmap_width` / `bitmap_height` are the canvas's own — `canvas` 5)
// The drawing buffer follows the size of the canvas; the viewport does not
gl.viewport(0, 0, bitmap_width as i32, bitmap_height as i32);

gl.clear_color(background[0], background[1], background[2], 1.0);
gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

// The whole picture: position and colour per vertex, positions in CSS pixels
let mut mesh = MeshBuilder::new();
paint_mesh(&mut mesh, model);

// CSS pixels → clip space, column by column: x 0..width → -1..1, y 0..height → 1..-1
let (sx, sy) = (2.0 / width as f32, -2.0 / height as f32);
#[rustfmt::skip]
let to_clip: [f32; 16] = [
     sx,  0.0, 0.0, 0.0,
     0.0, sy,  0.0, 0.0,
     0.0, 0.0, 1.0, 0.0,
    -1.0, 1.0, 0.0, 1.0,
];

gl.use_program(Some(&program));
gl.uniform_matrix4fv_with_f32_array(Some(&u_to_clip), false, &to_clip);
gl.bind_vertex_array(Some(&vao));
gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

// SAFETY: a view into wasm memory, handed to buffer_data at once — which copies it
unsafe {
    let view = js_sys::Float32Array::view(mesh.as_slice());
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &view,
        WebGl2RenderingContext::DYNAMIC_DRAW,
    );
}

gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, mesh.vertex_count());
```

- **One mesh, one draw call.** A rectangle is two triangles, a line is a thin rectangle. There is no dash in a mesh: dashed lines go on the text layer.
- **No scissor.** With one draw call there is no state per group of triangles, so what must stay inside an area is cut to it as the mesh is built — the builder is given the clip, and every rectangle and segment added while it is set is cut.
- **The bitmap size only when it differs** (`canvas` 5): assigning a WebGL canvas its size reallocates the drawing buffer. And assigning it the size it already has is not a way to wipe it — Chromium does not clear a WebGL canvas for that. To wipe it outside a frame, change the size for real: `0`, then back.
- **A view into wasm memory is not kept.** `js_sys::Float32Array::view` looks into memory that moves when it grows; `buffer_data` copies out of it before anything else can run.

## 6) Colours are numbers here

`canvas` 7) reads the palette as strings, and what `getComputedStyle` hands back for a token is the colour **as it was written** — `#16a34a`, `rgb(…)`, `oklch(…)`, `color-mix(…)`. A 2D context takes any of those as it is. A mesh takes `[f32; 4]`.

Each colour is turned into numbers ONCE, where the palette is read (`canvas` 7) — and dropped with it when the theme changes. It is never typed a second time, as a table of numbers standing next to the CSS: two tables drift, and the same line comes out one shade from the mesh and another from the 2D painter that stands in for it.

```rust
// ❌ WRONG — the same colour, typed twice: the first edit of one of them makes two colours
pub const UP_CSS: &str = "#1fc78a";
pub const UP: [f32; 4] = [31.0 / 255.0, 199.0 / 255.0, 138.0 / 255.0, 1.0];
```

A parser of `#rrggbb` is enough only while every token the mesh needs is written that way. What takes any colour CSS accepts is a 2D context: fill one pixel of a scratch canvas with it, and read the pixel back.

## 7) Other data without another mount

Every mount of the component is a new WebGL context, and a page may hold only so many: past the limit the browser takes the oldest one away (3). A WebGL canvas is given another series **in place** — `canvas` 3), "The same component, another series" — and never by a `key` that mounts it again.

## Cargo

On top of `canvas`: `js-sys`, and `web-sys` with the features `WebGl2RenderingContext`, `WebGlProgram`, `WebGlShader`, `WebGlBuffer`, `WebGlUniformLocation`, `WebGlVertexArrayObject`.

## Checklist

1. Two canvases in one box: WebGL below, a 2D canvas above with `pointer-events: none`; the handlers and `onresize` are on the box
2. A label and the plate under it are on the same layer
3. The 2D painter draws the whole picture on the text canvas — a canvas gives out one kind of context
4. Two painters, one model: nothing is decided in a painter
5. A renderer that cannot be created, or whose context `is_context_lost()`, hands the canvas to the 2D painter for good
6. `webglcontextlost` asks for a frame — through the list of live engines, not through a handle the engine would hold on itself
7. `blend_func_separate`: the frame's alpha stays 1
8. Every frame: `viewport`, `clear`, one mesh, one draw call; clipping is done while the mesh is built
9. A colour is read once and turned into numbers once — never typed twice
10. Another series is shown in place, not by mounting the canvas again
