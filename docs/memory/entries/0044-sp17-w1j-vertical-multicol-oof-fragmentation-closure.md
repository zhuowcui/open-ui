---
id: 0044
title: SP17 W1J closes vertical multicol out-of-flow fragmentation
tags: sp17, writing-mode, multicol, out-of-flow, fragmentation, positioned, accountability, handoff
status: active
created: 2026-08-23
updated: 2026-08-23
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/block.rs, bindings/rust/openui-layout/tests/sp17_multicol_positioned_fragmentation_tests.rs, tools/wpt/sp17_w1j_targets.json, tools/wpt/sp17_w1j_focused_ids.json, tools/wpt/test_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1J closes one atomic six-ID vertical multicol out-of-flow cohort: 063, 064,
066, 067, 118, and 119. The 17-ID focused proof includes existing exact guards
001, 050, 057, 062, 117, and 121–126 and finishes 17/17 exact with zero
mismatched pixels or errors.

The multicol continuation mapper now has one private logical record containing
the positioned box's source block interval, static anchor, containing-block
offset and size, visual translation, resolved logical insets and margins, and
writing direction. It intersects the source interval with column-flow
intervals, assigns source offsets, first/last flags, fragmentainer ownership,
block-axis clips, and decoration slices, then projects each continuation once
through W1H's vertical-lr/vertical-rl mapping. Descendants stay source-local
and each positioned source has one owner.

Block-in-inline relative translation now crosses from physical to logical
vector space once before normal-flow continuation slicing. Completed
positioned boxes remain physical at their layout boundary, while retained
visual metadata is converted only when logical source slicing consumes it.
This keeps relative inline containing blocks, RTL boundary affinity,
fragmented positioned blocks, and fragmented flex containing blocks on the
same path, including percentage inline sizes and asymmetric logical borders.

The six builders produce byte-identical no-write probes and repeated surgical
splices. No reference substitution, tolerance change, public geometry API, or
ID-specific branch was added. Parameterized regressions cover vertical-lr and
vertical-rl under LTR and RTL for fixed and stretched boxes, block and flex
containing blocks, continuation projection, source slices, ordering, clips,
and single ownership.

The authoritative complete run is 3,709 runnable, 3,412 exact, 297 failures,
and zero errors. There are 3,964 unported rows, 697 live
`needs_writing_mode` rows, 976 unported `sp13_multicol` rows, and 834
text-manifest IDs. All 3,267 kickoff exact IDs remain exact. The live SP17
validator requires exactly 145 promotions, and SP13-R's later-promotion
allowlist contains 42 IDs without changing any frozen ledger. The committed
`summary.json` SHA-256 is
`be9c87dcb549fd3566b288749cd278e8430c6a2ad2cf8ec560d6996b2de996b1`.

Keep flex safe alignment, absolute centering, transforms, filters,
containment, JavaScript, generated content, images/print cases, and every
other out-of-flow multicol test outside W1J. W2 retains authoritative
mixed-script bidi, vertical glyph orientation, fallback shaping, sideways
modes, and writing-mode 010–016.
