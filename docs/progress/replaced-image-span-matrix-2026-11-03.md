# Replaced image sampling through Chromium's matrix and shader spans, 3 November 2026

A replaced image's bilinear sample positions now come from the same matrix
and the same shader spans that Chromium's Skia uses. The census goes from
22,382 to 22,384 exact, with 7 more comparisons improved. No comparison is
lost or worse.

## Finding

`box-sizing-replaced-002` and its `-ref` had 1,720 mismatched pixels at 2x
and 733 at 1.5x. `box-sizing-replaced-001` and `-003` had 364 and 887 at
1.5x. Each image is a small source scaled up into a large box. Mismatched
pixels came in bands, with Open UI's bilinear weight one sixteenth off
Chrome's.

Blink calls `drawImageRect(src, dst)` with `dst` in CSS pixels, under the
compositor tile's device-scale and tile-origin matrix. Skia concatenates
`RectToRect(src, dst)` with that matrix and inverts the result in `f32`.
The step is `1 / (device_scale * (dst / src))`. At 1.5x with a 125 to 75
scale, that is the inverse of `0.90000004`, not of `0.9`. Open UI inverted
`physical_extent / src`, which rounds differently.

`SkBitmapProcShader` maps the start of each span through the inverse and
then accumulates the 32.32 fixed step per pixel. The rounding drift resets
at every span start. In Chromium the spans are:

- each partly covered edge column of `SkScan::AntiFillRect`, which shades
  alone;
- the interior, from `ceil(left)` to `floor(right)` in 24.8 fixed point,
  clipped to the compositor raster tile (254 device pixels after the first
  255);
- chunks of 127 pixels inside it, because `shadeSpan` fills a 128-word
  buffer and `maxCountForBufferSize` gives `(512 - 4) / 2` coordinates, two
  per bilinear pixel.

Open UI accumulated from the destination start without span breaks. It also
had two special cases fitted to earlier tests: an early return for quarter
origins, and a closure quantum for mip-decoded images. Both are removed. A
mip decode is drawn the same way, with Blink's scaled source rect.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The sample position is the composed `f32` matrix of the CSS destination, the source rect, the device scale and the tile origin, inverted as Skia inverts it. The quarter-origin and mip-closure special cases are removed. | `openui-paint` | `bindings/rust/openui-paint/src/image_resource.rs` (`replaced_sample_fixed_at`) |
| Horizontal accumulation restarts at each `AntiFillRect` edge column, at the interior start, at each compositor tile origin and every 127 pixels. | `openui-paint` | `bindings/rust/openui-paint/src/image_resource.rs` (`legacy_shader_span_start`, `LEGACY_BILERP_SPAN_CHUNK`) |

The `replaced_mip_decoded` argument of
`physical_quantized_image_patch_with_color_order` is now unused. It is
still passed by its callers, and can be removed in a later cleanup.

The unit test `replaced_sample_follows_chromium_composed_matrix_and_shader_spans`
checks Chrome's 4-bit bilinear weights at 1.25x, 1.5x and 2x. The 1.25x and
1.5x cases depend on the `f32` rounding of the composed step. The 2x case
enters a raster tile at 508 and crosses the 127-pixel chunk boundary at 635.
The test
`legacy_shader_spans_split_at_edges_tiles_and_buffer_chunks` checks the span
starts for an edge column, a rect inside one device column, a tile origin
and a chunk boundary. They replace
`replaced_sample_preserves_direct_and_mip_half_phase_closure`. The other
replaced-image tests pass unchanged.

## Measurement

This is a paired expanded census in WSL. The base is `547fcbac`, whose
rendering equals `main` at `5af90c2b`. The head is `eb3f2a51`. Both are
built with the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium is
identical in all 23,728 comparisons. Open UI's output changed in 9.

| Result | Comparisons |
| --- | --- |
| Exact | 22,382 to 22,384 |
| Gained, 2 | `box-sizing-replaced-002` and its `-ref` at 2x (1,720 to 0) |
| Improved, 7 | `box-sizing-replaced-001` and its `-ref` (364 to 4), `-002` and its `-ref` (733 to 3) and `-003` and its `-ref` (887 to 3) at 1.5x; `box-shadow_box-shadow-blur-definition-001` at 1.25x (359 to 3) |
| Lost, worsened | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/replaced-image-span-matrix-census-v1.json).

### Remaining failures

- `box-sizing-replaced-002` and its `-ref` at 1.5x keep 3 pixels, at
  (481, 181), (148, 328) and (481, 328). They are corner pixels of the other
  images, and they were already mismatched in the base. `-001` (4) and
  `-003` (3) at 1.5x were not examined. The owner is `openui-paint`.
- `box-shadow_box-shadow-blur-definition-001` at 1.25x keeps 3 pixels, at
  (362, 148), (377, 148) and (362, 161), each one level off. They were not
  examined further.
- `overflow-replaced-element-002` and its `-ref` at 1.25x (2,257),
  `css-flexbox-img-expand-evenly` at 1.25x (624) and
  `flexbox-min-width-auto-006` (251 at 1.25x, 301 at 1.5x) did not change.
  They were not examined.
- The column fragment clip residuals in `multicol-rule-002` and
  `multicol-rule-fraction-003` (192 at 1.5x) and
  `multicol-span-all-block-sibling-003` (1 or 2) are unchanged.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,382 to 22,384 exact; 2 gained, 7 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,340 passed, 0 failed, 2 ignored, at `eb3f2a51` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,502 passed, 0 failed, 21 ignored, at `eb3f2a51` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
