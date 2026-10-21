---
id: 0069
title: Vertical flow aligns on Blink's central baseline; text stays on its glyph baseline
tags: layout, inline, baseline, vertical, flex, blink
status: active
created: 2026-10-21
updated: 2026-10-21
refs: docs/progress/vertical-central-baseline-2026-10-21.md, bindings/rust/openui-text/src/font/metrics.rs, bindings/rust/openui-layout/src/inline/algorithm.rs
---

Since 21 October 2026 create_line_box measures every line from the container's FontBaseline (central for vertical flow unless sideways, Blink GetFontBaseline). FontMetrics::relative_to_baseline gives Blink's integer split Height-Height/2 / Height/2; atomic inlines without a baseline synthesize (size-size/2, size/2) from the margin box. Text, hyphen, ellipsis and clamp-marker fragments must keep their alphabetic geometry around the glyph baseline (line baseline + int alphabetic ascent - central ascent): moving the fragment top changes the vertical-lr Ahem anchor snap in the painter and shifts glyph columns (34 census losses before this). Baselines are exposed only in the same writing mode (block propagation and inline_block_last_line_baseline); vertical flex rows use frag baseline or synthesize at cross_size/2. Known residual: alignment_flex-align-baseline-line-clamp-003 (clamp marker line geometry). Census tools: out/census_delta.py pattern, paired base vs head with a shared Chromium cache.
