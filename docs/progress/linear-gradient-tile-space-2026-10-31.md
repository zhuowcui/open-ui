# Linear gradient shaders in the tile's own space, 31 October 2026

A linear gradient background is now built in its tile's own coordinates and
mapped onto the tile with a local matrix, as Blink's
`GradientGeneratedImage::Draw` does. Its line comes from Blink's
`EndPointsFromAngle`, and a stop list that starts below 0% or ends above
100% is normalized as Blink's `NormalizeAndAddStops` does. The census goes
from 22,241 to 22,243 exact, with 17 more comparisons improved, no losses
and no comparison worse.

## Finding

`GradientGeneratedImage::Draw`
(`platform/graphics/gradient_generated_image.cc`, 147.0.7727.50) passes
`SkMatrix::RectToRect(src_rect, dest_rect)` to `Gradient::ApplyToFlags`, so
the gradient line is placed in the tile's space and translated onto the
tile by the shader's local matrix. Open UI placed the line in page
coordinates with no local matrix. Skia then maps each pixel to the gradient
through a different matrix, which moves scattered pixels of a smooth
gradient by one level. The radial gradient path had the same fault; see the
[radial gradient record](radial-gradient-tile-space-2026-10-31.md).
`background-repeat-space-2` at 2x tiles `linear-gradient(to top left, red,
green)` at 32px and had 39 such pixels.

`CSSLinearGradientValue::CreateGradient` (`css/css_gradient_value.cc`)
lowers a corner direction to an angle with `90 - Rad2deg(atan2(rise, run))`
and computes the line with `EndPointsFromAngle`. `Gradient` then normalizes
the stops when `RequiresStopsNormalization` holds: the gradient repeats, the
first offset is below 0 or the last is above 1. `NormalizeAndAddStops`
redistributes the offsets over [0, 1], and
`AdjustedGradientDomainForOffsetRange` moves the line's ends to the first
and last stops. Open UI normalized only a repeating gradient. For
`red -100%, lime 200%`, Skia clamps the offsets to [0, 1], so the box's left
edge was red instead of a third of the way to lime.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The linear gradient line is computed in the tile's space with Blink's `EndPointsFromAngle` arithmetic, and a corner direction is lowered to its angle first. | `openui-paint` | `painter.rs` `linear_gradient_end_points` |
| All three linear shader paths take a local matrix that translates the tile's space to the tile. | `openui-paint` | `painter.rs` `paint_css_image_tile` |
| A repeating or out-of-range stop list is normalized over [0, 1], and the line moves to its first and last stops. | `openui-paint` | `painter.rs` `paint_css_image_tile` |

The unit test `linear_end_points_follow_blink_end_points_from_angle` checks
a cardinal angle, a 45 degree line in a 40x20 box and `to bottom right` in
the same box. The unit test
`linear_out_of_range_stops_extend_the_gradient_line` paints
`linear-gradient(90deg, red -100%, lime 200%)` into a 30x10 box. Its left
edge must be about two thirds red and one third lime. It fails when only a
repeating gradient is normalized: the edge is then 251, 4, 0.

## Measurement

This is a paired expanded census in WSL. The base is `74cf25f0`, whose
rendering equals `main` at `fc084938`. The head is `020627d5`. Both use one
shared Chromium cache, and Chromium is identical in every comparison.

| Result | Comparisons |
| --- | --- |
| Exact | 22,241 to 22,243 |
| Gained, 2 | `background-repeat-space-2` and its `-ref` at 2x, from 39 mismatched pixels each |
| Improved, 17 | at 2x: `background-repeat-space-1a`, `-1b`, `-1c`, `-4` and `-5`, 252 to 54 each; `-1-ref`, 122 to 38; `-3`, 168 to 36; `-3-ref`, 35 to 27; `-4-ref`, 142 to 9; `-5-ref`, 94 to 15. At 1.25x: `background-repeat-space-6` and `-7` and their `-ref`s, for example 3,711 to 3,626; `table-col-paint-vrl-rtl`, `table-row-paint-vrl-rtl` and `table-section-paint-vrl-rtl`, 1,094, 1,095 and 1,182 to 1,008 each |
| Lost, worsened | none |

The rows are in the
[census evidence](evidence/linear-gradient-tile-space-census-v1.json).

### Remaining failures

- The improved `background-repeat-space` and table comparisons keep the
  mismatched pixels listed above. They were not attributed in this change.
  The repeat-space tests also draw a raster image tile with
  `background-repeat: space`.
- `background-gradient-interpolation-001`, its `-notref` and
  `background-image-gradient-interpolation-repaint-ref` are unchanged.
  Their mismatched pixels lie in one row, every eighth column, where the
  dither threshold sits. Blink converts each stop colour to the
  interpolation space and back (`ConvertToColorSpaceForInterpolation`,
  `ToGradientStopSkColor4f`), and Open UI passes the stop colours unchanged.
  That difference was not established as the cause, and the owner is
  `openui-paint`.
- The census has no linear gradient with out-of-range stops, so that
  correction is covered only by the unit test.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,241 to 22,243 exact; 2 gained, 17 improved, 0 worsened, 0 losses |
| Other raster configurations | rerender of every comparison in `default`, `deterministic-alias`, `deterministic-alias-subpixel` and `chromium-linux-lcd` | the same 2 gains and 17 improvements, at `0493abfa`; no mismatch count changed in `deterministic-alias-subpixel` or `chromium-linux-lcd` |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint` | 8,329 passed, 0 failed, 2 ignored, at `0493abfa` |
| Windows workspace | `cargo test --workspace --locked`, with the `windows-msvc` CI environment | 9,492 passed, 0 failed, 20 ignored |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
