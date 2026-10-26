# Flex row break propagation, 26 October 2026

The largest remaining census family is fragmented flexbox in `css_break`. Many
of its failures appear only at 1.25× or 1.5×, on the boundary column between
two columns.

## Finding

In `flexbox_single-line-row-flex-fragmentation-015`, an absolutely
positioned 50×50 box sits at the bottom of the first column. Open UI's
fragment tree had a second copy of it at the bottom of the second column,
marked as a continuation. At 1.25× the column boundary falls at device x 87.5.
Chrome covers that pixel twice: the abspos box from the left and the flex
container from the right. Open UI covered it three times, because the copy
added one more half-pixel of green. Chrome's pixel is 25% red; Open UI's was
12.5% red.

The copy came from three special cases in `layout_multicol`. Each one cloned
a static-position abspos into the next column when a row flex sibling
followed it. Removing them made seven tests exact but left a red 50×50 hole in
`-017` and `-019`. So the clones had been covering for a missing feature.

The upstream titles of those tests name that feature: "break-before values on
the first row are propagated to the flex container". Blink joins the
break-before and break-after of every item in a flex row, then propagates the
first row's break-before and the last row's break-after to the container
(`FlexLayoutAlgorithm`, `row_break_between_outputs`, in
`third_party/blink/renderer/core/layout/flex/flex_layout_algorithm.cc` at tag
147.0.7727.50). Open UI followed only the first in-flow child. In `-019` the
avoid on the second item must move the preceding block into the next column.
In `-017` a forced break on a nested row item must move the whole flex row.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A single-line row flex container joins every item's break-before and break-after, as a grid container already did | `openui-layout` | `block.rs` `joins_child_break_values`, used by `propagated_break_before` and `propagated_break_after` |
| The three abspos clone special cases and their two helper predicates are removed | `openui-layout` | `block.rs` `layout_multicol` |

Multi-line row flex containers are unchanged. Only their first row's items
propagate break-before, and which items those are depends on line layout.

## Measurement

Paired expanded census in WSL against `main`'s rendering before PR 33 (clone
commits `86506085` and `806adda3`), sharing one Chromium cache. This branch was
then rebased onto PR 33, which changes only table part painting.

| Result | Comparisons |
| --- | --- |
| Exact | 22,021 to 22,035 |
| Gained | 14 |
| Improved | 1 |
| Lost, worsened | none |

These tests become exact at every profile:

- `flexbox_single-line-row-flex-fragmentation-015`, `-017`, `-019`, `-020` and `-024`
- `flexbox_multi-line-row-flex-fragmentation-021`, `-025`, `-032`, `-034` and `-091`

`flexbox_single-line-row-flex-fragmentation-014` becomes exact at 1.5×, and at
1.25× it goes from 200 to 100 pixels. Chromium is identical in every
comparison. Rows are in the
[evidence](evidence/flex-row-break-propagation-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,021 to 22,035 exact; 14 gains, 0 losses |
| Layout tests | `cargo test --locked -p openui-layout` | all pass |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,459 passed, 0 failed, 20 ignored |
