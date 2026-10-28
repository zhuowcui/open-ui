# Positioned insets in a translated inline containing block, 28 October 2026

In a multicol, the positioned descendants of a relatively positioned inline
that has a filter now keep their insets and static inline offsets.
The census goes from 22,163 to 22,167 exact with no losses.

## Finding

`out-of-flow-in-multicolumn-090` has a two-column `column-fill: auto`
multicol with 50px columns. It holds a span with `position: relative`,
`top: 100px; left: 50px` and `filter: blur(0px)`, so the span is the
containing block of both absolute and fixed descendants. The span holds:

- A red 50x200 absolutely positioned box with `top: 100px; left: 50px`.
- A relative block with `top: 100px; left: 50px`. Inside it, an absolutely
  positioned wrapper holds a green 50x200 fixed box at its static position.

Chrome 147 starts both boxes at flow offset 100, 50px into the second
column, and the span's translation moves their slices 50px right and 100px
down. The green box covers the red one at multicol x 150 to 250 and y 100 to
200.

In Open UI, `fragment_direct_positioned_children_in_multicol` (`block.rs`,
`openui-layout`) maps each positioned box from its physical offset, which
already includes the span's translation. Both slices landed at x 50 and
100, 100px left of Chrome, so the green box was almost entirely off the
page:

1. The rule for an empty translated inline containing block took the
   containing block's source flow as the block start, even with an explicit
   `top`. The red box's `top: 100px` was ignored, so it started at flow 0.
2. For a transform containing block the inline offset took its column from
   the box's x. That x held the translation and the `left` inset, so an
   offset reaching the 50px column's end selected the next column and fell
   to 0 inside it. The red box lost its `left: 50px`, and the fixed box its
   static 50px.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The empty translated inline containing-block rule applies only without block insets. A specified inset resolves from the containing block. | `openui-layout` | `block.rs` `fragment_direct_positioned_children_in_multicol` |
| With a translated inline transform containing block, an inline inset is measured from the containing block's source edge in that edge's column. A static inline position is measured in the column holding the box's source block start. | `openui-layout` | same |

`positioned_boxes_in_a_translated_inline_containing_block_keep_their_inline_offsets`
in `bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds `-090`
without the multicol's negative margins. It asserts that both boxes' slices
sit at (150, 100) and (200, 100). Without the change they sit at (50, 100)
and (100, 100).

## Measurement

Paired expanded census in WSL. The base is the PR 54 census head
`e9a25eb1`. The head is `db6e41bf`, which has the same code as the rebased
branch commit. Both share one Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,163 to 22,167 |
| Gained | 4: `out-of-flow-in-multicolumn-090` at all four profiles |
| Lost, improved, worsened | none |

Chromium is identical in every comparison. Rows are in the
[census evidence](evidence/translated-inline-containing-block-insets-census-v1.json).

A Windows 1x render of all 2,029 `css_multicol` and `css_break` fixtures
was compared with a render of the PR 54 head. It differs only in `-090`.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,163 to 22,167 exact; 4 gains, 0 losses |
| Windows 1x fixture renders | `pixel_compare render` over `css_multicol` and `css_break`, before and after | 1 changed: `-090` |
| Layout tests | `cargo test --locked -p openui-layout` | 5,949 passed, 0 failed, 5 ignored |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,480 passed, 0 failed, 20 ignored |
