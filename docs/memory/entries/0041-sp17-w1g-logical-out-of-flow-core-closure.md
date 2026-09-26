---
id: 0041
title: SP17 W1G closes the logical out-of-flow core
tags: sp17, writing-mode, out-of-flow, abspos, flex, intrinsic-sizing, aspect-ratio, accountability, handoff
status: active
created: 2026-08-22
updated: 2026-08-22
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/out_of_flow.rs, bindings/rust/openui-layout/src/block.rs, bindings/rust/openui-layout/src/flex/algorithm.rs, bindings/rust/openui-layout/tests/sp17_oof_logical_tests.rs, tools/wpt/test_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1G closes one atomic 27-ID cohort spanning vertical-flex abspos static
positioning, orthogonal abspos intrinsic sizing and physical margins, and
vertical abspos aspect-ratio transfer.

A private out-of-flow axis mapping preserves distinct complete writing
directions for the containing block, static-position parent, and abspos child
without changing `OutOfFlowCandidate` or any public API. Authored physical
sizes, insets, and margins remain physical. Intrinsic contributions, child
constraint spaces, static anchors, and final fragments cross explicit
logical/physical boundaries. Physical start polarity accounts for
horizontal-tb direction, vertical-lr versus vertical-rl block flow, and
vertical inline progression plus RTL.

Flex abspos static positioning now uses the existing flex main/cross mapping
and padding-box containing block. It handles reversal, asymmetric borders and
padding, an orthogonal child's hypothetical constraints, and a single final
physical projection. Abspos child available sizes, percentage bases, fixed
flags, clamped relayout, and aspect-ratio inputs are transposed before block
layout.

All 27 builders were dry-run and spliced in one transaction. They finish 27/27
exact with zero mismatched pixels or errors; repeat splicing is byte-idempotent,
and no writing-mode 010–016 builder was generated. Parameterized Rust
regressions cover the six writing-mode/direction polarities, flex static
positions with asymmetric edges, vertical intrinsic sizing, physical auto and
over-constrained margins, aspect-ratio and percentage transfer, constraint
transposition, clamped relayout, and physical fragments. A frozen-baseline
probe also pins Chromium's symmetric handling of negative vertical auto
margins.

The frozen 3,267-ID kickoff baseline remains 3,267/3,267 exact. The
authoritative complete run is 3,657 runnable, 3,358 exact, 299 failures, and
zero errors; 4,016 rows remain unported, 751 retain `needs_writing_mode`, and
the text manifest has 782 IDs. The live validator requires all 91 exact SP17
promotions. `summary.json` SHA-256 is
`3974f4cbda275e0f8a63ea5b2aedf589c611e2dac600f39b6ffa193abb8c056c`.

Continue W1 with fragmentation and multicol logical geometry. Keep
positioned-inline static positions, flex safe-alignment abspos behavior,
fragmented/multicol out-of-flow layout, upright and mixed vertical text,
sideways modes, and writing-mode 010–015 in their later scoped cohorts.
