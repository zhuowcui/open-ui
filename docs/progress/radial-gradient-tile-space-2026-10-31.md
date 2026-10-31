# Radial gradient shaders in the tile's own space, 31 October 2026

A radial gradient background is now built in its tile's own coordinates and
mapped onto the tile with a local matrix, as Blink's
`GradientGeneratedImage::Draw` does. Its end shape is sized with Blink's
`RadiusToSide` and `RadiusToCorner` arithmetic, and length stops are
measured along the horizontal radius. The census goes from 22,236 to
22,241 exact, with 3 more comparisons improved, no losses and no comparison
worse.

## Finding

`GradientGeneratedImage::Draw`
(`platform/graphics/gradient_generated_image.cc`, 147.0.7727.50) passes
`SkMatrix::RectToRect(src_rect, dest_rect)` to `Gradient::ApplyToFlags`. The
gradient itself is created in the tile's space, with its centre at, for
example, (100, 75) in a 200x150 tile. `RadialGradient::CreateShader`
(`gradient.cc`) then pre-scales an ellipse by `1 / aspect_ratio` about that
centre. Open UI built the shader with the centre in page coordinates and no
tile translation. Skia then maps each pixel to the gradient through a
different matrix, which moves scattered pixels of a smooth gradient by one
level. `background-image-centered` at 2x had 11 such pixels.

`CSSRadialGradientValue::CreateGradient` (`css_gradient_value.cc`) sizes the
end shape with `RadiusToSide` and `RadiusToCorner`. A circle sized to a
corner takes the Euclidean distance to that corner, and an ellipse keeps
its side radii's aspect ratio through the corner (`EllipseRadius`). Its
stop lengths are measured against `desc.r1`, the horizontal radius. Open UI
scaled the side radii by the square root of 2 and measured stops against the
larger radius. Those agree with Blink only for a centred box, or for an
ellipse that is wider than it is tall.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The radial gradient shader takes its centre in the tile's space and a local matrix that translates to the tile. An ellipse's matrix is then pre-scaled by the reciprocal of its aspect ratio about the centre. | `openui-paint` | `painter.rs` `paint_css_image_tile` |
| Side and corner end shapes are sized as Blink's `RadiusToSide`, `RadiusToCorner` and `EllipseRadius` do. Explicit sizes are unchanged. | `openui-paint` | `painter.rs` `radial_gradient_end_radii` |
| Length stops and hint positions are measured along the horizontal radius. | `openui-paint` | `painter.rs` `paint_css_image_tile` |

The unit test `radial_end_radii_follow_blink_side_and_corner` checks a
circle sized to an off-centre corner, a circle sized to its closest side,
and a centred ellipse sized to its farthest corner. The unit test
`radial_length_stops_follow_the_horizontal_radius` paints a 20x40 ellipse
with a red-to-lime transition between 9px and 11px. The pixel 7px right
of the centre must be red, and 13px right must be lime. It fails when stops
are measured against the larger radius.

## Measurement

This is a paired expanded census in WSL. The base is `2e196ef9`, whose
rendering equals `main` at `955f406b`. The head is `74cf25f0`. Both use one
shared Chromium cache, and Chromium is identical in every comparison.

| Result | Comparisons |
| --- | --- |
| Exact | 22,236 to 22,241 |
| Gained, 5 | `background-attachment-fixed-inside-transform-1` and its `-ref` at 1x and 1.25x, from 1 to 3 mismatched pixels; `intrinsic-percent-replaced-023` at 1.5x, from 2 |
| Improved, 3 | `background-image-centered` and its `-ref` at 2x: 11 to 5. `background-image-gradient-currentcolor-radial-repaint-ref` at 2x: 2 to 1. |
| Changed, same count, 2 | `background-attachment-fixed-inside-transform-1` and its `-ref` at 1.5x, 1 pixel each |
| Lost, worsened | none |

The rows are in the
[census evidence](evidence/radial-gradient-tile-space-census-v1.json).

### Remaining failures

- `background-image-centered` and its `-ref` keep 2 mismatched pixels at
  1x, 1 each at 1.25x and 1.5x, and 5 at 2x, each one level off. Four of
  the five 2x pixels lie outside the first 256-pixel tile. They may come
  from cc's per-tile raster translation, which Open UI does not model. That
  was not established.
- The census has no tall ellipse with length stops and no circle sized to
  an off-centre corner, so the sizing and stop-length corrections are
  covered only by the unit tests.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,236 to 22,241 exact; 5 gained, 3 improved, 0 worsened, 0 losses |
| Other raster configurations | rerender of every comparison in `default`, `deterministic-alias`, `deterministic-alias-subpixel` and `chromium-linux-lcd` | the same 5 gains and 3 improvements, at `b91c0c48`; no mismatch count changed in `deterministic-alias-subpixel` or `chromium-linux-lcd` |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint` | 8,327 passed, 0 failed, 2 ignored, at `b91c0c48` |
| Windows workspace | `cargo test --workspace --locked`, with the `windows-msvc` CI environment | 9,490 passed, 0 failed, 20 ignored |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
