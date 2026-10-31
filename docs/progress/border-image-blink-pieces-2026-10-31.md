# Border-image pieces in Blink's order, 31 October 2026

A border image's nine pieces are now painted in Blink's `NinePiece` order,
and a corner, or a piece stretched on both axes, is drawn as one
antialiased image rect, as Blink's `NinePieceImagePainter` does. The census
goes from 22,243 to 22,262 exact, with 6 more comparisons improved, no
losses and no comparison worse.

## Finding

`NinePieceImagePainter::Paint`
(`core/paint/nine_piece_image_painter.cc`, 147.0.7727.50) walks the pieces
in `NinePiece` enum order (`core/style/nine_piece_image.h`): top left,
bottom left, left, top right, bottom right, right, top, bottom, then
middle. A piece that is not tiled on either axis goes to
`GraphicsContext::DrawImage`, which issues one `drawImageRect` with
antialiased fill flags, the default `kClampImageToSourceRect` constraint and
linear sampling (`platform/graphics/graphics_context.cc`).

Open UI painted the pieces row by row and drew an untiled piece through the
physical quantized patch path, which snaps the destination to device pixels
and draws an analytic edge. At 1.25x the grid's edge widths are whole CSS
pixels, so seams between pieces fall on half device pixels. There Chrome
blends two antialiased edges over the background, and the result depends on
which piece is drawn first. In `border-image-017` at 1.25x, the pixel where
the top-left corner meets the top edge is 63, 96, 0 in Chrome's capture.
Open UI's snapped corner covered the whole pixel instead. Where the left
edge meets the bottom-left corner, Chrome's pixel is 46, 104, 0 because the
corner is painted before the edge; row by row, the edge came first.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A piece that is not tiled on either axis is drawn with one `draw_image_rect`: antialiased, `SrcRectConstraint::Strict` and linear sampling. | `openui-paint` | `painter.rs` `draw_border_image_patch` |
| The nine pieces are painted in `NinePiece` order: the left column, the right column, then top, bottom and middle. | `openui-paint` | `painter.rs` `paint_border_image` |

The unit test `border_image_seams_blend_like_blink_nine_piece_painter`
paints `border-image-017`'s green pieces over red at 1.25x and checks both
seam pixels against Chrome's values. It fails at the corner seam (red 97)
with the old corner path, and at the edge seam (red 47) with the old row
order.

## Measurement

This is a paired expanded census in WSL. The base is `020627d5`, whose
rendering equals `main` at `e5d4f6aa`. The head is `6b6e7ff5`; the
following commit adds only the unit test. Both use one shared Chromium
cache, and Chromium is identical in every comparison.

| Result | Comparisons |
| --- | --- |
| Exact | 22,243 to 22,262 |
| Gained, 19 | at 1.25x: `border-image-shorthand-001`, `-002` and `-003`, 212, 797 and 852 mismatched pixels; `border-image-slice-005` and `-007`, 4,191 each; `border-image-width-005` and `-006`, 182 each. At 1.5x: `border-image-017` to `-020` and `border-image-slice-001` and `-002`, 75 each; `border-image-shorthand-001` and `-003`, 32 and 279; `border-image-width-005` and `-006`, 2 each. `border-image-shorthand-001` at 1x and 2x, 24 and 112 |
| Improved, 6 | `border-image-017` to `-020` and `border-image-slice-001` and `-002` at 1.25x, 311 to 124 each |
| Lost, worsened | none |

The rows are in the
[census evidence](evidence/border-image-blink-pieces-census-v1.json).

### Remaining failures

- `border-image-017` to `-020` and `border-image-slice-001` and `-002` at
  1.25x keep 124 mismatched pixels each, in rows 43 and 106 from x 63 to
  124. They lie where the top and bottom border-image edges meet the
  content `<img>` of `50x50-green.png`. A Skia probe of that seam with a
  plain image rect gives Chrome's 47, 104, 0, so the difference is in Open
  UI's replaced-image painting, the physical quantized patch path. The
  owner is `openui-paint`.
- 26 other border-image comparisons remain inexact, all but one at 1.25x
  and 1.5x: `border-image-021`, the `outset`, `repeat`, `round` and
  `slice-percentage` tests, `border-image-slice-004` and `-006`,
  `border-image-width-001`, and `border-image-image-type-002` at 2x. Most
  have tiled pieces, which still use Open UI's own tiling rather than
  Blink's `DrawImageTiled` picture shader. They were not attributed in this
  change. The owner is `openui-paint`.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,243 to 22,262 exact; 19 gained, 6 improved, 0 worsened, 0 losses |
| Other raster configurations | rerender of every comparison in `default`, `deterministic-alias`, `deterministic-alias-subpixel` and `chromium-linux-lcd` | the same 19 gains and 6 improvements, 4 of the gains in `default` and the rest in `deterministic-alias`, at `3c3d53ae`; no mismatch count changed in `deterministic-alias-subpixel` or `chromium-linux-lcd` |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint` | 8,330 passed, 0 failed, 2 ignored, at `3c3d53ae` |
| Windows workspace | `cargo test --workspace --locked`, with the `windows-msvc` CI environment | 9,493 passed, 0 failed, 20 ignored, at `3c3d53ae` |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
