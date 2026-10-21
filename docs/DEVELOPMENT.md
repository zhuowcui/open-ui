# Developing Open UI v0.2

The supported workspace is `bindings/rust`. It contains the one pure-Rust
engine used by the safe Rust API, headless rendering, Linux runtime, and C ABI.
A Chromium checkout is not required for ordinary builds.

Application interaction runs in native Rust through public `Document` and
`Element` methods and Rust event callbacks. Open UI does not execute JavaScript.
When a consuming app needs behavior provided by a browser element API,
implement that behavior in the shared Rust engine and expose a public Rust
method for the app to call directly. Include its state changes, geometry and
events; the app handles interaction with Rust callbacks.
WPT scripts may be read by offline qualification tools to identify a final
visual state; they are never run by Open UI. See the
[native interaction contract](v02/supported-platforms.md#native-interaction-api).

Closing an interaction gap requires a public Rust method and coverage from a
native consumer exercising its state changes, events, or rendered result.
An internal Engine method or a test fixture alone does not complete that work.

Implement browser-style element behavior as native framework behavior. The
consuming Rust application must be able to perform the operation directly,
without a script, script binding, or JavaScript glue. A WPT label such as
`needs_javascript` describes the reference test's source; it does not defer
the native API. Keep missing public operations recorded as implementation
gaps until a consuming Rust application verifies them.

The renderer matrix checks the comparison executable's embedded build source
against the current tree. Rebuild the `pixel-compare` package after source
changes; building an application example does not update that executable.
See [renderer executable source checks](renderer/renderer-build-identity.md).

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
RUST_MIN_STACK=4194304 cargo test --workspace --locked
cargo run --locked --package hello
cargo run --locked --package hello --features linux
```

Headless is the default: Linux window dependencies are behind the `linux`
feature. Set `OUI_BACKEND=software` or `OUI_BACKEND=opengl` to force native
presentation. `Auto` attempts OpenGL and reports/falls back to software without
changing engine or scene semantics.
The workspace test command uses the same 4 MiB libtest worker-thread stack as
CI; a fragmented multicol integration case exceeds libtest's 2 MiB default.

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

The optional historical replay renders every archived ID to a temporary
directory and byte-compares it with the committed Open UI output. It records
how the renderer changed; those old Open UI images are not expected pixels:

```bash
cargo build --manifest-path bindings/rust/Cargo.toml --locked -p pixel-compare
python3 tools/accountability/verify_frozen_openui_pixels.py \
  --pixel-compare bindings/rust/target/debug/pixel_compare --jobs 4
```

Do not add test-ID branches, reference substitution, hidden fallback, synthetic
geometry, text stripping, network access, or unclassified exclusions. Qualify
renderer changes against the pinned Chromium oracle at the required profiles.
Keep the archived Open UI images and their provenance intact; record replay
differences as historical changes, not renderer failures.

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

## Windows x64 MSVC

Windows is an experimental development baseline for the headless workspace,
the C ABI and the Python tooling. Pinned Chromium references are still
captured on Linux, in WSL or on a Linux host; Windows text pixels will be
qualified separately against Windows Chrome-for-Testing 147.0.7727.50.

Install Rust 1.94.1 (`x86_64-pc-windows-msvc`), Visual Studio 2022 Build Tools
with the C++ workload and Windows SDK, LLVM (clang, libclang and
`llvm-readobj`), Python 3.12 with `Pillow==12.2.0` and `clang-format==18.1.8`,
Ninja and Node 22. Enable long paths for the checkout with
`git config core.longpaths true`. The repository's `.gitattributes` checks out
LF text regardless of `core.autocrlf`; clone auxiliary sources such as the
Chromium WPT sparse checkout with `git -c core.autocrlf=false`.

```powershell
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
$env:FORCE_SKIA_BUILD = "1"
$env:SKIA_NINJA_COMMAND = "ninja"
$env:RUST_MIN_STACK = "4194304"
$env:PYTHONUTF8 = "1"
$env:SKIA_GN_ARGS = 'extra_cflags += [ "-DSK_SUPPORT_LEGACY_CONIC_CHOP", "-DSK_ENABLE_LEGACY_SHADERCONTEXT", "-DSK_USE_FREETYPE_EMBOLDEN", "-DSK_GAMMA_EXPONENT=1.2", "-DSK_GAMMA_CONTRAST=0.2", "-fno-strict-aliasing", "-msse3" ]'
cd bindings\rust
cargo build --workspace --locked
cargo test --workspace --locked
cargo build --locked --package openui-ffi
python ..\..\tools\ffi\verify_abi.py --library <target>\debug\openui_ffi.dll
python ..\..\tools\ci\check_platform_scoped_tests.py
```

`PYTHONUTF8=1` is required: the generators read UTF-8 sources and Windows
otherwise defaults to the ANSI code page. `pixel-compare` finds Python through
`OPENUI_PYTHON`, then `python3`, `python` and `py -3`, rejecting the Windows
Store launcher alias. Use a short `CARGO_TARGET_DIR` such as `out\cargo-win`
to stay below path-length limits inside the Skia build.

Windows text references come from the pinned Windows Chrome-for-Testing
147.0.7727.50 `win64` build, unpacked to
`out/chrome-win/win-147.0.7727.50/chrome-win64/` from
`https://storage.googleapis.com/chrome-for-testing-public/147.0.7727.50/win64/chrome-win64.zip`.
`tools/qualification/windows_chrome.py` verifies its SHA-256 before launch.
`python tools/qualification/probe_windows_text_oracle.py --output <file>`
remeasures every Windows text scenario; tests that use the deterministic
aliased raster configuration are measured with Blink's `NoFontAntialiasing`
feature. Windows decodes WOFF and WOFF2 application fonts before DirectWrite.

The Win32 desktop runtime is behind the `windows` feature, the counterpart
of `linux`:

```powershell
cargo run --locked --package hello --features windows
python ..\..\tools\ffi\verify_native_window.py --library <target>\debug\openui_ffi.dll --protocol win32 --backend software
```

Build `openui-ffi` with `--features windows` before running the native window
consumers.

Tests whose expected values come from the Linux oracle, its FreeType raster or
its Fontconfig font resolution are marked
`#[cfg_attr(not(target_os = "linux"), ignore = "linux-oracle: <reason>")]`.
They still compile on Windows and still run on Linux, so the Linux gate is
unchanged. `tools/ci/check_platform_scoped_tests.py` requires the marked set
to equal `docs/v02/generated/windows-pending-tests.txt` exactly and rejects
unconditional or malformed markers; `--write` regenerates the ledger. Remove entries
from the ledger as Windows-native references qualify them; never add a test
whose expected value is platform-independent.

## Fuzzing and sanitizers

The isolated `bindings/rust/fuzz` workspace has targets for FFI values, tree
mutations, event sequences, resources, and animation sampling. Scheduled CI
runs these with a pinned nightly and also runs Miri plus address, leak, and
undefined-behavior sanitizers. See [CI](CI.md) and
[hardening](v02/hardening.md).

## Commits and historical code

Keep changes scoped and run `git diff --check`. Renderer closure waves use the
Chromium comparison matrix before commit and record the optional historical
replay separately. The root GN/C++ Blink backend, `openui-build`,
`openui-sys`, and SP2 Skia experiments are historical evidence only. Do not add
new application features to them or include them in the Rust workspace or
release packages.
