# oklch(), oklab() and color-mix() resolve as Chromium does, 9 November 2026

Open UI parses `oklch()`, `oklab()` and `color-mix()` and paints them with
the same bytes as pinned Chromium 147. Tailwind v4 and the current shadcn
themes define every color token with `oklch()` and every opacity modifier
with `color-mix(in oklab, … transparent)`, so this is plan unit A2's color
step (Workstream A, `docs/alpha/plan.md`). 54 swatches captured from
Chromium on Linux and on Windows match the native render exactly.

## Finding

Blink keeps these colors as float32 channels in their own space
(`platform/graphics/color.cc`) and converts to sRGB only when painting, in
`Color::toSkColor4f`. There is no gamut mapping (`kBakedGamutMapping` is
off by default): channels outside [0, 1] stay unclamped until Skia
quantizes them. `SkPaint::setColor` pins only alpha. A translucent color
with a channel above 1, such as `oklch(1 0 0 / 10%)`, therefore takes
Skia's float raster pipeline instead of the 8-bit blitter, and rounds
differently: Chromium paints it over black as (25, 26, 26), where
`rgba(255, 255, 255, 0.1)` gives (26, 26, 26).

`color-mix()` of two absolute colors resolves at computed-value time:
both colors are converted to the interpolation space (default oklab),
missing components are carried and substituted, channels are
premultiplied (not the hue), blended with `Blend(float, float, double)`
and unpremultiplied, and the alpha is scaled when the percentages sum to
less than 100%.

The source reading, with file and line citations at tag 147.0.7727.50,
is in [the Chromium color functions analysis](../analysis/chromium-147-color-functions.md),
with a float32 emulation in
`tools/qualification/reproducers/oklch_chromium_emulation.py`.

The components track measured the same clamp independently: the shadcn
dark separator (`--border: oklch(1 0 0 / 10%)` over `rgb(10 10 10)`)
paints (34, 35, 35) in Chromium and (34, 34, 34) in Open UI before this
change.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `oklch()` and `oklab()` parse with Blink's grammar and storage clamps (L in [0, 1], chroma at least 0, hue wrapped in double, alpha not rounded to 8 bits, `none`). `color-mix()` supports `in srgb`, `in oklab` (the default) and `in oklch` with the four hue methods; operands are these functions, hex, `rgb()`, named colors and nested mixes. Conversion follows Chromium's float32 operation order: `gfx::LchToLab`, `OklabToXYZD50`, `XYZD50ToSRGB`, `XYZD50ToOklab`, Chromium's own skcms transfer-function copies and run-time `skcms_Matrix3x3_invert` inverses. | `openui-style` | `bindings/rust/openui-style/src/color_function.rs`; `property.rs` (`color`, `legacy_srgb_color`) |
| Paint passes color channels to Skia unclamped; alpha stays clamped. | `openui-paint` | `bindings/rust/openui-paint/src/painter.rs` (`set_paint_css_color_with_alpha`) |

Every color property that accepts a typed color literal accepts these
functions, including `view!` `style:` attributes, which validate them at
compile time.

## Measurement

The oracle is the center pixel of 54 swatches, each a 20px square: a
backdrop with a child painted in the tested color. The list is
[`swatches.txt`](evidence/color-functions-v1/swatches.txt): the shadcn
neutral and chart tokens, translucent white over dark tokens, Tailwind
opacity mixes, out-of-gamut colors, every hue method, `none` channels,
percentage normalization and nested mixes. `capture_color_function_oracle.py`
captured them from pinned Chromium 147.0.7727.50 on Linux (the census
binary, in WSL) and on Windows (Chrome-for-Testing), headless with
`--disable-gpu`. Both captures are identical
([Linux](evidence/color-functions-v1/chromium-linux.json),
[Windows](evidence/color-functions-v1/chromium-windows.json)).

`openui/tests/native_color_functions.rs` builds the same swatches through
the public `Element` API and renders them headless. All 54 center pixels
match exactly on Windows. With the old paint clamp restored, 3 fail by one
level, for example `oklch(1 0 0 / 10%)` over black gives (26, 26, 26)
instead of (25, 26, 26).

The unit tests in `color_function_tests.rs` pin the derived matrices,
parse clamps, rejections, percentage normalization and hue methods, and
agree with an independent float32 emulation of the Chromium source.

The paired expanded census in WSL has no change. The base is `9d7bccf6`,
the head of the [spaced pattern](spaced-pattern-extent-2026-11-05.md)
census; `main` has changed no renderer code since. The head is
`bbb4bcb1`, this change. Both are built with the CI `SKIA_GN_ARGS` and
share one Chromium cache. Chromium is identical in all 23,728
comparisons, and Open UI's output is identical in all 23,728: 22,517
exact before and after, with nothing gained, lost, improved or worse. No
census test uses these functions, and every existing color stays inside
[0, 1], so the paint change cannot alter them. The delta is in the
[census evidence](evidence/color-functions-census-v1.json).

The CSS Grid coverage suite has no change either. The base is the
[coverage baseline](grid-coverage-2026-11-09.md) at `04d1eeab`; the head
is `0695e911`, this change before its last rebase onto `main`. Over the
4,204 comparisons the baseline measures, no Open UI image changed: 2,218
exact before and after, with nothing gained or lost. The run also included
the two references the baseline leaves out for unbounded renderer memory;
both failed to render, as the baseline records, and are not counted.

### Remaining failures and limits

- Not supported, and rejected rather than approximated: `color-mix()` with
  `currentcolor` (Blink resolves it at used-value time), `in srgb-linear`
  (Chromium paints it with skcms's approximate `powf_`), other spaces
  (`lab`, `lch`, `hsl`, `hwb`, `xyz`, `display-p3`), `rad`, `grad` and
  `turn` hues, `calc()` channels and relative colors. Owner: `openui-style`.
- Gradient stops convert colors to 8-bit `SkColor`
  (`skia_css_color_with_alpha`), so an out-of-gamut `oklch()` stop is
  clamped where Chromium passes floats. Unmeasured. Owner: `openui-paint`
  gradients.
- Only background fills were compared. Text, border, outline and shadow
  colors share the resolved color but were not captured.
- Open UI resolves these colors to sRGB at parse time. Transitions and
  animations between them interpolate in sRGB, where Chromium
  interpolates non-legacy colors in oklab. Owner: `openui-style`
  animation values, with the A2 token work.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Chromium oracle, Linux | `python3 tools/qualification/capture_color_function_oracle.py --platform linux --chrome ~/openui-oracle/chrome/linux-147.0.7727.50/chrome-linux64/chrome` (WSL) | 54 swatches |
| Chromium oracle, Windows | `python tools/qualification/capture_color_function_oracle.py --platform windows --chrome E:\open-ui\out\chrome-win\win-147.0.7727.50\chrome-win64\chrome.exe` | 54 swatches, identical to Linux |
| Native swatches, Windows | `cargo test -p openui --test native_color_functions`, with the `windows-msvc` CI environment | 54 of 54 exact |
| Style unit tests | `cargo test -p openui-style` | 105 passed |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,517 to 22,517 exact; 0 of 23,728 Open UI images changed |
| CSS Grid coverage | `run_renderer_matrix.py --suite coverage --ids-file tools/coverage/css_grid/manifest.json --jobs 16`, compared per test with the baseline | 2,218 to 2,218 exact; 0 of 4,204 measured Open UI images changed |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
