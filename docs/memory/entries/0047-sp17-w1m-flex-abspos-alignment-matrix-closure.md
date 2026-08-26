---
id: 0047
title: SP17 W1M closes the flex abspos alignment matrix
tags: sp17, writing-mode, flex, abspos, alignment, float, clearance, porter, accountability, handoff
status: active
created: 2026-08-24
updated: 2026-08-24
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/block.rs, bindings/rust/openui-layout/src/inline/algorithm.rs, bindings/rust/openui-layout/src/flex/algorithm.rs, bindings/rust/openui-layout/tests/sp17_float_clearance_wrapper_tests.rs, tools/wpt/sp17_w1m_targets.json, tools/wpt/sp17_w1m_focused_ids.json, tools/wpt/test_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1M closes one mandatory 39-ID cohort: 29 previously unported assertion-only
`check-layout` flex abspos layouts and ten existing runnable
fallback/justify-self/margin test-reference failures. The 58-ID proof adds all
17 W1L guards plus flex fragmentation 010/011. It finishes 58/58 exact with
zero mismatched pixels or errors.

The porter accepts scripting only when it is exactly the ordered
`testharness.js`, `testharnessreport.js`, and `check-layout-th.js` combination
with one body `checkLayout(...)` hook. The harness and body handler are stripped
from the comparison template, including selectors containing quoted `>`.
Inline code, unknown scripts, extra event handlers, dynamic mutation, and
general JavaScript remain rejected.

Flex abspos static positioning resolves distribution fallback, physical and
logical edges, flex reversal, wrap reversal, writing mode, direction, item
alignment, and safe overflow once. Physical `left`/`right` retain their
axis-specific fallback, `justify-self` remains ignored, auto margins contribute
zero to the hypothetical box, and specified margins align the complete margin
box exactly once.

Mixed block/inline layout now carries clearing-break extent through its
anonymous wrapper and parent exactly once. The clearing line fragment remains
zero-height, while flow advances by the greater of the float clearance and
the computed strut; a later clearing break that no longer advances clearance
keeps its normal strut. Parameterized regressions cover left/right/both floats,
margin boxes, short floats, and no-op clears.

The authoritative complete run is 3,746 runnable, 3,460 exact, 286 failures,
and zero errors. There are 3,927 unported rows, 676 live
`needs_writing_mode` rows, 976 unported `sp13_multicol` rows, and 871
text-manifest IDs. All 3,267 kickoff exact IDs remain exact. The live validator
authorizes exactly the W1M manifest's 12 frozen residual-ledger and 17 non-SP17
admissions and requires 166 SP17 promotions. SP13-R's later-promotion allowlist
remains 42 IDs. The committed `summary.json` SHA-256 is
`94574e79d0c0f5bbf979e57c6168e62aab35e56fed62f5977c2f3ed4784817df`.

Keep dynamic JavaScript and mutation tests, tables, grid, transforms,
generated content, safe-overflow 006, and W2 glyph work outside W1M.
