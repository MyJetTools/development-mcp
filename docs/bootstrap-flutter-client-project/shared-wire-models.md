# Shared Wire Models — Rust, reaching Dart as generated classes

The twin of the Dioxus client's `rest-api-shared` topic. Every model that crosses the wire is
defined once, in Rust — the `rest-api-shared` crate the server parses with, the WS contracts crate
both ends speak — and **Dart never defines a wire model of its own.** There is no Dart
`rest-api-shared`, no hand-written JSON class, no protobuf parsed in Dart.

## How a wire model reaches Dart

- **Requests are built by the core** from the shared `MyHttpInput` models (topic `rust-core`); Dart
  sends bytes it did not assemble.
- **Answers are read by the core** into the shared response models; what crosses to Dart is either
  the model itself or a view of it:
  - a **mirror** — `#[frb(mirror(Type))]` on a `pub use`d type whose fields are all `pub`; enums
    with struct variants cannot be mirrored;
  - or a **bridge struct with a `From` impl**, when the screen needs less than the model or a
    shape FRB translates better (no `u64`, no references).
- **Frames are parsed by the core** — text with the contracts' JSON parser, binary with prost.

A Dart class of a wire model is the drift `rest-api-shared` exists to prevent: the server renames
a field, the Rust stops compiling, and the Dart copy keeps compiling and silently reads nothing.

## What stays in lib/models/

Only **client view-state** that never crosses the wire, and helpers on the shared models as Dart
`extension`s — the twin of the extension-trait rule (shared types carry no behaviour; helpers
belong to the consumer):

```dart
// lib/models/side_look.dart — presentation only
extension SideLook on Side {
  Color color(MTokens tokens) => switch (this) {
        Side.buy => tokens.sideBuy,
        Side.sell => tokens.sideSell,
      };
}
```

A helper that **decides** — a number, a word, a fallback the user reads (a full name built from
two fields, a label when a name is empty) — is a core function, so that the web and the phone
cannot build it two ways.

## The words

The core's localization is a wire-adjacent contract too: one struct, every language, a field added
for one surface is a field both must fill. It reaches Dart as a generated `Words` class (topic
`bridge`, section 4) — `words.settingsTitle` in Dart is the same string `loc.settings_title` is in
the Dioxus client.
