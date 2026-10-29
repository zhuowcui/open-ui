# Linux subpixel positioning borrows a pixel from the ascent, 29 October 2026

On Linux above a device scale of 1, a font whose descent rounds down now
moves one pixel from its rounded ascent to its rounded descent, as Blink
does. The baseline of 10px and 13px DejaVu text rises by one CSS pixel,
and the line box keeps its height. This replaces a paint-only offset that
approximated the same move for 10px text. The census goes from 22,189 to
22,232 exact, with 35 more comparisons improved and no losses.

## Finding

Blink's `FontMetrics::AscentDescentWithHacks`
(`platform/fonts/font_metrics.cc`) rounds the ascent and the descent. On
Linux it then checks the font's render style: if the rounded descent is
less than the true descent, subpixel positioning is on, and the rounded
ascent is at least 1, it adds one to the descent and takes one from the
ascent.

Chromium's `gfx::GetFontRenderParams`
(`ui/gfx/font_render_params_linux.cc`, 147.0.7727.50) turns on subpixel
positioning when the device scale factor is above 1, and then asks for no
hinting. When Fontconfig disables antialiasing for a face, hinting is full
and subpixel positioning is off.

DejaVu Sans and DejaVu Sans Mono have an ascent of 0.928 em and a descent
of 0.236 em. At 10px the descent is 2.36 and rounds down. At 13px it is
3.07 and rounds down. At 11, 12 and 16px it is 2.59, 2.83 and 3.78 and
rounds up. A Chrome 147 capture at 2x of each face at 10, 11, 12, 13 and
16px, under the census Fontconfig, put the baseline one CSS pixel higher
at 10 and 13px only.

Open UI kept the rounded metrics at every scale. A paint-time offset,
`chromium_lcd_strike_y_offset` in `shape_result.rs`, raised unhinted LCD
glyphs at 10px by 1 CSS pixel above a scale of 1. It moved glyphs but not
line boxes, so it could not fix layout, and it ignored 13px.

The first census of the change made 10 comparisons worse, all in the
deterministic aliased profiles. The comparison Fontconfig `ahem_noaa.conf`
disables antialiasing for Ahem and its pinned fallback faces. Chromium
asks Fontconfig per face, so a `::scroll-button` label in Ahem has no
subpixel positioning and no borrow. Open UI draws control text through its
LCD profile in that configuration, so the borrow had applied to it.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| Above a device scale of 1, on the Linux Chromium backends, antialiased text whose descent rounds down borrows one pixel from the ascent. Its paint and shaping fonts are unhinted. | `openui-text` | `font/platform.rs` `linux_subpixel_ascent_borrow`, `FontPlatformData::with_source_configuration` |
| A face that the aliased comparison Fontconfig pins never borrows, including control text. | `openui-text` | `font/platform.rs` `FontPlatformData::with_source_configuration` |
| The 10px paint offset is removed. | `openui-text` | `shaping/shape_result.rs` |

Windows is unchanged. Blink compiles the borrow only for Linux, ChromeOS,
Android and Fuchsia, and Open UI's DirectWrite path never reaches the new
code.

Two tests in `platform.rs` cover the change:

- `linux_subpixel_borrow_moves_a_pixel_only_when_descent_rounds_down` checks
  the DejaVu metrics at 10, 11, 12, 13 and 16px, integral metrics, and an
  ascent that rounds to 0.
- `linux_subpixel_borrow_skips_faces_fontconfig_aliases` checks 16px Ahem at
  2x. Under LCD author text it borrows to 11.8 and 4.2. Under the aliased
  profile it keeps 12.8 and 3.2, as author text and as control text.

The removed offset's test went with it.

## Measurement

This is a paired expanded census in WSL. The base is `dee14f6b`, whose
rendering equals `main` at `5a4e22c0`. The head is `577c8887`. Both use
one shared Chromium cache, and Chromium is identical in every comparison.

| Result | Comparisons |
| --- | --- |
| Exact | 22,189 to 22,232 |
| Gained, 43 | `box-shadow_slice-inline-fragmentation-001` to `-003` and their references, `dynamic-isize-change-001-ref`, the four `static-position_htb-*` tests, `clone-` and `slice-nowrap-intrinsic-size` and their references, and the `clone-` and `slice-intrinsic-size` references, each at one or more of 1.25x, 1.5x and 2x |
| Improved, 35 | `clone-` and `slice-nowrap-intrinsic-size-bidi` and references: 7,103 to 264 mismatched pixels at 1.25x, 11,257 to 320 at 1.5x, 16,825 to 416 at 2x. `clone-` and `slice-intrinsic-size`: about 8,900 to 1,425 at 1.25x, 13,700 to 1,980 at 1.5x, 21,700 to 3,340 at 2x. `static-position_htb-*` at 1.5x: 10,470 to 1,525. `flexbox-align-self-baseline-horiz-001a` and `-001b`: 1,742 to 187, 2,316 to 454, 3,780 to 1,326. `multicol-span-all-block-sibling-003`: 373 to 1, 253 to 2, 593 to 200. The inline-fragmentation tests that did not become exact. |
| Lost, worsened | none |

The rows are in the
[census evidence](evidence/linux-subpixel-ascent-borrow-census-v1.json).
Before the aliased-face guard, the same census had 43 gains and 10
worsened comparisons: `scroll-markers_*` and
`position-absolute-semi-replaced-stretch-input*`. With the guard, a
rerender of every `deterministic-alias` and `deterministic-alias-subpixel`
comparison matched the base, at 10,589 of 11,668 and 28 of 40 exact.

### Remaining failures

- Chromium's rule turns off hinting for all antialiased text above a scale
  of 1. Open UI turns it off only for faces that borrow. Turning it off for
  every face at once worsened the 16px float `nowrap` fixtures. So 11, 12
  and 16px text above a scale of 1 keeps its configured hinting. The owner
  is `openui-text` `resolve_hinting`, and that mismatch is not yet
  attributed.
- The `-bidi` intrinsic-size tests still differ by 264 to 416 pixels, and
  the non-`nowrap` `clone-` and `slice-intrinsic-size` tests by 1,425 to
  3,342 pixels. These were not attributed in this change.
- `static-position_htb-*` still differ at 1.5x and 2x, and
  `flexbox-align-self-baseline-horiz-001*` at every scale. These were not
  attributed.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,189 to 22,232 exact; 43 gains, 35 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint` | 8,322 passed, 0 failed, 2 ignored |
| Text crate on Windows | `cargo test --locked -p openui-text`, with the `windows-msvc` CI environment | 348 unit tests passed, 0 failed |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,485 passed, 0 failed, 20 ignored |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
