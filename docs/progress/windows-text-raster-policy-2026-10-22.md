# Windows text raster policy, 22 October 2026

The [Windows text record](windows-directwrite-text-2026-10-21.md) checked
DirectWrite metrics against Windows Chrome but not text pixels. This change
measures the expanded suite's Ahem text tests against Windows Chrome and makes
Open UI's Chromium-parity text policy follow Windows Blink on Windows.

## Findings

Windows Blink differs from the Linux oracle in four ways that Open UI modelled
only for Linux. Sources are Chromium 147.0.7727.50:

| Windows Blink | Source | Linux oracle |
| --- | --- | --- |
| Conventional web fonts load through DirectWrite (`DefaultFontMgr()->makeFromData`) | `web_font_typeface_factory.cc` `MakeTypefaceDefaultFontMgr` | Fontations; system faces FreeType |
| `CreateSkFont` never sets hinting, so SkFont's default `Normal` stays | `win/font_platform_data_win.cc` | Fontconfig's per-face, per-strike answer |
| Subpixel positioning only with antialiasing; the shaping font is the paint font, so aliased HarfBuzz advances are rounded to whole pixels | `font_platform_data_win.cc`, `skia/skia_text_metrics.cc` | `WebFontRenderStyle::ApplyToSkFont` forces subpixel metrics unless hinting is full |
| One process-wide render style; `NoFontAntialiasing` aliases every face | `QuerySystemForRenderStyle` | Fontconfig pins aliasing to the deterministic faces |

Windows Chrome confirms the advance rounding: five Ahem glyphs at 18.72px,
13.3px and 20.5px measure 95, 65 and 105 with `NoFontAntialiasing` and 93.5625,
66.5 and 102.5 without it.

Before this change, Windows Open UI also replayed two Linux mask artifacts.
Under the deterministic text profile, Open UI repaints an Ahem run one row
higher when its fragment top lies at or past a half pixel. DirectWrite's
unrounded Ahem ascent of 13.008 (FreeType: 12.8) puts that top at 19.98 instead
of 20.19, so Windows painted an extra row above most Ahem text. The half-em
emphasis replay did the same.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `GlyphRasterizer::platform()` names the target's Chromium rasterizer: DirectWrite on Windows, FreeType or Fontations elsewhere | `openui-text` | `font/platform.rs` |
| Under the `ChromiumLinux` and `GaneshGl` backends on DirectWrite: author render style for every face, `Normal` hinting, subpixel positioning only with antialiasing, no linear metrics, shaping with the paint font | `openui-text` | `FontPlatformData::with_source_configuration` |
| Fontations-compatible outlines only for FreeType or Fontations | `openui-text` | `shaping/shape_result.rs` |
| Ahem block-start and emphasis row replays only for FreeType or Fontations | `openui-paint` | `painter.rs` `replays_aliased_ahem_start_for_raster_policy`, `emphasis_painter.rs` |
| Windows text comparison tool with unit tests, in both CI unittest lists; a page whose browser stops answering is retried once on a fresh browser and otherwise recorded as an error | qualification tooling | `tools/qualification/compare_windows_text.py`, `test_compare_windows_text.py` |

Every behavior change is gated on DirectWrite, so Linux output is unchanged.
The plain `Skia` backend keeps the application's requested policy on Windows.
Three unit tests now state each platform's expectation instead of the Linux one.

## Measurement

`compare_windows_text.py` builds each page from the checked-in templates
with the census harness. It loads the bundled faces through `@font-face` from
the repository bytes, captures Windows Chrome 147.0.7727.50 `win64` with
`NoFontAntialiasing` (plus the SP18 line-clamp features for SP18 targets) and
renders Open UI with the census raster configuration. It selects the 2,927
expanded-suite text-port tests outside the real-font list, at
`legacy-800x600@1`.

| Build | Windows exact | Of the 2,761 Linux-exact | Of the 166 Linux-different |
| --- | --- | --- | --- |
| `main` `fbf98d62` | 2,134 | 2,091 | 43 |
| This change `3db12944` | 2,785 | 2,718 | 67 |

The change gains 651 exact tests and loses none. Windows Chrome produced
byte-identical frames for all 2,927 tests in both runs. Per-test results
are in the [evidence](evidence/windows-text-raster-policy-v1.json). The Linux
column comes from the private central4 expanded census of `896015de` and is
used only to classify tests. This is one profile on one Windows host; it is
not a Windows census or a qualification claim.

## Remaining Windows differences

142 tests differ, 99 of which also differ on Linux. Of the 43 that are exact on
Linux:

| Group | Tests | Owner | Observation |
| --- | --- | --- | --- |
| Native theme | 22 | native controls | Range accent `(0,120,212)` against `(0,117,255)`, fieldset and button greys 240/156 against 239/155, scroll-button faces and arrows. Windows Chrome's theme colors differ from the Linux theme Open UI reproduces |
| Font fallback | 3 | font fallback | `css-flexbox-test1`, its reference and `webkit-line-clamp-008`: Windows Chrome reports Ahem for fullwidth digits that Ahem's cmap lacks, and picks a different face for one clamp-marker glyph |
| Text layout | 18 | inline layout | Line-clamp and block-ellipsis lines, vertical intrinsic sizing, `display: contents` sharing, iframe print pages, and `grid-nested-columns-crash`, where only Windows Open UI shows an extra line. Likely owner: Open UI uses the unrounded DirectWrite ascent where Blink uses `FontMetrics`' rounded value; not yet verified |

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,433 passed, 0 failed, 20 ignored (the full run, then the corrected `font_source_provenance` binary rerun) |
| Linux workspace, WSL | `cargo --config 'target.x86_64-unknown-linux-gnu.rustflags=[]' test --workspace --locked --no-fail-fast --exclude pixel-compare` | 9,432 passed, 0 failed, 15 ignored (`main`: 9,431; one new Windows-only test). Run before the provenance test edit, which leaves the Linux assertions unchanged |
| Windows text comparison | `python tools/qualification/compare_windows_text.py --pixel-compare <release pixel_compare.exe> --linux-summary <summary> --output <report>` for `main` and this change | 2,134 and 2,785 of 2,927 exact; 651 gains, 0 losses |
| Tool tests | `python -m unittest tools.qualification.test_compare_windows_text` on Windows and in WSL | 6 tests pass |

## Update, 22 October 2026

The suspected text-layout owner is confirmed for two tests: rounding the
DirectWrite ascent and descent as Blink does fixes them; see
[the rounded metrics record](windows-rounded-font-metrics-2026-10-22.md).
