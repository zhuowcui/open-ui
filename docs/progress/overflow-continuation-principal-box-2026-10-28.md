# A visible overflow continuation's own box, 28 October 2026

When a box's visible overflow continues into the next column of a multicol,
its continuation is no longer clipped to the box's source border box. Open
UI had cut the continued content at the source box's height. The census goes
from 22,138 to 22,153 exact with no losses.

## Finding

In `block-in-inline-000` a 100px square two-column `column-fill: auto`
multicol with a red background holds a span. The span holds:

- A 50px green block with a line break and a 100px green
  `break-before: column` block.
- A 50px green inline-block.

The 50px block and the inline-block fill the first column. The forced block
overflows the 50px block and fills the second column. Chrome 147 paints a
green square at every profile. In Open UI the forced block was cut at 50px
in the second column, and its lower half stayed red.

`layout_multicol` builds the second column's part through its visible
overflow path. That path clones the source fragment, shifts its children by
the consumed source and clips the clone to the block axis. The clone also
kept `principal_box_rect`, which marks the source's own 50px border box
inside its larger layout extent. Paint clips to `border_box_rect()`, which
returns that principal box, so the 100px continuation was clipped at 50px.

The same clip cut the overflowing content of:

- The 25px flex container in `flexbox_multi-line-row-flex-fragmentation-068`
  and its reference.
- The 10px block with a 180px child in `multicol-fill-balance-030`.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The visible overflow continuation drops the cloned `principal_box_rect`, so its box and clip are its fragmentainer slice. Its `decoration_paint_block_size` still limits the background. A monolithic source (`node_is_monolithic_for_fragmentation`) keeps the box, so its overflow stays clipped. | `openui-layout` | `block.rs` `layout_multicol` |

Two tests in `bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs`
cover the change:

- `visible_overflow_continuation_does_not_keep_the_source_principal_box`
  builds `block-in-inline-000` without the inline. It asserts that the 50px
  block's continuation in the second column has a 100px border box. On
  `main` its border box is 50px.
- `monolithic_overflow_continuation_keeps_the_source_box` builds
  `monolithic-with-overflow`. It asserts that the zero-height `contain: size`
  block's continuation keeps its zero-height border box. Without the
  monolithic exception it is 50px, and the paired census lost
  `monolithic-with-overflow` at 1.25x: its column seam took a third
  antialiased green layer.

## Measurement

Paired expanded census in WSL. The base is the PR 50 census head `a6151760`,
whose rendering code equals `main` at `c629ee04`. The head is `089bc85f`.
Rebased onto `main`, the branch differs from that head only by PR 51, which
changed only `out-of-flow-in-multicolumn-047`. Both share one Chromium
cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,138 to 22,153 |
| Gained | 15: `flexbox_multi-line-row-flex-fragmentation-068` and its reference and `multicol-fill-balance-030` at all four profiles, and `block-in-inline-000` at 1x, 1.5x and 2x |
| Improved | `block-in-inline-000` at 1.25x, from 3,949 to 167 mismatched pixels |
| Lost, worsened | none |

Chromium is identical in every comparison. Rows are in the
[census evidence](evidence/overflow-continuation-principal-box-census-v1.json).

A Windows 1x render of all 2,029 `css_multicol` and `css_break` fixtures
was compared with a render of the PR 49 head. Apart from
`trailing-child-margin-002`, which PR 50 changed, it differs only in the
four gained fixtures.

## Remaining failure

`block-in-inline-000` at 1.25x differs in one device column, the seam
between the two columns. Chrome composites two antialiased green layers
there and Open UI three. The first column's part of the 50px block still
holds the forced block, below the line break. That block is clipped only by
the column, so it paints in the first column's lower part. Chrome places it
only in the second column. The owner is the first part of the visible
overflow path in `layout_multicol` (`openui-layout`).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,138 to 22,153 exact; 15 gains, 1 improved, 0 losses |
| Windows 1x fixture renders | `pixel_compare render` over `css_multicol` and `css_break`, before and after | 5 changed against the PR 49 head: the 4 gained fixtures and `trailing-child-margin-002` from PR 50 |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,477 passed, 0 failed, 20 ignored |
