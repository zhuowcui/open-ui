---
id: 0038
title: SP17 W1D closes the vertical-container flex-flow matrix
tags: sp17, writing-mode, flex, vertical, geometry, accountability, handoff
status: active
created: 2026-08-20
updated: 2026-08-20
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/flex/algorithm.rs, bindings/rust/openui-layout/tests/flex_direction_wrap.rs, tools/wpt/test_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1D closes `flexbox-writing-mode-002`, 003, 005, and 006: vertical-rl/LTR,
vertical-lr/LTR, vertical-rl/RTL, and vertical-lr/RTL. One parameterized Rust
regression covers all eight direction/wrap combinations by original CMYK item
identity and verifies each item's physical size.

Vertical flex resolved logical main/cross sizes correctly, but provisional
child fragments retained the transposed fixed child-space pair. The shared fix
projects resolved main/cross border-box sizes into physical fragment
width/height for non-horizontal containers. Horizontal fragments remain
untouched because child layout may have reduced their block size during
fragmentation.

The four targets were spliced in one transaction. Their focused no-resume run
is 4/4 exact with zero mismatched pixels and errors; all existing reference
builders are byte-identical, the splice is byte-idempotent, and no 007/008
builder exists. The live validator requires exact actionable promotions
001–006 while all 3,267 kickoff exact IDs remain exact.

The authoritative full run is 3,572 runnable, 3,273 exact, 299 failures, and
zero errors; 4,101 rows remain unported, 836 retain `needs_writing_mode`, and
the text manifest has 697 IDs. `summary.json` SHA-256 is
`2c041fc33000a20ea899bf2be6bf53354dd10530a072acaa35fbf29d9aecf88d`.

Next drive orthogonal-child sizing with 007, then 008. Flex intrinsic and
aspect-ratio auditing, out-of-flow layout, fragmentation, multicol, and
vertical glyph shaping/paint remain outside W1D.
