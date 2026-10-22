# Windows rounded font metrics, 22 October 2026

The [Windows text raster record](windows-text-raster-policy-2026-10-22.md)
left 18 Linux-exact text-layout differences. One suspected owner was the
unrounded DirectWrite ascent. This change fixes it.

## Finding

Blink computes layout ascent and descent in
`FontMetrics::AscentDescentWithHacks` (Chromium 147.0.7727.50
`font_metrics.cc`). Outside VDMX and the canvas-only `subpixel_ascent_descent`
path, it rounds both with `SkScalarRoundToScalar`; on Windows no other
adjustment follows. Open UI kept Skia's unrounded values. With the aliased
Windows font (no linear metrics), DirectWrite reports GDI-compatible values,
for example 13.008 and 3.008 for 16px Ahem. Text fragment tops and heights
then sit about 1/64 px off Blink's, enough to move a line across a
fragmentainer or column boundary.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| Under the Chromium-parity backends on DirectWrite, `FontPlatformData` rounds ascent and descent as Blink's layout metrics do; Linux keeps its existing values | `openui-text` | `font/platform.rs` `with_source_configuration` |
| The Windows render-style unit test asserts 13 and 3 for 16px Ahem | `openui-text` | same file, tests |

## Measurement

Same tool, profile and selection as the raster record: 2,927 registered-Ahem
expanded-suite tests at `legacy-800x600@1` against Windows Chrome-for-Testing
147.0.7727.50.

| Build | Windows exact |
| --- | --- |
| `main` `c1148c25` | 2,785 |
| This change | 2,787 |

The two gains are `css_break/grid_grid-nested-columns-crash` and
`css_multicol/moz-multicol3-column-balancing-break-inside-avoid-1-ref`. In
both, only Windows Open UI had pushed a line into another column. There are no
losses, and no other test's mismatch count changes. Windows Chrome frames
are byte-identical across the runs, with no browser restarts. Results are in
the [evidence](evidence/windows-rounded-font-metrics-v1.json). This is one
profile on one host, not a qualification claim.

## Remaining Windows differences

140 tests differ: 99 also differ on Linux, and 41 are Linux-exact (22 native
theme, 3 font fallback, 16 text layout). The native-theme range accent is the
host's Windows accent color: Chrome's native theme reads
`os_settings_provider.AccentColor()` into its form-control `user_color`, and
this host's `HKCU\Software\Microsoft\Windows\DWM\AccentColor` is `0xFFD47800`
(RGB 0,120,212, the Windows default). Matching it needs Open UI to read the
same setting; that is separate native-control work.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Windows text comparison | `python tools/qualification/compare_windows_text.py --pixel-compare <release pixel_compare.exe> --linux-summary <summary> --output <report>` | 2,787 of 2,927 exact; 2 gains, 0 losses |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,433 passed, 0 failed, 20 ignored |
| Linux workspace, WSL | `cargo --config 'target.x86_64-unknown-linux-gnu.rustflags=[]' test --workspace --locked --no-fail-fast --exclude pixel-compare` | 9,432 passed, 0 failed, 15 ignored |
