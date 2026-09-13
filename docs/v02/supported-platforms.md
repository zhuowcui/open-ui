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

## Deferred

The following are not v0.2 defects or compatibility promises: macOS, Windows,
Android, iOS, Vulkan, Metal, Direct3D, JavaScript, navigation, browser DOM
compatibility, URL fetching, HTML loading, runtime CSS parsing, file/date/color
picker dialogs, media playback, interactive embedded documents, a visual
inspector, and a general plugin ecosystem.

The frozen WPT inventory contains 1,912 JavaScript-dependent and 30
nonvisual/crash-harness rows. They are explicit exclusions. The supported
5,731 rows must remain byte-exact at the frozen 800 by 600 logical viewport,
device scale factor 1.

## Build and release policy

- Rust MSRV: 1.85 until a release manifest explicitly raises it.
- Public dependencies use stable, lockfile-pinned releases; prereleases are
  rejected by the v0.2 contract verifier.
- Release builds do not require a Chromium checkout or resource pack.
- Every release carries generated API/reference metadata, native ABI layout
  metadata, an exported-symbol allowlist, checksums, licenses, an SBOM, and
  provenance.
- A rendering-baseline change requires an independently reviewed refreeze; it
  is never accepted as incidental implementation drift.

