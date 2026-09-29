# Open UI

Open UI is a typed, reactive desktop and headless UI framework built around one
pure-Rust renderer. Version 0.2 targets Linux on x86-64 and AArch64, with X11,
Wayland, OpenGL presentation, automatic software fallback, and a co-equal C ABI.

```text
safe Rust API ─┐
               ├─> openui-engine ─> style/layout/text ─> immutable scene
typed C ABI ───┘                                           │
Linux events ─────> interaction/accessibility ─────────────┼─> OpenGL window
headless clock ────────────────────────────────────────────└─> exact Skia raster
```

The supported application path has no Blink/Chromium runtime, resource pack,
HTML loader, CSS text parser, JavaScript engine, or network stack. Chromium 147
is retained only as the frozen reference used to prove renderer compatibility.
Rust applications handle interaction in native Rust through `openui::Document`,
`openui::Element`, signals, and Rust event callbacks. Document lookup by ID,
element and text-node mutation, class lookup and updates, focus, scrolling,
controls, and event handling use public Rust methods. When an application
needs an operation analogous to a browser element API, Open UI must expose
that behavior through a public native Rust method on the retained document
or element. The application never runs JavaScript. See the
[native interaction contract](docs/v02/supported-platforms.md#native-interaction-api).

## Verified status

The current v0.2 release candidate has:

- a historical archive of 5,731 Open UI renders, with 5,549 byte-identical
  on replay and 182 changed; these old screenshots are not pixel targets;
- a clean four-profile Chromium census with 21,244 of 22,924 comparisons
  exact, 1,680 different, and zero render errors in the
  [latest evidence index](docs/renderer/generated/four-profile-census-v32.json);
- clean 40-profile raster matrices with 640/640 focused and 960/960 primitive
  comparisons exact;
- 201 admitted native final-state cases, including one newly admitted case
  exact at all four profiles; 198 of 201 currently meet that gate and three
  remain failures. The other 35 AST-lowered cases remain pending;
- a 7/7 repository accountability audit over all 7,673 inventoried tests;
- 43 application scenarios covering retained updates, controls, editing,
  accessibility, resources, scrolling, animation, bidi, and multi-document use;
- generation-checked Rust and C handles, deterministic manual clocks, immutable
  scenes, X11/Wayland operation, software presentation, and OpenGL upload;
- 84 frozen retained-engine/headless C exports, with 104 current exports and
  checked layouts and an ABI checksum;
- sanitizer, Miri, fuzz, leak, latency, idle-work, and package gates defined
  in CI; several remain open or failing.

Chromium is the sole pixel target. The archived Open UI bytes disagree with
Chromium for some fixtures, which is why replaying old screenshots cannot be a
release gate. The four-profile Chromium census still has 1,680 differences,
so this repository is not yet declaring the final v0.2 release. Physical-GPU
and reference-machine qualification, automated AT-SPI operation, direct Skia
GPU qualification, retained per-node layers, compositor-owned animation
curves, a C-owned native event loop, and signed publication still remain. See
[current status](docs/progress/current-status.md)
and [release qualification](docs/v02/release.md).

## Rust quick start

Rust 1.85 or newer, C/C++ build tools, and the host C runtime development files
are required. A Chromium checkout is not.

```toml
[dependencies]
openui = { version = "0.2.0", features = ["linux"] }
```

```rust,no_run
use openui::prelude::*;

fn main() -> Result<(), Error> {
    let count = create_signal(0_i32);
    let app = App::builder()
        .title("Open UI")
        .size(LogicalSize::new(800.0, 600.0))
        .backend(BackendPreference::Auto)
        .build()?;

    app.run(move || view! {
        <button
            style:display={Display::Flex}
            style:padding="8px 16px"
            on:click={move |_| count.update(|value| *value += 1)}
        >
            {count.get()}
        </button>
    })
}
```

From this checkout:

```bash
cd bindings/rust
cargo run --locked --package hello                 # deterministic hello.png
cargo run --locked --package hello --features linux # native Linux window
cargo run --locked --package framework-test -- --headless /tmp/openui-framework-test.png
cargo run --locked --package framework-test --features linux -- --window
```

Use `OUI_BACKEND=software` or `OUI_BACKEND=opengl` to force a window backend.
Headless applications use `HeadlessApp::render_at(time)` for repeatable frames.
The [framework test app](bindings/rust/examples/framework-test/README.md) checks
a reactive click and writes the resulting PNG in headless mode.
If the linker reports missing `Scrt1.o` or `crti.o`, the host C runtime
development files are absent. On the Chromium-equipped maintainer machine,
the checked-in `.cargo/config.chromium.toml` supplies a pinned sysroot; add
`--config .cargo/config.chromium.toml` immediately after `cargo` in the
commands above.

## Native SDK

The v0.2 header uses length-delimited UTF-8, versioned configuration structs,
tagged style values, checked ownership, and structured thread-local errors.

```bash
python3 tools/release/build_v02_linux.py \
  --target x86_64-unknown-linux-gnu --format sdk --format deb
```

The release driver emits headers, static/shared libraries, pkg-config and CMake
metadata, C and Rust examples, detached debug symbols, licenses, an SPDX SBOM,
checksums, and SLSA-style provenance. RPM production runs on Fedora through the
release workflow. See [packaging instructions](docs/v02/packaging.md).

## Repository map

| Path | Purpose |
|---|---|
| `bindings/rust/openui` | Safe application framework and reactive runtime |
| `bindings/rust/openui-engine` | Retained document, interaction, animation, resources, accessibility |
| `bindings/rust/openui-compositor` | Immutable scenes and raster scheduling |
| `bindings/rust/openui-platform` | Feature-gated Linux event loop and presentation |
| `bindings/rust/openui-ffi` | Validated static/shared C ABI |
| `bindings/rust/openui-{style,layout,text,paint}` | Exact rendering pipeline |
| `include/` | Generated v0.2 C headers |
| `examples/c_v02/` | C examples matching the Rust examples |
| `tools/accountability/` | Chromium comparison inventory and historical evidence audit |
| `tools/release/` | Contract generation and reproducible packaging |
| `docs/v02/` | Supported architecture and release contract |

Historical GN/Blink and SP2 experiments remain in Git for provenance, but are
excluded from the workspace and v0.2 packages. They are not supported engines.

## Documentation

- [Development](docs/DEVELOPMENT.md)
- [Architecture](docs/architecture/rendering-pipeline-overview.md)
- [CI and release gates](docs/CI.md)
- [Rust/C migration guide](docs/v02/migration-v01-v02.md)
- [Unsupported features](docs/v02/unsupported-features.md)
- [Typed style reference](docs/v02/generated/style-properties.md)
- [C ABI guide](bindings/rust/openui-ffi/README.md)

Open UI is licensed under Apache-2.0. Bundled font and dependency notices are
preserved with the relevant sources and release artifacts.
