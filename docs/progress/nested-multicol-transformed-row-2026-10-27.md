# Fixed boxes in a transformed stacked nested multicol row, 27 October 2026

A fixed-position box whose containing block is a transformed box inside an
auto-height nested multicol now resumes in the outer column that shows the
transformed box's row, as Chrome 147 paints it. Open UI had placed it one row
early. The census goes from 22,097 to 22,103 exact with no losses.

## Finding

In `out-of-flow-in-multicolumn-037` a 100px square two-column
`column-fill: auto` multicol holds a 50px wide auto-height two-column
multicol, shifted by `margin-left: -100px`. Its content is a 400px block,
then a box with `transform: translateX(0)` holding a 400px block and a 25x400
abspos box at `top: 0`. That box holds a 400px red block and a 25x400 green
fixed box at `top: 0`, whose containing block is the transformed box.

The inner multicol lays out one 100px row of two 25px columns for each outer
fragmentainer. The 400px block fills rows 0 and 1, so the transformed box
starts in row 2. The outer multicol overflows into columns 2 and 3, which the
shift moves to x 0-100. There the green fixed box should cover the red
block. Chrome 147 paints the green square at every profile.

`fragment_direct_positioned_children_in_multicol` chose the column row of
each positioned continuation from `containing_block_offset.top`. For a
containing block promoted from a transformed ancestor, that offset is not the
transformed box's own block offset, so the fixed box was mapped through an
earlier row. A compensation in
`extract_nested_positioned_fragments` added one row's inline advance for each
source row below the first. A survey of the census showed it fired only in
`-037` to `-039`, and it did not correct them.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A positioned continuation whose containing block was promoted from a transformed ancestor takes its column row from `transform_containing_block_source_offset.top`, the transformed box's own block offset | `openui-layout` | `block.rs` `fragment_direct_positioned_children_in_multicol` |
| Remove the per-row inline compensation for transformed sources in a nested multicol, and its now unused `nested_row_inline_advance` and `owner_is_nested_multicol` parameters | `openui-layout` | `block.rs` `extract_nested_positioned_fragments` |

The test `abspos_in_transformed_stacked_nested_row_maps_through_that_row` in
`bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds the `-037`
layout with a transform containing block. It asserts that the abspos
fragments appear only in outer columns 2 and 3, at inline offsets 0 and 25.
On `main` they appear in outer columns 1 and 2, at inline offsets 100 and
125.

## Measurement

Paired expanded census in WSL. The base is the PR 43 head `73db0d1d`, whose
rendering code equals `main` at `3435c004`. The head is `9d82b235`, and both
share one Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,097 to 22,103 |
| Gained | 6: `out-of-flow-in-multicolumn-037` at all four profiles; `-038` at 1x and 2x |
| Improved | 3: `-038` at 1.25x (15,875 to 250) and 1.5x (22,650 to 150); `-039` at 1.25x (15,750 to 15,625) |
| Lost, worsened | none |

Twelve Open UI images changed, all in `-037` to `-039`. Chromium is
identical in every comparison. Rows are in the
[census evidence](evidence/nested-multicol-transformed-row-census-v1.json).

## Remaining failures

- `out-of-flow-in-multicolumn-038` at 1.25x and 1.5x: the square is placed
  correctly, but one or two device-pixel columns inside it (device x 87 and
  118 at 1.25x, 142 at 1.5x, over the square's full height) blend lighter
  than Chrome's. Not diagnosed; the likely owner is fractional fragment edge
  snapping in `openui-paint`.
- `out-of-flow-in-multicolumn-039` at every profile: its outer abspos box has
  no `top`, so its static position is 400px into the transformed box, and
  the fixed box inside a second abspos box has no `top` either. Open UI now
  paints no green there; on `main` it painted the square misplaced, with the
  same mismatch count at 1x, 1.5x and 2x. The owner is the static-position
  mapping of nested positioned continuations in `openui-layout` `block.rs`.
  Not diagnosed here.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,097 to 22,103 exact; 6 gains, 0 losses |
| Layout tests | `cargo test --locked -p openui-layout` | 5,937 passed, 0 failed |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,468 passed, 0 failed, 20 ignored |
