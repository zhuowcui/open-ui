# Candidate painter variants measured, 22 October 2026

The [principal-source record](multicol-principal-source-2026-10-21.md)
adopted candidate `3a8c9fbe`'s layout but deferred its 3,100-line painter
change (decoration replay and positioned column paint phases), because that
change produced both gains and losses. This record measures the painter change
against current `main` with paired Linux censuses and finds the owner of its
main loss. No code changes.

## Method

The candidate `painter.rs` was split into hunks against `main` `de28b52c`
(`out/hunk_bisect.py`). PR 15's Windows platform-gate hunks were kept as on
`main`. Two groups were identified earlier: decoration replay (hunks 2–4, 15,
20–24) and positioned column phases (hunks 8–12, 25–30). Each variant was
committed in one clone and censused with `build_renderer.py` and
`run_renderer_matrix.py --suite expanded --jobs 16` in WSL, sharing one
Chromium cache. Chromium is identical in all 23,728 comparisons of every run.

| Variant | Exact | Gains | Losses |
| --- | --- | --- | --- |
| `main` `de28b52c` | 21,686 | | |
| Positioned phases only | 21,630 | 4 | 60 |
| Decoration replay only | 21,683 | 8 | 11 |
| Whole candidate painter | 21,690 | 13 | 9 |

Per-comparison gains and losses are in the
[evidence](evidence/painter-candidate-variants-census-v1.json). The two groups
depend on each other: neither is safe alone.

The whole candidate gains `fieldset-002` (1×, 1.25×, 2×), `overflow-clip-003`
and `-008`, `out-of-flow-in-multicolumn-018`, `-074`, `-079` and `-080`,
`multicol-fill-balance-003` and `multicol-span-all-children-height-007`. It
loses `fieldset-001` (1.5×), `fieldset-004` (1.25×), `overflow-clip-014` to
`-017` (1.25×) and `out-of-flow-in-multicolumn-053` (1×, 1.5×, 2×), and
worsens `out-of-flow-in-multicolumn-037` and `-039` at every profile.

## Owner of the `out-of-flow-in-multicolumn-053` loss

The test nests a relatively positioned two-column multicol in another
multicol. Its in-flow red block holds a relatively positioned span with white
Ahem text and an absolutely positioned green box that must cover everything.
With the candidate, column 69 (the last pixel of the first outer column) gains
an LCD subpixel from a text run that starts at x = 70. The same fringe
reproduces on Windows with `--raster-config chromium-linux-lcd`.

Logging the draw order shows the cause. The candidate paints
positioned phases per outer fragmentainer:

1. the green decorations as a prepass;
2. the first outer column's red block and span text, then its green boxes;
3. the second outer column's red block and span text.

The second column's text therefore lands after the first column's green box,
and its LCD filter tap crosses the shared edge. Blink's `PaintLayerPainter`
paints each layer phase across all of the layer's fragments, then each child
layer across all of its fragments. Here that means the red block for every
fragment, then every span fragment, then every green fragment, so the
positioned green box is always last. The column-grouped phase order is a
design difference from Blink, not a local clip bug: the text is drawn under
the same block-axis-only column clip `main` uses.

## Next step

Adopting the candidate painter needs its positioned phases reordered to
Blink's per-layer, per-phase fragment iteration. The losses above are the
acceptance set, and the 13 gains should survive. Until then the painter change
stays deferred.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Four paired expanded censuses | `out/wsl-paint-census.sh` (clone commits `de28b52c`, `2abb5457`, `22950dee`, `7198b239`) | Counts above; Chromium identical in 23,728 of 23,728 comparisons each |
| Loss reproduction on Windows | `pixel_compare render wpt/css_break/out-of-flow-in-multicolumn-053 … --raster-config chromium-linux-lcd` with the whole-candidate build | Column 69 is `(0,128,95)` with the candidate and `(0,128,0)` on `main` |

## Update, 23 October 2026

The painter is adopted with three Blink layer-order repairs and no census
losses; see [the column paint phases record](column-paint-phases-2026-10-23.md).
