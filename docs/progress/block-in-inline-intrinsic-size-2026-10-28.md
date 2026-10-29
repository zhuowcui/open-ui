# A block-in-inline box in its container's intrinsic size, 28 October 2026

A shrink-to-fit container whose content is an inline holding a block-level
box now takes that block's width. Open UI had measured such a container as
0 wide. The census goes from 22,171 to 22,174 exact with no losses.

## Finding

`table-cell-padding-block-in-inline-crash` has a three-column 400px
multicol. It holds a `display: table` div, whose anonymous cell holds a
span. The span holds a table: a cell with a 300px block of Ahem text
"Should not freeze or crash.", then a row of two cells holding only line
breaks. Chrome 147 sizes the outer table to the 242.656px column, so the
text wraps to "Should not", "freeze or" and "crash.".

In Open UI the outer table and its anonymous cell were 0 wide, so the inner
table took its 96px min-content width, and the text wrapped one word per
line.

`compute_intrinsic_block_sizes_impl` (`intrinsic_sizing.rs`,
`openui-layout`) measures a container with inline children through the
flattened inline item stream (`compute_inline_items_intrinsic_sizes`). That
measure skipped `BlockInInline` items, so a block inside a span contributed
nothing. A float or table cell holding span > 100px block measured 0, while
an inline-block, measured through the item builder, measured 100px.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| Each block-in-inline item records its outer intrinsic inline contribution before the flattened measure. The measure then ends the current max-content line at the block and takes the block's min- and max-content sizes on a line of its own. | `openui-layout` | `intrinsic_sizing.rs` `measure_block_in_inline_items`, `compute_inline_items_intrinsic_sizes` |

`block_in_inline_contributes_its_width_to_a_shrink_to_fit_container` in
`bindings/rust/openui-layout/tests/sp13_c_block_in_inline_tests.rs` builds
span > 100px block inside a left float and inside a `display: table`
row and cell. It asserts that both containers are 100px wide. Without the
change both are 0.

## Measurement

Paired expanded census in WSL. The base is the PR 56 census head
`98a8b143`, whose rendering code equals `main` once PR 56 merges. The head
is `2aa6f582`. Both share one Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,171 to 22,174 |
| Gained | 3: `table-cell-padding-block-in-inline-crash` at 1x, 1.25x and 1.5x. It was already exact at 2x. |
| Lost, improved, worsened | none |

Chromium is identical in every comparison, and no other comparison changed.
Rows are in the
[census evidence](evidence/block-in-inline-intrinsic-size-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,171 to 22,174 exact; 3 gains, 0 losses |
| Layout tests | `cargo test --locked -p openui-layout` | 5,951 passed, 0 failed, 5 ignored |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,482 passed, 0 failed, 20 ignored |
