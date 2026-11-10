# A column flex item's automatic minimum is measured at its cross size, 10 November 2026

A column flex item with `min-height: auto` no longer grows taller than its
content when the content would wrap at a narrower width. Its content size
suggestion (the automatic minimum's content part) is now measured at the
item's used cross size, as Chromium measures it, not at its min-content
width. The components track found this: a text item in a column container
was clamped to one line per word.

## Finding

For a block-axis main size, `resolve_main_axis_min_max` in
`openui-layout/src/flex/algorithm.rs` took the content size suggestion from
`compute_intrinsic_block_sizes_with_layout`. For an item with no definite
inline constraint, that is the block size of a layout at the min-content
inline size. Text that fits on one line at the stretched width then
measures one line per word: for "XX XX XX" at a 20px font and line height
in a 300px wide column container, the minimum was 60px, three lines, and
the item could not be smaller.

Chromium 147 takes the content size suggestion for a block-axis main size
from `BlockSizeFunc(SizeType::kContent)`, inside `FlexLayoutAlgorithm`'s
content suggestion lambda (`third_party/blink/renderer/core/layout/flex/flex_layout_algorithm.cc`).
`BlockSizeFunc` lays the item out in `child_space`, the flex basis space,
whose inline size is the stretched or fit-content cross size, and returns
`layout_result->IntrinsicBlockSize()`: the content's block size, which does
not depend on the item's own `height`. For the example that is one line,
20px. This source was read at 147.0.7727.24, the local source checkout, which
is on the same branch as the pinned 147.0.7727.50.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| In a column container, an item whose inline flow is horizontal, left to right and unconstrained (no definite inline size and no semantic break) takes its content size suggestion from `resolve_content_based_size`: a layout at the used cross size with the item's main size treated as `auto`. | `openui-layout` | `bindings/rust/openui-layout/src/flex/algorithm.rs` |
| Row containers with orthogonal items, and items with a definite inline constraint, a semantic break, a right-to-left direction or an aspect ratio, keep the previous measurement. | `openui-layout` | same |
| `resolve_content_based_size` loses its unused constraint space parameter. | `openui-layout` | same |

Treating the main size as `auto` matches `IntrinsicBlockSize()`, which
ignores the item's own `height`. The existing test
`min_auto_column_flex_min_height` (`openui-layout/tests/flex_fixes_validation.rs`)
covers this: an empty item with `height: 150px` in a 100px column container
must shrink to fit, because its content suggestion is 0. An earlier draft of
this change measured at the item's height, made the suggestion 150px, and
failed that test.

Items with an aspect ratio keep the previous measurement because
`resolve_content_based_size` returns the size the ratio gives when the
cross size is definite, not the content's block size. The first census of
this change, at `f5118c44`, found this: WPT
`css/css-sizing/aspect-ratio/flex-aspect-ratio-050.html` and `-052.html`
went from exact to different at all four profiles. Each is a column item
with `aspect-ratio: 2/1`, 100px wide, holding 100px of content in a 0px
tall container. Its automatic minimum must be the 100px content height, and
the new path gave the 50px from the ratio.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Text item in a 300px column container, `flex-basis: auto` and `0`, with `flex-grow` | `cargo test -p openui-layout --test sp17_auto_height_flex_basis_tests column_auto_minimum` (Windows) | 2 passed. Each expects a 20px item; before the change each was 60px |
| The aspect-ratio WPT cases, `width: auto` and `100%` | `cargo test -p openui-layout --test sp17_auto_height_flex_basis_tests column_aspect_ratio` (Windows) | 1 passed, expecting a 100px item. Without the aspect-ratio exclusion it fails with 50px |
| Layout crate | `cargo test -p openui-layout --no-fail-fast` (Windows) | 5,955 passed, 0 failed, 5 ignored |
| Generators | `generate_properties.py --check`; `generate_v02_contract.py --check`; `generate_renderer_contract.py --check` | no drift |
| `cargo fmt --all --check` | in `bindings/rust` | pass |

CENSUS

The components track also previewed this change. That preview is not
evidence: it used the system FreeType so that WOFF2 text loads, and its base
was the components branch, not `main`. It applied only
`openui-layout/src/flex/algorithm.rs` from `96a9fb47`. Element border boxes
within 1/128 px of Chromium's `getBoundingClientRect` went from 152 to 184
of 192, and `card-basic` is 190px tall as in Chromium (274px before). Forty
images changed, all closer to Chromium, and the other 152 were
byte-identical. The 8 cases left are the grid `auto` column described below.

## Not done here

- Right-to-left and vertical items keep the min-content measurement; no
  measured case shows them differing from Chromium yet.
- Row containers with orthogonal (vertical writing mode) items keep the
  previous path.
- The CSS Grid auto track sizing gap the components track also found
  (auto tracks get only their minimum contribution, with no maximize step)
  is a separate fix.
