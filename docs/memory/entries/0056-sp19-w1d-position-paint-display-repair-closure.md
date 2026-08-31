---
id: 0056
title: SP19 W1D closes position, paint, and display repairs
tags: sp19 position sticky inline-containing-block rounded-clips masks display-contents accountability handoff
status: active
created: 2026-08-31
updated: 2026-08-31
refs: bindings/rust/openui-layout/src/block.rs, bindings/rust/openui-layout/src/inline/algorithm.rs, bindings/rust/openui-layout/src/sticky.rs, bindings/rust/openui-paint/src/painter.rs, bindings/rust/openui-style/src/computed.rs, tools/wpt/port_wpt.py
---

SP19 W1D repairs all 10 frozen position/paint/display targets while preserving every W1A–W1C repair and the complete SP18 exact surface. Shared fixes cover positioned-inline containing blocks and continuation shells, relative percentages, viewport and nested-scrollport sticky constraints, rounded overflow clips, raster mask compositing, mixed `display:contents` layout, and whitespace at unboxed inline boundaries. Structural rules distinguish genuine empty first inline boxes from empty fragmentation continuations without test-ID branches.

The authoritative no-resume proof is 4,139 exact, zero failures, and zero errors across all 4,139 runnable rows. The mapping now records no runnable residuals, the deferred ledger is empty, and the SP19 validator reports `wave=w1d_position_paint_display` while retaining the frozen 81-repair, 823-layout-target, 221-JavaScript-exclusion, 4,962-focused, and 2,711-projected-unported boundaries.

Verification passes the complete locked style/DOM/text/layout/paint Rust matrix, 150 historical Python closure/porter tests, release `pixel_compare`, two identical SP19 no-write checks, deterministic mapping/deferred/report and raw porter regeneration, formatting, diff checks, and repository audit 7/7.
