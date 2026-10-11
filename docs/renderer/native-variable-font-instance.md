# Native variable-font updates

Open UI never executes JavaScript. Applications register font bytes and change
font settings through public Rust methods and Rust callbacks. Pinned Chromium
is the pixel target; old Open UI images are immutable provenance.

## The bug

The public `Element::set_font_variation_settings` method updates the retained
style and resolves a new typeface. The existing Fontations outline adapter
then reads the original font bytes but uses the default variation location.
It paints the default glyph even though the app selected another instance.
This affects that adapter; the ordinary authored Skia path is not claimed to
have the same failure.

The correction reads the resolved typeface's design coordinates, normalizes
them through the font's own axes, and supplies that location to outline
hinting. It applies to the resolved axes generally, including authored and
derived settings. It adds no font-name, size or fixture rule.

## Native application and regression

The consuming Rust app registers the unchanged, licensed WPT variable-box
font. A Rust click callback sets the `UPWD` axis to 350. The app renders before
and after the callback, checks owned bounds, and compares the changed image
with the font's independently authored upper-block glyph. It checks callback
count, unregisters the font and verifies document teardown. The app selects
`RasterConfiguration::deterministic_aliased(true)` explicitly; its result does
not qualify other configurations or establish a Chromium comparison.

The named unit guard compares the same glyph before and after variation at
four sizes under four outline policies. An earlier guard compared different
characters and failed at 64px after the setting correction. Different
characters can receive different automatic hinting styles, so that comparison
does not isolate whether the selected variation reached painting. Its failed
result remains part of the evidence. The independent fixed-glyph comparison
in the consuming app remains required at all five scales.

## Completed native verification

Fresh baseline `ee1c98c7` and candidate `feb7e872` each pass fifteen read-only
checks. The baseline fails the corrected named guard with actual exit 101;
the candidate passes it and all 342 text tests. Its locked Linux workspace
passes 8,559 tests with zero failures and 13 ignored. All eight build stages
pass, including thirteen C and seven C++ consumers with the existing 113
exports and 30 layouts.

The baseline callback changes the setting but none of its five images change;
all five fail the independent fixed-glyph comparison. The corrected callback
changes the rendered glyph at every scale and matches the independent glyph
exactly at all five scales. Each source runs ten native processes, with all
thirty repeated PNGs identical within its own runs. Every callback runs once,
and teardown checks pass. These are native application checks, not five new
Chromium passes.

Focused 640/640 and primitive 960/960 Chromium comparisons are exact, with
zero errors and actual exits 0. The
[versioned evidence](generated/native-variable-font-instance-v1.json)
preserves the clean sources, actual exits, images, bounds, channel differences,
build logs and reviewable correction.

## Complete Chromium comparison

The [complete census evidence](generated/native-variable-font-instance-v2.json)
records all four suites on the same clean `feb7e872` source:

| Suite | Exact | Different | Errors | Actual exit |
|---|---:|---:|---:|---:|
| Focused | 640 / 640 | 0 | 0 | 0 |
| Primitive | 960 / 960 | 0 | 0 | 0 |
| Original | 21,334 / 22,924 | 1,590 | 0 | 1 |
| Expanded | 22,137 / 23,728 | 1,591 | 0 | 1 |

Every one of the 48,252 comparison rows preserves all nine recorded pixel,
oracle and difference invariants against the accepted renderer. There are
zero changed rows, zero exact gains and zero exact losses. All Chromium inputs
remain unchanged. All original rows agree with expanded, and 200/201 additions
remain exact at all four profiles. Existing residuals cover 882 original and
883 expanded test IDs; their required ownership and qualification remain open.

The correction is integrated into draft umbrella `e0e9382d`. That clean head
passes all fifteen read-only checks and all four hosted workflows, with
thirteen jobs passed, five skipped and none failed. Full hardening passes
all seven jobs. The new variation guard and six earlier native guards actually
run and pass in hosted parity. The consuming example bundles the same licensed
font and its license within the crate, so its include does not depend on a
neighboring workspace crate.

The complete pixel gates still fail. These results verify the native correction
in the measured configurations. Remaining native APIs, residual ownership and
release-lab qualification stay open. No new release case is admitted, and
skipped jobs are not passes.

Earlier source checks, compile failures and the unsuitable glyph-comparison
guard are preserved with their actual outcomes. None is counted as a
passing Rust test or a Chromium pixel result.
