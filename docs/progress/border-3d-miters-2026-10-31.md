# 3D border sides with Blink's corner miters, 31 October 2026

A groove, ridge, inset or outset border side now joins its neighbours as
Blink's `BoxBorderPainter` does. Each corner gets no miter, a soft
(antialiased) miter or a hard (aliased) one. The side is clipped only for
a hard miter, and otherwise an inset or outset side is filled as one
antialiased quad. The census goes from 22,292 to 22,304 exact, with 10 more
comparisons improved, no losses and no comparison worse.

## Finding

`BoxBorderPainter::PaintOneBorderSide` (`core/paint/box_border_painter.cc`,
147.0.7727.50) asks `ComputeMiter` about each corner of a straight side:

- no miter when the adjacent side has no width, or paints later with a
  style that fills the border area (`WillOverdraw`);
- a soft miter when the colours differ at the corner. For the 3D styles,
  `BorderStyleHasUnmatchedColorsAtCorner` makes the top-right and
  bottom-left corners differ, since the shading changes there;
- a hard miter when `BorderStylesRequireMiter` holds: either side is double,
  the adjacent side is groove or ridge, or the styles differ;
- otherwise no miter.

`MitersRequireClipping` clips a 3D side only when a corner has a hard miter.
`ClipBorderSidePolygon` then applies one clip of the side's quad when both
miters are the same, or one clip per miter when they differ. Each of those
follows its miter's diagonal at its own end and the bounding box at the
other, extended 0.1px past its end. It swaps the bottom and left sides'
miters so that each lands on its own corner. A soft miter's clip is
antialiased and a hard miter's is not. Without a clip,
`DrawLineForBoxSide` fills an inset or outset side as one antialiased quad,
mitered at its soft corners (`DrawSolidBoxSide`, `FillQuad`).

In a uniform inset border, such as an iframe's default `2px inset`, the sides
paint top, bottom, right, left. The top and bottom have no miters, since the
later sides overdraw their corners. The right and left have a soft miter at
the top-right and bottom-left corners and none at the others. Open UI
clipped every 3D side at both diagonals with an aliased clip, and widened
that clip by half a device pixel across the side, which moved the diagonal.
It then overwrote the diagonal pixels at the top-right and bottom-left
corners with an even blend of the two shades. In
`background-margin-iframe-root-ref` at 1.25x, Chrome's bottom-left corner
pixel (26, 216) is 228: the left side covers 1/8 of it over the bottom side.
Open UI's was 196.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| Each corner of an authored 3D side gets a no, soft or hard miter from `ComputeMiter`'s rules, including the unmatched top-right and bottom-left 3D corners. | `openui-paint` | `painter.rs` `paint_borders` |
| A 3D side's quad joins the outer and inner border rects' corners, and runs to the bounding box at an end without a miter. | `openui-paint` | `painter.rs` `paint_borders` |
| An inset or outset side with a soft miter and no hard miter is filled as one antialiased quad, without a clip. | `openui-paint` | `painter.rs` `paint_border_side_path` |
| A side with a hard miter is clipped as `ClipBorderSidePolygon` does: one clip for equal miters, one per miter otherwise, aliased for a hard miter. The existing half-pixel widening across the side now slides along each miter line, so the diagonal stays in place. | `openui-paint` | `painter.rs` `clip_blink_3d_side_miters` |
| The even blend of the 3D corner diagonals applies only to the platform fieldset border. | `openui-paint` | `painter.rs` `fix_corner_miter_pixels` |

The platform fieldset border keeps its own path, which replays the native
theme's corner cells.

The unit test `inset_border_takes_blinks_corner_miters_at_fractional_scale`
paints the iframe's border at 1.25x and checks four corner pixels against
Chrome's capture. It fails without the change: the top-right diagonal pixel
(404, 25) is then 154 instead of 196.

## Measurement

This is a paired expanded census in WSL. The base is `e70a5213`, whose
rendering equals `main` at `cbc0e7e4`. The head is `46195d6e`, which adds
only the unit test to `961663e0`. Both use one shared Chromium cache, and
Chromium is identical in every comparison.

| Result | Comparisons |
| --- | --- |
| Exact | 22,292 to 22,304 |
| Gained, 12 | `background-margin-iframe-root-ref` and `iframe-in-repeated-table-header-crash` at all four scales: 4, 6, 4 and 4, and 4, 38, 4 and 4 mismatched pixels at 1x, 1.25x, 1.5x and 2x; `ttwf-css3background-border-style-values` at 1x and 2x, 24 and 102; `crashtests_firefox-bug-1904428` at 1x, 2; `line-clamp_webkit-line-clamp-013` at 1.5x, 299 |
| Improved, 10 | at 1.25x: `ttwf-css3background-border-style-values`, 118 to 30; `line-clamp_webkit-line-clamp-013`, 746 to 2; `multicol-dynamic-change-inside-break-inside-avoid-001-ref`, 754 to 662; `multicol-rule-groove-000` and `-outset-000`, 819 to 806; `multicol-rule-inset-000` and `-ridge-000`, 943 to 930; `multicol-rule-groove-000-ref`, 1,266 to 1,240; `multicol-rule-ridge-000-ref`, 1,390 to 1,364. At 1.5x: `ttwf-css3background-border-style-values`, 1,394 to 333 |
| Lost, worsened | none |

The rows are in the
[census evidence](evidence/border-3d-miters-census-v1.json).

### Remaining failures

- `ttwf-css3background-border-style-values` keeps 30 mismatched pixels at
  1.25x and 333 at 1.5x. At 1.25x most are one or a few levels off along
  whole rows of groove and ridge sides, where an edge falls inside a device
  pixel. Open UI paints those edges with its physical coverage rectangles
  and the half-pixel clip widening. The cause was not established in this
  change. The owner is `openui-paint`.
- A groove or ridge side with a soft miter and no hard miter is still
  clipped by an antialiased polygon around Open UI's two halves, where Blink
  fills each half as its own mitered quad. Its effect was not measured
  separately. The owner is `openui-paint`.
- The multicol rule comparisons keep most of their mismatched pixels. A
  column rule is painted by a different path from a box border. The owner is
  `openui-paint`.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,292 to 22,304 exact; 12 gained, 10 improved, 0 worsened, 0 losses |
| Other raster configurations | rerender of every comparison in `default`, `deterministic-alias`, `deterministic-alias-subpixel` and `chromium-linux-lcd` | the same 12 gains and 10 improvements, the gains and 4 improvements in `deterministic-alias` and the 6 `multicol-rule` improvements in `chromium-linux-lcd`, at `46195d6e`; no mismatch count changed in `default` or `deterministic-alias-subpixel` |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint` | 8,333 passed, 0 failed, 2 ignored, at `46195d6e` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,496 passed, 0 failed, 20 ignored, at `46195d6e` |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
