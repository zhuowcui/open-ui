# Abspos boxes at a column end and fragmented flexbox ends, 27 October 2026

Open UI placed an absolutely positioned box whose static position falls
exactly at a column's block-end in the next column whenever `top` was set.
Chrome keeps it in the column that ends there. Correcting that exposed where
Open UI ends a fragmented flexbox, which several special cases had covered
for. This change ends flexboxes where Blink does. The census goes from
22,078 to 22,090 exact with no losses.

## Finding

`tools/qualification/probe_multicol_abspos_static.py` loads small cases in
pinned Windows Chrome-for-Testing 147.0.7727.50. Each is a 100x100
`column-fill: auto` multicol with block or flex content, then a 25x20 abspos
box. Every case is measured twice: once with the multicol as containing block
and once with the initial containing block. Both give the same rects, recorded
in the [Chrome evidence](evidence/multicol-abspos-column-end-chrome-v1.json).

| Content before the box | Chrome 147 rect (x, y) |
| --- | --- |
| 400px block, `top: 0` | (75, 0): the fourth column, not an overflow column |
| 400px block, `top` auto | (75, 100) |
| 100px block, `top` auto, then a 50px block | (0, 100): the box stays in the first column although the next block starts the second |
| Auto-height column flexbox, items 50, 50 and 300, with an avoided or forced break before the second item, `top` auto | (100, 50): the flexbox ends 50px into a fifth column |
| The same flexbox with `height: 400px` | (75, 100): a definite flexbox ends at its authored size |
| Row flexbox whose 50px item has a 150px bottom margin, `top` auto | (0, 200): the flexbox finishes in its first column |

The flexbox ends follow from how Blink fragments a box:

- **Auto height.** A fragment with a break inside fills the rest of its
  fragmentainer, so an auto-height flexbox also consumes the space its forced
  and avoided breaks leave.
- **Definite height.** A definite flexbox consumes only its authored size,
  except that a break before the flexbox itself moves its start. Blink joins
  a forced break before a line's first item into the flexbox's own
  break-before (`row_break_between_outputs` in `FlexLayoutAlgorithm`). In
  `flexbox_multi-line-column-flex-fragmentation-025` this moves the whole
  350px flexbox from 50px into the first column to the second column. The
  following block then starts 50px into the fifth column, at (80, 50), with
  or without the test's `break-before: avoid` on the first item.
- **Row flexbox with a trailing margin.** Its item border boxes end inside
  the first column, so the flexbox finishes there and Blink truncates the
  item's margin at the break.

The probe also showed that Open UI did not apply an avoided break that falls
exactly on a fragmentainer edge between two column flex items. Blink rejects
that breakpoint and breaks before the previous item instead, as it does
between row flex lines
([row flex avoid record](flex-row-avoid-between-lines-2026-10-26.md)).

## Change

| Change | Owner | Source |
| --- | --- | --- |
| An abspos box at the exact block-end of a column belongs to that column, whatever its insets | `openui-layout` | `block.rs` `layout_multicol`, `block_end_owned_by_previous_column` and `flex_origin_block_end` |
| An avoided break at a column flex item boundary on the fragmentainer edge moves the previous item, and the rest of its line, to the next fragmentainer. It applies when the previous item is not the line's first and starts inside the fragmentainer | `openui-layout` | `block.rs` `apply_column_flex_fragmentation_breaks` |
| The static position after an auto-height column flexbox includes its break extension | `openui-layout` | `block.rs` `flex_origin_block_end` |
| When the break pass moves every flex line, a definite column flexbox starts and ends that much later, for its following sibling and for static positions | `openui-layout` | `block.rs` `column_flex_leading_break_shift` |
| A row flexbox whose item border boxes end inside its first column finishes there: the static position is its unfragmented end in that column | `openui-layout` | `block.rs` `row_flex_finishes_in_first_fragmentainer` |
| A column flex item that crosses a continuation start no longer grows past the flexbox's fragmented end | `openui-layout` | `block.rs` multicol continuation slices |
| The nowrap row flex branch of the abspos static-position special case, and its predicate, are removed | `openui-layout` | `block.rs` `direct_oof_static_position` |

Tests in `bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs`:

- `static_inline_position_after_full_columns_stays_in_last_column` now
  expects (75, 0), as Chrome measures. It had expected the box in an overflow
  column.
- `static_position_after_auto_column_flex_follows_break_shift`,
  `..._follows_forced_break`,
  `static_position_after_definite_column_flex_uses_authored_end` and
  `static_position_after_row_flex_trailing_margin_stays_in_first_column`
  assert the probe's flexbox rects.
- `avoided_column_flex_break_keeps_last_item_height_in_overflow_column`
  asserts that the last item keeps its own height.
- `forced_column_flex_break_extends_ink_but_not_fixed_static_flow` is renamed
  `forced_break_before_column_flex_line_start_moves_definite_flexbox`. It now
  expects the following block 50px into the fifth column, as Chrome measures.
  It had expected 0.

## Measurement

Paired expanded census in WSL against `main` at `49440881`. The head is
`01bd8c00`, and both share one Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,078 to 22,090 |
| Gained | 12 |
| Lost, improved, worsened | none |

All the gains are at 1.25× and 1.5×. They are
`flexbox_single-line-column-flex-fragmentation-016` and `-017`, and
`flexbox_multi-line-column-flex-fragmentation-018`, `-019` and `-020` at both
profiles, plus `flexbox_multi-line-column-flex-fragmentation-025` and `-027`
at 1.25×. Each had missed by 149 to 312 pixels along a column seam, where Open
UI had put an item in the wrong column. All seven tests are now exact at all
four profiles. `flexbox_single-line-row-flex-fragmentation-040`, which the
removed special case had covered, stays exact at all four.

`flexbox_single-line-column-flex-fragmentation-018` still differs at 1.25×
and 1.5×, with the same Open UI pixels as on `main`; its residual is not
diagnosed here. Chromium is identical in every comparison. Rows are in the
[census evidence](evidence/multicol-abspos-column-end-census-v1.json).

Three intermediate heads were measured on the way and were not accepted:

- **Column-end rule alone** (`aeec6182`): 10 losses and 6 worsened.
- **Plus the flexbox ends** (`2fe32f26`): 6 gains and 7 losses. Items grew to
  fill an overflow column.
- **Plus the item cap** (`95abac81`): 11 gains and 3 losses, all in `-025`.
  The previous head had matched `-025` only because the grown items covered
  the red.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Chrome probe | `python tools/qualification/probe_multicol_abspos_static.py` | rects above, in the evidence |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,078 to 22,090 exact; 12 gains, 0 losses |
| Layout tests | `cargo test --locked -p openui-layout` | 5,934 passed, 0 failed |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,465 passed, 0 failed, 20 ignored |
