# Fractional device edges of a border-box raster background, 30 October 2026

A raster background clipped to the border box now keeps partial coverage
where the box edge falls inside a device pixel, as Blink does. At 1.25x and
1.5x, `background-repeat-space` references, `background-repeat` tests and
`attachment-local-clipping-image-6-ref` improve in 22 comparisons. The
census stays at 22,232 exact, with no losses and no comparison worse.

## Finding

Blink's `BoxPainterBase::PaintFillLayer`
(`core/paint/box_painter_base.cc`, 147.0.7727.50) clips a fill layer to
`ToPixelSnappedRect` only for `padding-box` and `content-box`. For
`border-box` it applies no clip. The image is drawn with antialiasing into
`BackgroundImageGeometry::SnappedDestRect`, which is snapped in CSS pixels.
The census captures lay out and snap in CSS pixels and then scale, so at
1.25x a box edge at CSS x=38 lands at device x=47.5. A Chrome 147 capture
at 1.25x of image, color and gradient boxes at such edges showed half
coverage in the edge column.

For a raster border-box layer, Open UI's `paint_background_layers` set a
non-antialiased clip at the CSS rectangle. Skia rounds that clip to the
nearest device pixel edge, which cut the edge column at its centre and
left it background-coloured. An antialiased clip there instead
double-applied coverage: it took `background-repeat-space-1-ref` at 1.25x
from 2,576 to 3,042 mismatched pixels.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A raster layer whose clip is the border box clips without antialiasing to its rectangle rounded outward to whole device pixels, so the antialiased image draw owns the edge coverage. | `openui-paint` | `painter.rs` `paint_background_layers` |
| A `local` image in a scroll container keeps its scrollport clip, because Blink paints it in the scrolling contents clipped to the padding box. | `openui-paint` | `painter.rs` `paint_background_layers` |

Rounded, transformed, generated-image and physical-patch clips are
unchanged, as are padding-box and content-box clips.

The unit test `border_box_raster_keeps_fractional_device_edge_coverage`
paints a 32px image at CSS x=38 at 1.25x. Device column 47 must be neither
the background nor the full image colour, and column 46 must be the
background. The test fails with the change disabled.

## Measurement

This is a paired expanded census in WSL. The base is `577c8887`, whose
rendering equals `main` at `794bca37`. The head is `55663999`. Both use
one shared Chromium cache, and Chromium is identical in every comparison.

| Result | Comparisons |
| --- | --- |
| Exact | 22,232 to 22,232 |
| Improved, 22 | `background-repeat-space-1-ref` and `-3-ref` to `-7-ref`: about 15 to 21 fewer mismatched pixels at 1.25x and 190 to 520 fewer at 1.5x. `background-repeat-space-3` at 1.5x: 392 to 294. `attachment-local-clipping-image-6-ref`: 1,257 to 822 at 1.25x, 1,707 to 1,470 at 1.5x. `background-repeat-no-repeat`: 222 to 102 at 1.25x, 145 to 1 at 1.5x. `background-repeat-repeat-y` at 1.25x: 240 to 124. `background-repeat-repeat-x` and `background-repeat-space` by 2 to 24 pixels. |
| Changed, same count, 5 | `background-size-contain` and `-cover` at 1.25x and 1.5x, `background-repeat-repeat-y` at 1.5x |
| Gained, lost, worsened | none |

The rows are in the
[census evidence](evidence/background-border-box-raster-clip-census-v1.json).
A first version without the scrollport exception made
`background-attachment-local-scrolling` at 1.25x worse, 954 to 971, in the
`deterministic-alias` profile. With the exception, a rerender of every
comparison in the `default`, `deterministic-alias`,
`deterministic-alias-subpixel` and `chromium-linux-lcd` configurations
had no comparison worse than the base.

### Remaining failures

- Most of the remaining `background-repeat-space-*` mismatch is inside the
  tiles: Open UI's bilinear samples at fractional tile phases differ from
  Chromium's. The likely owner is the physical patch sampling in
  `openui-paint` `image_resource.rs`. It was not attributed in this change.
- The gradient tiles in the same references are unchanged. Gradient layers
  keep the half-open physical clip.
- `background-size-contain` and `-cover` changed pixels without changing
  their mismatch counts. Their remaining mismatch was not attributed.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,232 to 22,232 exact; 22 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint` | 8,323 passed, 0 failed, 2 ignored |
| Paint crate on Windows | `cargo test --locked -p openui-paint`, with the `windows-msvc` CI environment | 254 unit tests and every integration test passed, 0 failed |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,486 passed, 0 failed, 20 ignored |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
