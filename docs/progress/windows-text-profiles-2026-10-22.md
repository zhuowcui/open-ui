# Windows text across the census profiles, 22 October 2026

The [Windows text raster record](windows-text-raster-policy-2026-10-22.md) and
the [rounded metrics record](windows-rounded-font-metrics-2026-10-22.md)
measured only `legacy-800x600@1`. This record measures the same 2,927
registered-Ahem expanded-suite tests at all four census profiles on `main`
`69e0fb47`. It changes no code.

## Results

`python tools/qualification/compare_windows_text.py --pixel-compare <release pixel_compare.exe> --profile <profile> --linux-summary <summary> --output <report>`,
Windows Chrome-for-Testing 147.0.7727.50 `win64`:

| Profile | Windows exact | Linux exact (same tests) | Linux-exact, Windows-different | Linux-different, Windows-exact | Errors |
| --- | --- | --- | --- | --- | --- |
| `legacy-800x600@1` | 2,787 | 2,761 | 41 | 67 | 0 |
| `mobile-375x667@2` | 2,698 | 2,656 | 24 | 66 | 0 |
| `desktop-1280x720@1.25` | 2,364 | 2,338 | 32 | 58 | 2 |
| `desktop-1920x1080@1.5` | 2,487 | 2,422 | 24 | 89 | 2 |
| Total | 10,336 of 11,708 | 10,177 | 121 | 280 | 4 |

The Linux column comes from the private central4 expanded census of
`896015de` and is used only to classify tests. Per-test results with RGBA
hashes are in the [evidence](evidence/windows-text-profiles-v1.json). This is
one host; it is not a Windows census or a qualification claim.

## Remaining differences

Most Windows differences are shared with Linux. Of the tests exact on Linux
but not on Windows:

| Group | Owner | Observation |
| --- | --- | --- |
| Native theme, all profiles | native controls | As at `legacy-800x600@1`: the range accent is the host's Windows accent color, plus Windows fieldset, button and scroll-button colors |
| Font fallback and text layout, all profiles | font fallback, inline layout | As at `legacy-800x600@1` |
| Scale-specific, 17 tests | layout at fractional device scale | Two at 2× (`scroll-buttons-enabled` and its reference, 5 pixels each). Eleven at 1.25×: `flexbox-writing-mode-011` to `013`, `flexbox-align-self-horiz-004` and `block-size-with-min-or-max-content-1*`, where boxes beside 12px and 20px Ahem text sit one device column off. Four at 1.5×. Not yet attributed; not caused by the metric rounding (the pre-rounding build `3db12944` shows the same 1.25× differences, 2,363 exact, and rounding loses none) |

The four errors are `css_overflow/overflow-replaced-element-002` and its
reference at 1.25× and 1.5×. Windows Chrome's screenshots of them never
stabilized, even on a fresh browser, so they are reported as errors, not
comparisons. Both tests also differ on Linux. Eight of the nine browser restarts
across the two fractional profiles came from these tests. The ninth, a
dropped connection, recovered on retry.

## Verification

| Check | Result |
| --- | --- |
| Four profile runs with the release build of the `main` rendering sources | Complete; counts above |
| Chrome identity | `chrome.exe` SHA-256 recorded in the evidence |
