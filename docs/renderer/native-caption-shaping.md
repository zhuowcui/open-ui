# Caption sizing and glyph placement

Open UI never executes JavaScript. Native applications create, measure and
change elements through public Rust APIs and Rust callbacks. Pinned Chromium
defines expected pixels and geometry. Historical Open UI images are provenance.

## Completed glyph follow-up

The [subsequent native glyph trial](native-glyph-mask.md) reduces and repairs
the sizing glyph failures without restoring the wrong layout fraction. Its
native app matches twenty images and rectangles, and both focused and
primitive suites pass. Full censuses still regress, so the source remains
private and rejected. The separate empty-outline app exposes a one-step
corner discrepancy with the opposite delta from the caption; it does not
fully reproduce that caption failure. The measurements below retain the
original `db177050` source and outcomes.

## Shared correction

Clean private `db177050` builds on the measured font-unit correction `4046d366`.
It measures intrinsic text through the same shaper as inline layout and
preserves positive shaped fractions. A table's caption minimum and grid border
box constrain the same wrapper: their maximum is the floor. An empty grid's
spacing must not be added to the caption minimum. The change contains no
fixture edits, test IDs, font-name or font-size conditions, post-raster patches,
private Fontations factory, or positioned intrinsic-sizing trial.

Both native callback guards execute and pass. The caption guard checks three
border spacings, five scales, text changes through a Rust callback, owned bounds
and weak-handle teardown. The repeated consuming Rust font-unit app runs 1,200
cases and callbacks per run; all 2,400 bounds match the unchanged Chromium
geometry input. Both outputs are byte-identical. That app captures no native
pixels and does not qualify automatic reactive font-unit declarations.

All **8,563 workspace tests pass**, zero fail and 13 are ignored. Both new
native guards and the strict RGBA guards actually execute in the workspace.
All **18 read-only checks pass**. Actual ABI consumers preserve 113 exports
and the existing layouts, run thirteen C examples and seven C++ consumers.
An independent Git index reconstructs the complete tracked candidate exactly
from public `4129d8fc`, including the eight production files. This is source
proof, not a qualification pass.

## Rejected pixel result

The own-source diagnostic renders five cases at all four required profiles:
three `block-size-with-min-or-max-content-1` variants and two
`multicol-span-all-004` variants. All Chromium image and oracle identities stay
fixed. The matrix has **12/20 exact, eight different, zero errors**, actual
exit 1. It restores the two caption comparisons that `4046d366` had lost at
800x600, scale 1. Against the accepted renderer it gains and loses no exact
comparison, but six existing sizing comparisons worsen:

| Scale | Variants | Wrong pixels before | Wrong pixels after |
|---|---:|---:|---:|
| 1.25 | 3 | 57 | 114 |
| 1.5 | 3 | 384 | 406 |

The whole owner exits 1 and stops before original, expanded, focused or
primitive matrices. Those matrices were **not run on this source**. The
candidate stays **private, unapplied and unqualified**. The accepted original
count remains **21,334/22,924 exact**; no release state is admitted.

## Geometry and raster diagnosis

Six sizing boxes are queried twice in independent pinned Chromium processes
at each required profile. All repeated geometry and image bytes agree, and
all four fresh images match the immutable oracle images exactly. Fragment
dumps from the parent and candidate retain their own clean source and binary
identities. **24/24 candidate rectangles match Chromium; the parent matches
0/24.** The parent adds 1/64 pixel to each intrinsic child size and 2/64 pixel
to its enclosing size. The correction removes that fraction without changing
Chromium inputs.

The remaining pixel changes are glyph cells. At 1.25x, six one-row regions of
nineteen pixels contain extra black ink; alpha is unchanged. At 1.5x, most
differences are extra black glyph cells, with smaller missing-ink and border
overlap regions. The evidence retains complete bounds, connected regions,
channel deltas and all profile images for every residual.

Text and paint own the glyph strike/origin gap. The precise mask or origin
correction and a further reduced native glyph reproducer remain open. Adding
the old size fraction back would hide raster differences and disagree with
measured Chromium geometry. No tolerance or qualification gate is weakened.

The first capture attempt fails during Chromium profile cleanup. A diagnostic
wrapper then stops the complete owned Chromium process group before profile
removal and completes all eight repeated captures. The failed capture and an
intermediate helper preparation error are preserved with their actual exits.
The pinned harness file, Chromium binary, fixture HTML and oracle bytes remain
unchanged. The diagnostic's process cleanup and `--disable-dev-shm-usage`
switch are recorded; its success does not qualify unrelated browser behavior.

## Native API work

Upright vertical `ch` still has the previously measured 160 differences.
Pinned source confirms its missing-zero-glyph compliant fallback feature is
stable, and its vertical advance fallback rounds ascent and descent separately.
That source review does not implement or qualify a native fix. Automatic typed
`ch`, `ex` and `lh` declarations, complete nested contexts and C parity remain
required native API work. They never require JavaScript in Open UI.

The [completed evidence](generated/native-caption-shaping-v1.json) preserves
204 freshly hash-verified archive members: source reconstruction, consuming
app, guards, every actual stage, the rejected pixel selection, fragment and
Chromium geometry, reviewed ownership, primary source, failed attempts and
byte-preserving storage moves. Complete renderer and release gates stay open.
