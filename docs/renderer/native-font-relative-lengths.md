# Native Rust font-relative length queries

Open UI never executes JavaScript. Applications use public native Rust methods
and Rust callbacks to create, change and operate elements. Needed element
operations belong in the native framework API. Excluding a scripted Chromium
test from pixel admission does not waive the corresponding native behavior.

## Current application API

A consuming application can use `Element::computed_style()` and the public
`openui_text::FontRelativeLengthResolver::from_style_in_collection` helper.
Pass the same font collection supplied to the document, then assign the owned
resolved pixel value through an ordinary native setter:

```rust
let style = element.computed_style()?;
let units = openui_text::FontRelativeLengthResolver::from_style_in_collection(
    &style,
    fonts.clone(),
);
let width = units.resolve(2.5, openui_text::FontRelativeUnit::Ch);
element.set_width(openui::LengthValue::px(width))?;
```

This API returns a value captured at that style boundary. The application
currently recomputes it after changing relevant font properties. `LengthValue`
does not yet expose `Ch`, `Ex` or `Lh` declarations that update themselves.
Those declarations, complete nested length resolution and C parity remain
native API work. JavaScript and script bindings are never the solution.

## Completed private upright correction

The [glyph/API follow-up](native-glyph-mask.md) verifies private `36b5db31`
through the unchanged consuming Rust app. All **1,440/1,440 rectangles** match
the preserved Chromium observations, fixing 160 differences with no losses.
Both runs repeat identically and execute 720 callbacks each. The strict
geometry gate exits 0 and eighteen source checks pass. No native pixels are
captured. Missing-zero and metric-override cases are implemented but not runtime
tested. The source remains private and unapplied; its glyph parent regresses
the complete renderer gates. Automatic declarations, full context resolution,
animation updates, pixel and C qualification remain required native work.

## Measured horizontal correction

Clean private `4046d366` corrects shared font and native style computation:

- Effective resolved font sizes use pinned Chromium's two-decimal cache precision.
- A zero-glyph advance is rounded when the selected SkFont has no subpixel positioning.
- Numeric `lh` resolution truncates its font-size basis and percentage product to the layout grid.
- Authored percentage line heights become fixed lengths at the computed font size after Chromium's integral percentage conversion.

The consuming Rust app repeats **1,200 cases, 2,400 bounds and 1,200 callbacks
per run**. All **2,400/2,400 full rectangles** match the unchanged repeated
Chromium geometry observations. `ch`, `ex` and `lh` each match 800/800 states.
Both native outputs are byte-identical. The app uses two font families, four
starting sizes, five line-height forms, two coefficients and five device
scales. It checks owned styles and bounds, callback mutations and weak-handle
teardown. These are horizontal geometry results; this app captures no native
pixels and does not qualify every font, orientation or raster configuration.

All **8,562 workspace tests pass**, zero failed and 13 ignored. The new native
callback guard and strict RGBA guards actually execute and pass. All **17
read-only checks pass**. The actual Rust/C/C++ ABI consumers preserve 113
exports and run thirteen C examples and seven C++ consumers successfully.
Both source trees reconstruct exactly from the public checkpoint through an
independent Git index. No Fontations, caption or intrinsic-sizing trial is
included in this source.

An earlier private `cea2310c` matches 2,240/2,400 bounds: `ch` and `ex` each
match 800/800; 160 percentage `lh` states still differ. Its native guard passes,
but the whole diagnostic exits 1 and stops before workspace, ABI or pixels.
The failed trial remains preserved. A source-preparation guard matching error
also remains preserved with its actual failure and completed recovery; it
starts no native or Chromium processes.

## Remaining qualification and APIs

All four complete pixel matrices finish on `4046d366`:

| Suite | Exact | Different | Errors | Actual exit |
|---|---:|---:|---:|---:|
| Original | 21,332/22,924 | 1,592 | 0 | 1 |
| Expanded | 22,135/23,728 | 1,593 | 0 | 1 |
| Focused | 640/640 | 0 | 0 | 0 |
| Primitive | 960/960 | 0 | 0 | 0 |

The original and expanded sweeps each lose two previously exact comparisons:
`multicol-span-all-004` and its reference at 800x600, scale 1. Each changes
from zero to 76 wrong pixels, in four one-pixel-wide glyph regions. Both cases
retain their previous results in the other three profiles. All Chromium image
and oracle identities remain fixed, and all other comparison invariants remain
unchanged. The twelve known sizing neighbors also retain their prior results:
six exact and six different. The whole owner exits 1. The source stays
**private, unapplied and unqualified**. The accepted original result remains
**21,334/22,924 exact**.

The subsequent [caption and shaping trial](native-caption-shaping.md) reviews
and repairs the caption cause: intrinsic text used unshaped widths, and an
empty grid added spacing to the caption minimum instead of contributing a
separate wrapper floor. Clean `db177050` restores both legacy comparisons and
passes its native caption callback guard. Six neighboring sizing glyph
comparisons worsen, so it also remains unapplied. Corrected sizing geometry
matches fresh repeated Chromium queries; glyph mask and origin work remains.
No production condition is added for a test, font family or font size.
Exact geometry and narrowed passes do not waive complete qualification.

## Upright vertical ch measurements

The [separate vertical diagnostic](generated/native-vertical-ch-v1.json) runs a
public Rust consuming app on the same private SDK. Both native runs repeat
**720 cases, 1,440 bounds and 720 callbacks** byte-identically. Ten independent
Chromium captures repeat their images and queries at five scales. **1,280/1,440
bounds are exact; 160 differ**, all in upright vertical DejaVu Sans text.
The diagnostic exits 0 because capture and measurement complete successfully;
those 160 differences remain failures of geometry parity. This app captures
no native pixels and admits no release state.

The root cause is reviewed: the helper reads a horizontal zero-glyph width
for upright vertical `ch`. Pinned Chromium uses the vertical zero-glyph
advance, with rounded ascent/descent fallback for a face without vertical
metrics. The existing helper needs the same orientation-aware native metric
selection. Missing-glyph fallbacks, retained reactive declarations, setter-order
independence, inherited/root and pseudo contexts, animation updates, nested
lengths and C parity also remain open. The unchanged-frame zero-work path must
remain intact.

The [source-identified evidence](generated/native-font-relative-v1.json)
preserves source patches, exact reconstruction, both native trials, callback
logs, actual stage exits, source checks, pinned source reviews and the API
scope audit. Its archive members are hash-verified. Earlier capture cleanup
failures and the repeated diagnostic retry are preserved in the
[isolated sizing evidence](generated/native-intrinsic-isolated-v1.json).
Chromium alone supplies expected pixels and bounds. Historical Open UI images
remain provenance. No reference is rewritten and no release state is admitted.
