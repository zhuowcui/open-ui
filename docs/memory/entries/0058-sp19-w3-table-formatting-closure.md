---
id: 0058
title: SP19 W3 closes the table formatting partition
tags: sp19 table fragmentation border-collapse captions rowspan colspan writing-mode accountability handoff
status: active
created: 2026-09-01
updated: 2026-09-01
refs: bindings/rust/openui-layout/src/table.rs, bindings/rust/openui-layout/src/block.rs, bindings/rust/openui-paint/src/painter.rs, tools/wpt/generate_sp19_closure.py, tools/wpt/test_sp19_closure.py
---

SP19 W3 implements the table formatting model and installs all 341 frozen table-only targets. The engine covers anonymous table fixup, captions and sections, auto and fixed sizing, colspan and rowspan occupancy, baselines and vertical alignment, separated and collapsed borders, layered table backgrounds, positioned descendants, writing modes, intrinsic contributions, repeated sections, and row fragmentation. Shared flex, multicol, overflow, sizing, and display-contents paths now consume table wrappers and internal boxes without target-specific branches.

The remaining paint deltas were resolved generically by quantizing normalized radii and decomposing asymmetric solid rounded borders when their inner radii are non-renderable, including eccentric and saturated-corner tangent clips. Existing symmetric rounded borders remain on the corrected double-round-rect path.

The authoritative no-resume release proof is 4,480/4,480 exact with zero failures and zero errors. The transactional SP19 promoter publishes exactly the complete W3 prefix while preserving non-target CSV records byte-for-byte and rejecting non-exact or out-of-order waves. Verification passes all 174 Python tests, including exactly 24 SP19 tests; the locked style/DOM/text/layout/paint Rust matrix; release `pixel_compare`; repeated SP19 closure checks and regeneration; formatting and diff checks; and the PNG-backed repository audit 7/7.
