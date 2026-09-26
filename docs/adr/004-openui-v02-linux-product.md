# ADR 004: Open UI v0.2 Linux and headless product architecture

- Status: Accepted
- Date: 2026-09-12
- Decision owners: Open UI maintainers

## Context

Open UI reached an exact 5,731-case static rendering baseline with a pure-Rust
style, DOM, layout, text, and paint stack. The public Rust and C application
APIs still select a separate Blink-backed implementation, however. Keeping
both paths supported would make interaction, accessibility, animation, and
platform work land twice and would leave pixel behavior dependent on which API
an application chose.

**Evidence and contract correction (2026-09-26):** “Exact” above describes the
historical claim, not the pixels proven by its comparator. The archived records
used a channel tolerance of 4 and excluded the rightmost 15 pixels. The
architecture decision remains in force. Chromium is the sole pixel target;
archived Open UI images are immutable historical evidence, not a compatibility
requirement. The [frozen oracle audit](../renderer/frozen-oracle-audit.md)
explains the discrepancy.

## Decision

Version 0.2 has one rendering engine. `openui-engine` owns documents and turns
their retained state into immutable scenes. The safe `openui` crate calls the
engine directly. `openui-ffi` validates and translates C calls directly into
the same engine. `openui-compositor` consumes scenes using either Skia raster
or OpenGL, and `openui-platform` owns Linux windows and input normalization.

```text
Rust API ---+
            +--> openui-engine --> immutable scene --> compositor
C ABI ------+                              ^              |      |
                                           |              GL     CPU
Linux input -------------------------------+               \    /
                                                            window
```

The UI API is thread-affine. Application state, callbacks, and the mutable
document remain on the owning thread. Only immutable scene snapshots and
immutable resource payloads may cross to a render thread. Handles carry a
document identity, arena index, and generation; stale and cross-document use
returns an error.

Applications implement interaction with native Rust callbacks and public
`Document` and `Element` methods. The C ABI translates its calls to the same
engine. Browser-style element operations needed by applications receive
native APIs; JavaScript execution is not part of this product.

`openui-style` is the only public source of style value definitions. Property
metadata drives Rust setters, macro checks, C values, invalidation, animation,
and reference documentation. Runtime CSS text, runtime property-name strings,
HTML loading, and stylesheet injection are not part of v0.2.

The supported native target is Linux on x86_64 and aarch64, on X11 and native
Wayland. Headless rendering remains platform independent. OpenGL is the
accelerated backend and software raster is the mandatory fallback. Platform
dependencies are feature-gated out of headless builds.

## Compatibility baseline

The v0.2 implementation must render the declared cases exactly like the pinned
Chromium oracle at every required profile. It may change pixels from the old
Open UI screenshots when that is needed to match Chromium. The archive and
historical result files remain byte-pinned for provenance, not as expected
outputs. The generated files in `docs/v02/generated` bind the test identities,
historical result bytes, source and resource inventories, viewport, scale
factor, Chromium build identity, font inputs, public APIs, example artifacts,
and historical application fixtures to cryptographic hashes.

The C ABI is a deliberate v0.2 break. Removed v0.1 calls are recorded in the
generated migration ledger; they do not receive a second production backend
or an indefinite compatibility shim.

## Consequences

Blink/Chromium remains a test oracle for the declared renderer corpus, not
a supported application runtime or release dependency. Linux and headless
closure takes priority over other operating systems and graphics APIs. macOS,
Windows, mobile platforms, Vulkan, browser execution, network fetching, media,
and specialized native pickers remain explicitly deferred.
