# Later rows of a nested auto-filled multicol, 28 October 2026

An auto-filled, auto-height multicol nested after earlier content in an outer
column now gives each later row the full outer fragmentainer, as Chrome 147
does. Only its first row is limited to the space left in the outer column.
Open UI had given every row that shortened first-row capacity. The census
goes from 22,118 to 22,126 exact with no losses.

## Finding

In `multicol-nested-013` a 100px square two-column `column-fill: auto`
multicol with a red background holds a 50px green block and then an
auto-filled two-column multicol. That multicol holds a 50px green block and a
100px green `break-inside: avoid` block. The nested multicol's first row has
the 50px left in the first outer column. The avoided block moves to the
second row, which starts the second outer column. Chrome 147 makes that row
100px tall and paints a green square at every profile. Open UI laid out the
second row at 50px, so its column box clipped the avoided block at half its
height and left red below it.

`multicol-nested-014` has the same structure with five inner columns, floats
and a `contain: size` block. It fails the same way.

In the final layout pass the nested multicol's constraint space has a
`fragmentainer_block_size` of 50px, the space left, and an
`outer_fragmentainer_block_size` of 100px. Open UI already has per-row
capacities for this case, `SplitPrincipalRowCapacities`: the first row takes
the given height and later rows the fresh outer fragmentainer. That type was
used only for a finite split portion.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| Use `SplitPrincipalRowCapacities` for a nested auto-filled row whose outer fragmentainer is taller than its first row, not only for a finite split portion | `openui-layout` | `block.rs` `layout_multicol` |

The test
`nested_auto_filled_row_after_outer_progress_gives_later_rows_a_fresh_fragmentainer`
in `bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds the
`-013` tree. It asserts that the nested continuation in the second outer
column has one 100px row holding the avoided block. On `main` that row is
50px.

## Measurement

Paired expanded census in WSL. The base is the PR 46 head `fe8b546b`, whose
rendering code equals `main` at the PR 46 merge. The head is `efc08156`,
which has the same tree as the rebased branch commit. Both share one
Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,118 to 22,126 |
| Gained | 8: `multicol-nested-013` and `-014` at all four profiles |
| Lost, improved, worsened | none |

Sixteen Open UI images changed. The other eight are
`flexbox_multi-line-column-flex-fragmentation-035` and
`flexbox_multi-line-row-flex-fragmentation-039`, `-040` and `-059` at 1.25x
and 1.5x. They have the same nested structure and still differ by the same
counts, all in fractional seam pixels. At `-035` 1.25x, the first changed
pixel moves from `(15, 120, 0)` to `(31, 112, 0)`; Chrome has
`(63, 96, 0)`. They are exact at 1x and 2x before and after. Chromium is
identical in every comparison. Rows are in the
[census evidence](evidence/nested-multicol-fresh-rows-census-v1.json).

A Windows 1x render of all 2,029 `css_multicol` and `css_break` fixtures,
compared before and after the change, differs only in `-013` and `-014`.

## Remaining failures

- The fractional seam pixels in the four flexbox tests above, as in the
  other column seam residuals. Owner: fragment edge snapping in
  `openui-paint`. Not diagnosed.
- `multicol-nested-028` and `multicol-nested-column-rule-003` have separate
  nested row causes and follow as their own changes.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,118 to 22,126 exact; 8 gains, 0 losses |
| Windows 1x fixture renders | `pixel_compare render` over `css_multicol` and `css_break`, before and after | 2 changed, 2,027 identical |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,471 passed, 0 failed, 20 ignored |
