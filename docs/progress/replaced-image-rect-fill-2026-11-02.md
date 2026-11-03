# Unclipped replaced images as Skia's antialiased rect fill, 2 November 2026

A replaced image that no clip bounds is now drawn as Blink draws it: its
full samples, through one antialiased rect fill at the fractional
destination. Skia's raster blitter then applies the edge coverage. The
census goes from 22,335 to 22,382 exact, with 6 more comparisons improved.
No comparison is lost or worse.

## Finding

At 1.25x and 1.5x, `multicol-rule-color-001-ref`,
`multicol-rule-fraction-001-ref` and many image tests had edge pixels one
level away from Chrome. Each was a pixel that an `<img>` edge covers only
partly.

Blink paints an unclipped replaced image with one antialiased
`drawImageRect`. `SkBitmapDevice::drawImageRect` turns that into
`drawRect` with an image shader, which `SkScan::AntiFillRect` rasterizes.
An edge gets coverage `256 - (L & 0xFF)` at the start and `R & 0xFF` at
the end, in 24.8 fixed point (`SkFixedToFDot8`). Chromium's Skia defines
`SK_ENABLE_LEGACY_SHADERCONTEXT`, so an N32 device uses the legacy
`SkARGB32_Shader_Blitter`. For an opaque image it shades into the device
and blends a partly covered column with `SkFourByteInterp(src, dst,
coverage)`, which is `dst + ((src - dst) * (coverage + 1) >> 8)`. The edge
is a lerp from the device colour toward the image, not a source-over of a
pre-multiplied coverage value.

Open UI baked the edge coverage into the image patch and composited it
source-over. That is one level off for most device colours. Chrome gives
these values, which the unit tests check:

- An opaque blue image at x = 69 to 81 CSS pixels at 1.25x, over gray 192,
  covers device column 101 by a quarter. Chrome draws (143, 143, 207).
- A blue image at x = 120 to 125 and then a yellow image at x = 125 to 160,
  at 1.25x over white, share device column 156. Chrome draws
  (239, 239, 62). Source-over of baked coverage gives 238.

A first version kept the baked patch and used Skia's coverage formula in
it. The census lost 47 comparisons with that version, because source-over
cannot reproduce the lerp. Drawing the full-sample patch through Skia's
own rect fill lets the blitter do the blend, as it does in Chrome.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A replaced image patch with neither full-precision sampling nor clip coverage bounds keeps full samples at its edge texels. Its geometric coverage is 255 for any texel that Skia's rect fill touches, and 0 otherwise. | `openui-paint` | `bindings/rust/openui-paint/src/image_resource.rs` (`replaced_image_patch_draws_as_skia_rect_fill`, `physical_quantized_image_patch_with_color_order`) |
| `paint_replaced_content` draws that patch with antialiasing as one rect at the fractional device destination, instead of a non-antialiased rect at the aligned destination. A clipped image and a full-precision (JPEG) image keep the existing edge model. | `openui-paint` | `bindings/rust/openui-paint/src/painter.rs` (`paint_replaced_content`) |

The unit test `unclipped_replaced_image_patch_keeps_full_edge_samples`
(`image_resource.rs`) checks that the edge texels of an unclipped patch keep
the image's full value, for an opaque and a translucent image. The test
`unclipped_replaced_image_edges_lerp_over_the_device_like_chromium` draws
the two cases above through the same shader and rect fill as the painter
and checks Chrome's values, (143, 143, 207) and (239, 239, 62). Its
surfaces are 20 rows tall, because Skia scales a rect one scanline tall by
255/256.

## Measurement

This is a paired expanded census in WSL. The base is `580272cb`, whose
rendering equals `main` at `0099fcfb`. The head is `547fcbac`. Both are
built with the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium is
identical in all 23,728 comparisons. Open UI's output changed in 55.

| Result | Comparisons |
| --- | --- |
| Exact | 22,335 to 22,382 |
| Gained, 47 | Replaced images in `css_backgrounds` (`border-image-017` to `-020`, `border-image-slice-001` and `-002`, `border-radius-013`), `css_break`, `css_flexbox`, `css_multicol`, `css_overflow` and `css_sizing`, at 1.25x and 1.5x |
| Improved, 6 | `box-shadow_box-shadow-blur-definition-001` (1,030 to 359), `css-flexbox-img-expand-evenly` (1,002 to 624) and `overflow-replaced-element-002` with its `-ref` (2,702 to 2,257) at 1.25x; `flexbox-min-width-auto-006` at 1.25x (626 to 251) and 1.5x (602 to 301) |
| Changed, same count | `border-image-outset-004` at 1.25x (496) and `flex-minimum-height-flex-items-022` at 1.25x (125) |
| Lost, worsened | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/replaced-image-rect-fill-census-v1.json).

### Remaining failures

- `box-sizing-replaced-002` and its `-ref` still have 733 mismatched pixels
  at 1.5x and 1,720 at 2x. They were not examined after this change. The
  owner is `openui-paint`.
- `overflow-replaced-element-002` and its `-ref` at 1.25x (2,257),
  `css-flexbox-img-expand-evenly` at 1.25x (624),
  `box-shadow_box-shadow-blur-definition-001` at 1.25x (359) and
  `flexbox-min-width-auto-006` (251 at 1.25x, 301 at 1.5x) improved but are
  not exact. They were not examined.
- `multicol-rule-002` and `multicol-rule-fraction-003` still have 192
  mismatched pixels at 1.5x. They are glyph rows cut at the column
  fragment's clip, one device row apart from Chrome (126 against 125). The
  owner is the column fragment clip in `openui-paint`.
- `multicol-span-all-block-sibling-003` still has 1 or 2 mismatched pixels
  at 1.25x, 1.5x and 2x. They were not examined.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,335 to 22,382 exact; 47 gained, 6 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,339 passed, 0 failed, 2 ignored, at `547fcbac` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,501 passed, 0 failed, 21 ignored, at `547fcbac` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
