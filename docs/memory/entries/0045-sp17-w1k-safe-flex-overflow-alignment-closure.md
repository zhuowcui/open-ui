---
id: 0045
title: SP17 W1K closes safe flex overflow alignment
tags: sp17, writing-mode, flex, abspos, alignment, overflow, accountability, handoff
status: active
created: 2026-08-23
updated: 2026-08-23
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/flex/algorithm.rs, bindings/rust/openui-layout/tests/sp17_flex_abspos_safe_alignment_tests.rs, tools/wpt/sp17_w1k_targets.json, tools/wpt/sp17_w1k_focused_ids.json, tools/wpt/test_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1K closes one atomic seven-ID safe flex overflow-alignment cohort: all three
`flex-abspos-staticpos-align-self-safe-*` test/reference pairs and
`flexbox-safe-overflow-position-005`. The 18-ID focused proof also includes
safe overflow 001–004 and all six horizontal-tb/vertical-lr/vertical-rl ×
LTR/RTL abspos auto-position guards. It finishes 18/18 exact with zero
mismatched pixels or errors.

The private flex abspos static-position helper now preserves signed main/cross
free space and complete alignment values. It resolves main placement through
the shared content-alignment helper and cross placement through the shared
item-alignment helper. Oversized safe alignment falls back to logical start,
default/unsafe center keeps its negative offset, fitting safe end reaches
logical end, `align-self:auto` inherits position and overflow from
`align-items`, and reverse flow, wrap reversal, writing mode, and direction are
projected once.

The child margin box remains authoritative for alignment. W1G's flex padding
box/containing-block geometry and physical out-of-flow boundary remain intact,
so margins are applied exactly once. No public style, fragment, candidate,
constraint-space, or layout API changed, and there is no target-specific
branch.

Parameterized regressions cover row, row-reverse, column, and column-reverse
under horizontal-tb, vertical-lr, and vertical-rl with LTR and RTL. They include
safe and unsafe overflow, fitting end alignment, asymmetric margins/borders/
padding, inherited alignment overflow, wrap-reverse safe flex-start, physical
projection, and single margin application. Both no-write probes and both
transactional splices are byte-identical.

The authoritative complete run is 3,716 runnable, 3,419 exact, 297 failures,
and zero errors. There are 3,957 unported rows, 690 live
`needs_writing_mode` rows, 976 unported `sp13_multicol` rows, and 841
text-manifest IDs. All 3,267 kickoff exact IDs remain exact. The live SP17
validator requires exactly 152 promotions, and SP13-R's later-promotion
allowlist remains 42 IDs without changing any frozen ledger. The committed
`summary.json` SHA-256 is
`a3a9be11bd242c330e9de765bbbb7917df4f95ea714c3672f97f6df5567f10df`.

Keep absolute centering, JavaScript-backed abspos alignment,
`flexbox-safe-overflow-position-006`, existing `flexbox-align-self-vert-*`
failures, transforms, tables, generated content, and unrelated paint work
outside W1K. W2 retains authoritative mixed-script bidi, vertical glyph
orientation, fallback shaping, sideways modes, and writing-mode 010–016.
