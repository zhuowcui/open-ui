# A will-change:transform layer rastered on transparency, 31 October 2026

A box with `will-change: transform` now paints its contents into its own
transparent group and then draws that group over the page, as Chrome's
compositor does with a composited layer's tiles.
`contain-strict-with-opacity-and-oof` becomes exact in all four profiles.
The census goes from 22,232 to 22,236 exact, with no losses and no
comparison worse.

## Finding

`will-change: transform` promotes the box to a composited layer
(`CompositingReasonFinder`, `core/paint/compositing`, 147.0.7727.50). cc
rasters the layer's display list into tiles cleared to transparent and
then draws the tiles over the content below. A group inside the layer,
such as an `opacity: 0.2` box, is therefore blended onto transparency
first and onto the page second.

Open UI painted the same group straight onto the white page. For a red box
covered by green inside `opacity: 0.2`, Skia's `saveLayer` restored onto
white gives 203, 229, 203. The tile path gives 204, 230, 204, which is what
the Chrome capture of `contain-strict-with-opacity-and-oof` shows. Every
pixel of that box differed by one level.

The first version also wrapped rotated layers. `transform-007` rotates a
fragmented `will-change` box by 25 degrees, and its antialiased edges then
moved by one level: 103 to 431 mismatched pixels at 1x. cc rasters a layer
in the layer's own space and antialiases a rotated edge only when it draws
the tiles. Drawing the rotated box directly gives the same edge coverage,
which the base already matched exactly at those pixels. A device-space
group instead blends the edge coverage into transparency first.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A box that owns its element box and has `will-change: transform` paints its subtree inside a transparent group, opened after its transform and closed before the transform is restored. Only a layer whose total matrix keeps rectangles axis-aligned gets the group. | `openui-paint` | `painter.rs` `paint_fragment_contents` |
| Four stale fixtures are re-spliced with the sanctioned single-fixture command, adding only the `WillChangeTransform(true)` style their sources declare. | `pixel-compare` | `wpt_css_break.rs`, `wpt_css_position.rs` |

The four re-spliced fixtures are `contain-strict-with-opacity-and-oof`, its
`-ref`, `table/oof-in-cell-with-alignment-002` and
`position-sticky-scroll-reposition-ref`. Each gained one line.

The unit test `will_change_transform_rasters_an_opacity_group_on_transparency`
paints the red and green boxes inside `opacity: 0.2` and expects
204, 230, 204 with `will-change` and 203, 229, 203 without it. The unit
test `rotated_will_change_layer_keeps_direct_edge_coverage` paints a
rotated layer and requires the same frame with and without `will-change`.
It fails without the axis-aligned condition.

## Measurement

This is a paired expanded census in WSL. The base is `55663999`, whose
rendering equals `main` at `d3e03cba`. The head is `2e196ef9`. Both use one
shared Chromium cache, and Chromium is identical in every comparison.

| Result | Comparisons |
| --- | --- |
| Exact | 22,232 to 22,236 |
| Gained, 4 | `contain-strict-with-opacity-and-oof` in every profile: 10,000, 15,625, 22,374 and 40,000 mismatched pixels to 0 |
| Changed, same count, 6 | `transform-000` to `-005` at 1.25x. One already-mismatched pixel each moved by one level. |
| Lost, improved, worsened | none |

The rows are in the
[census evidence](evidence/composited-layer-transparent-raster-census-v1.json).
The first version, without the axis-aligned condition, made
`transform-007` worse in all four profiles, for example 103 to 431 at 1x.

### Remaining failures

- Nine committed fixtures are stale against their sources in ways
  unrelated to this change, so they were not re-spliced and still lack
  their `will-change` flag: `out-of-flow-in-multicolumn-110` and `-111`,
  `css-multicol/composited-under-clip-under-multicol`,
  `overflow-clip-content-visual-overflow` and its `-ref`,
  `overflow-scroll-resize-visibility-hidden`,
  `invalidate-opacity-negative-z-index-ref`,
  `background-margin-will-change-root` and `scrollbar-gutter-003`. The owner
  is the fixture generator, `tools/wpt/splice_text_port.py`; batch
  regeneration is out of scope (`docs/SP17-HANDOFF.md`).
- A rotated or skewed layer is drawn directly rather than rastered in its
  own space and resampled. A group inside such a layer still blends onto
  the page. No census comparison was found that depends on it.
- `transform-007` keeps its 103 to 921 mismatched pixels. They were not
  attributed in this change.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,232 to 22,236 exact; 4 gained, 0 worsened, 0 losses |
| Other raster configurations | rerender of every comparison in `default`, `deterministic-alias`, `deterministic-alias-subpixel` and `chromium-linux-lcd` | 4 gained in `deterministic-alias`, the same `contain-strict` comparisons; no mismatch count changed in the other three |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint` | 8,325 passed, 0 failed, 2 ignored |
| Windows workspace | `cargo test --workspace --locked`, with the `windows-msvc` CI environment | 9,488 passed, 0 failed, 20 ignored |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
