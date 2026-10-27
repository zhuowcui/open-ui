# Avoided breaks between row flex lines, 26 October 2026

`flexbox_multi-line-row-flex-fragmentation-033` and `-035` were exact at 1×
and 2× but missed by 126 pixels at 1.25× and 150 at 1.5×, along the seam
between the two columns.

## Finding

Each test has two columns 100px tall. A 50px block comes first, then a
wrapping row flex container whose items are each a column wide, so every
item is its own flex line: 25px, 25px, then 75px. `break-before: avoid` on
the third item's child (`-033`), or `break-after: avoid` on the second item's
child (`-035`), falls on the boundary between the second and third lines.
That boundary lands exactly on the column edge. An abspos box with
`top: 75px` fills the rest of the square.

A Chrome trace (`trace_chrome_layers.py`) shows that Blink keeps the second
and third lines together. The second line moves to the top of column 2 and
the third line follows at 25. The abspos box sits in column 1, 75 to 100.
Blink joins the break-after of every item in the previous row with the
break-before of every item in the next row. When that boundary avoids a
break, it uses the earlier breakpoint before the previous row
(`FlexLayoutAlgorithm`, `row_break_between_outputs`, in
`third_party/blink/renderer/core/layout/flex/flex_layout_algorithm.cc` at tag
147.0.7727.50).

Open UI broke between the second and third lines. A static-position special
case in `layout_multicol` then moved the abspos box into column 2, at 75 to
100, to cover the gap the third line left. Disabling that special case alone
lost 8 exact comparisons in `-033`, `-035` and
`flexbox_single-line-row-flex-fragmentation-040`. The two cases moved the
same green into different columns, and the seam at fractional scales showed
the difference.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| When the boundary between two row flex lines falls on a fragmentainer edge and the joined break value is `avoid` or `avoid-column`, the previous line moves to the next fragmentainer. This applies only when it is not the container's first line and starts inside a fragmentainer | `openui-layout` | `block.rs` `apply_row_flex_fragmentation_breaks` |
| The wrapped row flex branch of the abspos static position special case, and its predicate, are removed | `openui-layout` | `block.rs` `layout_multicol`, `direct_oof_static_position` |
| `avoid_between_row_flex_lines_breaks_before_previous_line` covers both break-before and break-after. It fails on `main` (second line at 25) and passes with the change | `openui-layout` tests | `tests/sp13r_multicol_tests.rs` |

The nowrap branch of the special case is kept. `-040` still depends on it,
because Open UI does not truncate a flex item's trailing margin at a
fragmentainer break.

## Measurement

Paired expanded census in WSL against `main` before PR 39 (clone commits
`8a6b1490` and `3ca27b96`), sharing one Chromium cache. This branch was then
rebased onto PR 39, which changes only column flex breaks.

| Result | Comparisons |
| --- | --- |
| Exact | 22,069 to 22,073 |
| Gained | 4 |
| Lost, improved, worsened | none |

`flexbox_multi-line-row-flex-fragmentation-033` and `-035` become exact at
1.25× and 1.5×, so they are now exact at all four profiles. Chromium is
identical in every comparison. Rows are in the
[evidence](evidence/flex-row-avoid-between-lines-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,069 to 22,073 exact; 4 gains, 0 losses |
| Layout tests | `cargo test --locked -p openui-layout` | 5,929 passed, 0 failed, with the new test |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,460 passed, 0 failed, 20 ignored, with the new test |
