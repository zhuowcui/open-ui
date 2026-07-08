---
id: 0017
title: Uncommitted SP14 glyph-parity WIP in tree (continuation point)
tags: sp14, text, wip, uncommitted
status: active
created: 2026-07-08
updated: 2026-07-08
refs: bindings/rust/openui-text/src/font/platform.rs, bindings/rust/pixel-compare/src/main.rs, tools/accountability/run_all_pixel_comparisons.py
---

As of 2026-07-08 the working tree has UNCOMMITTED SP14 glyph-parity WIP (from recovered session 4ab79aa9), the literal continuation point:
- platform.rs: added env-var overrides for SkFont rasterization to tune parity without recompiling — OPENUI_SUBPIXEL(0/1), OPENUI_HINTING(none/slight/normal/full), OPENUI_EDGING(alias/aa/subpixel), OPENUI_AUTOHINT(0/1). Useful harness; keep.
- main.rs sp13_inline_single_span(): now builds a real ElementTag::Text child (proves text path).
- run_all_pixel_comparisons.py + inline_single_span/test.html: smoke test switched to Ahem 20px black.
KNOWN INCONSISTENCY to fix first: the builder text ("A single inline span") does NOT match the reference template text ("Xpqg") — reconcile them before trusting the 0.356% smoke number. result.json is an untracked run artifact.
