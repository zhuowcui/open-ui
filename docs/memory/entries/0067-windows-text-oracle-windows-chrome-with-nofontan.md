---
id: 0067
title: Windows text oracle: Windows Chrome with NoFontAntialiasing for aliased tests
tags: windows, fonts, directwrite, oracle, chromium
status: active
created: 2026-10-21
updated: 2026-10-21
refs: docs/progress/windows-directwrite-text-2026-10-21.md, tools/qualification/windows_chrome.py, tools/qualification/probe_windows_text_oracle.py
---

Windows Open UI text (DirectWrite) is qualified against pinned Windows Chrome-for-Testing 147.0.7727.50 win64 (chrome.exe sha256 5a261cd9...28505, unpacked under out/chrome-win). Tests using RasterConfiguration::deterministic_aliased must be measured with --enable-blink-features=NoFontAntialiasing, which Blink's FontPlatformData::QuerySystemForRenderStyle maps to aliased text; antialiased Windows Chrome gives different ex and ch values. Load Ahem with @font-face, never install it. Windows Chrome matched every DirectWrite metric Open UI already computed for the converted tests; the remaining vertical flex mismatch (18.5 vs 18) is a general layout gap: Blink's GetFontBaseline uses the central baseline for vertical flow, Open UI aligns atomics on an alphabetic synthesized baseline. rust-skia's Windows Skia has no FreeType, so WOFF/WOFF2 are decoded with wuff before DirectWrite.
