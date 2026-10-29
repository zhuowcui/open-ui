# An inline box's background through a clamped single tile, 28 October 2026

An inline box with an opaque border and an empty positioning area now
paints its GIF background image, and a background image drawn as one tile
no longer samples the image outside its visible part. The census goes from
22,188 to 22,189 exact, with 7 more comparisons improved and no losses.

## Finding

`background-repeat-round-empty-crash` gives an empty inline `span` an
opaque border and a `round round` background. That background is a GIF
from a base64 data URL without its trailing `=`. Open UI painted no image,
for four reasons:

- The porter decoded base64 strictly. It rejected the unpadded payload,
  so the fixture had no image layer. Data URLs use forgiving-base64,
  which accepts it.
- `decode_image_resource` did not accept `image/gif`.
- `paint_background_layers` skipped a layer whose positioning area was
  empty. Blink's `BackgroundImageGeometry` has no such early return. Its
  destination is the painting area, and only `round` requires a nonempty
  snapped positioning area and tile.
- Open UI applied its square opaque-border bleed clip to the inline box.
  `InlineBoxFragmentPainter` paints fill layers with
  `kBackgroundBleedNone`, so Chrome applies no such clip.

With those fixed, the tile's edge columns still differed. When one tile
covers the destination, Blink's `DrawTiledBackground` takes
`OptimizeToSingleTileDraw`. It draws the visible source subset into the
snapped destination with `DrawImage`. `DrawImage` clamps sampling to that
source rect by default (`kClampToSourceRect`). Open UI's single-tile patch
sampled bilinearly across the subset's edge into hidden image columns.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The bleed clip for an opaque border is skipped for an inline box fragment. | `openui-paint` | `painter.rs` `paint_background_layers` |
| A layer with an empty positioning area still paints into its painting area. `round` applies only when the positioning area and the tile are nonempty. | `openui-paint` | `painter.rs` `paint_background_layers` |
| A single-tile draw clamps sampling to its visible source subset. | `openui-paint` | `image_resource.rs` `physical_quantized_image_patch_with_color_order` |
| GIF resources decode through Skia. | `openui-paint` | `image_resource.rs` `decode_image_resource` |
| The fixture keeps its GIF layer. | `pixel-compare` fixtures | `wpt_css_backgrounds.rs` `css_backgrounds_background_repeat_round_empty_crash` |
| The porter decodes `data:` URLs with forgiving-base64. | WPT porter | `tools/wpt/port_wpt.py` `_forgiving_base64_decode` |

The fixture builder was edited directly rather than regenerated, as in the
[bleed layer record](bleed-layer-unloaded-image-2026-10-28.md).

`single_tile_background_samples_only_its_visible_subset` in
`image_resource.rs` draws the first 3 columns of a 16x1 image. The first 3
columns are red and the rest are blue. The tile is drawn at 1.5x and
clipped to those 3 columns. The test asserts that the last visible device
column has no blue. Without the clamp it has 127.
`test_unpadded_base64_data_url_background_is_decoded` in
`tools/wpt/test_sp13p_paint_closure.py` covers the porter.

## Measurement

This is a paired expanded census in WSL. The base is `fc678b3e`, whose
rendering code is the head of the
[bleed layer change](bleed-layer-unloaded-image-2026-10-28.md). The head is
`dee14f6b`. Both use one shared Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,188 to 22,189 |
| Gained | `background-clip/clip-content-box_with_position` at 1.5x |
| Improved | `background-repeat-round-empty-crash`: 11 to 5 mismatched pixels at 1.25x, and 15 to 2 at 1.5x. `background-clip/clip-content-box`: 1,811 to 1,705 at 1.25x, 142 to 10 at 1.5x, and 2,701 to 2,303 at 2x. `clip-content-box_with_position`: 3,088 to 2,799 at 1.25x, and 3,724 to 3,387 at 2x. |
| Lost, worsened | none |

The two `background-clip` fixtures draw a `no-repeat` image clipped to the
content box, so the clamp applies to them too. Chromium is identical in
every comparison, and no other comparison changed. Rows are in the
[census evidence](evidence/inline-background-single-tile-census-v1.json).

### Remaining failures

`background-repeat-round-empty-crash` still differs at 1.25x by 5 pixels
and at 1.5x by 2 pixels. Each is one level off on a partly covered edge
pixel of the tile, such as alpha 193 against 192 at three-quarter
coverage. This is the edge-coverage quantization in `openui-paint`
`physical_patch_axis_bands` and `draw_physical_patch_with_analytic_bounds`.
It was not changed, because other exact fixtures depend on it.

The remaining `background-clip/clip-content-box*` pixels were not
attributed in this change.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,188 to 22,189 exact; 1 gain, 7 improved, 0 losses |
| Single-tile clamp test | `cargo test --locked -p openui-paint single_tile_background_samples_only_its_visible_subset` | passes; 127 blue without the clamp |
| Paint and fixture crates on Windows | `cargo test --locked -p openui-paint -p pixel-compare`, with the `windows-msvc` CI environment | 1,264 passed, 0 failed |
| Porter test | `python tools/wpt/test_sp13p_paint_closure.py` | 9 tests OK |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,484 passed, 0 failed, 20 ignored |
