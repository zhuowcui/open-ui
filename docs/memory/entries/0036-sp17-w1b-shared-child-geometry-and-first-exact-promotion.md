---
id: 0036
title: SP17 W1B shared child geometry admits the first exact actionable target
tags: sp17, writing-mode, flex, rtl, geometry, accountability, handoff
status: active
created: 2026-08-19
updated: 2026-08-19
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/logical_geometry.rs, bindings/rust/openui-layout/src/flex/algorithm.rs, bindings/rust/openui-layout/src/inline/algorithm.rs, tools/wpt/generate_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1B centralizes block- and flex-child `ConstraintSpace` creation around the
child's computed writing direction. It transposes available and percentage
size pairs together at an orthogonal boundary. Normal block children, floats,
atomic inline and block-in-inline children, and final flex-item layout use this
path. Flex resolves axes, sizes, gaps, margins, wrapping, alignment, item
placement, and final physical fragments in the container's writing direction.
`WritingModeConverter` owns direction/block flips; authored flex reverse owns
only logical item reversal. Overflowing right/RTL-start inline alignment now
preserves the aligned edge.

`wpt/css_flexbox/flexbox-writing-mode-001` is the first live SP17 promotion. It
is authored horizontal-tb, so it proves porter/cascade admission and the shared
flex path but not vertical text. Its result is 800x600, zero mismatched pixels,
and `0.0%`. Four existing RTL gap builders (`gap-001-rtl-ref`,
`gap-003-rtl-ref`, `gap-006-rtl`, `gap-006-rtl-ref`) were regenerated because
their historical output lowered logical margins incorrectly or omitted
retained text. The full `gap-00` slice is 32/32 exact, and repeating the
five-builder splice produces no byte changes.

The authoritative full no-resume run is 3,567 runnable, 3,268 exact, 299
functional failures, and zero errors. All 3,267 kickoff exact IDs remain exact;
unported inventory is 4,106 and live `needs_writing_mode` ownership is 841.
The immutable 842-row kickoff inventory and 311 actionable / 531 residual
ledgers did not change. Historical validators now treat original runnable
counts as floors and accept a later promotion only with matching mapping,
template, summary identity, and exact `0.0%` proof. SP13-R's 1,018 multicol
residuals remain strictly frozen and unported.

Verification is complete for this checkpoint: 122 Python closure/porter tests,
full locked Rust style/text/layout/paint tests, release `pixel-compare`, both
ledger checks, two byte-identical mapping/deferred/report regeneration passes,
and the unflagged PNG-backed 7/7 audit. Live `summary.json` SHA-256 is
`6f99e956b4a3eed7a2c4427c429ccff54baaeb7fa5af3f161c81354ae80c5df4`.

Do not call W1 complete. Normal block still contains physical-axis decisions;
flex intrinsic/content and aspect-ratio branches, out-of-flow/static positions,
fragmentation, multicol, and vertical/sideways shaping and paint remain. Probe
horizontal-RTL `flexbox-writing-mode-004` next without resume. Then use
002/003/005/006 and 007/008 to drive genuine vertical and mixed orthogonal
geometry. Every promotion must be surgical, exact at `0.0%`, zero-error, and
must preserve the frozen baseline.
