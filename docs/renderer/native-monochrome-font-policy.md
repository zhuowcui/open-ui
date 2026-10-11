# Native monochrome font policy

Open UI never executes JavaScript. Consuming applications select native Rust
options, create retained elements, and handle interaction through Rust callbacks.
Needed browser-style element behavior remains public native API work.
Pinned Chromium alone supplies expected pixels.

## Shared native API

`RasterConfiguration::chromium_linux_monochrome()` selects the pinned Linux
monochrome author-text policy: alias edging, full hinting, and no subpixel
positioning. Native control labels use their separate configuration. Select it
in `EngineOptions` before constructing the document. The main `openui` crate
and its prelude expose the constituent raster enums and settings directly.

[Chromium's pinned Linux font policy](https://chromium.googlesource.com/chromium/src/+/refs/tags/147.0.7727.50/ui/gfx/font_render_params_linux.cc)
forces full hinting when Fontconfig disables antialiasing. Its font service
passes that setting to Blink. Pinned Skia's monochrome no-hint path uses linear
FreeType advances; the full-hint path uses fitted advances. The preserved
8,640-configuration probe repeats 34,560 samples identically in two processes.
For both native font collections, a 16px Ahem space is 16.000030517578125px
without hinting and 16px with full hinting. The word is 144.00027465820312px /
144px. Strict intrinsic ceiling then produces 16.015625px / 16px and
144.015625px / 144px. No advance epsilon or output pixel correction is applied.

## Clean native application measurement

The [public example](../../bindings/rust/openui/examples/native_monochrome_font.rs)
is byte-identical to the independently compiled consumer at clean `85f87e2b`.
It uses public Document, Element and TextNode operations; a Rust click callback
changes the text. Both native font collections compare to installed-font
Chromium with the same immutable Ahem bytes.

| Scope | Exact pictures | Exact bounds |
|---|---:|---:|
| Four required profiles; both collections; before/after callback | 16/16 | 768/768 |
| Additional 3× scale; both collections; before/after callback | 0/4 | 192/192 |
| All five scales | 16/20 | 960/960 |

At 3×, 6,672 pixels differ before the callback and 6,720 after it, in each
collection. The connected differences are single physical rows of glyph ink;
the exact raster cause remains open. Full pixel qualification fails. The
profile is explicit and is not enabled as the release qualification default.
No complete census, focused or primitive matrix pass is claimed for it.

Twenty native processes repeat their pictures and measurements identically.
Callbacks, unchanged frames and weak teardown pass. Sixteen local compiler
records and 29 compiled paths are independently audited. Every command is
terminal; new audit and reaping proofs precede subsequent target use.
The three existing native font API tests pass, preserving the separate
fractional advance requirement and font-registration behavior.

Fresh captures of the original flex fixture remain RGBA-exact against all four
immutable cached Chromium references. Historical Open UI images remain history.
The [immutable evidence index](generated/native-monochrome-font-v1.json)
preserves 586 members / 2,825,304 stored bytes: native source and lockfile,
compiler records, two-process font samples, reference captures, bounds,
channel deltas, connected regions, failed-build receipts, and terminal proofs.
Owned binaries remain outside Git with their hashes recorded. Earlier evidence
and reference bytes remain unchanged.

## Remaining work

Resolve the 3× glyph raster difference, verify the complete 40-profile focused
and primitive matrices, and run the original and expanded census with matched
font policies. The last complete original result remains 21,329/22,924 exact,
with 1,595 differences and no errors. Every remaining needed native Rust API,
compositor, hardware and release gate remains required.
