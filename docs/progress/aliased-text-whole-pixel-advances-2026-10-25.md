# Whole-pixel advances for aliased Linux text, 25 October 2026

A survey of near misses found the `background-clip_clip-*` tests differing in
their 16pt (21.333px) Ahem paragraph. Chrome advances each glyph exactly 21
px: "Test" spans 84 px. Open UI advanced 21.33 px, which leaves a one-pixel
gap every third glyph and lets later glyphs drift.

## Finding

The census's registered-Ahem text uses a Fontconfig file that disables
antialiasing for Ahem and its fallbacks
(`tools/accountability/data/fonts/ahem_noaa.conf`). Chromium's Linux font
render parameters force **full hinting** and turn subpixel positioning off
whenever antialiasing is disabled (`ui/gfx/font_render_params_linux.cc`).

Two Blink rules follow from that:

- `WebFontRenderStyle::ApplyToSkFont` force-enables subpixel positioning
  only when hinting is not full, so this text is never subpixel-positioned.
- `SkFontGetGlyphWidthForHarfBuzz` rounds every glyph advance to a whole pixel
  for a non-subpixel font.

Open UI's aliased census profile keeps raster hinting off. It applied the
forcing rule to that unhinted configuration, so it shaped with fractional
advances. Ahem at integer sizes hid this, because its advance is then already
whole.

An explicit census of the existing `chromium-linux-monochrome` policy measured
both halves together: full-hinted raster plus whole-pixel advances gave 284
gains and 10 losses. The losses came mostly from full-hinted rasterization,
which differs between Open UI's and Chromium's FreeType at large and
fractional strikes. This change therefore takes only the shaping half, which
is the part Blink's source determines.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| On Linux Chromium backends, aliased text shapes with a non-subpixel font, so HarfBuzz advances round to whole pixels; the paint strike keeps its configured raster hinting | `openui-text` | `font/platform.rs` `with_source_configuration` |
| The shaping-font unit test expects no forced subpixel positioning for aliased configurations | `openui-text` | same file, tests |
| `atomic_intrinsic_text_keeps_chromium_fraction_after_native_font_alias_change` expects 144px for nine 16px aliased Ahem glyphs on every platform. Its earlier 144.015625 was a Chromium observation of an `@font-face` Ahem, which Blink renders with the default antialiased style rather than the aliased Fontconfig rule this configuration models | `openui` | `tests/native_font_units.rs` |
| `actual_source_and_shaping_settings_distinguish_metric_positioning` expects no forced subpixel shaping font for aliased text | `openui-engine` | `tests/font_source_provenance.rs` |

Windows text is unchanged (its shaping already uses the paint font).

## Measurement

Paired expanded census in WSL against `main`'s rendering (clone commits
`accfd7e6` and `1bdddcb4`), sharing one Chromium cache:

| Result | Comparisons |
| --- | --- |
| Exact | 21,750 to 21,984 |
| Gained | 239 (130 tests; 30 at all four profiles) |
| Improved | 182 |
| Lost | 5 |
| Worsened | 14 |

By profile, the gains are 71 at 1×, 35 at 2×, 50 at 1.25× and 83 at 1.5×. Most are multicol
(122), flexbox (58), overflow (27) and backgrounds (16) tests with 16pt or other
fractional-size Ahem text. Chromium is identical in every comparison. Rows are
in the [evidence](evidence/aliased-text-whole-pixel-advances-census-v1.json).

### Losses

| Test | Profile | Pixels | Observation |
| --- | --- | --- | --- |
| `css_sizing/block-size-with-min-or-max-content-1-ref`, `-1a`, `-1b` | 1.25× | 114 each | Vertical (sideways) 15px Ahem runs end one device row lower than Chrome's; they matched with the previous fractional advance. |
| `css_overflow/scroll-markers_scroll-buttons-enabled` and `-ref` | 2× | 5 each | The default 13.333px button text's glyph ends one column short. Chrome's result matched the fractional advance, so button text may not take the aliased path in Chromium. |

Both owners remain open in `openui-text` (vertical glyph run placement, and
button text render style). The 14 worsened comparisons were already inexact;
most are vertical or flex text tests whose remaining differences shift by
small amounts.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 21,750 to 21,984 exact; 239 gains, 5 losses |
| Linux unit tests, WSL | `cargo test --locked -p openui-text -p openui-layout -p openui-paint --lib` | 346, 749 and 252 pass |
| Linux font tests, WSL | `cargo test --locked -p openui --test native_font_units -p openui-engine --test font_source_provenance` | 3 and 8 pass |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,459 passed, 0 failed, 20 ignored |
