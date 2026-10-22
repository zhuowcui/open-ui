---
id: 0073
title: Windows text raster policy follows Windows Blink; compare_windows_text.py measures it
tags: windows, text, raster, directwrite, qualification
status: active
created: 2026-10-22
updated: 2026-10-22
refs: bindings/rust/openui-text/src/font/platform.rs, bindings/rust/openui-text/src/shaping/shape_result.rs, bindings/rust/openui-paint/src/painter.rs, bindings/rust/openui-paint/src/emphasis_painter.rs, tools/qualification/compare_windows_text.py, docs/progress/windows-text-raster-policy-2026-10-22.md
---

openui_text::GlyphRasterizer::platform() is DirectWrite on Windows, FreeTypeOrFontations elsewhere. Under the ChromiumLinux/GaneshGl backends on DirectWrite, FontPlatformData models Windows Blink CreateSkFont: author_text settings for every face (native/embedded included), FontHinting::Normal, subpixel = edging != Alias, no linear metrics, shaping font = paint font (aliased advances round to whole pixels). Fontations-compatible outlines and the Ahem block-start/emphasis row replays apply only to FreeTypeOrFontations; they are Linux mask emulations. The Skia backend keeps the requested policy. tools/qualification/compare_windows_text.py builds pages from the templates, injects the bundled faces via @font-face, runs Windows CfT 147.0.7727.50 with NoFontAntialiasing (+SP18 features), and renders Open UI with the census raster config; use a release pixel_compare (debug is ~5 s/test). legacy-800x600@1, 2,927 Ahem tests: main 2,134 exact, this change 2,785, 0 losses. Remaining: native theme colors (accent 0,120,212; greys 240/156), Windows Chrome font fallback for fullwidth digits, and text layout (DirectWrite unrounded ascent 13.008 is a suspected owner).
