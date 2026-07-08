---
id: 0018
title: ROOT CAUSE: OpenUI Skia font-mgr doesn't resolve user-installed Ahem (parity blocker)
tags: sp14, text, font, finding, blocker
status: superseded
created: 2026-07-08
updated: 2026-07-08
refs: bindings/rust/openui-text/src/font/cache.rs:89, bindings/rust/openui-text/src/font/platform.rs
---

Smoke test sp13/inline_single_span stuck at 0.31% because of a FONT-RESOLUTION ASYMMETRY, not rasterization:
- Chromium reference renders REAL Ahem: darkpx=1280, bbox x[20..99] y[20..39] (4x 20px em blocks).
- OpenUI renders a proportional FALLBACK: darkpx~143, bbox x[21..59] y[23..37] (~9.5px/char) — clearly not Ahem.
- fc-match Ahem -> Ahem.ttf (fontconfig DOES know it); Ahem is at ~/.local/share/fonts (USER dir); DejaVu at /usr/share/fonts (SYSTEM). Chrome (fontconfig) finds Ahem; OpenUI's Skia FontMgr::default() (openui-text/src/font/cache.rs:89 match_family_style) does NOT.
Consequence: the env-var rasterization harness in platform.rs CANNOT fix this — the two sides render different fonts. Fix must make both sides use the same font.
FIX PATH (skia-safe 0.82 API available): FontMgr::new_from_data(&ttf_bytes, None) loads a Typeface directly from a .ttf; TypefaceFontProvider registers named typefaces; OrderedFontMgr chains providers so system fonts still resolve. Architecturally correct: a pixel-parity engine should PIN exact fonts (Ahem, DejaVu) in-process, not depend on ambient system font config. Avoids the entry-0007 "install Ahem system-wide" regression concern.
RESOLVED 2026-07-08: two fixes landed — (1) pinned Ahem in openui-text FontCache via OrderedFontMgr+TypefaceFontProvider (vendored bindings/rust/openui-text/fonts/Ahem.ttf, include_bytes!); (2) the builder must set font_family/font_size ON THE ElementTag::Text NODE (font does NOT inherit span->Text child in the pixel-compare builder path). Result: OpenUI now renders real Ahem, bbox identical to Chromium, mismatch 0.31%->0.04% (200px). Remaining residual is AA/hinting only — tuning via OPENUI_* env harness.
