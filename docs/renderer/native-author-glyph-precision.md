# Authored glyph position precision

Open UI uses native Rust APIs and callbacks for interaction. It runs no
JavaScript. Pinned Chromium alone supplies expected pixels, with zero tolerance.

## Existing image evidence

The [source and image audit](generated/native-review-v4.json) examines ten
existing Ahem Fontations images at scale 1: five sizes, before and after a
native callback changes `X` to blue `XX`. All 640 geometry states match
Chromium. All five single-glyph images are exact. The two-glyph images differ
at sizes 10, 20 and 24; sizes 12 and 16 are exact.

The failures occur at origins 8, 24, 40 and 56 sixty-fourths of a pixel.
The first glyph's left edge stays correct. Twelve measured second-glyph edge
checks match the preceding native LCD phase. This is analysis of preserved
images, not a new renderer pass or a change to reference bytes.

## Source-supported cause

The raster trial rounds authored glyph origins to 1/64 before Skia selects its
LCD phase. Fontations computes shaped advances with finer precision. Ahem's
advances at sizes 10, 20 and 24 fall slightly below the integer size; rounding
them discards that difference. At a phase boundary, the second glyph can then
select a different mask from Chromium.

A model using the checked-in Skrifa fixed-point metric calculation, float32
positions and Skia's phase selection predicts every observed phase across the
ten images. The audit retains hashes and bytes for its fourteen supporting
font and source inputs. Chromium source `147.0.7727.24` supports the analysis;
the actual reference binary is `147.0.7727.50`. Those versions are recorded
separately. Actual runtime advances still require direct verification.

## Prepared correction and remaining checks

Private source `3b2e0d1f` preserves authored shaped advances until Skia selects
the physical LCD phase. It retains existing native-control positioning. The
production change contains no fixture, font-family or test-ID selection.

Test-only baseline `347d901c` adds a guard using real shaping for four families,
five sizes and five scales. It checks that adding the container origin retains
the run's shaped precision for the same physical strike. Eleven read-only
source and generated-contract checks pass. The named baseline failure, fixed
guard, native application comparisons and all renderer matrices have not run.

This source inherits raster trial `e0dc491e`, which loses 83 original exact
comparisons and is rejected for application. A passing precision guard would
not qualify that parent or close those regressions. The 327 different Rust
font images still need individual review; the model does not qualify them or
assign formal WPT residual ownership. The accepted renderer remains
21,334/22,924 exact. No new release state is admitted.
