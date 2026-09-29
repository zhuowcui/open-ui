# Fractional rectangular overflow clip coverage

Chromium 147 is the pixel target. This investigation used the clean
[four-profile census](generated/four-profile-census-v31.json) and the unchanged
Chromium oracle. The source edit below was a dirty diagnostic and has been
reverted. It is not qualification evidence or a release fix.

## Observable edge

The native final-state case `wpt/css_position/containing-block-change-button`
has a purple button with `overflow: clip` and an opaque green child whose left
edge coincides with the button's content clip. At 1280×720@1.25, only the
physical column x=87, y=275..399 differs: 125 pixels. At `(87,300)`, Chromium
stores `(95,127,95,255)` and Open UI stores `(143,127,143,255)`. Both store
opaque green `(0,128,0,255)` at `(88,300)`. The other three required profiles
are exact. The original corpus's `containing-block-change-button-ref` has the
same 125-pixel fractional-scale signature.

The values are consistent with two coincident fractional edges multiplying
their coverage in Open UI, where Chromium paints one effective half-covered
edge. This is a coverage hypothesis, not a proven description of Chromium's
compositing path. The generated fixtures are Engine-backed native final-state
reproductions; a smaller independent reproducer and a five-scale phase sweep
are still needed before assigning a reviewed root cause.

Open UI computes the rectangular clip in `compute_overflow_clip_reference_rect`
and applies it in `paint_with_overflow_clip`. At fractional scale,
`antialias_rectangular_overflow_clip` keeps analytic coverage for authored
`overflow: clip` and `hidden`. Chromium's
`FragmentPaintPropertyTreeBuilder::UpdateOverflowClip` stores a layout clip
rectangle and a pixel-snapped paint clip rectangle for ordinary boxes. That
source distinction alone does not establish how Blink/Skia combines a child
edge with a coincident clip edge.

## Rejected hard-clip experiment

Temporarily changing `antialias_rectangular_overflow_clip` to retain analytic
coverage for `hidden` but not `clip` made both button cases exact at 1.25 and
also made `css_break/clipping-001` exact at 1.5. The native final-state button
became 4/4 exact. The edit was then run against 848 selected original test IDs
at all four required profiles, covering 3,392 comparisons. The baseline had
3,015 exact and 377 different; the diagnostic had 2,984 exact and 408
different, with zero errors in both. It regressed 33 previously exact
comparisons, including overflow clipping, clip margins, fragmentation, and
flex sizing. Sixteen existing residual comparisons changed size. All 3,392
selected Chromium decoded hashes remained identical. The
[diagnostic index](generated/rectangular-clip-hard-diagnostic-v1.json) records
the source/report identities and all 56 changed Open UI images.

The test selection was generated from the immutable 5,731-case manifest by
matching `clip`, `overflow`, `button`, `control`, or `fieldset` in the ID. The
diagnostic command was:

```sh
python3 tools/qualification/run_renderer_matrix.py --suite full \
  --ids-file /dev/shm/openui-clip-ids-v1.json \
  --pixel-compare bindings/rust/target/debug/pixel_compare \
  --cache-dir out/renderer-qualification-cache \
  --results-dir /dev/shm/openui-clip-hard-diagnostic-v1 \
  --allow-dirty-diagnostics --jobs 8
```

The changed `clip` rule cannot be used. The remaining work belongs to paint
clip and compositor coverage: determine when Chromium preserves analytic clip
coverage and when coincident child and clip edges are resolved as one edge,
then verify that rule against the exact overflow cases before another full
census. The button case remains different at 1.25, and the release gate stays
open.

## Rejected contained-child experiment

A second dirty experiment kept analytic clipping except when both axes used
`overflow: clip` and every direct child was a simple leaf box whose
border box fit within the clip. It made the native final-state button case
4/4 exact. Across the 848 selected original IDs at the two fractional
profiles, however, the baseline was 1,421/1,696 exact and the experiment was
1,418/1,696 exact. It fixed the button reference but regressed four previously
exact comparisons: both fractional profiles of
`css_flexbox/min-size-auto-overflow-clip` and
`css_overflow/overflow-clip-margin-visual-box-and-value`. All selected
Chromium decoded hashes stayed fixed. The
[second diagnostic index](generated/rectangular-clip-contained-diagnostic-v1.json)
records the five changed Open UI images and report identities. This edit was
also reverted. Border-box containment alone is insufficient to decide clip
coverage, including when flex sizing or a nondefault clip reference box is
involved.
