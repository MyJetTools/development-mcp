# CI — the toolchain, the guards, versions, flavors, release

The twin of the Dioxus client's `ci` topic. A Dioxus client ships as a Docker image of static
files; a phone app ships as an `.ipa` and an `.aab` to the stores — no image, no stack. What carries
over is the habit: **the toolchain is pinned, the build refuses another one, and the guards run
before the commit.** The pinned versions: topic `project-files`.

## Commands

```bash
flutter pub get
flutter_rust_bridge_codegen generate           # after any change in rust/src/api
dart run tool/gen_tokens.dart                  # after any change in the CSS tokens

flutter run                                    # a device or a simulator; native assets build rust/
                                               # a Rust change needs a full re-run

./build.sh appbundle                           # Android — checks the Flutter version, then flutter build appbundle --release
./build.sh ipa                                 # iOS — then flutter build ipa --release --export-options-plist=ios/ExportOptions.plist
```

## The guards — before every commit

The twins of what `build.rs` enforces in a Dioxus client. All of them, every time; a red one is
fixed, never skipped.

| Guard | Command | Catches |
|---|---|---|
| analysis | `flutter analyze --fatal-infos --fatal-warnings` | any warning — the zero-warnings policy |
| format | `dart format --output=none --set-exit-if-changed .` | unformatted code |
| Dart tests | `flutter test` | state classes and the guard tests below |
| bindings current | `flutter_rust_bridge_codegen generate && git diff --exit-code -- lib/bridge rust/src/frb_generated.rs` | `rust/src/api` changed, bindings not regenerated |
| tokens current | `test/tokens_are_current_test.dart` | the CSS tokens changed, `tokens.g.dart` not regenerated |
| words current | `cd rust && cargo test words_are_current` | the core's localization changed, `api/words.rs` not regenerated |
| routes | `test/routes_guard_test.dart` | a route the vocabulary does not have — and a vocabulary path the app does not declare |
| label width | `test/chart_label_width_test.dart` | the chart's label font no longer matches the engine's character width |
| the bridge | `cd rust && cargo test && cargo clippy -- -D warnings` | the bridge crate |
| the core, natively | `cargo test --features chart && cargo clippy --features chart -- -D warnings` in the core | a broken rule; I/O or a browser call left in the pure core — without `web` those crates do not exist |
| the core, for the phone | `cargo build --features chart --target aarch64-linux-android` (and `aarch64-apple-ios`) | something in the core that does not build for a phone |
| the web client | `dx build` | a core change that broke the Dioxus client |

The last row is the price of sharing: a change to the core is a change to every surface, and it is
built for all of them before it is committed.

## Versions and tags

- `pubspec.yaml`: `version: 0.1.0+1` — the version, and the build number after `+`. The build
  number only ever grows: the stores refuse a number they have seen.
- The tag: `<app>-<version>`, the monorepo's `<service>-<version>`, pushed after the upload
  succeeded.

## Flavors — one per brand

A store app has no address bar, so the host it talks to is built in. One flavor per brand.

`flavors/<brand>.json`, read with `--dart-define-from-file`:

```json
{
  "ORIGIN": "https://app.example.com",
  "USER_AGENT_PRODUCT": "ProductName/1",
  "APP_NAME": "Product"
}
```

- **The flavor carries the origin and the User-Agent's product token** — used by
  `native/http.dart` and `ws/socket.dart` only (topic `api-calls`).
- The bundle id, the display name and the icon per brand are an Android `productFlavor` and an iOS
  scheme. A second brand is a second store listing.
- **A start-up gate still runs** against the flavor's host, when the web client has one: the
  server decides whether that host is served, exactly as for the web.

## Release — first by hand, then CI

### Phase 1 — signed locally, uploaded by hand

- **Signing material never enters the repo**: the Android upload keystore and its `key.properties`
  (gitignored), the Apple certificates and profiles — in the machine's keychain and the team's
  password manager.
- Android: Play App Signing with an upload key; Play Console, internal testing first.
- iOS: upload with Transporter or Xcode's Organizer; TestFlight first.
- Production after a pass on both platforms: sign-in, the confirmation flow, the feed through a
  background → foreground, an order placed and refused, the chart's gestures.

### Phase 2 — CI

GitHub Actions: Android on Ubuntu, iOS on macOS; Flutter pinned, Java 17, the Rust targets of
`rust-toolchain.toml`, the guards, then the builds; uploads through fastlane or the stores' APIs
with secrets from the repository's settings.

- **Caching**: the native-assets backend builds Rust into the hook's output directory — a
  `rust/target` cache does not help; cache that directory, or use `sccache`.
- No pre-baked builder image: that is the answer for services built on Ubuntu, and this build
  needs Xcode.

## Store and server items before the first release

- **Deep links** — the `/.well-known/` files on each brand's host (topic `deep-links`).
- **Android's Auto Backup** excludes the secure storage (topic `local-storage`).
- **16 KB pages** — NDK r28+ and AGP 8.5.1+, or Google Play refuses the bundle.
- **Edge-to-edge** — Android draws edge-to-edge by default since Flutter 3.27; the safe-area rules
  (`get_flutter_design_patterns`, topic `theme`) are what make that correct.
- **The iOS socket's User-Agent** — checked on a device when the server checks it (topic
  `websocket`).
