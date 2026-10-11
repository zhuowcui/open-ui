# Background repeat-space tile-edge sampling

Chromium is the sole pixel target. This investigation held the pinned Chromium
oracle, WPT fixtures, and zero-tolerance comparator fixed. It tested two
temporary paint-path changes against all 23 `background-repeat-space*` IDs in
the immutable complete manifest at all four required profiles (92 image
comparisons). Both changes were rejected and removed from the source tree.

## Residual shape

The clean [v16 census](generated/four-profile-census-v16.json) has 42 exact and
50 different comparisons in this selection, with zero render errors. Several
tests are exact at integer scale but differ at fractional scales. In
`background-repeat-space-1a` at 1280×720@1.25, 28 pixels differ, mostly by
one channel level at the edges of the spaced 32 CSS-pixel image tiles. The
fixture contains a 106×106 CSS-pixel box with a 1 CSS-pixel border and a
32×32 raster background repeated with `space`; its second box uses a gradient.
The WPT reference places nine separate 32×32 children with flex spacing.
This gives a shared placement target without changing either fixture.

## Rejected diagnostics

At clean code base `e6b5beff`, the first dirty build bypassed the spaced
picture shader and painted each tile through the existing per-tile path.
The temporary condition was `false && uses_spaced_background_shader(...)`.
It produced **32 exact, 60 different, zero errors**. Ten previously exact
comparisons regressed; none became exact. Aggregate differing pixels rose from
40,484 to 746,198. Its full diagnostic report has SHA-256
`e10b9e13f0cfd6ff4c84b95d2c188833c48ddf02e713126e7b33171e82532afa`.

The second dirty build retained the picture shader but changed only its
`picture.to_shader` sampling from `FilterMode::Linear` to
`FilterMode::Nearest`. It produced **42 exact, 50 different, zero
errors**. No status changed, but 18 Open UI images changed and aggregate
differing pixels rose to 390,832. Its full diagnostic report has SHA-256
`c0394d151881d1c9545f61367fe91a73467f71639606232554d74ffd30d2e646`.
All 92 Chromium oracle identities and decoded hashes matched the clean base
in both diagnostics.

The source edits were reverted. After rebuilding with the checked-in pinned
Chromium toolchain profile, a fresh clean 92-comparison selection again had
42 exact and 50 different results. All 92 Open UI and Chromium decoded hashes
matched the v16 census. These experiments do not qualify a new renderer
checkpoint.

The per-tile path and nearest shader sampling are ruled out as shared repairs.
The remaining edge differences need a reduced Engine-backed raster fixture
that isolates picture bounds, linear sampling phase, analytic coverage, and
final channel rounding at each device scale. The residuals remain unowned
until that cause is reviewed.
