---
id: 0081
title: Aliased Linux text uses whole-pixel advances
tags: text, shaping, linux, hinting, census
status: active
created: 2026-10-25
updated: 2026-10-25
refs: bindings/rust/openui-text/src/font/platform.rs, docs/progress/aliased-text-whole-pixel-advances-2026-10-25.md
---

Chromium Linux forces full hinting and no subpixel positioning when Fontconfig disables antialiasing (font_render_params_linux.cc); Blink then rounds HarfBuzz advances (SkFontGetGlyphWidthForHarfBuzz). Open UI's shaping font for aliased Chromium-backend text is now non-subpixel (+239 census gains, 21,750 -> 21,984). Do NOT also switch raster hinting to Full: the full chromium-linux-monochrome policy measured +284/-10, the extra losses from full-hinted rasterization (input text, 40vh strikes). 5 losses remain: vertical sideways 15px Ahem at 1.25 (block-size-with-min-or-max-content-1*) and 13.333px button text at 2x (scroll-buttons-enabled).
