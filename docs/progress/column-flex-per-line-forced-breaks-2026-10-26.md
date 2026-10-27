# Column flex forced breaks per line, 26 October 2026

Many fragmented column-flex tests failed only at 1.25× or 1.5×, on single
rows or columns of pixels at fragment boundaries.

## Finding

`flexbox_multi-line-column-flex-fragmentation-021` has two column-flex
lines in a five-column multicol. A forced `break-before: column` sits on the
second item of line 1 and on the third item of line 2. A DevTools trace of
Chrome's paint records (`tools/qualification/trace_chrome_layers.py`) shows
line 2's 100px item filling the first column whole and its next item at the
top of the second column.

Open UI applied every forced-break gap to all lines at the same main-axis
coordinate (`apply_column_flex_fragmentation_breaks`). It split line 2's item
at line 1's break point, then pushed the rest of line 2 down by line 1's gap.
At 1× the extra green was hidden under other green. At 1.25× it covered a
half-pixel seam once more than Chrome did.

Blink lays column-flex lines out as parallel flows, with an offset adjustment
for each line (`FlexLine::item_offset_adjustment` in
`third_party/blink/renderer/core/layout/flex/flex_layout_algorithm.cc` at tag
147.0.7727.50):

- A forced break before an item moves the rest of that line to the next
  fragmentainer, measured from where the line's earlier content ended. The
  item keeps its margin after the break.
- For a column item, `has_container_separation` holds when an earlier item of
  the same line was processed or the item's own offset is positive. A line's
  first item at offset 0 has none. Its break-before instead joins the
  container's break-before (`row_break_between_outputs`), which moves every
  line.

Traces of `-039` and `-034` confirm both cases. The second line's first item,
offset by a 100px margin, is drawn at flow offset 200, not 100. A forced item
whose margin starts after a 50px item is drawn at 200 with its margin kept.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| Forced break-before gaps are computed per flex line from the line's content end. A positive-offset line-start item takes a full fragmentainer; a zero-offset line-start item moves the whole container. Items with a negative start margin keep the earlier anchoring | `openui-layout` | `block.rs` `apply_column_flex_fragmentation_breaks` |
| The special case that stretched a positive-margin forced item by a whole fragmentainer is removed; the per-line gap now places that item | `openui-layout` | same |
| `forced_column_flex_break_splits_crossing_parallel_item` and the renamed `forced_column_flex_item_after_full_start_margin_moves_past_next_fragment` expect Chrome's placement, citing the traced tests | `openui-layout` tests | `tests/sp13r_multicol_tests.rs` |

## Measurement

Paired expanded census in WSL against `main`'s rendering before PR 38 (clone
commits `33204b70` and `5d5d13a1`; PR 38 changes only fieldset scrollbars),
sharing one Chromium cache:

| Result | Comparisons |
| --- | --- |
| Exact | 22,065 to 22,070 |
| Gained | 5 |
| Improved | 1 |
| Lost, worsened | none |

At 1.25×, `flexbox_multi-line-column-flex-fragmentation-021`, `-022`, `-039`
and `-047`, and `flexbox_single-line-column-flex-fragmentation-034` become
exact. `-032` improves from 1,362 to 1,337 pixels. Chromium is identical in
every comparison. Rows are in the
[evidence](evidence/column-flex-per-line-forced-breaks-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,065 to 22,070 exact; 5 gains, 0 losses |
| Layout tests | `cargo test --locked -p openui-layout` | 5,928 passed, 0 failed |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,459 passed, 0 failed, 20 ignored |
