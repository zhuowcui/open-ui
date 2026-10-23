---
id: 0074
title: Windows layout uses Blink's rounded ascent and descent
tags: windows, text, metrics, directwrite, layout
status: active
created: 2026-10-22
updated: 2026-10-22
refs: bindings/rust/openui-text/src/font/platform.rs, docs/progress/windows-rounded-font-metrics-2026-10-22.md
---

Under the Chromium-parity backends on DirectWrite (windows_chromium in FontPlatformData::with_source_configuration), metrics.ascent and metrics.descent are rounded after overrides, matching Blink FontMetrics::AscentDescentWithHacks for layout (subpixel_ascent_descent is canvas-only). DirectWrite's non-linear values (16px Ahem 13.008/3.008) shifted fragment tops by 1/64 px and moved lines across column boundaries. Linux keeps FreeType's float metrics. Windows Ahem census at legacy-800x600@1: 2,785 to 2,787 exact, 0 losses. The Windows range accent (0,120,212) is the host DWM AccentColor that Chrome's native theme reads as user_color. All four profiles on main 69e0fb47: 2,787 / 2,698 (@2) / 2,364 (@1.25) / 2,487 (@1.5) exact of 2,927; overflow-replaced-element-002(+ref) never stabilize in Windows Chrome at fractional scales (recorded as errors).
