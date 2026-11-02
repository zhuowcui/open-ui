# 3D border sides as Blink's integer halves, 1 November 2026

A straight, opaque groove, ridge, inset or outset border side is now painted
as Blink's `BoxBorderPainter::PaintOneBorderSide` paints it: in CSS pixels
under the device scale, from integer side rects, with groove and ridge split
at Blink's integer midpoint. The census goes from 22,304 to 22,308 exact,
with 3 more comparisons improved, no losses and no comparison worse.

## Finding

Chrome paints a border in CSS pixels under the device scale transform.
`PaintSide` (`core/paint/box_border_painter.cc`, 147.0.7727.50) takes the
side rect from `ToRoundedRect` of the outer border rect, cut to the side's
integer width. `PaintOneBorderSide` clips the side with
`ClipBorderSidePolygon` when a corner has a hard miter, and then draws it
with no miters. Otherwise it passes the adjacent widths of its mitered
corners to `DrawLineForBoxSide`.

`DrawRidgeOrGrooveBoxSide` splits a groove or ridge side at
`(a + b + 1) / 2` in integer CSS pixels, and draws each half as an inset or
outset side with the halved adjacent widths. A groove's outer half is
inset and its inner half outset; a ridge is the reverse. A side thinner than
2px is drawn solid. `DrawSolidBoxSide` fills a half as one antialiased rect
when neither corner is mitered, and as one antialiased quad otherwise.
Under the 1.25x and 1.5x scales, the outer edge, the midpoint and the inner
edge of a 5px side fall at different fractions of a device pixel, and each
piece blends its own edge.

Open UI painted these sides from rects in device pixels with its own
coverage rules, widened its miter clips by half a device pixel across the
side, and clipped both halves of a groove or ridge with one polygon. Its
edge rows were one or a few levels off Chrome's. In
`ttwf-css3background-border-style-values` at 1.25x, the groove box's
top-right edge pixel (1367, 57) is 196 in Chrome, and Open UI painted 203.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A straight, opaque 3D side is painted from the rounded outer rect cut to its integer width, as `PaintSide` does. | `openui-paint` | `painter.rs` `paint_blink_3d_border_side` |
| A side with a hard miter is clipped as `ClipBorderSidePolygon` clips it, with its 0.1px extensions and the bottom and left sides' swapped miters, and then drawn with no miters. Only a soft miter's clip is antialiased. | `openui-paint` | `painter.rs` `clip_blink_border_side_polygon` |
| Groove and ridge sides split at Blink's integer midpoint, with halved adjacent widths, and a side thinner than 2px is solid. Each inset, outset or solid piece is one antialiased rect, or one antialiased quad when a corner is mitered. | `openui-paint` | `painter.rs` `draw_blink_line_for_box_side` |

The existing path still paints rounded and translucent 3D borders and the
platform fieldset border.

The unit test `groove_border_paints_blinks_integer_halves_at_fractional_scale`
paints a 5px groove at 1.25x and checks six edge pixels against Chrome's
capture of `ttwf-css3background-border-style-values`. It fails without the
change: pixel (142, 7) is then 203 instead of 196.

## Measurement

This is a paired expanded census in WSL. The base is `46195d6e`, whose
rendering equals `main` at `af55a7b9`. The head is `446e554f`, which adds
only the unit test to `55b0d300`. Both use one shared Chromium cache, and
Chromium is identical in every comparison.

| Result | Comparisons |
| --- | --- |
| Exact | 22,304 to 22,308 |
| Gained, 4 | `ttwf-css3background-border-style-values` at 1.25x and 1.5x, 30 and 333 mismatched pixels; at 1.25x, `multicol-dynamic-change-inside-break-inside-avoid-001-ref`, 662, and `line-clamp_webkit-line-clamp-013`, 2 |
| Improved, 3 | at 1.25x: `multicol-rule-inset-000` and `-ridge-000`, 930 to 868; `multicol-rule-ridge-000-ref`, 1,364 to 1,240 |
| Lost, worsened | none |

The rows are in the
[census evidence](evidence/border-3d-integer-halves-census-v1.json).

### Remaining failures

- The multicol rule comparisons keep most of their mismatched pixels. A
  column rule is painted by a different path from a box border. The owner is
  `openui-paint`.
- Rounded and translucent 3D borders, and the platform fieldset border,
  keep the earlier path with Open UI's device-pixel coverage. Their effect
  was not measured separately. The owner is `openui-paint`.

The two remaining failures recorded for
`ttwf-css3background-border-style-values` and for groove or ridge sides
with a soft miter in the
[3D border miter record](border-3d-miters-2026-10-31.md) are fixed by this
change for straight, opaque borders.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,304 to 22,308 exact; 4 gained, 3 improved, 0 worsened, 0 losses |
| Other raster configurations | rerender of every comparison in `default`, `deterministic-alias`, `deterministic-alias-subpixel` and `chromium-linux-lcd` | the same 4 gains and 3 improvements, the gains in `deterministic-alias` and the improvements in `chromium-linux-lcd`, at `446e554f`; no mismatch count changed in `default` or `deterministic-alias-subpixel` |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint` | 8,334 passed, 0 failed, 2 ignored, at `446e554f` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,497 passed, 0 failed, 20 ignored, at `446e554f` |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
