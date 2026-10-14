# Native Rust font-relative lengths

Open UI never executes JavaScript. Applications use public native Rust methods
and Rust callbacks to create, change and operate elements. Needed element
operations belong in the native framework API. Excluding a scripted Chromium
test from pixel admission does not waive the corresponding native behavior.

## Current application API

A consuming application retains font-relative declarations directly:

```rust
element.set_width(LengthValue::Ch(2.5))?;
element.set_padding(Edges::all(LengthValue::Ex(1.0)))?;
element.set_height(LengthValue::Lh(2.0))?;
```

The shared Engine resolves them against the selected font and final font and
line-height properties, including declarations made later. Ancestor font
changes, native font registration and shared font collection changes refresh
the declarations. The application can change a font through a Rust callback
without resetting the width. Nested edges, gaps, radii and transform translations
retain their units. C uses the same Engine with appended `OUI_LENGTH_CH`,
`OUI_LENGTH_EX` and `OUI_LENGTH_LH` tags; existing exports and layouts stay intact.

`Element::computed_style()` and the public
`openui_text::FontRelativeLengthResolver::from_style_in_collection` helper remain
available for an owned metric query. A captured pixel value stays captured:

```rust
let style = element.computed_style()?;
let units = openui_text::FontRelativeLengthResolver::from_style_in_collection(
    &style,
    fonts.clone(),
);
let width = units.resolve(2.5, openui_text::FontRelativeUnit::Ch);
element.set_width(openui::LengthValue::px(width))?;
```

Use retained declarations when the dimension should respond to later changes.
Open UI executes no JavaScript and provides no script bindings.

## Retained relative line-height

A consuming app assigns the declaration directly:

```rust
element.set_line_height_length(LengthValue::Ch(2.0))?;
element.set_width(LengthValue::Lh(2.5))?;
```

The Engine retains both units and recomputes them after native font changes.
`ch` and `ex` use the element's final selected font; `lh` inside line-height
uses the parent's line height, avoiding a cycle. Other `lh` dimensions use the
element's resulting line height. Inherited values remain computed lengths.
`Style::line_height_length` provides the same declaration for native style and
pseudo builders. Existing `set_line_height(LineHeight)` remains available.
C sends a length-tagged value to the existing line-height property over the
same Engine; no export or struct layout changes are needed.

The [consuming Rust example](../../bindings/rust/openui/examples/native_line_height.rs)
changes only the parent's font from a Rust click callback. Its six bars retain
`ch`, `ex`, `lh`, `em`, `rem` and percentage declarations. Two native processes
and two Chromium processes repeat their captures; all ten images and sixty
bounds agree at five scales. The native app also checks callback count,
unchanged repeated rendering and weak-handle teardown. Three public native
API guards cover declaration ordering, shorthand, inheritance, pseudo styles,
same-unit animation, cloning and invalid relative values. The existing C and
C++ consumers exercise the same units and callback mutations.

These measurements cover six empty rectangle cases. Calculated line heights,
mixed-unit animation, true text line boxes and glyphs, adjusted fonts, root,
orientation and missing-glyph contexts remain open. Existing number/length/
percentage line-height transports also need a complete invalid-input audit.
They are required native API and rendering work.

## Current measured checkpoint

Fresh complete CPU verification at clean `883ea716` is exact on focused
**640/640** and primitive **960/960** comparisons. Original **21,342/22,924**
and expanded **22,145/23,728** are exact, with zero render errors. Both full
pixel gates still exit 1 for 1,582 and 1,583 differences. All 48,252 comparison
rows retain the preceding native pixels and every Chromium reference field,
with no gains, losses or worsened differences.

The new public `Element::set_line_height_length(LengthValue)` and
`Style::line_height_length` retain relative declarations through the shared
Engine. A native Rust callback changes only the parent font. All **10/10
images and 60/60 bounds** match independently repeated pinned Chromium captures
at the four required profiles and a 3× scale neighbor. The six measured units
are `ch`, `ex`, `lh`, `em`, `rem` and percentage; these rectangle cases do not
qualify all text line boxes, glyphs or font contexts.

All **8,640 locked all-targets tests** and **8,642 workspace and documentation
tests** pass with every feature enabled; 13 existing workspace tests remain
ignored. All 30 C and 24 C++ consumers pass, preserving 131 exports and 34
layouts. Seven explicit hosted hardening jobs pass at the same code checkpoint.
Mixed-unit animation, broader native APIs, residual ownership, compositor,
physical lab and release qualification remain open. Open UI executes no
JavaScript; consuming apps use native Rust methods and Rust callbacks.
[Completed native API and renderer evidence](generated/native-line-height-v1.json).

A separate reference-only probe records 100 Chromium geometry observations
at five scales, with two independent processes agreeing. It covers relative
line-height values and mixed-unit animation for remaining native API work.
No native app runs in that probe; it qualifies no native API or pixels. The
relative line-height subset has subsequent native app evidence above;
mixed-unit animation remains open.

### Earlier native app verification at `45d21648`

Clean canonical `45d21648` passes all **8,625 workspace all-targets tests**,
zero failures or ignores. Four new Engine guards cover declaration ordering,
nested units, parent and pseudo contexts, same-unit animation changes and
unchanged-frame work. The public Rust callback guard checks **40/40 bounds**
from the preserved independent Chromium observations at five scales. A second
public guard verifies shared font registration and removal through owned styles
and bounds. All **30 C and 24 C++ consumers** pass, including native callbacks,
retained unit tags and invalid finite-value checks. The ABI has the same
**131 exports and 34 layouts**.

The fresh native build verifies **16 compiler records and 29 artifact paths**.
At all four required profiles, two independent native app processes and two
independent Chromium processes repeat identically. The app's callback changes
only the ancestor font; all **8/8 images and 32/32 bounds** are exact before and
after the change, with zero pixel tolerance. Expected pixels come only from
pinned Chromium. The four preceding diagnostic app profiles include two
different viewport dimensions and are not counted as this required matrix.

The first dirty diagnostic passes one guard and fails three on small advance
differences. Its actual exit is 101 and its logs are preserved. The corrected
dirty diagnostic passes four Engine guards and the public callback test; its
actual exit is 0. Complete clean verification follows at the checkpoint above.
Metric queries use the effective font size, zero-glyph advance/orientation and
computed line-height rules without replacing the global font factory or shaped
glyph advance pipeline. The earlier regressing private font patches below
remain unapplied.

Complete focused, primitive, original and expanded matrices have run at the
combined checkpoint above; both full pixel gates still fail. Mixed-unit
animation and complete root, font-size-adjust, orientation,
missing-glyph and metric-override contexts remain open. These app results do
not qualify every needed native API or the release.
[179 preserved artifacts and terminal proofs](generated/native-font-units-v1.json).

## Earlier private investigations

The following results retain their original source identities and scope.

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
**private, unapplied and unqualified**. At that investigation's public checkpoint,
the accepted original result was **21,334/22,924 exact**.

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
