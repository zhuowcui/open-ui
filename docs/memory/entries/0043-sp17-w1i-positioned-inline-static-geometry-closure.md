---
id: 0043
title: SP17 W1I closes positioned-inline static geometry
tags: sp17, writing-mode, positioned-inline, static-position, multicol, out-of-flow, accountability, handoff
status: active
created: 2026-08-23
updated: 2026-08-23
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/block.rs, bindings/rust/openui-layout/src/inline/algorithm.rs, bindings/rust/openui-layout/src/out_of_flow.rs, bindings/rust/openui-layout/tests/sp17_positioned_inline_geometry_tests.rs, tools/wpt/test_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1I closes one atomic 30-ID positioned-inline cohort spanning direct block
flow and the two selected multicol shapes in vertical-lr and vertical-rl. The
same shared fix repairs the two existing horizontal-tb RTL family failures.

Both inline layout entry points now use one private logical candidate path.
First and last inline-fragment containing-block endpoints derive from the
inline ancestor's direction and logical line positions, ignore synthetic empty
continuations when real content exists, and account for text indent, relative
inline translation, asymmetric edges, atomic-inline bubbling, and
block-in-inline interruption exactly once. Bidi splitting remaps out-of-flow
placeholders so candidates retain the relevant continuations.

The physical out-of-flow boundary projects a candidate's logical static
anchor, containing-block offset, and containing-block size together. It
preserves the inline containing-block node, direction, and zero-border contract
instead of substituting the enclosing block padding box. Vertical multicol
maps the first/last endpoints through the same column index/remainder and
vertical-lr/vertical-rl projection used by W1H; positioned results remain
outside the final in-flow projection and have single ownership.

The 30 builders dry-run deterministically and two surgical splices produce
byte-identical affected artifacts. The required 35-ID family proof is 35/35
exact with zero mismatched pixels or errors. Three frozen regression guards
remain exact, including relative positioning and both horizontal multicol
out-of-flow cases used while separating continuation affinity.

The frozen 3,267-ID kickoff baseline remains 3,267/3,267 exact. The
authoritative complete run is 3,703 runnable, 3,406 exact, 297 failures, and
zero errors; 3,970 rows remain unported, 703 retain `needs_writing_mode`, and
the text manifest has 828 IDs. The committed `summary.json` SHA-256 is
`13a7c9181f86b213a81a183eb164d0d05d1bd03911850bdc22d57e42c65cf416`.
The live validator requires exactly 139 SP17 promotions. SP13-R's explicit
later-promotion allowlist is 36 IDs after adding the 20 multicol-position
admissions; no frozen ledger changed.

Keep true out-of-flow multicol fragmentation, fragmented abspos boxes, flex
safe alignment, absolute centering, tables, transforms, generated content,
images/print, and tolerance or reference changes outside W1I. W2 retains
authoritative mixed-script bidi, vertical glyph orientation, fallback shaping,
sideways modes, and writing-mode 010–016.
