---
id: 0055
title: SP19 W1C closes the fragmentation repair partition
tags: sp19 fragmentation multicol inline-continuation floats margins outlines accountability handoff
status: active
created: 2026-08-31
updated: 2026-08-31
refs: bindings/rust/openui-layout/src/block.rs, bindings/rust/openui-layout/src/inline/algorithm.rs, bindings/rust/openui-layout/src/intrinsic_sizing.rs, bindings/rust/openui-paint/src/painter.rs, tools/wpt/port_wpt.py
---

SP19 W1C repairs all 10 frozen fragmentation targets while preserving the 4,119 exact W1B surface. Shared fixes cover fragmentainer progress for oversized lines and block-start decoration, inline and relative continuations, propagated trailing margins, fragmented flex descendants and floats, repeated fragmented outlines, preserved-newline float source positions, and physical font-size units in deterministic ports. Three repaired fixtures intentionally advance the retained-text manifest from 1,289 to 1,292 rows.

The authoritative no-resume proof is 4,129 exact, 10 failures, and zero errors across 4,139 runnable rows. The remaining failures exactly equal the frozen W1D position/paint/display partition. The complete `css_break` area is 721/721 exact, including historical line-clamp and tall-line regressions encountered while tightening the generic fragmentation rules.

Verification passes the 10-ID W1C release gate, the complete locked style/DOM/text/layout/paint Rust matrix, 150 historical Python tests, release `pixel_compare`, two identical SP19 no-write checks reporting `wave=w1c_fragmentation`, deterministic mapping/deferred/report/template regeneration, formatting, diff checks, and repository audit 7/7.
