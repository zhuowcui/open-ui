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
source and generated-contract checks pass. Fresh guard owner `1658` waits for
all 36 preceding whole owners, including text retry `1656`, before named
baseline/fixed guards and the full text suite. Its source path and restore
branch are checked before queueing. The
[prepared guard evidence](generated/native-review-v5.json) preserves the new
probes. That queued owner is interrupted before execution; a process-absence
witness preserves its incomplete receipt. Fresh owner `1667` on a new root
reproduces the named baseline failure with exit 101, passes the fixed guard,
and passes all 342 text tests. The
[actual guard evidence](generated/native-text-inheritance-v1.json) preserves
every exit and both sources. Native application comparisons and renderer
matrices have not run; this completed queue covers the guard only.

This source inherits raster trial `e0dc491e`, which loses 83 original exact
comparisons and is rejected for application. A passing precision guard would
not qualify that parent or close those regressions. The 327 different Rust
font images still need individual review; the model does not qualify them or
assign formal WPT residual ownership. The accepted renderer remains
21,334/22,924 exact. No new release state is admitted.
