# A fitted aliased Ahem run keeps its subpixel phases, 3 November 2026

The strong-hinted outline that Open UI substitutes for an aliased Ahem run
now keeps the resolved font's subpixel positioning. Before, it always had
subpixel positioning off, so each glyph's x origin was rounded to a whole
device pixel. Chromium keeps the phase. The census goes from 22,435 to
22,437 exact. No comparison is lost, improved or worse.

## Finding

`range-percent-intrinsic-size-1` and its `-ref` had 24 mismatched pixels
at 1.5x before the [Ahem grid fit](ahem-grid-fit-2026-11-03.md) and 48
after it. The comparison uses the `deterministic-alias-subpixel` raster
configuration, because it contains `<input type=range>`.

In that configuration the Ahem advance at 16px is 16.00003 CSS px, from
FreeType's linear metrics. The `abc` run is therefore 48.0156 px wide, the
flex input is 1/64 px narrower, and the glyph origin lands at 72.984375
CSS px, which is 109.477 device px at 1.5x. Chromium draws that glyph's
first column at device x 110. The fitted font, with subpixel positioning
off, rounded the origin to 109, one column early.

With subpixel positioning kept, a WSL render of the fixture is exact at
1x, 1.25x, 1.5x and 2x.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The fitted outline for an aliased Ahem run keeps subpixel positioning when the resolved font has it. Other runs are unchanged. | `openui-text` | `bindings/rust/openui-text/src/shaping/shape_result.rs` (`fontations_compatible_font`) |

The unit test `aliased_ahem_is_grid_fitted_up_to_skias_atlas_limit` also
checks that a 16px source font with subpixel positioning gives a fitted
font with subpixel positioning.

## Measurement

This is a paired expanded census in WSL. The base is `d127794f`, whose
rendering equals `main` at `c7aff7a8`. The head is `a46b489d`. Both are
built with the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium is
identical in all 23,728 comparisons. Open UI's output changed in 2.

| Result | Comparisons |
| --- | --- |
| Exact | 22,435 to 22,437 |
| Gained, 2 | `range-percent-intrinsic-size-1` and its `-ref` at 1.5x (48 to 0) |
| Improved, lost, worsened | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/ahem-subpixel-phase-census-v1.json).

### Remaining failures

- The 16.00003 px Ahem advance in `deterministic-alias-subpixel` is not
  compared with Chromium's advance. It was not changed, and it may still
  cause a residual elsewhere.
- `select-4-option-optgroup-display-none` and its `-ref` at 2x keep
  24,700 mismatched pixels, from the list box.
- Ahem runs drawn under a CSS transform use the device scale alone to
  pick the fitted outline and the 256 px limit. This was not measured.
- The column fragment clip residuals in `multicol-rule-002`,
  `multicol-rule-fraction-003` and `multicol-span-all-block-sibling-003`
  and the 1 pixel left in `multicol-span-all-004` at 1.5x are unchanged.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,435 to 22,437 exact; 2 gained, 0 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,341 passed, 0 failed, 2 ignored, at `a46b489d` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,503 passed, 0 failed, 21 ignored, at `a46b489d` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
