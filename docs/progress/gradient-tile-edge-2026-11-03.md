# A single border-box gradient tile keeps its fractional device edges, 3 November 2026

A background layer with one generated tile and `background-clip:
border-box` is now drawn with antialiasing into its CSS-snapped
destination, intersected with the clip, and with no hard clip. Before, a
hard clip snapped to pixel centres cut the tile at fractional device
edges, so the partly covered edge columns and rows were lost. Blink does
not clip a border-box image layer. The census stays at 22,437 exact, with
13 comparisons improved, no losses and none worse.

## Finding

`background-repeat-space-1-ref` lays out nine 32px `div`s with
`linear-gradient(to top left, red, green)` at 5px gaps. At 1.25x and 1.5x,
most tiles start and end at fractional device pixels. In Chromium each
tile's edge column and row has partial coverage blended over white. Open
UI drew the full tile but clipped it with `half_open_physical_clip_rect`,
which keeps only pixels whose centres lie inside the tile, so those edge
pixels were white or fully covered.

The same rule already applied to raster image layers through an outward
clip (`border_box_raster`). Generated images with a single tile had no
such exception unless their own tile was fractional in device pixels
(`direct_generated_image_owns_rect_clip`).

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A single generated tile in a border-box layer, with a fractional device edge on the tile or its clip, no border radius, no transform skew and no scrollport, canvas or border-bleed clip, is drawn antialiased into its tile intersected with the clip, without a hard clip. | `openui-paint` | `bindings/rust/openui-paint/src/painter.rs` (`paint_background_layers`, `border_box_generated_tile_owns_edges`) |

The unit test `border_box_gradient_keeps_fractional_device_edge_coverage`
paints a 32px gradient at CSS (38, 38) at 1.25x. It checks that device
columns 47 and 87 are partly covered and differ from their inner
neighbours, and that columns 46 and 88 stay white. Without the change the
test fails.

## Measurement

This is a paired expanded census in WSL. The base is `a46b489d`, whose
rendering equals `main` at `1cb6603f`. The head is `d48f71ee`. Both are
built with the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium is
identical in all 23,728 comparisons. Open UI's output changed in 14.

| Result | Comparisons |
| --- | --- |
| Exact | 22,437 to 22,437 |
| Improved, 13 | `background-repeat-space-1-ref` and `-3-ref` to `-7-ref` at 1.25x and 1.5x (for example 2,560 to 1,122 and 1,360 to 208 for `-1-ref`); `background-attachment-fixed-inline-scrolled-ref` at 1.25x (2,031 to 894) |
| Changed, same count | `background-attachment-fixed-inline-scrolled-ref` at 1.5x (1,365) |
| Gained, lost, worsened | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/gradient-tile-edge-census-v1.json).

### Remaining failures

- The improved `background-repeat-space-*-ref` comparisons keep their
  raster image tiles (a 32×32 PNG in 32px `div`s at fractional device
  positions). The tile edges differ by one level and some interior pixels
  by up to 15, from bilinear sampling phase. This was not changed.
- `background-repeat-space-1a` to `-1c`, which use `repeat: space` with
  several tiles, take the spaced shader path and are unchanged.
- The column fragment clip residuals in `multicol-rule-002`,
  `multicol-rule-fraction-003` and `multicol-span-all-block-sibling-003`
  and the 1 pixel left in `multicol-span-all-004` at 1.5x are unchanged.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,437 to 22,437 exact; 0 gained, 13 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,342 passed, 0 failed, 2 ignored, at `d48f71ee` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,504 passed, 0 failed, 21 ignored, at `d48f71ee` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
