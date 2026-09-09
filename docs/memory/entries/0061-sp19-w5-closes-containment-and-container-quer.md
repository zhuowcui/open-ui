---
id: 0061
title: SP19 W5 closes containment and container queries
tags: sp19 containment container-queries content-visibility intrinsic-sizing fragmentation accountability handoff
status: active
created: 2026-09-08
updated: 2026-09-08
refs: bindings/rust/openui-layout/src/containment.rs, bindings/rust/openui-layout/src/block.rs, bindings/rust/openui-paint/src/painter.rs, tools/accountability/data/wpt_ported/sp19_layout_partitions.json
---

SP19 W5 ports all 231 frozen containment-only targets and proves 4,919/4,919
runnable tests exact with zero failures and zero errors. The engine now covers
size, inline-size, layout, style, and paint containment; intrinsic-size
substitution; deterministic `content-visibility:auto`; static size-container
queries; containment-established formatting and containing blocks; paint clips
and overflow-clip margins; passive marker controls; and the required
flex/multicol/fragmentation interactions.

Containment-sensitive fragmentation behavior is gated by actual contained
subtrees so ordinary floats, positioned parallel flows, and flex visual
overflow keep their established behavior. This preserves the complete W4
baseline while allowing monolithic descendants and contained floats to resume
correctly across fragmentainers.

The authoritative no-resume proof is 4,919 exact results with PNG evidence.
Verification passes all 174 Python tests (exactly 24 SP19 tests), the locked
style/DOM/text/layout/paint Rust matrix, the release `pixel_compare` build,
two byte-identical porter/accountability regeneration cycles, both closure
validators, formatting and diff checks, and the repository audit 7/7. The
remaining 2,754 unported rows are explicitly owned; W6 consists of the final
43 table/Grid/containment intersections.
