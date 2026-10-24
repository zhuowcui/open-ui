# Windows fieldset groove shades, 24 October 2026

The [accent record](windows-form-control-accent-2026-10-24.md) left the
fieldset and button greys as the remaining native-theme differences. They
were attributed to system colors baked into the generated ports. They are
not: the fieldset's default border is `2px groove` with black `currentColor`.
Open UI paints Chromium's theme grays for that border from fixed values:
155 dark, 197 at the corner junctions and 239 light, measured from Linux
Chrome. Windows Chrome paints 156, 198 and 240 for the same border at every
census profile.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `PLATFORM_GROOVE_SHADES` holds the dark, corner and light grays: Windows `(156, 198, 240)`, elsewhere `(155, 197, 239)`. The corrected fieldset tone and the corner samples use it | `openui-paint` | `painter.rs` `shade_3d_dark`, `shade_3d_light`, `paint_native_fieldset_corner_samples` |

The uncorrected generic pair (154 and 238) is unchanged. No Windows
measurement covers it.

## Measurement

`compare_windows_text.py` with the host accent, 2,927 registered-Ahem tests
per profile, against `main` `d9cb0c26` (the accent runs):

| Profile | Before | After | Gains | Losses | Improved |
| --- | --- | --- | --- | --- | --- |
| `legacy-800x600@1` | 2,800 | 2,815 | 15 | 0 | 4 |
| `mobile-375x667@2` | 2,710 | 2,710 | 0 | 0 | 19 |
| `desktop-1280x720@1.25` | 2,383 | 2,383 | 0 | 0 | 19 |
| `desktop-1920x1080@1.5` | 2,506 | 2,506 | 0 | 0 | 19 |
| Total | 10,399 | 10,414 | 15 | 0 | 61 |

The gains are the fieldset tests at 1×: `fieldset-baseline-alignment`,
`aspect-ratio_fieldset-element-001`/`-002`, `multicol-span-all-fieldset-*`,
`display-contents-dynamic-generated-content-fieldset-001-ref`,
`line-clamp_webkit-line-clamp-027` and `scroll-marker-group-011`, with their
references. At fractional and 2× scales the same tests improve but stay
inexact; their remaining differences are fractional-edge coverage. Nothing
worsens, and Windows Chrome frames are identical across the runs. Per-test
results are in the [evidence](evidence/windows-fieldset-groove-shades-v1.json).
Linux is unchanged by construction.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Windows text comparison, four profiles | as above | 10,399 to 10,414 exact; 15 gains, 0 losses, 61 improved |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,457 passed, 0 failed, 20 ignored |
