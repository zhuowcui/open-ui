# Developing Open UI v0.2

The supported workspace is `bindings/rust`. It contains the one pure-Rust
engine used by the safe Rust API, headless rendering, Linux runtime, and C ABI.
A Chromium checkout is not required for ordinary builds.

## Prerequisites

- Rust 1.85 or newer.
- Python 3.12 or newer for generators and accountability tools.
- A C/C++ compiler, Ninja, pkg-config, Fontconfig/FreeType development files,
  and OpenGL headers for building rust-skia.
- On Linux UI builds: X11, Wayland, xkbcommon, EGL/GLX, and related development
  packages.

Ubuntu-family setup:

```bash
sudo apt-get install build-essential clang libclang-dev ninja-build pkg-config \
  libfontconfig1-dev libfreetype-dev libgl1-mesa-dev libwayland-dev \
  libx11-dev libx11-xcb-dev libxcb1-dev libxkbcommon-dev
```

## Build and test

```bash
cd bindings/rust
cargo build --workspace --locked
cargo test --workspace --locked
cargo run --locked --package hello
cargo run --locked --package hello --features linux
```

Headless is the default: Linux window dependencies are behind the `linux`
feature. Set `OUI_BACKEND=software` or `OUI_BACKEND=opengl` to force native
presentation. `Auto` attempts OpenGL and reports/falls back to software without
changing engine or scene semantics.

The public Rust graph is versioned together at 0.2.0. Internal dependencies
must keep exact `version = "=0.2.0"` plus `path` so local work and crates.io
packages resolve the same graph.

## Generated contracts

Never hand-edit generated style, ABI, migration, or closure artifacts.

```bash
python3 tools/style/generate_properties.py --check
python3 tools/ffi/generate_ffi.py --check
python3 tools/release/generate_v02_contract.py --check
python3 tools/wpt/generate_sp20_closure.py --check
```

Run the generator without `--check` only when intentionally updating its source
schema. Generated output must be deterministic, checked in, and read-only clean
on two consecutive checks.

## Correctness gates

```bash
python3 -m unittest discover -s tools -p 'test_*.py'
python3 tools/conformance/verify_v02.py
python3 tools/performance/verify_v02.py
python3 tools/accountability/audit.py
python3 tools/ffi/verify_abi.py
```

The complete exact replay renders every frozen ID to a temporary directory and
byte-compares it with the committed Open UI output:

```bash
cargo build --manifest-path bindings/rust/Cargo.toml --locked -p pixel-compare
python3 tools/accountability/verify_frozen_openui_pixels.py \
  --pixel-compare bindings/rust/target/debug/pixel_compare --jobs 4
```

Do not add test-ID branches, reference substitution, hidden fallback, synthetic
geometry, text stripping, network access, or unclassified exclusions. A changed
frozen pixel is a stop condition unless an independent compatibility review
explicitly refreezes it.

## Exact parity profile

`bindings/rust/.cargo/config.chromium.toml` records the maintainer workstation's
pinned Chromium sysroot and compiler paths. It exists only to reproduce the
frozen Chromium-compatible raster environment:

```bash
cd bindings/rust
cargo --config .cargo/config.chromium.toml test --locked -p openui-paint
```

Never make this file the default or use it for release artifacts. Hosted CI
sets an equivalent portable toolchain explicitly.

## Fuzzing and sanitizers

The isolated `bindings/rust/fuzz` workspace has targets for FFI values, tree
mutations, event sequences, resources, and animation sampling. Scheduled CI
runs these with a pinned nightly and also runs Miri plus address, leak, and
undefined-behavior sanitizers. See [CI](CI.md) and
[hardening](v02/hardening.md).

## Commits and historical code

Keep changes scoped and run `git diff --check`. Renderer closure waves use a
full exact replay before commit. The root GN/C++ Blink backend, `openui-build`,
`openui-sys`, and SP2 Skia experiments are historical evidence only. Do not add
new application features to them or include them in the Rust workspace or
release packages.
