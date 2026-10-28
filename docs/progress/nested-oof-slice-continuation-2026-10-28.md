# Placed positioned slices in a nested multicol continuation, 28 October 2026

The positioned overflow continuation of a nested multicol no longer repeats
positioned slices that the inner multicol already placed in an earlier outer
column. Open UI had carried those slices into the continuation and widened
them to the outer column, so they painted outside the multicol. The census
goes from 22,138 to 22,142 exact with no losses.

## Finding

In `out-of-flow-in-multicolumn-047` a 100px square two-column
`column-fill: auto` multicol holds a relatively positioned, auto-filled
two-column multicol with a red background. The inner multicol holds:

- A 50x100 green abspos at `top: 100px`, whose containing block is the inner
  multicol.
- A relatively positioned 200px block holding a 25x200 green abspos.

The inner multicol is 100px tall and fits the first outer column. Its two
25px columns hold the two 100px slices of the 25x200 abspos, so the first
outer column is green. The abspos at `top: 100px` starts below the inner
multicol and fills the second outer column. Chrome 147 paints a green square
at every profile.

In Open UI the second outer column held a continuation of the inner
multicol for the abspos at `top: 100px`. The positioned overflow path of
`layout_multicol` builds it by cloning the inner multicol's fragment. Once
the inner columns' source is consumed, the path drops the inner column boxes
from the clone. It kept the two slices of the 25x200 abspos, which
`fragment_direct_positioned_children_in_multicol` had already placed in those
columns. It then shifted them up by the consumed 100px and widened each
absolutely positioned child to the outer column. The second slice painted a
25px green strip to the right of the square.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| When the positioned overflow continuation drops the consumed inner column boxes, it also drops positioned slices (`positioned_fragmentation` with a `decoration_slice`) that end within the consumed source | `openui-layout` | `block.rs` `layout_multicol` |

The test
`positioned_continuation_of_nested_multicol_omits_slices_already_placed` in
`bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds the
`-047` tree. It asserts that the inner multicol's continuation in the second
outer column holds only the abspos at `top: 100px`. On `main` it also holds
both slices of the 25x200 abspos.

## Measurement

Paired expanded census in WSL. The base is the PR 50 census head `a6151760`,
whose rendering code equals `main` at `c629ee04`. The head is `e4ca1b21`,
which has the same code as the rebased branch commit. Both share one
Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,138 to 22,142 |
| Gained | 4: `out-of-flow-in-multicolumn-047` at all four profiles |
| Lost, improved, worsened | none |

Four Open UI images changed, all gains. Chromium is identical in every
comparison. Rows are in the
[census evidence](evidence/nested-oof-slice-continuation-census-v1.json).

A Windows 1x render of all 2,029 `css_multicol` and `css_break` fixtures,
compared before and after the change, differs only in
`out-of-flow-in-multicolumn-047`.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,138 to 22,142 exact; 4 gains, 0 losses |
| Windows 1x fixture renders | `pixel_compare render` over `css_multicol` and `css_break`, before and after | 1 changed, 2,028 identical |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,475 passed, 0 failed, 20 ignored |
