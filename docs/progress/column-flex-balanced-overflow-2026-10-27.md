# Overflow columns after a forced column flex break in a balanced multicol, 27 October 2026

A balanced multicol (`column-fill: balance`, the initial value) with a
definite height now creates overflow columns for a column flexbox whose
forced break pushes content past its declared columns. Open UI had dropped
that content. The census goes from 22,090 to 22,093 exact with no losses.

## Finding

In `flexbox_single-line-column-flex-fragmentation-018` a 100px square,
four-column multicol holds a 25px column flexbox whose items are 50, 50 and
300px tall, with `break-before: column` on the second. The break moves the
third item to 150..450 in the flexbox's fragmented flow, so its last 50px
belong in a fifth, overflow column. A white abspos box covers that column. At
1× and 2× it covers it exactly. At 1.25× and 1.5× the column's edges are
fractional, so Chrome's anti-aliased green and white leave a faint green
fringe, for example (207, 231, 207) where 25% green coverage is 25% painted
over. Open UI drew no fifth column, so those 94 and 75 pixels were white.

`tools/qualification/probe_column_flex_balanced_overflow.py` measures the third
item's client rects in pinned Windows Chrome-for-Testing 147.0.7727.50, in the
[Chrome evidence](evidence/column-flex-balanced-overflow-chrome-v1.json):

| Multicol height | Chrome 147 rects of the 300px item (x, y, width, height) |
| --- | --- |
| 100px | (25, 50, 25, 50), (50, 0, 25, 100), (75, 0, 25, 100), (100, 0, 25, 50): the last in a fifth column |
| auto | (25, 50, 25, 66.67), (50, 0, 25, 116.67), (75, 0, 25, 116.66): balanced, no overflow column |

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A balanced multicol's rule that a column flexbox with a forced break never advances past the declared column count now applies only when the multicol's height is auto. A definite-height multicol creates overflow columns for the rest of the flexbox, as it does for other content | `openui-layout` | `block.rs` `layout_multicol`, `forced_column_flex_has_next_declared_column` |

The test
`forced_column_flex_break_in_definite_balanced_multicol_creates_overflow_column`
in `bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` asserts the
fifth column and the third item's 50px end in it. It fails on `main` with 4
columns instead of 5.

## Measurement

Paired expanded census in WSL. The base is the PR 41 census at `01bd8c00`,
whose rendering code differs from `main` at `36f62675` only by rustfmt. The
head is `99d3aa72`, and both share one Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,090 to 22,093 |
| Gained | 3 |
| Lost, improved, worsened | none |

The gains are `flexbox_single-line-column-flex-fragmentation-018` at 1.25×
and 1.5×, and `flexbox_single-line-column-flex-fragmentation-049` at 1.25×.
`-049` is the same case in a two-column multicol with a 200px column
flexbox. Both tests are now exact at all four profiles. Chromium is identical
in every comparison. Rows are in the
[census evidence](evidence/column-flex-balanced-overflow-census-v1.json).

## Remaining failure

The probe's auto-height case still differs. Open UI lays it out as two 350px
columns, where Chrome balances three columns to 116.67px. The owner is
`openui-layout` column balancing (`layout_multicol`) for a column flexbox
with a forced break. No census test covers this case, and it is not changed
here.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Chrome probe | `python tools/qualification/probe_column_flex_balanced_overflow.py` | rects above, in the evidence |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,090 to 22,093 exact; 3 gains, 0 losses |
| Layout tests | `cargo test --locked -p openui-layout` | 5,935 passed, 0 failed |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,466 passed, 0 failed, 20 ignored |
