---
id: 0022
title: SP14 pilot batch: plan + key porter gap (font on Text nodes)
tags: sp14, text, pilot, port_wpt, next-steps, plan
status: active
created: 2026-07-08
updated: 2026-07-08
refs: tools/wpt/port_wpt.py:3020, tools/wpt/port_wpt.py:3103, tools/wpt/batch_port_sp12.py
---

Goal: port a single-line LTR Ahem WPT pilot so text is pixel-compared to Chromium, then drive to parity + reclassify out of needs_text.
UNBLOCKED: WPT corpus present at ~/chromium/src/third_party/blink/web_tests/external/wpt/css (batch_port_sp12.py CHROMIUM_WPT). 4381 needs_text rows to pick single-line Ahem pilots from. Porter: tools/wpt/port_wpt.py; EMIT_TEXT_NODES=False default (per-pilot allowlist via EMIT_TEXT_FOR). Generated Rust -> bindings/rust/pixel-compare/src/wpt/wpt_css_*.rs; templates via write_html_templates.
KEY PORTER GAP (must fix first): EMIT_TEXT_NODES (port_wpt.py ~3103) emits `doc.create_node(ElementTag::Text)` + `.text=...` but sets NO font/color on the Text node. AND 'font-family' is missing from INHERITED_PROPS (~3084). In OpenUI's builder render path, the inline items builder shapes text from the Text node's OWN computed style (font does NOT inherit parent->Text child — proven by the smoke test). So ported text would render at default 16px, wrong font.
DECIDED APPROACH (low-risk, matches validated smoke fix): porter-side — set font_family/font_size/color ON the emitted Text node from the inherited context (add font-family to INHERITED_PROPS; reuse generate_style_code with {font-family, color, font-size=parent_font_size px}). Engine-side inheritance is the nicer-but-riskier alternative (defer).
CYCLE: fix porter -> pick tiny Ahem allowlist -> port+generate -> cargo build --release pixel-compare -> focused compare (snapshot summary.json first! entry 0008) -> fix bugs -> guard slices -> full wpt/ (must stay 2671/735/0) -> regen mapping/deferred -> audit 7/7 -> reclassify passing out of needs_text -> verifier -> commit. Only Ahem is pinned (entry 0019); Ahem tests resolve, non-Ahem fall through to system.
