# Windows DirectWrite text against Windows Chrome, 21 October 2026

Windows Open UI uses DirectWrite text. Its reference is the Windows build of
the pinned release, Chrome-for-Testing 147.0.7727.50 `win64`, while Linux Open
UI keeps the Linux oracle. This record covers application-font loading, the
Windows reference tooling and the qualification of Windows-pending text tests.

## Windows reference browser

| Item | Value |
| --- | --- |
| Archive | `storage.googleapis.com/chrome-for-testing-public/147.0.7727.50/win64/chrome-win64.zip`, SHA-256 `d014348f…3522e9` |
| `chrome.exe` | SHA-256 `5a261cd9e0fa71f40fa5df4b9841da4f3311f9175d2e5868c27cb510ccd28505`, with the `147.0.7727.50.manifest` beside it |
| Location | ignored `out/chrome-win/win-147.0.7727.50/chrome-win64/` |
| Driver | `tools/qualification/windows_chrome.py`, using the harness's CDP client and ending every browser child before profile removal |

Tests that use Open UI's deterministic aliased raster configuration are
measured with `--enable-blink-features=NoFontAntialiasing`. Blink's
`FontPlatformData::QuerySystemForRenderStyle` turns antialiasing off under
that feature, the Windows counterpart of the Linux oracle's no-antialiasing
Fontconfig. Ahem is loaded with `@font-face` from the repository bytes and
never installed system-wide.

## Accepted changes

| Change | Owner | Source |
| --- | --- | --- |
| WOFF and WOFF2 application fonts are decoded to sfnt with `wuff` 0.2.9 before DirectWrite, as Chromium's OpenType Sanitizer does; registration keeps the original bytes; Linux unchanged | `openui-text` | `font/collection.rs` `platform_font_data` |
| Windows Chrome session and text reference probe, with unit tests | qualification tooling | `tools/qualification/windows_chrome.py`, `probe_windows_text_oracle.py`, `test_windows_chrome.py` |
| Six formerly ledgered tests take Windows expectations from the [Windows reference evidence](evidence/windows-text-oracle-v1.json); Linux expectations are unchanged | tests | `openui/tests/native_font_units.rs`, `native_line_height.rs`, `openui-layout/src/block.rs`, `flex/algorithm.rs` |

The `wuff` lockfile change is additive: `wuff`, `brotli-decompressor` and
`alloc-no-stdlib`.

## Windows Chrome agrees with Open UI's DirectWrite metrics

For every converted value, Windows Chrome reproduces what Open UI already
computes on Windows, and differs from the Linux FreeType oracle:

| Scenario | Linux oracle | Windows Chrome and Open UI on Windows |
| --- | --- | --- |
| Loaded Ahem `ABCDEFGHI` at 16px | 144.015625 | 144 |
| `2.5ex`, Ahem 18.72px → 20.37px | 37.40625 → 40.734375 | 37.484375 → 39.984375 |
| `line-height: 2ex` inherited width | 121.625 | 119.9375 |
| `2.5ex`, `3.75ex`, then 32px, then cancel | 96, 144, 192, 128 | 95.03125, 142.546875, 195.109375, 130.078125 |
| Two cleared `<br>` in system sans-serif | 39 (DejaVu Sans) | 38 (Arial) |

The `ch`, `lh`, `em`, `rem` and percentage values agree with Linux. Blink's
font-size quantization for font-relative references (18.72px behaves as
18.71) is platform-independent and already modelled.

## Remaining Windows failures

| Test | Owner | Observation |
| --- | --- | --- |
| `openui-layout::flex::algorithm::tests::vertical_column_cross_margin_reduces_fit_content_before_wrapping` | inline layout | Windows Chrome's vertical-lr line block size for system sans-serif (Arial 16px) is 18; Open UI gives 18.5. Open UI aligns vertical-flow atomics on an alphabetic baseline synthesized at their block-end edge, while Blink's `ComputedStyle::GetFontBaseline` uses the central baseline for vertical flow (except sideways). DejaVu's metrics hide the difference on Linux. The fix changes every vertical line box and needs its own census-verified task |
| `openui-engine::source_layer_raster::original_edge_and_all_eight_controls_keep_real_property_boundaries` | overflow-control paint / Skia toolchain | Unchanged: passes only with the pinned Chromium toolchain |

The WOFF and WOFF2 registration failures are fixed.

## Linux-oracle ledger

Five tests remain ignored outside Linux because their premise is Linux-only.
Three assert that FreeType keeps a fractional advance for loaded Ahem, which
DirectWrite does not. Two assert Fontconfig fallback face selection. They are
listed in [windows-pending-tests.txt](../v02/generated/windows-pending-tests.txt).

## Verification

Local measurements of this branch:

| Check | Command | Result |
| --- | --- | --- |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,402 passed, 2 failed, 18 ignored (13 pre-existing plus 5 ledgered). The failures are the vertical baseline gap and the toolchain-dependent arrow test. Before this branch: 9,395 passed, 3 failed, 24 ignored |
| Linux workspace, WSL | `cargo --config 'target.x86_64-unknown-linux-gnu.rustflags=[]' test --workspace --locked --no-fail-fast --exclude pixel-compare` | 9,402 passed, 1 failed (the arrow test), 13 ignored, unchanged from `main` |
| WOFF tests, Windows | `cargo test -p openui-text --lib font::collection`; `cargo test -p openui-ffi --lib c_font_metadata` | Pass |
| Windows reference probe | `python tools/qualification/probe_windows_text_oracle.py --output docs/progress/evidence/windows-text-oracle-v1.json` | All six scenarios repeat exactly |
| Tool and ledger tests | `python -m unittest tools.qualification.test_windows_chrome tools.ci.test_check_platform_scoped_tests`; `python tools/ci/check_platform_scoped_tests.py` | Pass; 5 recorded, ledger exact |
| Lockfile | `cargo metadata --locked` | Pass; additive change only |
