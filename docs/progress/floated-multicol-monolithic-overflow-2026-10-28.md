# A floated multicol with overflowing monolithic content, 28 October 2026

A floated multicol inside a multicol now ends at its own block size when
monolithic content overflows its inner column. Open UI had continued the
float into a further outer column. The census goes from 22,167 to 22,171 exact
with no losses.

## Finding

In `nested-floated-multicol-with-monolithic-child` a one-column 100px
`column-fill: auto` multicol with a red background holds a right float
with `margin-top: 10px`. The float is itself an auto-height one-column
`column-fill: auto` multicol with a red background around a 100px
`contain: size` block, which holds a 90px green block. A 10px green
absolutely positioned bar covers the float's margin.

Chrome 147 lays the float's column out in the 90px left in the outer
column. The monolithic block overflows that column, and the float ends
there, so the green square is complete and nothing is painted beside it.

Open UI relaid the float out with the remaining 90px, as for an in-flow
nested multicol, and its used block size was 90px. The direct-float path in
`layout_multicol` (`block.rs`, `openui-layout`) then fragmented the float by
`fragment_in_flow_block_bottom`. That bottom includes the 100px monolithic
block that overflows the inner column, so the float continued into an outer
overflow column as a 10px red slice beside the square. The same nested
multicol in normal flow already ended at 90px.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A floated multicol's fragmentation extent is its child height, which counts owned overflow columns, or its used block size, not its in-flow block bottom. | `openui-layout` | `block.rs` `layout_multicol`, `direct_float_fragmentation_height` |

`floated_multicol_with_overflowing_monolithic_child_does_not_continue` in
`bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds the
fixture without the positioned bar. It asserts that the float has one
fragment, 90px tall at block offset 10. Without the change it also has a
10px continuation.

## Measurement

Paired expanded census in WSL. The base is the PR 55 census head
`db6e41bf`. The head is `98a8b143`, which has the same rendering code as
the rebased branch commit. Both share one Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,167 to 22,171 |
| Gained | 4: `nested-floated-multicol-with-monolithic-child` at all four profiles |
| Improved | `CSS2/floats/crashtests/float-rewind-parallel-flow-3-crash` at 1.5x, from 4,341 to 12 mismatched pixels |
| Lost, worsened | none |

`float-rewind-parallel-flow-3-crash` holds a right-floated three-column
multicol in a multicol. Its remaining 12 pixels at 1.5x and its 39 at 1.25x,
which this change leaves as they were, are not analysed here.

Chromium is identical in every comparison. Rows are in the
[census evidence](evidence/floated-multicol-monolithic-overflow-census-v1.json).

A Windows 1x render of the 2,234 `css_multicol`, `css_break` and
`css_position` fixtures was compared with a render of the PR 55 head. It
differs only in this fixture.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,167 to 22,171 exact; 4 gains, 1 improved, 0 losses |
| Windows 1x fixture renders | `pixel_compare render` over `css_multicol`, `css_break` and `css_position`, before and after | 1 changed: `nested-floated-multicol-with-monolithic-child` |
| Layout tests | `cargo test --locked -p openui-layout` | 5,950 passed, 0 failed, 5 ignored |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,481 passed, 0 failed, 20 ignored |
