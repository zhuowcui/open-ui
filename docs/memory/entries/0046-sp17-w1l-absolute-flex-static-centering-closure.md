---
id: 0046
title: SP17 W1L closes absolute flex static-position centering
tags: sp17, writing-mode, flex, abspos, static-position, centering, fragmentation, accountability, handoff
status: active
created: 2026-08-24
updated: 2026-08-24
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/out_of_flow.rs, bindings/rust/openui-layout/src/flex/algorithm.rs, bindings/rust/openui-layout/tests/sp17_flex_abspos_static_edge_tests.rs, tools/wpt/sp17_w1l_targets.json, tools/wpt/sp17_w1l_focused_ids.json, tools/wpt/test_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1L closes the atomic absolute-center pair. The shared fix promotes runnable
`position-absolute-center-001`, and the surgical one-builder splice admits its
vertical-rl transpose 002. The 17-ID focused proof also retains center 003–004,
all seven W1K targets, and six writing-mode/direction abspos auto-position
guards. It finishes 17/17 exact with zero mismatched pixels or errors.

Out-of-flow candidates now retain start/center/end affinity for both physical
static-position axes. Flex emits an alignment anchor plus edge bias; the
generic positioned solver derives a forward, backward, or symmetric interval,
then aligns the complete margin box for shrink-to-fit and known-size results.
Vertical auto physical height is repositioned after content layout supplies
its final size. Style and layout function signatures remain unchanged.

Fragmentation exposed the important reconstruction rule: an unfragmented
center/end block-flow anchor cannot be mapped to a column before its
hypothetical margin-box start is materialized. Doing so selected an overflow
column at exact boundaries and regressed flex-container-fragmentation 010/011.
The generic reconstruction step now consumes the retained edge before column
projection, and positioned-fragment metadata preserves both edge fields across
logical/physical normalization.

The authoritative complete run is 3,717 runnable, 3,421 exact, 296 failures,
and zero errors. There are 3,956 unported rows, 688 live
`needs_writing_mode` rows, 976 unported `sp13_multicol` rows, and 842
text-manifest IDs. All 3,267 kickoff exact IDs remain exact. The live SP17
validator requires exactly 154 promotions, and SP13-R's later-promotion
allowlist remains 42 IDs without changing any frozen ledger. The committed
`summary.json` SHA-256 is
`365354dae47ca97f6370a2dcd4edc3e69286de0866ee7e9169e8f7ee3e4f1cd5`.

Keep fallback/justify-self/JavaScript-backed abspos alignment,
`flexbox-safe-overflow-position-006`, tables, transforms, generated content,
and unrelated paint work outside W1L. W2 retains authoritative mixed-script
bidi, vertical glyph orientation, fallback shaping, sideways modes, and
writing-mode 010–016.
