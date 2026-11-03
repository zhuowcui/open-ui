# Aliased Ahem is grid-fitted as in Chromium, 3 November 2026

Open UI now draws an aliased Ahem glyph from its strong-hinted outline
whenever its device size fits Skia's glyph atlas, as Chromium does. The
author-text replays that painted an extra Ahem row from layout-phase
heuristics are gone. A single-line text input now clips its text to the
inner editor's line box. The census goes from 22,390 to 22,435 exact,
with 75 more comparisons improved and no losses. Four comparisons are
worse.

## Finding

The fixtures set `font-family: Ahem`, and the deterministic Fontconfig
profile (`ahem_noaa.conf`) turns hinting off for it. Open UI therefore
painted the raw outline. Chromium does not. Measured with absolutely
positioned `X` cells in Chromium 147.0.7727.50 at every 1/64 CSS px top
phase:

- An aliased Ahem square's row and column counts depend only on its device
  size D (font size × device scale), never on its layout phase.
- At 1x, it has one extra row when D mod 5 = 3 (8, 13, 18, …, 253).
- At all 94 sizes compared with skrifa, the strong-hinted outline's rows
  equal Chromium's rows. The raw outline's rows match at only 72.
- Columns also follow the strong-hinted outline, for example 22.5 → 23,
  37.5 → 38 and 12.5 → 13 device px.

Larger sizes, probed in Chromium at 1x, show where this stops:

| CSS size | 238 | 243 | 248 | 253 | 255 | 256 | 257 | 258 | 263 | 288 | 358 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Rows | 239 | 244 | 249 | 254 | 255 | 256 | 257 | 258 | 263 | 288 | 358 |

Above 256 device px, Skia draws a glyph as a path instead of from its
atlas (`kSkSideTooBigForAtlas`), and that path has no grid fitting. This
explains the `position-sticky-fixed-ancestor` losses an earlier draft
had at 1.25x and 1.5x (D = 360 and 432).

The same draft lost `input-scrollable-region-001` at 1x. Chromium clips
the text of a single-line `<input>` to its inner editor, which is one
line tall. An 18px Ahem value has 19 rows in a `div` and in a
`<textarea>`, but only 18 in an `<input>`. The 13px and 23px inputs
likewise show 13 and 23 rows. The hidden row is the fitted one above
the ascent. A `div` with `overflow: hidden` clips it the same way.

The removed replays (`should_replay_ahem_preceding_row`,
`should_replay_line_clamp_ahem_preceding_row` and
`chromium_fractional_strike_retains_preceding_row`) approximated the
fitted row from the line's layout phase. With every author replay
disabled and the raw outline kept, the census lost 50 comparisons and
gained 17, so they could only be removed together with the fitted
outline. The native button replay (`replays_native_small_ahem_start`) is
unchanged.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| An aliased Ahem run whose device size is at most 256 px uses the strong-hinted fontations outline, even when its resolved font has hinting off. Other unhinted aliased fonts keep the raw outline. | `openui-text` | `bindings/rust/openui-text/src/shaping/shape_result.rs` (`fontations_compatible_font`, `SKIA_MAX_ATLAS_GLYPH_SIZE`) |
| The author-text preceding-row replays and their unit tests are removed. | `openui-paint` | `bindings/rust/openui-paint/src/painter.rs` (text fragment painting) |
| A single-line text input clips its value or placeholder text to the inner editor's line box. Selection, caret and composition painting are unchanged. | `openui-paint` | `bindings/rust/openui-paint/src/painter.rs` (`paint_text_input_control`) |

The unit test `aliased_ahem_is_grid_fitted_up_to_skias_atlas_limit` checks
the fitted outline's height at 13, 18, 20 and 253 px (14, 19, 20 and 254
rows, as Chromium shows), and that a 258px glyph keeps the raw outline.

## Measurement

This is a paired expanded census in WSL. The base is `8abfe63e`, whose
rendering equals `main` at `e665679c`. The head is `d127794f`. Both are
built with the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium is
identical in all 23,728 comparisons. Open UI's output changed in 124.

| Result | Comparisons |
| --- | --- |
| Exact | 22,390 to 22,435 |
| Gained, 45 | flexbox (15), fragmentation (12), line-clamp and overflow (8), backgrounds (6), `display-contents-table-002` and its `-ref` at 1x, and floats and multicol (1 each) |
| Improved, 75 | 72,395 fewer mismatched pixels in all; 32 of them are flexbox |
| Lost | none |
| Worsened, 4 | `range-percent-intrinsic-size-1` and its `-ref` at 1.5x (24 to 48); `select-4-option-optgroup-display-none` and its `-ref` at 2x (24,508 to 24,700) |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/ahem-grid-fit-census-v1.json).

### Remaining failures

- `range-percent-intrinsic-size-1` and its `-ref` at 1.5x gain a
  24-pixel column at a seam between two adjacent Ahem glyphs, where the
  fitted outline leaves a partly covered column. It was not examined
  further.
- `select-4-option-optgroup-display-none` and its `-ref` at 2x gain one
  row of 192 pixels in the list box. The comparison already had 24,508
  mismatched pixels, which come from the list box, not from text
  hinting.
- Ahem runs drawn under a CSS transform use the device scale alone to
  pick the fitted outline and the 256 px limit. Skia uses the full
  device matrix. This was not measured.
- The column fragment clip residuals in `multicol-rule-002`,
  `multicol-rule-fraction-003` and `multicol-span-all-block-sibling-003`
  and the 1 pixel left in `multicol-span-all-004` at 1.5x are unchanged.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,390 to 22,435 exact; 45 gained, 75 improved, 4 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,341 passed, 0 failed, 2 ignored, at `d127794f` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,503 passed, 0 failed, 21 ignored, at `d127794f` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
