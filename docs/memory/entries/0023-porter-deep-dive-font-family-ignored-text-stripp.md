---
id: 0023
title: Porter deep-dive: font-family IGNORED + text stripped from HTML template
tags: sp14, text, pilot, port_wpt, finding
status: active
created: 2026-07-08
updated: 2026-07-08
refs: tools/wpt/port_wpt.py:131, tools/wpt/port_wpt.py:3448, tools/wpt/port_wpt.py:3084
---

port_wpt.py details for the SP14 text pilot (two separate output paths):
1. Rust builder: generate_rust_fn(parser.root) -> gen_node; EMIT_TEXT_NODES gates Text-node emission (~3103). font-family is in IGNORED_PROPERTIES (~131) so NO element emits it and there is NO font-family->Rust generator. INHERITED_PROPS (~3084) lacks font-family; EXPLICIT_INHERIT_PROPS builds child_inherited (~3245). parent_font_size passed to children = parent element's node_font_size. generated modules `use openui_style::*` (FontFamilyList/GenericFontFamily available).
2. HTML template (Chrome ref): generate_html_template (~3279) STRIPS ALL bare text via TextStripper (~3448) — so for a text pilot the template must retain text too.
Fix points for pilot: (a) add a font-family->FontFamilyList generator; (b) thread font-family to text nodes via EXPLICIT_INHERIT_PROPS (NOT INHERITED_PROPS — that would apply to elements and churn the byte-identical box corpus) and also extract family from the `font` shorthand; (c) EMIT_TEXT_NODES: set font_size(=parent_font_size), font_family, color on the Text node; (d) make generate_html_template retain text for the pilot allowlist; (e) register the pilot module + templates for pixel compare. Keep EMIT_TEXT_NODES off => byte-identical box output.
