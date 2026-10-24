# Platform form-control accent, 24 October 2026

The [Windows text raster record](windows-text-raster-policy-2026-10-22.md)
found that Windows Chrome draws range sliders and checked controls in
`(0,120,212)`, where Open UI drew Chromium's default `(0,117,255)`. Chromium's
native theme reads the operating system's accent into its `user_color`
(`ui/native_theme/native_theme.cc`, `os_settings_provider.AccentColor()`). On
Windows that is the DWM accent, `HKCU\Software\Microsoft\Windows\DWM\AccentColor`;
on this host it is `0xFFD47800`, RGB `(0,120,212)`, the Windows default.
Shades derived from the accent follow it too: the thumb edge, for example, is
an even blend of the accent and `(118,118,118)`.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The document carries an optional platform accent. `form_control_accent()` returns it, or Chromium's default `(0,117,255)` | `openui-dom` | `tree.rs` |
| `Engine::set_platform_accent_color` sets it and invalidates paint | `openui-engine` | `lib.rs` |
| Checkbox, radio and range painting use `form_control_accent()` instead of the fixed blue | `openui-paint` | `painter.rs` |
| `openui_platform::system_accent_color()` reads the DWM accent on Windows (`None` elsewhere). `App::run_document` applies it before the event loop starts | `openui-platform`, `openui` | `lib.rs`, `app.rs` |
| `pixel_compare render … --accent-color RRGGBB` | pixel-compare | `main.rs` |
| `compare_windows_text.py` passes the host DWM accent (or `--accent-color`) and records it in the report | qualification tooling | `compare_windows_text.py` |

With no platform accent, painting is unchanged, so Linux and every existing
Linux comparison are unaffected.

## Measurement

`compare_windows_text.py` with the host accent `0078d4`, against `main`
`1dbc7ce3` without an accent (the
[linear metrics](windows-linear-font-metrics-2026-10-23.md) runs), 2,927
registered-Ahem tests per profile:

| Profile | Before | With the accent | Gains | Losses |
| --- | --- | --- | --- | --- |
| `legacy-800x600@1` | 2,794 | 2,800 | 6 | 0 |
| `mobile-375x667@2` | 2,702 | 2,710 | 8 | 0 |
| `desktop-1280x720@1.25` | 2,375 | 2,383 | 8 | 0 |
| `desktop-1920x1080@1.5` | 2,498 | 2,506 | 8 | 0 |
| Total | 10,369 | 10,399 | 30 | 0 |

The gains are every range test in the selection: `flexbox_slider-in-multicol-crash`,
`flexitem-stretch-range` and `range-percent-intrinsic-size-1`, `-2` and `-2a`
with their references. Per-test results are in the
[evidence](evidence/windows-form-control-accent-v1.json). This is one host with
the default Windows accent, not a qualification claim.

## Remaining

The other native-theme differences are system colors. Windows Chrome's
`ButtonFace` and related greys (240 and 156) differ from the Linux values
(239 and 155) that the generated test ports bake in as literal colors. Fixing
them needs system colors resolved per platform at runtime.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Windows text comparison, four profiles | as above | 10,369 to 10,399 exact; 30 gains, 0 losses |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,457 passed, 0 failed, 20 ignored |
| Linux workspace, WSL | `cargo --config 'target.x86_64-unknown-linux-gnu.rustflags=[]' test --workspace --locked --no-fail-fast --exclude pixel-compare` | 9,456 passed, 0 failed, 15 ignored |
| Windows feature build | `cargo build -p openui --features windows`; `cargo test -p openui-platform --features windows --lib` | Pass; 7 tests |
| Tool tests | `python -m unittest tools.qualification.test_compare_windows_text` | 8 tests pass |
