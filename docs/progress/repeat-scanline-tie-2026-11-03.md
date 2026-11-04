# A repeated scanline's half-texel tie follows Skia's f32 inverse, 3 November 2026

When a repeated raster background lands exactly on a half-texel tie, Open
UI now settles the tie the way Chromium's legacy repeating bitmap shader
does. It maps the first pixel of the span through an f32 inverse of the
tile's device matrix and then adds the 32.32 step for each pixel. Before,
Open UI always closed the tie to the predecessor texel with weight 7/16.
The census goes from 22,437 to 22,440 exact. No comparison is lost,
improved or worse.

## Finding

`background-repeat-round-1-ref`, `-1c` and `-1e` draw a 32×32 image in
36px tiles from CSS x 21. At 1.25x and 1.5x some device columns sample at
exactly half a texel. Chromium gives 7/16 at device x 37 at 1.25x, but
8/16 at device x 58 at 1.5x. Open UI used 7/16 in both cases, so the three
comparisons at 1.5x had 20 mismatched pixels each.

Skia's legacy shader inverts the tile's total matrix in f32 and divides by
the image width. It maps the span's first pixel centre, relative to the
compositor raster tile origin, to 32.32 fixed point, subtracts half a
filter texel, and adds the fixed step for each later pixel. The weight is
bits 12 to 15 of the 16.16 position. With the f32 rounding of the inverse,
the position at 1.25x falls just below the tie and the one at 1.5x just
above it, which gives 7 and 8.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The horizontal sample position of a repeated raster tile with coverage bounds is computed as Skia's legacy shader computes it: an f32 inverse of the tile matrix, the span start mapped to 32.32 fixed point, then a fixed step per pixel. The forced 7/16 weight at a half-texel tie and its rounded-origin exception are removed. The exact integer phase rule and the vertical axis are unchanged. | `openui-paint` | `bindings/rust/openui-paint/src/image_resource.rs` (`LegacyRepeatScanline`, `repeated_subrect_sample_fixed_at`) |

The unit test `repeated_scanline_closes_half_texel_with_its_f32_inverse`
replaces `translated_repeated_scanline_closes_half_texel_to_predecessor`.
It checks weight 7 at device x 37 at 1.25x and weight 8 at device x 58 at
1.5x for the fixture's tile, and weight 8 at 1.25x without the scanline
model.

## Measurement

This is a paired expanded census in WSL. The base is `d48f71ee`, whose
rendering equals `main` at `edf3da21`, after the
[gradient tile edge](gradient-tile-edge-2026-11-03.md) change. The head is
`cc62ca61`, whose `bindings` tree equals this change rebased onto `main`
at `edf3da21`. Both are built with the CI
`SKIA_GN_ARGS` and share one Chromium cache. Chromium is identical in all
23,728 comparisons. Open UI's output changed in 3.

| Result | Comparisons |
| --- | --- |
| Exact | 22,437 to 22,440 |
| Gained, 3 | `background-repeat-round-1-ref`, `-1c` and `-1e` at 1.5x (20 to 0 each) |
| Improved, lost, worsened | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/repeat-scanline-tie-census-v1.json).

### Remaining failures

- The vertical axis keeps its direct sample position. It was not changed.
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
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,437 to 22,440 exact; 3 gained, 0 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,342 passed, 0 failed, 2 ignored, at `cc62ca61` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,504 passed, 0 failed, 21 ignored, at `cc62ca61` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
