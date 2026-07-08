---
id: 0019
title: SP14 smoke at 0.04%: residual is sub-pixel AA/text-gamma, not geometry
tags: sp14, text, font, parity, finding, near_miss_aa
status: active
created: 2026-07-08
updated: 2026-07-08
refs: bindings/rust/openui-text/src/shaping/shape_result.rs:281, bindings/rust/openui-text/src/font/platform.rs
---

After pinning Ahem + fixing the builder, sp13/inline_single_span is 0.0425% (200px, max_channel_diff=88). Geometry is EXACT (both 1280 dark px, bbox x[20..99] y[20..39]). The 200 diff px are ONLY on vertical glyph-edge columns (x in 19/20,39/40,59/60,79/80,99/100), full height, mid-tone on BOTH sides.
Root cause: OpenUI positions Ahem glyphs at exact INTEGER x (to_text_blob advances=20.0 -> crisp 0/255 edges); Chromium renders the same glyphs with sub-pixel AA + text gamma/contrast (edge pixels e.g. 32 & 231, not 0 & 255). So Chrome's edges are softened/gamma'd; OpenUI's are crisp.
Consequence: the OPENUI_* env-var rasterization harness (platform.rs) has ZERO effect on this test — glyphs are integer-positioned, so subpixel/edging/hinting have nothing to act on (verified: default vs alias/none/subpixel0 -> 0 px change).
Last-mile options: (A) match Chromium's text gamma/contrast + AA in openui-paint glyph raster (hard, but the real 0.0% path and generalizes); (B) per plan.md policy, AA-only near-misses are tracked in a `near_miss_aa` bucket (never hidden) while exact 0.0% stays the bar. NOTE: font-manager change (pinned Ahem) still needs a zero-regression guard/full-wpt run before commit.
