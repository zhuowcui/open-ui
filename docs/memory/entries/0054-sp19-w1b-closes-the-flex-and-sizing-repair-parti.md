---
id: 0054
title: SP19 W1B closes the flex and sizing repair partition
tags: sp19 flex sizing baseline intrinsic-size aspect-ratio paint-order accountability handoff
status: active
created: 2026-08-31
updated: 2026-08-31
refs: bindings/rust/openui-layout/src/flex/algorithm.rs, bindings/rust/openui-layout/src/intrinsic_sizing.rs, bindings/rust/openui-layout/src/inline/algorithm.rs, tools/wpt/port_wpt.py
---

SP19 W1B repairs all 28 flex/sizing targets and preserves the 33 W1A repairs plus the full SP18 non-target surface. Shared fixes cover multiline flex baselines, generated flex items, absolute flex static positions, flex paint order, intrinsic min/max contributions, fit-content endpoints, aspect-ratio feedback, inline decoration rounding, vertical-writing baselines, and deterministic Ahem shaping. The live proof is 4,119 exact, 20 failures, and zero errors across 4,139 runnable rows; the remaining failures exactly equal the frozen W1C fragmentation and W1D position/paint/display partitions. Verification passes the combined 67-ID release gate, the complete locked style/text/layout/paint matrix, 150 historical Python tests, two identical SP19 no-write checks reporting wave=w1b_flex_sizing, deterministic mapping/deferred/template hashes, formatting, diff checks, and repository audit 7/7.
