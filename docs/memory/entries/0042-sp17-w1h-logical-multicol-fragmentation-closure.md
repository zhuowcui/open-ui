---
id: 0042
title: SP17 W1H closes logical multicol and vertical fragmentation
tags: sp17, writing-mode, multicol, fragmentation, flex, overflow, paint, accountability, handoff
status: active
created: 2026-08-22
updated: 2026-08-22
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/block.rs, bindings/rust/openui-layout/src/fragment.rs, bindings/rust/openui-paint/src/painter.rs, bindings/rust/openui-layout/tests/sp17_multicol_vertical_tests.rs, tools/wpt/test_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1H closes one atomic 16-ID cohort spanning vertical multicol sizing and
projection, fragmented decoration and clipping, vertical float overflow, and
three vertical flex continuation shapes.

Multicol now keeps column resolution, balancing, spanner placement, break
progress, and continuation geometry in logical inline/block coordinates.
Writing-aware child and relayout spaces transpose available sizes, percentage
bases, intrinsic contributions, and fixed/stretch state. A shared finalizer
projects columns, rules, in-flow fragments, baselines, decoration metadata, and
overflow once while leaving positioned fragments on their existing physical
contract.

Fragments carry optional fragmentation writing-direction metadata. Paint uses
it to select horizontal Y, vertical-lr X-from-left, or vertical-rl X-from-right
for column/overflow clips, source slices, border polarity, backgrounds,
shadows, radii, and ink overflow. RTL independently reverses vertical inline
progression. Logical subtree normalization also gives row/column flex
continuations and visible in-flow overflow one authoritative block extent.

All 16 builders were dry-run and spliced transactionally. The 18-ID proof,
including the existing exact border references, is 18/18 exact with zero
mismatched pixels or errors. Repeat splicing is byte-identical and no
writing-mode 010–016 builder was generated. Parameterized Rust regressions
cover all six writing-mode/direction projections, asymmetric physical edges,
visible logical overflow balancing, orthogonal spanner intrinsic sizing, clip
axes, decoration source polarity, and first/interior/last physical borders.

The frozen 3,267-ID kickoff baseline remains 3,267/3,267 exact. The
authoritative complete run is 3,673 runnable, 3,374 exact, 299 failures, and
zero errors; 4,000 rows remain unported, 735 retain `needs_writing_mode`, and
the text manifest has 798 IDs. The committed `summary.json` SHA-256 is
`ed78d9c2fd09c64a52c5de47ece6e7077a8fbab34e3eff6a12d6814ede1c6474`.
The live validator requires exactly 107 SP17 promotions without modifying
kickoff or historical ledgers.

Keep positioned-inline containing blocks, fragmented/multicol out-of-flow
layout, neighboring flex-abspos cases, tables, images and print-only cases,
sideways modes, material vertical text/bidi, extreme column-rule geometry, and
writing-mode 010–016 in later scoped cohorts.
