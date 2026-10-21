# Skia legacy shader context in every hosted configuration, 21 October 2026

`openui-engine::source_layer_raster::original_edge_and_all_eight_controls_keep_real_property_boundaries`
failed on every hosted Linux and Windows run, and on local builds with system
clang. Earlier records attributed it to the compiler toolchain, because only
the maintainer's pinned Chromium toolchain passed. That attribution was wrong.

## Cause

`bindings/rust/.cargo/config.chromium.toml`, the configuration behind the
historical census and the checkpoint's passing run, compiles Skia with
`-DSK_ENABLE_LEGACY_SHADERCONTEXT`. The hosted workflows, the fuzz workspace
and the Windows instructions omitted that define. Without it, Skia samples
images through the raster pipeline rather than the legacy shader context. The
viewport scrollbar nine-patch track and arrows are drawn by resampling an
image, so their pixels differed from the pinned Chromium frame by 1–8 levels
at 1.25×, 1.5× and 1.75×.

## Change

`-DSK_ENABLE_LEGACY_SHADERCONTEXT` is added after
`-DSK_SUPPORT_LEGACY_CONIC_CHOP` in every portable Skia configuration:

- `.github/workflows/ci.yml` (`rust-parity` and `windows-msvc`)
- `.github/workflows/hardening.yml`
- `.github/workflows/release-v02.yml`
- `bindings/rust/fuzz/.cargo/config.toml`
- the Windows instructions in `docs/DEVELOPMENT.md`

Hosted builds now use the same Skia defines as the pinned parity profile.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Arrow test, Windows | `cargo test --locked -p openui-engine --test source_layer_raster`, with the define | 7/7 pass (it failed without the define) |
| Linux workspace, WSL, system clang 21 | `cargo --config 'target.x86_64-unknown-linux-gnu.rustflags=[]' test --workspace --locked --no-fail-fast --exclude pixel-compare` | 9,403 passed, 0 failed, 13 ignored |
| Paired renderer census, expanded suite, WSL | `build_renderer.py` then `run_renderer_matrix.py --suite expanded --jobs 16` for unmodified `main` (`ad491392`) built without and with the define, sharing one Chromium cache | 23,728 comparisons, Chromium identical in all. Exact rises from 21,663 to 21,681: 18 gains, 0 losses, 68 improved, 12 worsened |

The paired census compares the same source with only the Skia define changed.
Gains are in backgrounds, border radius clipping, replaced overflow and image
sizing. The 12 worsened comparisons are all at 1.25×: `background-334`,
`border-image-017` to `-020`, `border-image-slice-001` and `-002` (249 to 311
pixels each), two radius `background-clip` tests (2 pixels each) and three
`flexbox-whitespace-handling-001` variants (63 to 65). They remain inexact
either way. The per-comparison delta is in
[skia-legacy-shader-context-census-v1.json](evidence/skia-legacy-shader-context-census-v1.json).
This is a paired measurement of one source tree, not a full qualification
census.
