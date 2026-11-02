# WASM Optimization — Dioxus web bundles

How to ship the smallest `.wasm` from a Dioxus project: a client-side app
(`dioxus/web`) or the web half of a fullstack app. Every number here is
measured — dx 0.7.10, Rust 1.99.1, binaryen 129, the settings-service UI,
16.10.2026, and from section 7 on the bundles of mt-client, mt-client-mobile and
mt-admin, 01–02.11.2026 — and every dx behaviour is read from the dx source, not
guessed.

**When:** before every production release of a Dioxus project, and whenever its release build is
created or changed. A project that fails the checklist is fixed before it is released — the
release guide (`get_release_guide`) has this as a step of the release.

## The checklist

1. `dx build --release --web --debug-symbols false` — in the local build script
   AND in CI. Without the flag the bundle keeps its debug info.
2. The profile is `[profile.wasm-release]`, not `[profile.release]`:
   `opt-level = "z"`, `lto = true`, `codegen-units = 1`.
3. No `strip` and no `panic = "abort"` in any profile.
4. Build on the latest stable toolchain.
5. The build script fails when wasm-opt fails, and replaces the served bundle
   only after a successful build.
6. Measure what goes over the wire: each `.wasm` under brotli quality 11 with a
   16 MB window, before and after (section 7).
7. The host compresses with brotli: `COMPRESSION_ALGORITHM=br`,
   `MAX_COMPRESSION=true`.
8. For a bundle over 1 MB: go through section 8.
9. With `--wasm-split`, count every module, not only the main one (section 10).

## What each step gives

The settings-service UI, the same code built six ways:

| Build | `.wasm`, bytes | gzip -9, bytes |
| --- | --- | --- |
| `dx build --release --web`, no profile in `Cargo.toml` | 2 998 751 | 822 289 |
| + `[profile.release]` lto, codegen-units 1 (debug symbols still on) | 1 709 442 | 616 843 |
| `--debug-symbols false`, no profile | 1 373 196 | 560 934 |
| `--debug-symbols false` + `wasm-release`: `opt-level = 3` | 1 639 568 | 615 614 |
| `--debug-symbols false` + `wasm-release`: `opt-level = "s"` | 1 240 164 | 502 941 |
| **`--debug-symbols false` + `wasm-release`: `opt-level = "z"`** | **1 097 932** | **460 368** |

The `3` and `"s"` lines were built with `panic = "abort"` in the profile as well;
it moves the module by less than 0.1% (see section 2).

From the first line to the last: −63% of the module, −44% of its gzip.

## 1. `--debug-symbols false`

`dx build --release` keeps debug symbols: in dx 0.7 the `--debug-symbols` flag
defaults to `true` even for a release build. Then wasm-bindgen keeps the debug
info and dx runs wasm-opt with `--debuginfo`, so the DWARF sections stay in the
shipped module: 1 709 442 bytes against 1 240 164 for the same build without
them, +38%.

```bash
dx build --release --web --debug-symbols false
```

With the flag wasm-bindgen drops the debug info and wasm-opt gets
`--strip-debug`. The flag applies to the web build only, so it is safe for a
fullstack project too.

`dx bundle` takes the same flag. **Known gap:** the release workflow that
`ci-utils` 0.1.3 generates for a fullstack service (`as_dioxus_fullstack_service`)
runs `dx bundle --web --release` without it — a fullstack bundle built by that
workflow keeps its debug info until the template in `ci-utils` is fixed.

`dx build --wasm-split` splits a bundle into a module per route. It needs the
cargo feature `dioxus/wasm-split` and a special link flag, see
`get_dioxus_client_side_bootstrap_guide`, topic `routing`.

What the main module must keep, dialogs as a module and fetching modules ahead:
section 10.

## 2. `[profile.wasm-release]`, not `[profile.release]`

dx builds every platform in its own cargo profile: `wasm-release` for the web
bundle, `server-release` for the server of a fullstack app. When `Cargo.toml`
has no `[profile.wasm-release]`, dx makes one on the fly: it inherits `release`
and sets `opt-level = "s"`. So:

- `opt-level` written in `[profile.release]` never reaches the wasm — dx
  replaces it with `"s"`. (`opt-level = 3` there is a no-op for the bundle.)
- everything else in `[profile.release]` reaches the wasm AND, in a fullstack
  project, the server binary too.

Declare the web profile yourself. dx then takes it as it is:

```toml
[profile.wasm-release]
inherits = "release"
opt-level = "z"
lto = true
codegen-units = 1
```

Leave `[profile.release]` alone: in a fullstack project it is the profile of the
server.

**`opt-level`:**
- `"z"` — the smallest module; the default for admin consoles and forms.
- `"s"` — what dx uses when the profile is not declared: +13% here.
- `3` — +49% here. Only when a profiler shows the page is CPU-bound in Rust code.
  A chart is not a reason by itself: on trading terminals at 100 frames a second,
  the standard library from its sources cost the desktop +1.4% of main-thread
  time, and `"s"` → `"z"` with that library cost the phone +3.7% (script time +7%
  and +8%). `"z"` against `"s"`: the desktop's file −16%, the phone's −15%.

**`lto = true` + `codegen-units = 1`:** 1 373 196 → 1 240 164 bytes at the same
`opt-level`. The cost is a slower release build.

**Not `panic = "abort"`:** `wasm32-unknown-unknown` aborts on panic already
(its target spec says `"panic-strategy": "abort"`), so the bundle gains nothing —
with it the `"z"` module was even 910 bytes larger. Put into `[profile.release]`
of a fullstack project, it makes any panic in a server request handler kill the
whole server process.

**Not `strip`:** dx turns cargo's strip off during the build and strips the
module afterwards with the toolchain's `rust-objcopy`. On macOS that fails:
`dyld: Library not loaded: @rpath/libLLVM.dylib` and `Failed to strip binary` —
dx points the tool at the library with `LD_LIBRARY_PATH`, which macOS ignores.
With Rust 1.99.0 (LLVM 23) `--strip-all` also removed the
`__wasm_bindgen_unstable` section and broke wasm-bindgen. `--debug-symbols false`
removes the same debug info without running `rust-objcopy` at all.

## 3. wasm-opt: dx runs it, keep `-Oz`

A release build runs binaryen's wasm-opt on the module after wasm-bindgen. dx
downloads it itself (binaryen 129, into `~/.dx/tools/`) and uses `-Oz` unless
`Dioxus.toml` sets `[web.wasm_opt] level`. Do not set a lower level.

**A failed wasm-opt does not fail dx.** It logs one line —
`ERROR wasm-opt failed with status code …` — and ships the module unoptimised.
The build script has to look for that line (section 5).

**Memory.** `-Oz` on a large module can need far more RAM than a CI runner has:
in my-prop-trading the trader cabinet needs 19+ GB, and nearly all of it is one
pass, `coalesce-locals`. A GitHub runner has 7.8 GB, so wasm-opt is killed there
and dx ships the module unoptimised. On such a runner skip that pass — 0.53 GB,
and the result is within ~1% of the full `-Oz` (measured there):

```toml
# Dioxus.toml - only where wasm-opt runs out of memory
[web.wasm_opt]
extra_features = ["--skip-pass=coalesce-locals"]
```

## 4. The toolchain: the latest stable

Build the bundle with the latest stable Rust. Update it and pin the build to it,
so a toolchain override, a `rust-toolchain` file or `RUSTUP_TOOLCHAIN` left in
the shell can not build it with an old one:

```bash
rustup update stable --no-self-update
rustup target add wasm32-unknown-unknown --toolchain stable
export RUSTUP_TOOLCHAIN=stable
```

dx 0.7.10 builds on stable 1.99.1. The 1.98.0 pin some projects used was a
workaround for the `strip` failure of section 2, and a project without `strip`
does not need it.

## 5. The build script

The local release build of a client-side project, with every rule above
(`dx build` copies over its output folder instead of cleaning it, so the folder
is removed first):

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

PKG=your-project-name
DX_OUT=./target/dx/$PKG/release/web/public
WWWROOT=./wwwroot
LOG="$(mktemp)"
trap 'rm -f "$LOG"' EXIT

rustup update stable --no-self-update >/dev/null
rustup target add wasm32-unknown-unknown --toolchain stable >/dev/null
export RUSTUP_TOOLCHAIN=stable
echo "$(rustc --version), $(dx --version)"

# dx copies over its output folder: a file removed from public/ would stay there
rm -rf "$DX_OUT"
dx build --release --web --debug-symbols false 2>&1 | tee "$LOG"

# a failed wasm-opt leaves dx successful and the module unoptimised
if grep -q "wasm-opt failed" "$LOG"; then
    echo "wasm-opt FAILED - the bundle is unoptimised, $WWWROOT is left as it was" >&2
    exit 1
fi

# the served bundle is replaced only by one that built
rm -rf "$WWWROOT"
mkdir -p "$WWWROOT"
cp -r "$DX_OUT"/* "$WWWROOT/"

WASM="$(ls "$WWWROOT"/assets/*.wasm)"
echo "wasm $(wc -c < "$WASM" | tr -d ' ') bytes, gzip $(gzip -9 -c "$WASM" | wc -c | tr -d ' ') bytes"
```

A CI step does the same: the flag on the `dx build`, and the `grep` after it.
Give the step `shell: bash` — a step without it runs without `pipefail`, and a
failing `dx build | tee` would pass.

## 6. Verifying a build

- **Size.** The script prints the `.wasm` and its gzip; compare with the bundle
  it replaces.
- **No debug info.** `dx build --release --web --debug-symbols false --verbose`
  prints `Running wasm-opt: … -Oz … --strip-debug`. `--debuginfo` there means
  the flag did not reach dx.
- **Fully optimised.** A second wasm-opt pass over the shipped module gains
  almost nothing — 0.2% on a properly built one, 28% on one that kept its debug
  info:

  ```bash
  ~/.dx/tools/binaryen-129/bin/wasm-opt shipped.wasm -Oz --strip-debug \
      --enable-reference-types --enable-bulk-memory --enable-mutable-globals \
      --enable-nontrapping-float-to-int --enable-threads -o /tmp/again.wasm
  ls -l shipped.wasm /tmp/again.wasm
  ```

- **It still works.** Open every page of the release build in a browser and read
  the console. A panic in a release build prints its text only when
  `dioxus_utils::set_panic_hook()` is the first line of `main()`.

## 7. Size over the wire

`web-app-host` compresses a file once, at its first request, and keeps it. Give
it the strongest setting:

```yaml
environment:
- COMPRESSION_ALGORITHM=br
- MAX_COMPRESSION=true
deploy:
  resources:
    limits:
      memory: 512Mb
```

That is brotli quality 11 with a 16 MB window; without the two variables it is
zstd 11. A 5 MB wasm: 1.40 MB → 1.17 MB. The first request of a wasm after a
start takes ~1.5 s a megabyte and ~110 MB of memory, hence the limit. Ask for
the files once after a deploy, so that no visitor pays for it.

Judge every change by this number, not by the file and not by gzip. They
disagree: `--outlining` makes the file smaller and the wire 35 KB larger. A
20-line Rust binary over the `brotli` crate (quality 11, `lgwin` 24) matched
what the host served to 2 bytes.

## 8. Beyond the profile

Over the wire, KB. The steps were measured one after another, so they do not add
up exactly.

| Step | Desktop terminal | Phone terminal | Admin, main module |
| --- | ---: | ---: | ---: |
| Start | 690 | 631 | 567 |
| `opt-level = "z"`, no `tracing` in release | done before | −67 | |
| One static instead of a struct built in every caller | | −32 | |
| Standard library from its sources | −30 | −23 | |
| `flate2` without crc32fast's tables | −20 | −20 | |
| wasm-opt flags | −2.5 | −3 | |
| No column in panic locations | −2.4 | | |
| The settings above, together | | | −60 |
| Dialogs in a module of their own | | | −250 |
| No `chrono` | | | −4.5 |
| Result | 635 | 484 | 251 |

**No `tracing` in release:**

```toml
# Only for the feature: a release build carries no journal of Dioxus.
tracing = { version = "*", features = ["release_max_level_off"] }
```

**The standard library from its sources.** The prebuilt one is built for speed:
a sort costs 13 KB a type. `optimize_for_size` took sorts from 95 KB to 9, and
`backtrace-trace-only` takes the symbol demangler out. This runs on the stable
compiler; the variables pass through dx and reach only this shell:

```bash
rustup component add rust-src   # once; the first build needs the network for std's own dependencies

export RUSTC_BOOTSTRAP=1
export CARGO_UNSTABLE_BUILD_STD=std,panic_abort
export CARGO_UNSTABLE_BUILD_STD_FEATURES=optimize_for_size,backtrace-trace-only

sysroot="$(rustc --print sysroot)"
WASM_RUSTFLAGS="-Zlocation-detail=file,line"
for prefix in "$HOME"/.cargo/registry/src/index.crates.io-* "$HOME/.cargo/git/checkouts" "$PWD" \
    "$sysroot/lib/rustlib/src/rust/library"; do
    WASM_RUSTFLAGS="$WASM_RUSTFLAGS --remap-path-prefix=$prefix/="
done

RUSTFLAGS="$WASM_RUSTFLAGS" dx build --release --web --debug-symbols false
```

`-Zlocation-detail=file,line`: a panic still says the file and the line, only
the column goes. `--remap-path-prefix`: 15 KB of the file, almost nothing over
the wire, and the home directory of whoever built it is not shipped. The build
script stops, naming the `rustup` command, when
`$sysroot/lib/rustlib/src/rust/library/std` is not there.

**wasm-opt flags:**

```toml
[web.wasm_opt]
extra_features = ["--low-memory-unused", "--optimize-added-constants-propagate", "-Oz"]
```

dx passes them as they are, after its own `-Oz`, so the last `-Oz` is a second
pass that uses them. Where section 3's `--skip-pass` is needed, keep it in the
same list.

**A value built in every caller.** A function that returns a large struct by
value gets inlined into every caller. A table of translations was 12 KB in 15
callers, 180 KB of the file and 32 KB over the wire. Keep it in the data
section:

```rust
// ❌ WRONG — several hundred fields written out in every caller
pub fn words(&self) -> Localization {
    Localization::create_by_id(self.language())
}

// ✅ CORRECT — one copy, and reading it costs nothing. create_en / create_ru are const fn
pub fn of(lang: LangId) -> &'static Self {
    static EN: Localization = Localization::create_en();
    static RU: Localization = Localization::create_ru();
    match lang {
        LangId::English => &EN,
        LangId::Russian => &RU,
    }
}
```

**Tables that do not compress.** brotli folds code and text; bytes that look
random cost their whole size. crc32fast's 16 KB cost 20 KB over the wire,
`POWER_OF_FIVE_128` of `f64::from_str` costs 10 KB. FlUrl links `GzEncoder` for
`.compress()`, decided at run time, so the table stays even when nothing
compresses; a small replacement crate on `miniz_oxide` took it out.

**`[patch.crates-io]` fails in silence.** A replacement that lacks a feature its
user names is no candidate, a version below the one in `Cargo.lock` is not taken,
and a higher one is not taken until the lock moves: cargo keeps the real crate,
and its warning does not reach the output of `dx build`. Give the replacement a
higher version and the features its users name, run `cargo update -p <crate>`,
and have the build script stop while the removed crate is still in `Cargo.lock`.

## 9. Finding what is big

dx's own flags do not show it: `--keep-names` is undone by wasm-opt's
`--strip-debug`, and `--debug-symbols true` keeps std's DWARF, binaryen then
skips passes and the code comes out 7% larger than shipped. This works:

```bash
RUSTFLAGS="-C symbol-mangling-version=v0" CARGO_TARGET_DIR=<short dir> dx build --release --web

~/.dx/tools/wasm-bindgen-<version>/wasm-bindgen --target web --keep-lld-exports --out-name app \
    --no-typescript --out-dir pp <target>/wasm32-unknown-unknown/wasm-release/app.wasm

~/.dx/tools/binaryen-129/bin/wasm-opt pp/app_bg.wasm -Oz -o out.wasm --debuginfo \
    --enable-reference-types --enable-bulk-memory --enable-mutable-globals \
    --enable-nontrapping-float-to-int --enable-threads
```

`out.wasm` has every function's name, and the `name[hash]` markers of v0
mangling give the crate. Check that its code section is within 0.2% of the
shipped one before believing it. Names can mislead: `subsecond::HotFunction` and
`ListenerCallback::new` carry components and handlers inlined into them.

- **Inlined everywhere:** rebuild with `-Cllvm-args=--inline-threshold=0` and
  compare by function name. What grows by tens of KB is what was inlined into
  every caller. The flag is a probe, not a gain: other thresholds gave nothing
  or made it worse.
- **Tables:** add `-C link-arg=--emit-relocs`, read the `linking` section for
  data symbols by name, and measure their entropy.

## 10. `--wasm-split`

`[profile.wasm-release]` keeps `lto = true` and `debug = true`: dx warns without
them. `--debug-symbols false` still removes the debug info from what ships. A
path over 256 characters panics dx (`ConstStr` in `const-serialize`), so build
from a short directory.

With section 8 the link flag of the split stays: `RUSTFLAGS` replaces, it does
not add, so the build line takes `RUSTFLAGS="$WASM_RUSTFLAGS -C link-arg=-O0"`.
Without the flag the build still succeeds, and the split zeroes constants another
module reads.

**The main module is the only shared one.** dx 0.7.10 never fills its shared
chunk (`build_split_chunks`): what the main module does not reach is copied into
every module that reaches it. Taking code out of the main module can therefore
make the bundle larger: moving the dialogs out made 2 MB of copies, +690 KB over
the wire. Compare the total, and count copies by name across the modules
(`--debug-symbols true` keeps their name sections, mangled). A function whose
address `main` takes keeps what it reaches in the main module, and nothing has
to call it:

```rust
// main(), under #[cfg(feature = "split")]. Never called: taking its address is the point.
std::hint::black_box(shared_code::keep as fn());
```

```rust
// shared_code.rs. The list is what the copies were made of, largest first.
use std::hint::black_box;

use dioxus::dioxus_core::{ComponentFunction, Properties, VComponent};
use dioxus::prelude::*;

pub fn keep() {
    black_box(reload_directory as fn());
    component(DataTable);
    component(ConfirmDialog);
}

/// What drawing a component reaches, the way a page's `rsx!` reaches it.
fn component<P: Properties + 'static, M: 'static>(render: impl ComponentFunction<P, M>) {
    let props = unsafe { std::ptr::read(black_box(std::ptr::null::<P>())) };
    black_box(VComponent::new(render, props, ""));
}
```

**Dialogs are one module, loaded right behind the first paint.** They open from
any page, so routes do not split them, and they were a third of the main module:
567 → 251 KB. All modules together went 1710 → 1697 KB, so this moves bytes, it
does not remove them. With the load started by the shell, the first dialog opens
in 22 ms with no spinner and the page is drawn no later:

```rust
use dioxus::wasm_split;

static MODULE: wasm_split::LazyLoader<(), Element> =
    wasm_split::lazy_loader!(extern "dialogs" fn render_open_dialog(props: ()) -> Element);

#[component]
pub fn RenderDialog() -> Element {
    use_future(|| async {
        MODULE.load().await;
    });

    if matches!(*consume_context::<Signal<DialogState>>().read(), DialogState::None) {
        return rsx! {};
    }
    rsx! {
        SuspenseBoundary { fallback: |_| render_dialog_loading(), OpenDialog {} }
    }
}

#[component]
fn OpenDialog() -> Element {
    // A press before the module has come waits for the same load.
    let load = use_resource(|| async move { MODULE.load().await });
    // Here already: no suspense, so the spinner does not blink before every dialog.
    if let Ok(dialog) = MODULE.call(()) {
        return dialog;
    }
    load.suspend()?;
    render_dialog_loading()
}
```

Put `MODULE`, the `use_future` and this `OpenDialog` under the same cargo
feature as `--wasm-split`. Without the flag there is no module to load, so that
build needs an `OpenDialog` that calls `render_open_dialog(())` directly.

**Pages are fetched in the background, one at a time.** A script written into
`index.html` after the build, because the names exist only then, and after the
cache-buster, because the loader asks for the plain name. It starts 3 s after
`#main` got its first child and goes through the page modules with
`fetch(file, { priority: "low" })`. Not `<link rel="prefetch">`: its concurrency
is the browser's, and the host compresses a file at its first request, so
requests at once compress at once. The host answers `no-cache` + ETag, so the
same version costs a `304` a file and a new one costs its bytes once. A fetch
only fills the cache: a first opening still compiles the module.

`%(files)s` is the JSON list of every `module_*.wasm` but the dialogs', which the
shell loads itself, and `%(delay)d` is 3000:

```html
<script>
(function () {
    var files = %(files)s;
    function fetchAll() {
        setTimeout(async function () {
            for (var i = 0; i < files.length; i++) {
                try {
                    var response = await fetch("/assets/" + files[i], { priority: "low" });
                    if (!response.ok) return;
                    await response.arrayBuffer();
                } catch (e) {
                    return;
                }
            }
        }, %(delay)d);
    }
    var main = document.getElementById("main");
    if (main.firstChild) {
        fetchAll();
    } else {
        new MutationObserver(function (_, observer) {
            observer.disconnect();
            fetchAll();
        }).observe(main, { childList: true });
    }
})();
</script>
```

**A load that fails never answers.** The generated glue logs
`Failed to load wasm-split module` and does not call back, so `load()` stays
pending. A tab left open across a deploy stays on Loading until a reload.

## 11. The page before the wasm

dx reads `index.html` from the crate root when there is one and never writes it
there. Without the file it uses its built-in template. Put the spinner BESIDE
`#main`, never inside: Dioxus only appends into `#main` and clears nothing, so
anything inside stays under the app.

```html
<div id="main"></div>
<div id="boot-loader">
    <span class="spinner"></span>
</div>
```

```css
#main:not(:empty) ~ #boot-loader { display: none; }
```

Keep `#main` empty, not even a space: this one rule takes the loader off with
the app's first node, with no script. A wasm that never comes keeps it spinning.
Under `dx serve` the rebuild toast is gone: it lives in dx's built-in template.

## 12. Measured, and not worth doing

| Tried | Result |
| --- | --- |
| wasm-opt `--outlining` | smaller file, +35 KB over the wire |
| wasm-opt `--reorder-functions` | +5 KB |
| wasm-opt `-O4` | larger over the wire |
| LLVM inline thresholds | no gain or worse |
| `-Zfmt-debug` | under 1 KB |
| Removing unused CSS | −1.4 KB, and the list held classes still in use |
| Ordering functions by name after dx | −2 to −3 KB for one more build step |
| `--wasm-split` by tab of a phone terminal | the tabs and what only they use are 6% of the code |

## Moving an existing project over

- [ ] `--debug-symbols false` on every release `dx build` / `dx bundle`: the
      local script and CI
- [ ] `[profile.wasm-release]` with `opt-level = "z"`, `lto`, `codegen-units = 1`
- [ ] wasm settings moved out of `[profile.release]`; no `panic = "abort"`, no
      `strip`
- [ ] the build script fails on `wasm-opt failed` and replaces the bundle only
      after success
- [ ] the latest stable toolchain, no old pin
- [ ] the `.wasm` and its gzip measured before and after; every page opened in a
      browser
- [ ] `COMPRESSION_ALGORITHM=br` and `MAX_COMPRESSION=true` in the compose, memory
      limit 512 MB
- [ ] sizes judged by brotli quality 11, window 16 MB
- [ ] `tracing` with `release_max_level_off`
- [ ] the standard library from its sources, `rust-src` installed
- [ ] `#boot-loader` beside `#main`
- [ ] with `--wasm-split`: every module counted, dialogs in a module of their own
