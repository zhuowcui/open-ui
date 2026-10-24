---
id: 0074
title: Windows layout uses Blink's rounded ascent and descent
tags: windows, text, metrics, directwrite, layout
status: active
created: 2026-10-22
updated: 2026-10-23
refs: bindings/rust/openui-text/src/font/platform.rs, docs/progress/windows-rounded-font-metrics-2026-10-22.md
---

Under the Chromium-parity backends on DirectWrite (windows_chromium in FontPlatformData::with_source_configuration), metrics.ascent and metrics.descent are rounded after overrides, matching Blink FontMetrics::AscentDescentWithHacks for layout (subpixel_ascent_descent is canvas-only). DirectWrite's non-linear values (16px Ahem 13.008/3.008) shifted fragment tops by 1/64 px and moved lines across column boundaries. Linux keeps FreeType's float metrics. Windows Ahem census at legacy-800x600@1: 2,785 to 2,787 exact, 0 losses. The Windows range accent (0,120,212) is the host DWM AccentColor that Chrome's native theme reads as user_color. All four profiles on main 69e0fb47: 2,787 / 2,698 (@2) / 2,364 (@1.25) / 2,487 (@1.5) exact of 2,927; overflow-replaced-element-002(+ref) never stabilize in Windows Chrome at fractional scales (recorded as errors). Superseded 23 October 2026: Windows ascent/descent now come from linear metrics (set_linear_metrics(true)), unrounded, matching Linux FreeType inputs; layout does its own Blink rounding. Four profiles 10,342 -> 10,369 exact (+33/-6); 4 losses equal Linux Open UI diffs, 2 (sticky-fixed-ancestor-002/003 @1.5, 40vh = 648px strike) need DirectWrite hinted metrics. A hinted-rounding-delta variant was +33/-7 and not adopted.
