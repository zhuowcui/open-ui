# Square double borders as two stroked rings, 31 October 2026

A uniform double border with square corners is now painted at every device
scale as two stroked rects, one per ring, as Blink's
`GraphicsContext::FillDRRect` does. Before, only a border whose edges fell on
whole device pixels took the ring path; at 1.25x and 1.5x it was painted as
four mitered sides. The census goes from 22,262 to 22,291 exact, with 6 more
comparisons improved, no losses and no comparison worse.

## Finding

`BoxBorderPainter::DrawDoubleBorder`
(`core/paint/box_border_painter.cc`, 147.0.7727.50) paints a uniform
double border as two rings through `DrawBleedAdjustedDRRect`, at any scale.
Each ring's inner edge is the border box inset by the stripe width,
`roundf(UsedWidth() / 3)` from `BorderEdge::GetDoubleBorderStripeWidth`
(`core/style/border_edge.cc`). `GraphicsContext::FillDRRect`
(`platform/graphics/graphics_context.cc`) then checks `IsSimpleDRRect`: all
four sides have the same width and every corner is square or isotropic. A
simple ring is drawn as one rect stroked along its centre line, with the
ring's width as the stroke width, not as a `drawDRRect`.

Open UI took its ring path only when every edge of the box, the border and
the stripe fell on a whole device pixel. At 1.25x and 1.5x a square double
border was then painted as four sides clipped at the miter. Their inner
corners leave the diagonal to two separate antialiased edges, so the corner
pixels differed from Chrome's. With the gate removed, filling the ring with
`draw_drrect` still left 463 pixels one level off on the right and bottom
edges of `border-image-repeat-004` at 1.25x. Stroking the ring's centre
line, as `FillDRRect` does, matches.

In `border-image-repeat-002` and `-004`, `border-image-repeat-round`,
`border-image-round-and-stretch` and `border-image-slice-percentage`,
Chrome's capture shows the red `double` border with no border image over it,
and so does Open UI. Their mismatches were these double-border corners. Why
the image does not paint in those captures was not established.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A uniform square double border takes the ring path at every device scale; the whole-device-pixel gate is removed. | `openui-paint` | `painter.rs` `paint_borders` |
| Each ring is one rect stroked along its centre line, inset by half the stripe width, with the stripe width as the stroke width. | `openui-paint` | `painter.rs` `paint_borders` |

The unit test `fractional_uniform_double_border_strokes_two_rings_like_blink`
paints a 40px red double border at 1.25x into a 180x180 box at (10, 10).
It checks the outer ring's corner pixel, its inside, its inner edge, and
the inner ring's two edges against the stroked rings' coverage. It fails
without the change: the corner pixel (12, 12) is then white.

## Measurement

This is a paired expanded census in WSL. The base is `6b6e7ff5`, whose
rendering equals `main` at the border-image merge. The head is `c57ad017`;
the following commit adds only the unit test. Both use one shared Chromium
cache, and Chromium is identical in every comparison.

| Result | Comparisons |
| --- | --- |
| Exact | 22,262 to 22,291 |
| Gained, 29 | at 1.25x: `background-repeat-space-padding-box`, `background_repeat_space_border_box` and `_content_box`, 833 to 836 mismatched pixels each; `border-image-outset-001` and `-002`, 19 and 14; `border-image-repeat-002` and `-004`, 56 and 88; `border-image-repeat-round`, `border-image-round-and-stretch` and `border-image-slice-percentage`, 14, 10 and 14; `border-image-slice-004` and `-006`, 67 each; `border-image-width-001`, 154; `box-shadow-039`, `-040` and `-042`, 45, 30 and 44; `ttwf-css3background-border-style-double`, 386. At 1.5x: `background-attachment-local-scrolling`, 33; the three `background-repeat-space` box tests, 6, 18 and 18; `border-image-outset-001` and `-002`, 12 and 15; `border-image-repeat-002` and `-004`, 27 each; `border-image-repeat-round`, `border-image-round-and-stretch` and `border-image-slice-percentage`, 15, 4 and 15; `ttwf-css3background-border-style-double`, 9 |
| Improved, 6 | at 1.25x: `background-attachment-local-scrolling`, 954 to 926; `background-clip_clip-border-area-ref`, 216 to 187; `box-shadow-041`, 133 to 90; `ttwf-css3background-border-style-values`, 126 to 118. At 1.5x: `background-clip_clip-border-area-ref`, 182 to 149; `ttwf-css3background-border-style-values`, 1,413 to 1,394 |
| Lost, worsened | none |

No comparison at 1x or 2x changed, where the ring path already applied.
The rows are in the
[census evidence](evidence/double-border-stroked-rings-census-v1.json).

### Remaining failures

- The improved comparisons keep the mismatched pixels listed above. They
  were not attributed in this change.
- `multicol-rule-double-000` and its `-ref` at 1.25x are unchanged, at 806
  and 1,240 mismatched pixels. A column rule is painted by a different path
  from a box border. The owner is `openui-paint`.
- The border-image comparisons in the
  [border-image record](border-image-blink-pieces-2026-10-31.md) that
  remain inexact are unchanged: `border-image-017` to `-021`,
  `border-image-slice-001` and `-002`, `border-image-outset-003`, its
  `-ref` and `-004`, `border-image-repeat-005` and `border-image-repeat-1`
  and its `-ref` at 1.25x, `border-image-repeat-005` at 1.5x and
  `border-image-image-type-002` at 2x. The owner is `openui-paint`.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,262 to 22,291 exact; 29 gained, 6 improved, 0 worsened, 0 losses |
| Other raster configurations | rerender of every comparison in `default`, `deterministic-alias`, `deterministic-alias-subpixel` and `chromium-linux-lcd` | the same 29 gains and 6 improvements, 9 of the gains in `default` and the rest in `deterministic-alias`, at `59ca0841`; no mismatch count changed in `deterministic-alias-subpixel` or `chromium-linux-lcd` |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint` | 8,331 passed, 0 failed, 2 ignored, at `59ca0841` |
| Windows workspace | `cargo test --workspace --locked`, with the `windows-msvc` CI environment | 9,494 passed, 0 failed, 20 ignored, at `59ca0841` |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
