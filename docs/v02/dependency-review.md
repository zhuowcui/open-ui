# Dependency review

Public runtime dependencies added for Open UI v0.2 are pinned in
`bindings/rust/Cargo.lock` and must be stable releases compatible with Rust
1.85 or newer. `generate_v02_contract.py` rejects prerelease packages and
freezes the lockfile digest.

| Package | Version | License | Purpose and maintenance review |
|---|---:|---|---|
| accesskit | 0.25.0 | MIT OR Apache-2.0 | Canonical cross-platform semantic node, tree-update, and action vocabulary. Stable release from the maintained AccessKit project; its Rust 1.85 MSRV matches Open UI. |
| accesskit_winit | 0.34.0 | Apache-2.0 | Maintained AccessKit adapter for winit and the Unix AT-SPI bridge; stable and Rust 1.85 compatible. |
| winit | 0.30.13 | Apache-2.0 | Maintained X11/Wayland event-loop and window abstraction; stable with an MSRV below Open UI's. |
| glutin | 0.32.3 | Apache-2.0 | Maintained EGL/GLX context and surface abstraction; stable with an MSRV below Open UI's. |
| glutin-winit | 0.5.0 | MIT | Maintained winit bootstrap integration for choosing native EGL/GLX configurations; stable with an MSRV below Open UI's. |
| gl | 0.14.0 | Apache-2.0 | Khronos-registry-derived OpenGL function loader used by the texture presenter. |
| softbuffer | 0.4.8 | MIT OR Apache-2.0 | Maintained CPU framebuffer presentation on X11 and Wayland; stable with an MSRV below Open UI's. |
| smithay-clipboard | 0.7.3 | MIT | Wayland seat data-device clipboard ownership without an XWayland dependency; stable and Rust 1.85 compatible. |
| x11-clipboard | 0.9.3 | MIT | X11 CLIPBOARD/PRIMARY selection implementation over x11rb. |

Transitive package `uuid` 1.26.1 is used by AccessKit tree identifiers and is
covered by the same lockfile, license, MSRV, and advisory checks. Scheduled CI
compiles the public headless and Linux framework surfaces with Rust 1.85; tag
CI repeats that compiler floor while producing both native SDK architectures.

The isolated fuzz workspace pins `libfuzzer-sys` 0.4.13 under its combined
MIT/Apache-2.0 and NCSA license. It is maintained by the Rust Fuzz project,
requires a nightly compiler only for fuzz execution, and is excluded from the
runtime workspace and all release artifacts.

All eleven crates.io packages declare Apache-2.0, repository/homepage metadata,
the Rust 1.85 floor, and version 0.2.0. Internal dependencies carry both a
local path and an exact registry version. The non-publishable `openui-ffi`
crate is packaged as the native SDK after the same dependency review.
