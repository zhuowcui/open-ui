---
id: 0040
title: SP17 W1F closes vertical flex, logical gaps, and atomic inline layout
tags: sp17, writing-mode, flex, gap, atomic-inline, vertical-text, intrinsic-sizing, accountability, handoff
status: active
created: 2026-08-22
updated: 2026-08-22
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/block.rs, bindings/rust/openui-layout/src/flex/algorithm.rs, bindings/rust/openui-layout/src/inline/algorithm.rs, bindings/rust/openui-layout/src/intrinsic_sizing.rs, bindings/rust/openui-paint/src/painter.rs, tools/wpt/test_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1F closes one atomic 44-test cohort spanning vertical flex direction and
wrapping, logical gaps, inline-flex intrinsic sizing, vertical atomic-inline
geometry, homogeneous rotated Ahem text, and the column-wrap intrinsic crash.

Private axis mappings now keep normal block, atomic inline, flex main/cross,
and intrinsic contributions logical until a defined physical fragment
boundary. Automatic flex minima use the child's logical main axis, while
explicit physical min/max properties resolve at the physical-property
boundary. Percentage edge resolution retains the containing block's inline
measure across orthogonal child layout.

Vertical atomic inline fragments are measured and positioned in the parent's
logical axes before one writing-mode projection. Homogeneous Latin/Ahem runs
under vertical mixed orientation shape horizontally and rotate their complete
paint stack clockwise, including shadows, decorations, glyphs, emphasis,
clipping, and culling. Upright CJK, mixed-script splitting, sideways modes, and
writing-mode 010–015 remain W2 work.

All 44 builders were spliced in one transaction and finish 44/44 exact with
zero mismatched pixels or errors. Shared references remain byte-identical,
repeat splicing is byte-idempotent, and no writing-mode 010–016 builder was
generated. The live validator requires all 64 exact SP17 promotions.

The frozen 3,267-ID baseline remains 3,267/3,267 exact. The authoritative full
run is 3,630 runnable, 3,331 exact, 299 failures, and zero errors; 4,043 rows
remain unported, 778 retain `needs_writing_mode`, and the text manifest has 755
IDs. `summary.json` SHA-256 is
`76b70d2d6b32e5899de6b03a76df28c1111643f0307475e386cbbb822bc00a55`.

Next carry logical geometry through out-of-flow/static positions,
fragmentation, and multicol. Keep upright/mixed-script and sideways text in W2.
