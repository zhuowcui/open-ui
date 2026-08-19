---
id: 0020
title: Testing: authoritative set is wpt/ only; legacy .text builders render blank
tags: testing, accountability, text, guard, gotcha
status: active
created: 2026-07-08
updated: 2026-07-08
refs: tools/accountability/run_all_pixel_comparisons.py, bindings/rust/pixel-compare/src/main.rs
---

The authoritative 2671/735 baseline tracks ONLY the wpt/ suite (3406 tests). The sp11/*, sp13/* etc. builder tests are a SEPARATE dev set (pixel_compare `list` includes them) and are NOT in summary.json's tracked results.
GOTCHA for guards: sp11/* text tests render BLANK in OpenUI (openui_dark=0 while chrome paints) — the pre-existing "text not painted" bug: OpenUI's inline items builder paints text only from ElementTag::Text nodes, NOT from `.text` set on a Div/Span (which add_text_block and most legacy sp11/sp13 builders use). So sp11/* is NOT a valid guard for font/text changes — they were already failing for an unrelated reason.
Implication for SP14 porting: the pilot batch must emit ElementTag::Text nodes AND set font_family/font_size on the Text node itself (font does not inherit to the Text child in the builder path).
Valid regression check for the pinned-Ahem FontCache change: full wpt/ (box-only -> font resolution not invoked -> expected no-op) plus any ElementTag::Text test. The change is additive: OrderedFontMgr tries the pinned provider (Ahem only) then the system FontMgr, so non-Ahem families resolve exactly as before.
