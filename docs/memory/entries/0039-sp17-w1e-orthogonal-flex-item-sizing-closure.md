---
id: 0039
title: SP17 W1E closes orthogonal flex-item sizing
tags: sp17, writing-mode, flex, orthogonal, intrinsic-sizing, geometry, accountability, handoff
status: active
created: 2026-08-20
updated: 2026-08-20
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/flex/algorithm.rs, bindings/rust/openui-layout/tests/flex_direction_wrap.rs, tools/wpt/test_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1E closes a 14-test orthogonal flex cohort spanning writing-mode 007–009,
intrinsic and fit-content sizing, center/stretch alignment, percentage padding,
aspect-ratio transfer, wrapped flexing, and overflow padding. A private
`FlexItemAxisMapping` now translates flex container main/cross sizes into each
child's logical inline/block coordinates exactly once.

Available sizes, percentage bases, fixed/stretch flags, intrinsic measurement
spaces, aspect-ratio transfer, and final physical projection all use the same
mapping. Horizontal and same-mode output stays unchanged, and horizontal child
fragments retain layout-owned fragmentation reductions.

The 14 targets were spliced in one transaction and finish 14/14 exact with zero
mismatched pixels or errors. The existing writing-mode 007–009 reference
builders remain byte-identical, the repeat splice is byte-idempotent, and no
010–016 builder was generated. The live validator now requires all 20 exact
SP17 promotions while preserving every frozen kickoff and historical ledger.

The frozen 3,267-ID baseline remains 3,267/3,267 exact. The authoritative full
run is 3,586 runnable, 3,287 exact, 299 failures, and zero errors; 4,087 rows
remain unported, 822 retain `needs_writing_mode`, and the text manifest has 711
IDs. `summary.json` SHA-256 is
`bb87ab04fdc9fad9b4c6cd935ed4220ffd2fc413bec62ff123a9eede5ab6429d`.

Next close the remaining vertical flex families in W1, followed by
out-of-flow/static positions, fragmentation, and multicol. Keep writing-mode
010–015 for W2 vertical-text shaping and paint.
