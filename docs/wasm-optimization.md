# WASM Optimization — Dioxus web bundles

How to ship the smallest `.wasm` from a Dioxus project: a client-side app
(`dioxus/web`) or the web half of a fullstack app. Every number here is
measured — dx 0.7.10, Rust 1.99.1, binaryen 129, the settings-service UI,
16.10.2026 — and every dx behaviour is read from the dx source, not guessed.

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
6. Measure: the size of the `.wasm` and of its gzip, before and after.

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
- `3` — +49% here. Only when a profiler shows the page is CPU-bound in Rust code
  (a chart redrawn every frame, a book of thousands of rows). Runtime speed was
  not measured for this guide.

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
