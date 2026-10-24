# Windows linear font metrics, 23 October 2026

The [profiles record](windows-text-profiles-2026-10-22.md) left 17
scale-specific Windows differences unattributed. Most come from the font
metrics layout receives. This change gives Windows layout the same linear
metrics Linux layout receives.

## Finding

Open UI's layout applies Blink's integer metric handling to float design
metrics: `int_ascent` and `int_descent`, the central baseline's integer split,
and half-leading. On Linux those floats are FreeType's linear values (12px
Ahem: 9.6 and 2.4). On Windows, the
[rounded metrics change](windows-rounded-font-metrics-2026-10-22.md) fed
layout DirectWrite's GDI-compatible values rounded at the CSS size. Those
values are already grid-fitted (12px Ahem: 9.996 and 2.004, which round to 10
and 2). At 1.25×, Blink measures the zoomed 15px strike (ascent 12, descent 3,
so 9.6 and 2.4 CSS px), so Windows vertical-text boxes sat half a device pixel
off. Linear metrics remove that double grid-fitting.

For Ahem, the linear values round to the same integers as Blink's rounded
hinted values in nearly every case, so Windows layout now matches Linux
layout. The exception is very large strikes: at 648 physical pixels,
DirectWrite's hinted ascent and descent (516.456 and 131.544) round two pixels
away from the linear ones (518.4 and 129.6). Windows Chrome uses the hinted
values there.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| Under the Chromium-parity backends on DirectWrite, ascent and descent come from the font's linear metrics, unrounded, unless metric overrides apply. This replaces the CSS-size rounding of hinted metrics | `openui-text` | `font/platform.rs` `with_source_configuration` |
| The Windows render-style unit test asserts 12.8 and 3.2 for 16px Ahem | `openui-text` | same file, tests |

A variant that added the hinted rounding difference at the physical strike
fixed the two 1.5× losses below, but lost those tests at 2× and one more 2×
test (+33/−7), so it was not adopted.

## Measurement

`compare_windows_text.py`, the 2,927 registered-Ahem tests at all four
profiles, Windows Chrome-for-Testing 147.0.7727.50, release builds of `main`
`0a74d934` and this change:

| Profile | `main` exact | This change | Gains | Losses |
| --- | --- | --- | --- | --- |
| `legacy-800x600@1` | 2,788 | 2,794 | 6 | 0 |
| `mobile-375x667@2` | 2,699 | 2,702 | 4 | 1 |
| `desktop-1280x720@1.25` | 2,367 | 2,375 | 10 | 2 |
| `desktop-1920x1080@1.5` | 2,488 | 2,498 | 13 | 3 |
| Total | 10,342 | 10,369 | 33 | 6 |

Gains include `flexbox-writing-mode-011` to `-013` and their references at
1.25×, `vert-block-size-small-or-larger-than-container-with-min-or-max-content-2*`
at 1×, 2×, 1.25× and 1.5×, and `line-clamp-027` at every profile.

Four of the six losses now reproduce Linux Open UI's existing difference from
Chrome exactly, with the same mismatch counts: `moz-multicol3-column-balancing-break-inside-avoid-1-ref`
at 2× (224), `line-clamp-auto-with-ruby-003` and `-005.tentative` at 1.25×
(40 and 80), and `static-position_vlr-rtl-rtl` at 1.5× (1,152). Windows had
matched Chrome there only by its grid-fitted metrics. The remaining two,
`sticky_position-sticky-fixed-ancestor-002` and `-003` at 1.5×, are the
648-pixel strike above (font-size `40vh`). Windows Chrome frames are
identical across both runs. Per-test results are in the
[evidence](evidence/windows-linear-font-metrics-v1.json). Linux is unchanged
by construction. This is one Windows host, not a qualification claim.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Windows text comparison, four profiles | as above | 10,342 to 10,369 exact; 33 gains, 6 losses |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,455 passed, 0 failed, 20 ignored |
