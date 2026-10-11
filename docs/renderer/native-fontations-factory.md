# Native Fontations typeface factory

Open UI executes no JavaScript. Applications use native Rust methods and Rust
callbacks. Pinned Chromium supplies the expected pixels and element bounds;
historical Open UI images remain immutable provenance.

The [completed native app follow-up](native-fontations-app.md) now uses the
real factory from a clean current-branch source. It matches 200/200 Chromium
images, with 38 gains and no losses over the outline trial. Element bounds
still fail in 1,280 states. The standalone measurements below retain their
original source and scope.

## The missing backend

The [Linux font policy trial](native-linux-font-policy.md) matches 162/200
Chromium images, with 38 images and 1,280 element bounds still different. Its
outline adapter uses FreeType for shaping and glyph advances. Custom outlines
also bypass some of the real Fontations scaler's mask decisions.

A standalone prototype now builds the actual Fontations typeface and scaler
from the existing rust-skia pin, without upgrading Skia. It links that code
alongside the existing Skia library. Ahem and DejaVu Sans probes both finish
with exit 0. Serialization identifies the returned typeface as `fnta`, while
the ordinary FreeType control returns a different factory ID. Malformed font
data returns `None`.

The Rust wrapper borrows opaque Skia arguments through the documented wrapper
traits and takes exactly one owned typeface reference. The native factory owns
copied font bytes. It receives the complete collection, variation and palette
arguments. It does not replace a global typeface decoder or reinterpret Rust
wrapper layouts. Variation, collection and palette behavior still need direct
verification.

## Measured advances

The real factory reports these unhinted, linear Ahem advances:

| Font size | Glyph advance |
|---|---:|
| 10 | 9.999984741 |
| 12 | 12.000091553 |
| 16 | 16.000030518 |
| 20 | 19.999984741 |
| 24 | 23.999923706 |

The independent Chromium app observations retain widths of 12.015625 and
16.015625 at sizes 12 and 16. Rounding the factory's advances upward to the
1/64-pixel layout grid could explain those widths. This is a candidate cause;
the prototype has not rendered the consuming application's document or
compared its pixels with Chromium.

The pinned Skia's Fontations C++ typeface, Rust bridge, base and hinting files
are byte-identical to the reviewed Chromium source files. The reviewed source
is Chromium 147.0.7727.24; the immutable capture binary is 147.0.7727.50. This
source comparison does not trace the binary's runtime factory selection.

## Reproduction and status

The [versioned evidence](generated/native-fontations-factory-v1.json) preserves
all five failed build attempts and the successful clean source, build logs,
probe observations, binary hash and source comparisons. Every member of the
[evidence archive](evidence/native-fontations-factory-v1/completed-evidence.tar.gz)
is hash-verified. The
[standalone source archive](evidence/native-fontations-factory-v1/standalone-source.tar.gz)
contains the exact compiled tree at `947fc0d8`. Its Cargo lockfile pins Skrifa
0.36.0, read-fonts 0.34.0, font-types 0.9.0 and CXX 1.0.168, matching the native
bridge's pinned source dependencies.

To reproduce the standalone probe, unpack the source and point
`OPENUI_PINNED_SKIA_INCLUDE` at the unchanged rust-skia
`a31b86ba3b767344d39af3b8c30043003d8fc991` checkout's `skia-bindings/skia`
directory. Build `factory_probe` with the repository's Chromium Cargo config
and the archived lockfile, then pass the unchanged Ahem.ttf or DejaVuSans.ttf
font path. The archived pipeline records the exact commands and build settings.

This is a backend prototype, not an integrated native SDK result. It admits no
release case, assigns no formal census residual owner, and changes no accepted
renderer total. The native app, full renderer matrices, workspace tests,
MSRV, Miri and C ABI consumers have not run on this source. The next integration
must start from the current branch, retain the variable-font correction, supply
real Fontations advances and scaler behavior, and preserve the existing
explicit FreeType path. The old descriptor trial's 29 census losses do not
qualify that trial for promotion.

Separately, documentation/evidence checkpoint `9f0d2211` passes all three PR
workflows: six jobs pass, five skip and none fail. All seven native guards
actually run and pass in hosted parity. Full manual hardening was measured on
the preceding `0c39998c` checkpoint and has not run on `9f0d2211`. Those hosted
results do not qualify the private backend or the remaining pixel gates.
