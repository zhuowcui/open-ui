# Open UI v0.2 supported-platform contract

## Supported

Open UI v0.2 supports Linux x86_64 and aarch64 applications through native X11
and native Wayland, plus platform-independent deterministic headless rendering.
The Linux runtime provides OpenGL acceleration with automatic software fallback.
The safe Rust API and versioned C ABI are co-equal front ends to the same engine.

Core product behavior includes typed styles, reactive views, pointer and
keyboard input, focus, clipboard, IME, drag and drop, core form controls,
AccessKit/AT-SPI semantics, and typed transitions/keyframe/scroll animations.

Headless consumers do not enable or compile window-system dependencies. All
resource bytes used by deterministic rendering are supplied synchronously by
the application or an immutable resource registry; the engine performs no
network access.

## Native interaction API

Application behavior runs in Rust. The public `openui` crate exposes retained
`Document` and `Element` handles; the application may keep those handles and
call native methods from Rust callbacks. There is no JavaScript execution,
`eval`, script binding, or embedded browser runtime in the application path.
Browser-style effects are provided by native operations where v0.2 needs them:

| Application task | Public Rust API |
|---|---|
| Create, move, or remove elements | `Element::create`, `append_child`, `insert_before`, `remove` |
| Change text, attributes, or typed style | `set_text`, `set_attribute`, `set_property` and generated typed setters |
| Handle input | `Element::on`, `on_capture`; Rust callbacks in `view!` |
| Focus, scroll, or inspect geometry | `focus`, `blur`, `scroll_to`, `scroll_by`, `bounding_rect`; `Document::hit_test` |
| Update form controls | `set_control_value`, `set_selection`, `set_checked` |

These methods operate on the same retained document as rendering and native
input. A browser DOM or Web API surface is not promised. If a product feature
needs another element operation, expose it through the public native Rust API
and the shared engine rather than introducing JavaScript.

## Deferred

The following are not v0.2 defects or compatibility promises: macOS, Windows,
Android, iOS, Vulkan, Metal, Direct3D, JavaScript, navigation, browser DOM
compatibility, URL fetching, HTML loading, runtime CSS parsing, file/date/color
picker dialogs, media playback, interactive embedded documents, a visual
inspector, and a general plugin ecosystem.

The frozen WPT inventory contains 1,912 test files with JavaScript in their
Chromium source and 30 nonvisual/crash-harness rows. Those scripts are not run
by Open UI. Test tooling may read a script to construct a fixed native Rust
fixture for its final visual state; the renderer then compares that state with
Chromium at all four required profiles. This test process does not add a
JavaScript runtime or browser API promise. The original 5,731-case inventory
is immutable, but old Open UI screenshots are historical evidence, not
expected pixels.

## Build and release policy

- Rust MSRV: 1.85 until a release manifest explicitly raises it.
- Public dependencies use stable, lockfile-pinned releases; prereleases are
  rejected by the v0.2 contract verifier.
- Release builds do not require a Chromium checkout or resource pack.
- Every release carries generated API/reference metadata, native ABI layout
  metadata, an exported-symbol allowlist, checksums, licenses, an SBOM, and
  provenance.
- Chromium oracle captures and their input identities are immutable. Renderer
  changes qualify only against the pinned Chromium pixels; historical Open UI
  outputs are retained for provenance.
