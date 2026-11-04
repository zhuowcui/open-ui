# A partly covered image row restarts its scanline every 100 pixels, 4 November 2026

The bilinear sample position of an image's partly covered top or bottom
row now restarts every 100 pixels, as Skia's antialiased rect fill hands
such a row to the shader. Full rows keep their 127-pixel shader chunks.
The census goes from 22,473 to 22,480 exact. No comparison is lost,
improved or worse.

## Finding

`box-sizing-replaced-001`, `-002` and `-003` and their `-ref`s at 1.5x
kept 3 or 4 mismatched pixels each after the
[span matrix](replaced-image-span-matrix-2026-11-03.md) change. Each pixel
is on the partly covered first or last row of a scaled image. In
`box-sizing-replaced-001` they sit at device row 61 and x 448, 898, 2413
and 2716. The row below is exact, but on the edge row Chromium samples a
different column phase. For example at (448, 61) it gives
(126, 189, 162) where Open UI gave (126, 182, 158).

`antifilldot8` in `SkScan_Antihair.cpp` sends a fractional top or bottom
row, or a rect one row high, through `do_scanline`. That row's interior
goes to `call_hline_blitter`, which passes `blitAntiH` runs of at most
`HLINE_STACK_BUFFER`, 100 pixels. Full rows go through `blitRect`, whose
spans the shader splits every 127 pixels. `SkBitmapProcShader` maps the
first pixel of each run again and accumulates its 32.32 step from there.
So on an edge row the drift resets at the interior start plus 100. Each
mismatch is 103 to 108 pixels after the first painted column of its
image, which starts at 340, 795, 2310 and 2613, so just after such a
restart.
`AntiFillRect` intersects the rect with the raster tile clip first, so
a run also starts at a tile origin.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| On a row that `SkScan::AntiFillRect` blits as a partly covered scanline, the horizontal sample of a replaced image or of a repeated tile with coverage bounds restarts every 100 pixels from the interior start or the raster tile origin. Other rows keep the 127-pixel shader chunk. | `openui-paint` | `bindings/rust/openui-paint/src/image_resource.rs` (`anti_fill_rect_partial_row`, `legacy_shader_span_start`, `ANTI_FILL_RECT_HLINE_RUN`, `replaced_sample_fixed_at`, `LegacyRepeatScanline`) |

The unit test `anti_fill_rect_blits_only_fractional_edge_rows_as_scanlines`
checks which rows are partly covered scanlines: fractional top and bottom
rows and a rect one row high, but not integral edges.
`legacy_shader_spans_split_at_edges_tiles_and_buffer_chunks` now also
checks the 100-pixel runs of an edge row, from the interior start and from
a raster tile origin.

## Measurement

This is a paired expanded census in WSL. The base is `fba061f9`, the head
of the [fieldset 3D border](fieldset-blink-3d-2026-11-04.md) census. Its
rendering equals `main` at `73e70bcc`, which only adds a unit test. The
head is `9bebead6`, this change on `main` at `73e70bcc`. Both are built
with the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium is
identical in all 23,728 comparisons. Open UI's output changed in 7.

| Result | Comparisons |
| --- | --- |
| Exact | 22,473 to 22,480 |
| Gained, 7 | `box-sizing-replaced-001` and its `-ref` (4 to 0), `-002` and its `-ref` (3 to 0) and `-003` and its `-ref` (3 to 0) at 1.5x; `box-shadow_box-shadow-blur-definition-001` at 1.25x (3 to 0) |
| Improved, lost, worsened | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/hline-run-spans-census-v1.json).

### Remaining failures

- The vertical axis keeps its direct sample position per row. It was not
  changed.
- The `background-repeat-space-*-ref` raster tiles keep their one-level
  edge and bilinear phase differences.
- `background-image-centered` keeps 1 to 5 pixels that differ by one
  level of blue.
- The column fragment clip residuals in `multicol-rule-002`,
  `multicol-rule-fraction-003` and `multicol-span-all-block-sibling-003`
  and the 1 pixel left in `multicol-span-all-004` at 1.5x are unchanged.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,473 to 22,480 exact; 7 gained, 0 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,344 passed, 0 failed, 2 ignored, at `9bebead6` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,506 passed, 0 failed, 21 ignored, at `9bebead6` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
