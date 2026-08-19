---
id: 0004
title: Text pivot is porting+parity, NOT a from-scratch engine
tags: text, finding, port_wpt
status: active
created: 2026-07-08
updated: 2026-07-08
refs: tools/wpt/port_wpt.py, bindings/rust/openui-text, bindings/rust/openui-layout/src/inline/algorithm.rs
---

The text engine already exists (SP11 openui-text: font metrics, HarfBuzz/Skia shaping, bidi, hyphenation, emoji, emphasis). Inline layout already shapes text; glyph painter already renders. DOM supports ElementTag::Text + NodeData.text. THE GAP: tools/wpt/port_wpt.py emits box-only builders, so there are 0 ElementTag::Text nodes across ported WPT -> text is never pixel-compared to Chromium. Also legacy sp13/* builders mis-set .text on spans; proper Text nodes render correctly (matches Chromium bbox).
