---
id: 0027
title: BREAKTHROUGH: fontconfig kills Chrome text AA -> exact 0-mismatch text passes
tags: sp14, text, font, finding, fontconfig
status: active
created: 2026-07-14
updated: 2026-07-14
refs: tools/accountability/run_all_pixel_comparisons.py, bindings/rust/openui-text/fonts/Ahem.ttf
---

FONTCONFIG_FILE with <match target=font><test family eq Ahem><edit antialias=false hinting=false autohint=false rgba=none> makes Chrome headless rasterize Ahem binary {0,255}. OpenUI default render already matches EXACTLY: sp13/inline_single_span = 0 mismatched px / 471000 (was 0.04% near_miss_aa). Supersedes the 'accept near_miss_aa' ceiling from 0019 for Ahem-forced tests. Config must <include> system fonts.conf + point <dir> at vendored bindings/rust/openui-text/fonts. Zero risk to text-free corpus (no glyphs painted). Strategy: text-retaining ports force body font-family:Ahem on BOTH sides (template style override + vp.font_family in Rust fn), scoped per re-ported test only (ch/ex units in untouched tests unaffected).
