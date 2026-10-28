# A wrapped column flex item's negative margin after its forced break, 28 October 2026

In a multi-line column flexbox inside a multicol, an item with a forced
break and a negative start margin now starts after its break and keeps its
margin there, as Blink places it. The census goes from 22,158 to 22,163
exact with no losses.

## Finding

In `flexbox_multi-line-column-flex-fragmentation-032` a five-column
`column-fill: auto` multicol, 100px square, holds a 500px wrapping column
flexbox:

- Line 1 holds a 150px item, then a 350px `break-before: column` item with a
  -50px top margin.
- Line 2 holds a 150px item, then a 300px `break-before: column` item.

A white absolutely positioned box covers the 50px above the third column.
Chrome 147 paints a green square: line 1's forced item starts 50px above the
third column, under the white box, and nothing paints above the other
columns.

Blink lays column-flex lines out as parallel flows
(`FlexLine::item_offset_adjustment` in
`third_party/blink/renderer/core/layout/flex/flex_layout_algorithm.cc` at tag
147.0.7727.50). A forced break moves the rest of the line to the
fragmentainer after the line's earlier content, and the item keeps its
margin there, as the
[column flex per-line forced break record](column-flex-per-line-forced-breaks-2026-10-26.md)
describes.

Open UI had three faults, all in `openui-layout`:

1. `apply_column_flex_fragmentation_breaks` anchors a forced item with a
   negative margin at its fragmentainer boundary only in a single-line
   flexbox. The forced-point pass then expects that anchoring for every
   negative-margin item. In the multi-line flexbox the item therefore kept
   its unbroken offset, 100px, instead of 150px: 200px minus its margin.
2. The multicol column clip extends above a column for a forced item's
   negative margin whenever that item has a negative top in the column. A
   later continuation of the item also has one, so the clip of every column
   after the break extended 50px upward. The earlier source shown there
   painted green above the second and fourth columns.
3. Each column's slice of the flexbox holds every item, shifted by the
   consumed source. The column before the break therefore also painted the
   forced item's first 50px in its lower half, under the green
   absolutely positioned box. At 1.25x that doubled the box's antialiased
   top edge.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| In a multi-line column flexbox, a forced item with a negative start margin whose margin pulls it over its line's earlier content is moved to the fragmentainer after that content and keeps its margin. The rest of its line moves with it, and the forced-point pass does not move it again. | `openui-layout` | `block.rs` `apply_column_flex_fragmentation_breaks` |
| Only the column that the forced break starts the item in, where its top is no higher than its margin, extends its clip above the multicol. | `openui-layout` | `block.rs` `layout_multicol`, `negative_forced_flex_item_top` |
| A column flexbox's slice in a column drops a forced negative-margin item whose margin box starts at or after the column's end. | `openui-layout` | `block.rs` `layout_multicol` |

`forced_negative_margin_item_of_wrapped_column_flex_starts_after_its_break`
in `bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds `-032`
without the positioned boxes. It asserts that only the third column extends
50px above the multicol, that the forced item's border box begins there, and
that the second column holds none of it. Without the change, the second,
third and fourth columns all extend 50px upward.

## Measurement

Paired expanded census in WSL. The base is the PR 53 census head
`25b93820`, whose rendering code equals `main` at `eafdb356`. The head is
`e9a25eb1`, which has the same code as the rebased branch commit. Both share
one Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,158 to 22,163 |
| Gained | 5: `flexbox_multi-line-column-flex-fragmentation-032` at all four profiles, and `flexbox_single-line-column-flex-fragmentation-035` at 1.25x |
| Lost, improved, worsened | none |

In the single-line `-035`, a 100px-wide forced item with a -150px margin
starts in the third column. The single-line anchoring already placed it, but
the second column's slice still held it at that column's end. At 1.25x the
column's bottom edge falls on a half device pixel, and the item's top edge
painted into that row. The second column now drops it.

Chromium is identical in every comparison. Rows are in the
[census evidence](evidence/column-flex-negative-forced-margin-census-v1.json).

A Windows 1x render of all 2,029 `css_multicol` and `css_break` fixtures
was compared with a render of the PR 53 head. It differs only in `-032`.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,158 to 22,163 exact; 5 gains, 0 losses |
| Windows 1x fixture renders | `pixel_compare render` over `css_multicol` and `css_break`, before and after | 1 changed: `-032` |
| Layout tests | `cargo test --locked -p openui-layout` | 5,948 passed, 0 failed, 5 ignored |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,479 passed, 0 failed, 20 ignored |
