# Column paint phases adopted, 23 October 2026

The [painter variants record](painter-candidate-variants-2026-10-22.md)
measured candidate `3a8c9fbe`'s deferred painter change at +13/−9 against
`main` and attributed its main loss to paint order. This change adopts that
painter with four repairs, each following Blink's `PaintLayerPainter`, which
paints each phase of a layer across all of the layer's fragments, then each
child layer across all of its fragments. With the repairs the change has 13
gains and no losses.

## Repairs

| Loss | Cause | Repair | Source |
| --- | --- | --- | --- |
| `out-of-flow-in-multicolumn-053` (1×, 1.5×, 2×) | A z:auto positioned owner entered the per-fragment positioned phase while its positioned descendant sat inside an inline span the phase cannot hoist. Each owner fragment painted that descendant, so the next fragment's LCD text landed on the previous column's green box | An owner with an inline-level positioned descendant keeps the ordinary path | `fragmented_positioned_phase_is_simple`, `has_inline_positioned_descendant` |
| `overflow-clip-014` to `-017` (1.25×) | The shared column prepass painted out-of-flow subtrees column by column with in-flow backgrounds. At a fractional column edge, a later column's in-flow background covered half of the previous column's out-of-flow piece | Out-of-flow subtrees leave the in-flow prepass. Positioned owners paint after in-flow decorations: the owner's own background in every column, then its descendants column by column | `collect_shared_column_decorations`, `prepaint_shared_column_root_decorations` |
| `fieldset-001` (1.5×), `fieldset-004` (1.25×) | A fieldset whose rendered legend extends its first decoration was dropped from the shared prepass for the ColumnBox complete replay, and its ordinary slice painted as well, so the edge was covered twice | Such a fieldset keeps the shared column-root decoration, as on `main` | `collect_complete_column_decoration_replays` |
| `out-of-flow-in-multicolumn-037` and `-039` worsened at every profile (10,000 to 15,000 pixels at 1×) | A fixed box with a transform containing block inside the multicol is fragmented into the columns like an absolute box, but the phase admitted only absolute, relative and sticky boxes. The absolute box was hoisted and painted last, over the fixed box that Blink paints above it | The phase admits a fixed box that layout fragmented through the multicol; an unfragmented (viewport) fixed box stays out | `fragmented_positioned_phase_is_simple` |

The candidate's Chromium-frame test
(`openui-engine/tests/fragmented_column_paint_phases.rs` with
`column_paint_phase_oracles/`, lossless crops of archived pinned Chromium 147
captures) is added. It now runs, and the census confirms its two tests exact.

## Measurement

Paired expanded censuses in WSL (`build_renderer.py`,
`run_renderer_matrix.py --suite expanded --jobs 16`), sharing one Chromium
cache. Chromium is identical in all 23,728 comparisons.

| Build | Exact | Gains | Losses |
| --- | --- | --- | --- |
| `main` `de28b52c` | 21,686 | | |
| Candidate painter | 21,690 | 13 | 9 |
| Plus the `053` repair | 21,693 | 13 | 6 |
| Plus the prepass and fieldset repairs | 21,699 | 13 | 0 (9 worsened) |
| All four repairs (this change) | 21,699 | 13 | 0 (1 worsened) |

Gains: `fieldset-002` (1×, 1.25×, 2×), `overflow-clip-003` and `-008`,
`out-of-flow-in-multicolumn-018`, `-074`, `-079` and `-080`,
`multicol-fill-balance-003` and `multicol-span-all-children-height-007`.
`fieldset-002` at 1.5× improves from 11,850 to 600 differing pixels. Per-comparison
rows are in the [evidence](evidence/column-paint-phases-census-v1.json).

## Remaining

`out-of-flow-in-multicolumn-037` and `-039` return to `main`'s values. They stay
inexact because the green square is 50 px off in layout. `spanner-fragmentation-002`
at 1.25× gains one differing pixel (275 to 276).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | as above (clone commit `5d3d1afa`) | 21,686 to 21,699 exact; 13 gains, 0 losses; one comparison worsens by one pixel; Chromium identical in 23,728 |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,455 passed, 0 failed, 20 ignored |
| Linux workspace, WSL | `cargo --config 'target.x86_64-unknown-linux-gnu.rustflags=[]' test --workspace --locked --no-fail-fast --exclude pixel-compare` | 9,454 passed, 0 failed, 15 ignored |
| Loss reproductions on Windows | `pixel_compare render` against the census Chromium frames for the text-free tests; `--raster-config chromium-linux-lcd` for `053` | All listed losses exact or unchanged from `main` |
