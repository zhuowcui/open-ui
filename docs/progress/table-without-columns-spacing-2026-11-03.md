# A table without columns has no border spacing, 3 November 2026

A table grid with no columns now applies no border spacing in either axis,
as Blink does. A caption-only table is as wide as its caption, not its
caption plus two inline spacings. The census goes from 22,384 to 22,390
exact, with 2 more comparisons improved. No comparison is lost or worse.

## Finding

`multicol-span-all-004` and its `-ref` had 325 mismatched pixels at 1x,
1,098 at 1.25x, 1,239 at 1.5x and 2,295 at 2x. Their spanning table has
a caption and no rows. Open UI made it 135 px wide; Chrome made it 133 px,
the caption's width.

Chromium 147.0.7727.50 gives these sizes for tables with
`border-spacing: 10px 7px`, measured with `getBoundingClientRect`:

| Table | Chrome size |
| --- | --- |
| Empty | 0 × 0 |
| Caption only | the caption's size |
| An empty `tbody` with one empty `tr` of height 10px, no cells | 0 × 10 |
| `border: 3px` and `padding: 5px`, no columns | 16 × 16 |
| The same, with a caption | the caption's width (the larger of the two, not the sum) |
| One `<col style="width: 20px">`, no rows | 40 × 0 |
| One `<col>` and one empty row | 40 × 14 |

Inline spacing applies once the grid has a column, even an empty one.
Block spacing applies once it has a column and a row. Without a column,
neither axis gets spacing, and rows add only their own height.

Open UI applied the inline spacing to every grid. It also added the
caption's minimum width to the grid's border box for a table without
columns, instead of taking the larger of the two.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A grid with no columns has zero inline and block border spacing. | `openui-layout` | `bindings/rust/openui-layout/src/table.rs` (`resolved_grid_spacing`, used by `compute_table_intrinsic_sizes_with_layout` and `table_layout_contents`) |
| The intrinsic block size adds no spacing gaps when the table has no rows. | `openui-layout` | `bindings/rust/openui-layout/src/table.rs` (`compute_table_intrinsic_sizes_with_layout`) |
| An auto table's width is the larger of its grid border box and its caption's minimum width. The special case that added the two for a table without columns is removed. | `openui-layout` | `bindings/rust/openui-layout/src/table.rs` (`table_layout_contents`) |

The unit test `table_without_columns_has_no_border_spacing` lays out three
tables with `border-spacing: 10px 7px`. They are an empty table (0 × 0),
a table whose only row is empty and 10px tall (0 × 10), and a caption-only
table with a 30 × 16 caption (30 × 16). It also checks that each table's
min-content inline size equals its width, and that the empty table's
intrinsic block size is 0.

## Measurement

This is a paired expanded census in WSL. The base is `eb3f2a51`, whose
rendering equals `main` at `6c32c8c2`. The head is `8abfe63e`. Both are
built with the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium is
identical in all 23,728 comparisons. Open UI's output changed in 8.

| Result | Comparisons |
| --- | --- |
| Exact | 22,384 to 22,390 |
| Gained, 6 | `multicol-span-all-004` and its `-ref` at 1x (325 to 0), 1.25x (1,098 to 0) and 2x (2,295 to 0) |
| Improved, 2 | `multicol-span-all-004` and its `-ref` at 1.5x (1,239 to 1) |
| Lost, worsened | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/table-without-columns-spacing-census-v1.json).

### Remaining failures

- `multicol-span-all-004` and its `-ref` at 1.5x keep 1 mismatched pixel.
  It was not examined.
- `display-contents-table-002` and its `-ref` (224 at 1x) are not a table
  sizing failure. Every Ahem run whose baseline lies exactly on a half CSS
  pixel gets one extra row from the preceding-row replay in `openui-paint`
  (`painter.rs`, `should_replay_ahem_preceding_row`). An experiment with
  all author-text replays disabled gained 17 comparisons and lost 50, so
  that heuristic is neither right nor removable as it stands. Measured in
  Chromium, an aliased Ahem strike's rows and columns depend only on its
  physical size: they equal the strong-hinted outline at all 94 measured
  sizes, while the unhinted outline matches only 72. That replacement is
  the next task. It is not part of this change.
- The column fragment clip residuals in `multicol-rule-002` and
  `multicol-rule-fraction-003` (192 at 1.5x) and
  `multicol-span-all-block-sibling-003` (1 or 2) are unchanged.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,384 to 22,390 exact; 6 gained, 2 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,341 passed, 0 failed, 2 ignored, at `8abfe63e` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,503 passed, 0 failed, 21 ignored, at `8abfe63e` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
