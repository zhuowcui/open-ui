# Border side corners left unmitered as Blink leaves them, 31 October 2026

A dotted, dashed or double border side is now clipped at a corner only
where Blink's `BoxBorderPainter::ComputeMiter` gives that corner a miter.
Where the adjacent side has no width, or is a solid side that paints later
and covers the corner, the stroked side runs to the border box's edge
instead of stopping at the diagonal. The census goes from 22,291 to 22,292
exact, with 9 more comparisons improved, no losses and no comparison worse.

## Finding

`BoxBorderPainter` (`core/paint/box_border_painter.cc`, 147.0.7727.50)
paints mixed sides in `ComplexBorderInfo` order: by alpha, then by style
priority (dotted, dashed and double before 3D styles before solid), then by
side (top, bottom, right, left). Before it paints a side,
`ComputeMiter` decides each of the side's two corners:

- no miter when the adjacent side has no width;
- no miter when `WillOverdraw` holds: the adjacent side is painted later and
  its style fills the border area, which is every style but dotted, dashed
  and double;
- a soft miter when the colours differ at the corner;
- a hard miter when `BorderStylesRequireMiter` holds: either side is double,
  the adjacent side is groove or ridge, or the styles differ;
- otherwise no miter.

`ClipBorderSidePolygon` clips the side only at a corner that has a miter.
Where one corner has a miter and the other has none, the clip runs to the
side's bounding box at the unmitered end. Open UI clipped a dotted, dashed
or double side at both diagonals whatever its neighbours were. In
`ttwf-css3background-border-style` at 1x, a 5px black box with a solid
top, a dotted right, a dashed bottom and a double left, the dotted side's
first dot was cut at the top-right diagonal. Chrome draws that dot whole,
and the solid top side paints over it later.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A dotted, dashed or double side's clip polygon reaches the border box's edge at a corner where `ComputeMiter` gives no miter: the adjacent side has no width or is a later side whose style fills the border area, or the colours match and the styles need no miter. | `openui-paint` | `painter.rs` `paint_borders` |

The unit test `dotted_side_is_unmitered_where_a_later_solid_side_overdraws_it`
paints that test's box at 1x and checks four pixels of the dotted side's
first dot against Chrome's values. It fails without the change: pixel
(179, 21) is then 47 instead of 56.

## Measurement

This is a paired expanded census in WSL. The base is `c57ad017`, whose
rendering equals `main` at `d4d87179`. The head is `e70a5213`; the
following commit adds only the unit test. Both use one shared Chromium
cache, and Chromium is identical in every comparison.

| Result | Comparisons |
| --- | --- |
| Exact | 22,291 to 22,292 |
| Gained, 1 | `background-clip_clip-border-area-ref` at 1x, from 13 mismatched pixels |
| Improved, 9 | `ttwf-css3background-border-style`, `-shorthand` and `-shorthand-missing-left`: at 1x, 4 to 2 each; at 1.25x, 8 to 6, 8 to 6 and 3 to 1; at 1.5x, 9 to 6, 9 to 6 and 6 to 3 |
| Lost, worsened | none |

The rows are in the
[census evidence](evidence/border-corner-miters-census-v1.json).

### Remaining failures

- In the three `ttwf-css3background-border-style` comparisons at 1x, the 2
  remaining pixels are in the dotted right side's last dot, next to the
  bottom corner, and Open UI's are slightly darker. The dots are placed by
  Blink's `SelectBestDashGap` arithmetic, and drawing them as a path rather
  than a line gives the same pixels. The cause was not established; a
  difference in Skia's round-cap stroking is possible. The owner is
  `openui-paint`.
- The mismatched pixels left at 1.25x and 1.5x were not attributed in this
  change.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,291 to 22,292 exact; 1 gained, 9 improved, 0 worsened, 0 losses |
| Other raster configurations | rerender of every comparison in `default`, `deterministic-alias`, `deterministic-alias-subpixel` and `chromium-linux-lcd` | the same gain and 9 improvements, the gain in `default` and the improvements in `deterministic-alias`, at `ac996b11`; no mismatch count changed in `deterministic-alias-subpixel` or `chromium-linux-lcd` |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint` | 8,332 passed, 0 failed, 2 ignored, at `ac996b11` |
| Windows workspace | `cargo test --workspace --locked`, with the `windows-msvc` CI environment | 9,495 passed, 0 failed, 20 ignored, at `ac996b11` |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
