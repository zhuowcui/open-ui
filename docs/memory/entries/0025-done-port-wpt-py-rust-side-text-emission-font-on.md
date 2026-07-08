---
id: 0025
title: DONE: port_wpt.py Rust-side text emission (font on Text nodes) implemented + verified
tags: sp14, text, pilot, port_wpt, progress
status: active
created: 2026-07-08
updated: 2026-07-08
refs: tools/wpt/port_wpt.py
---

Implemented + verified the Rust-side porter fix (uncommitted->committing): EMIT_TEXT_NODES now emits the Text node WITH font on it. Added _font_family_to_rust (family/generic -> FontFamilyList), _family_from_font_shorthand (extract family from `font: 20px/1 Ahem`), threaded font-family via EXPLICIT_INHERIT_PROPS only (NOT INHERITED_PROPS, so elements/box-only output unaffected), and made generate_style_code derive font-size from the `font` shorthand (so effective font-size threads to text). EMIT_TEXT_NODES block now sets font_size(=parent_font_size), font_family, color on the emitted Text node. Verified on a sample: `font:20px/1 Ahem; color:green` div -> Text node emits font_size=20.0, font_family=FontFamilyList::single("Ahem"), color green, text. EMIT_TEXT_NODES=False -> Text nodes skipped (box-only unchanged). Remaining for pilot: separate-module porting (see divergence gotcha), HTML-template text retention, registry, build, compare, guard, full wpt, audit, reclassify.
