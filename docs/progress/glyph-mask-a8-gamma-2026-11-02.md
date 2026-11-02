# Grayscale glyph masks without Skia's A8 gamma, 2 November 2026

Skia is now built without `SK_GAMMA_APPLY_TO_A8`, as Chromium builds it.
A grayscale (A8) glyph mask from FreeType then keeps its raw coverage
instead of passing through the gamma and contrast preblend. The census goes
from 22,308 to 22,323 exact, with 3 more comparisons improved, no losses and
no comparison worse.

## Finding

In `multicol-rule-none-000` at 1.25x, which paints no column rule, every
mismatched pixel was one level off Chrome. It lay where an Ahem glyph is
drawn over a pixel that a black box's edge covers by half: Chrome's
(63, 63, 63) was Open UI's (64, 64, 64).

The glyphs are 62.5 device pixels. That is above Skia's
`SK_MAX_SIZE_FOR_LCDTEXT` of 48, so the `chromium-linux-lcd` subpixel text
falls back to an A8 mask. Skia's FreeType port then decides whether the
mask gets the preblend:

- `SkFontHost_FreeType.cpp` (`filterRec`) calls `rec->ignorePreBlend()` for
  a mask that is not LCD, unless `SK_GAMMA_APPLY_TO_A8` is defined.
- `SkFontHost_FreeType_common.cpp` applies `preBlend.fG` to an A8 mask only
  when `SK_GAMMA_APPLY_TO_A8` is defined.

Skia's own `BUILD.gn` defines `SK_GAMMA_APPLY_TO_A8` in its `skia_private`
config, which applies to Skia's own sources, so every rust-skia build
has it. Chromium builds Skia
through its own `//skia` targets, which do not define it on Linux.
Chrome's software raster (captures run with `--disable-gpu`) therefore
draws large and grayscale glyphs from raw FreeType coverage, and Open UI
drew them preblended with gamma 1.2 and contrast 0.2.

The preblend maps coverage 0 to 0 and 255 to 255, so it changes only
partially covered glyph pixels. Most census text is Ahem at integer
positions, which is why only 18 comparisons change.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| Skia is built with `-USK_GAMMA_APPLY_TO_A8` after the existing gamma defines, in every Skia build configuration. | build | `.github/workflows/ci.yml` (`linux`, `windows-msvc`), `hardening.yml`, `release-v02.yml`, `bindings/rust/.cargo/config.chromium.toml`, `bindings/rust/fuzz/.cargo/config.toml`, `docs/DEVELOPMENT.md` |

`extra_cflags` follow Skia's own defines on the compiler command line, so
the `-U` removes the define. The census and the unit test below show the
raw coverage.

The unit test
`large_grayscale_glyph_mask_keeps_raw_coverage_as_chromium_skia`
(`openui-paint/src/render.rs`) draws a 62.5px Ahem "X" with subpixel
antialiasing over a black box whose bottom edge half covers a row at 1.25x.
It checks that the row inside the box stays black and the half-covered row
is (63, 63, 63), Chrome's value. With the preblend it is (64, 64, 64). The
test depends on FreeType, so it runs only on Linux and is recorded in
`docs/v02/generated/windows-pending-tests.txt`.

## Measurement

This is a paired expanded census in WSL. The base is `a79e9af1`, whose
rendering equals `main` at `889c6f6f`. The head is `a03dcd38`, built with
the new Skia argument. Both use one shared Chromium cache, and Chromium is
identical in all 23,728 comparisons. Open UI's output changed in 18.

| Result | Comparisons |
| --- | --- |
| Exact | 22,308 to 22,323 |
| Gained, 15 | at 1.25x: `multicol-margin-002`; `multicol-rule-none-000`, `-hidden-000`, `-solid-000`, `-inset-000`, `-outset-000`, `-groove-000`, `-ridge-000`, `-dashed-000`, `-double-000`; the `-ref` files of `-solid-000`, `-groove-000`, `-ridge-000`, `-dashed-000` and `-double-000` |
| Improved, 3 | at 1.25x: `multicol-rule-dotted-000`, 744 to 50, and its `-ref`, 1,216 to 100; at 2x: `multicol-span-all-block-sibling-003`, 200 to 2 |
| Lost, worsened | none |

The census covers every raster configuration, so the `default`,
`deterministic-alias` and `deterministic-alias-subpixel` comparisons are in
these rows. None of them changed.

The rows are in the
[census evidence](evidence/glyph-mask-a8-gamma-census-v1.json).

### Remaining failures

- `multicol-rule-dotted-000` keeps 50 mismatched pixels at 1.25x, 3 at 1.5x
  and 6 at 2x, and its `-ref` twice as many. Dotted, dashed and double
  column rules still use the border side painter. The owner is
  `openui-paint` (`painter.rs` `paint_column_rule`).
- `multicol-rule-color-001-ref` and `multicol-rule-fraction-001-ref` at
  1.25x and 1.5x, and `multicol-rule-002` and `multicol-rule-fraction-003`
  at 1.5x, keep between 192 and 500 mismatched pixels in
  `deterministic-alias`. In the two `-ref` files at 1.25x, an image edge
  that covers a pixel by half is one level high: Chrome's 128 is Open UI's
  129. The cause was not established. The owner is `openui-paint`.
- `multicol-span-all-block-sibling-003` keeps 2 mismatched pixels at 2x.
  They were not examined.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the new `SKIA_GN_ARGS` | 22,308 to 22,323 exact; 15 gained, 3 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the new `SKIA_GN_ARGS` | 8,336 passed, 0 failed, 2 ignored, at `a03dcd38` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment and the new `SKIA_GN_ARGS` | 9,498 passed, 0 failed, 21 ignored, at `a03dcd38` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
