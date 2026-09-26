---
id: 0037
title: SP17 W1C closes horizontal RTL flex flow with shared direction propagation
tags: sp17, writing-mode, flex, rtl, geometry, accountability, handoff
status: active
created: 2026-08-19
updated: 2026-08-19
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/flex/algorithm.rs, bindings/rust/openui-layout/tests/flex_direction_wrap.rs, tools/wpt/test_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1C closes only `wpt/css_flexbox/flexbox-writing-mode-004`, the
horizontal-tb/RTL companion to exact target 001. A parameterized layout
regression covers row, row-reverse, column, column-reverse, wrap, and
wrap-reverse. It asserts offsets by original cyan, magenta, yellow, and black
item identity so an ordering error cannot be hidden by mirroring.

The regression exposed a shared propagation defect. Flex axis construction
used the resolved container `WritingDirectionMode`, but final item placement
converted logical offsets using the parent `ConstraintSpace` direction. The
final-placement boundary now receives the resolved container direction. This
is a general logical-to-physical conversion fix; W1C adds no per-test offset,
builder substitution, raster override, or vertical-writing behavior.

Target 004 was spliced transactionally without regenerating its already-exact
reference or any 002–008 builder. Its focused no-resume result is 1/1 exact,
zero mismatched pixels, and zero errors. The splice is byte-idempotent. The
SP17 live validator now requires both 001 and 004 in the exact actionable
promotion set while every kickoff and historical ledger remains byte-pinned.

The frozen 3,267-ID baseline rerun is 3,267/3,267 exact. The authoritative full
no-resume suite is 3,568 runnable, 3,269 exact, 299 functional failures, and
zero errors. The inventory is 4,105 unported and 840 live
`needs_writing_mode` rows. `summary.json` SHA-256 is
`50c8a52b36c479357f9e89b5e8ea520608a69974d04616d36a7099639d25f0d1`.

Verification includes 178 focused flex/logical-writing tests, the full locked
style/text/layout/paint matrix, release `pixel-compare`, all 122 SP13-R through
SP17 Python tests, both ledger checks, two byte-identical mapping/deferred/HTML
generations, surgical-splice idempotence, and the unflagged PNG-backed audit
7/7. Next drive genuine vertical and mixed orthogonal geometry with 002, 003,
005, and 006, then 007 and 008. Flex intrinsic/aspect-ratio work,
out-of-flow/static positions, fragmentation, multicol, and vertical glyph work
remain outside this checkpoint.
