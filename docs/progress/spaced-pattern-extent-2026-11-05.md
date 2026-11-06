# A spaced pattern keeps its fractional border-box edge, 5 November 2026

A spaced background pattern on a border-box layer is no longer cut at the
pixel centre of its fractional right and bottom edge. Its edge pixels
keep the partial coverage of the antialiased draw, as Chromium's
`Image::DrawPattern` and `GeneratedImage::DrawPattern` give them. The
census goes from 22,516 to 22,517 exact, with 12 more comparisons
improved. No comparison is lost or worse.

## Finding

The [Skia CPU opts record](skia-cpu-opts-2026-11-04.md) left
`background-repeat_gradient-repeat-spaced-with-borders` at 369 pixels at
1.5x. Most were in column and row 232, at the pattern's right and bottom
edge at 232.5 device px. Chromium painted partial blue and green there,
and Open UI left white.

Blink draws a spaced pattern of any image as one shader over the
destination rect. `DrawTiledBackground` passes the snapped rect to
`DrawImageTiled`, which calls `Image::DrawPattern`, and that fills the
rect with antialiasing. A border-box layer gets no clip of its own, so a
fractional edge keeps half coverage. Open UI already left a border-box
raster image unclipped, behind an outward-rounded device-pixel clip. A
spaced pattern was still clipped to the snapped border box without
antialiasing, which cut the half-covered column. The `background-repeat-space`
cases at 1.25x and 1.5x had the same cut column under their bitmap tiles.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A border-box layer that is not a local image in a scroller uses the outward-rounded device-pixel clip when it is a raster image, as before, or when it draws as one spaced pattern shader. | `openui-paint` | `bindings/rust/openui-paint/src/painter.rs` (`paint_background_layers`, `border_box_antialiased_draw`) |

The unit test `spaced_pattern_keeps_its_fractional_border_box_edge`
paints a spaced 30px radial gradient pattern in a 135px box at 1.5x. It
checks the half-covered column and row at device px 232, their corner,
and the white pixel past it, against levels that the antialiased draw
gives. It fails without the change.

## Measurement

This is a paired expanded census in WSL. The base is `88a9ba2b`, the head
of the [Skia CPU opts](skia-cpu-opts-2026-11-04.md) census, which renders
as `main` at `0f42ebf8`. The head is `9d7bccf6`, this change on `main`.
Both are built with the CI `SKIA_GN_ARGS` and share one Chromium cache.
Chromium is identical in all 23,728 comparisons. Open UI's output changed
in 13.

| Result | Comparisons |
| --- | --- |
| Exact | 22,516 to 22,517 |
| Gained, 1 | `background-repeat_gradient-repeat-spaced-with-borders` at 1.5x (369 to 0) |
| Improved, 12 | `background-repeat-space-4` to `-7` at 1.25x, for example `-space-7` (147 to 24); `background-repeat-space-1a` to `-1c` at 1.5x (612 to 24); `background-repeat-space-3` to `-7` at 1.5x, for example `-space-6` (942 to 72) |
| Lost, worsened | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/spaced-pattern-extent-census-v1.json).
Before the census, a batch of the 80 expanded comparisons with `space` in
a background declaration gave the same 13 changes.

### Remaining failures

- The `background-repeat-space` cases keep one-level bilinear differences
  at their bitmap tiles' edges. `background-repeat-space-1a` at 1.5x keeps
  24 pixels, for example (79, 32), where Chromium paints (146, 165, 165)
  and Open UI paints (147, 166, 165). The owner is `openui-paint`
  background painting, in the tiled bitmap's sampling. It was not
  diagnosed further.
- `background-repeat-round-1-ref` and `-1a` to `-1e` keep 4 pixels at
  1.25x. `background-repeat-round-2` and its `-ref` keep 4 at 2x, and
  `-round-3` and its `-ref` keep 2 at 2x. The owner is `openui-paint`
  background painting.
- `ttwf-css3background-border-style` and its `-shorthand` keep their
  double-side pixels, 5 at 1.25x and 3 at 1.5x.
- `border-radius-clip-002` keeps its 2 pixels of rounded-rect
  antialiasing residue at 1x.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,516 to 22,517 exact; 1 gained, 12 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,349 passed, 0 failed, 2 ignored, at `9d7bccf6` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,511 passed, 0 failed, 21 ignored, at `9d7bccf6` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
