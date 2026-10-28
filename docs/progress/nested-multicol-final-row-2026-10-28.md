# A block's final slice in a nested multicol row, 28 October 2026

The final source slice of a block in a nested auto-filled multicol now ends
where the block's content ends. Open UI had grown it to the full row
capacity, so the nested multicol consumed a whole extra row in its outer
column. The census goes from 22,126 to 22,130 exact with no losses.

## Finding

In `multicol-nested-028` a 100px square four-column `column-fill: auto`
multicol with a red background holds three things in order:

- An auto-filled two-column multicol with `min-height: 125px` and a green
  background, around a 205px block.
- A 75px green `break-inside: avoid` block.
- A 200px green `break-inside: avoid` block.

The 205px block fills the nested multicol's first row and leaves 5px for its
second, so the nested multicol is 125px tall. Its continuation in the second
outer column is 25px, the 75px block fits below it, and the 200px block fills
the third and fourth columns. Chrome 147 paints a green square at every
profile.

In Open UI the third inner column's slice of the 205px block was 100px tall,
with a source offset of 200 of 205. That made the nested multicol 200px tall.
Every later block moved one outer column right, and the last one painted
outside the square.

The slice height comes from `layout_multicol`'s visual part height. For a
child of a wrapped-row nested multicol, `expand_fragment_visual_height`
holds, and the `sequential_outer_rows && wraps_rows` branch grew every part
to the row capacity, including the part that holds the rest of the block.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| In that branch, a part that holds the rest of its block (`content_in_part == remaining_content`) keeps its own height | `openui-layout` | `block.rs` `layout_multicol` |

The test `nested_auto_filled_final_row_contributes_only_its_content` in
`bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds the
`-028` tree. It asserts that the second outer column holds the 25px nested
continuation and then the 75px block at offset 25. On `main` it holds only a
100px nested continuation.

## Measurement

Paired expanded census in WSL. The base is the PR 47 head `efc08156`, whose
rendering code equals `main` at `d5745fb7`. The head is `f382fbe9`, which has
the same code as the rebased branch commit. Both share one Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,126 to 22,130 |
| Gained | 4: `multicol-nested-028` at all four profiles |
| Lost, improved, worsened | none |

Four Open UI images changed, all gains. Chromium is identical in every
comparison. Rows are in the
[census evidence](evidence/nested-multicol-final-row-census-v1.json).

A Windows 1x render of all 2,029 `css_multicol` and `css_break` fixtures,
compared before and after the change, differs in `-028` and
`out-of-flow-in-multicolumn-120`. The census leaves out `-120` because it
runs script. Its fixture models the document before the script runs. That
layout was compared with Windows Chrome 147.0.7727.50 on the same document
with the script removed. The 200x100 multicol region goes from 5,000
mismatched pixels to 0.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,126 to 22,130 exact; 4 gains, 0 losses |
| Windows 1x fixture renders | `pixel_compare render` over `css_multicol` and `css_break`, before and after | 2 changed, 2,027 identical |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,472 passed, 0 failed, 20 ignored |
