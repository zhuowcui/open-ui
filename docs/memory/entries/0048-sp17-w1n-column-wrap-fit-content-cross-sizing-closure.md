---
id: 0048
title: SP17 W1N closes column-wrap fit-content cross sizing
tags: sp17, writing-mode, flex, wrap, intrinsic-sizing, fit-content, accountability, handoff
status: active
created: 2026-08-26
updated: 2026-08-26
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/flex/algorithm.rs, bindings/rust/openui-layout/tests/sp17_column_wrap_fit_content_tests.rs, tools/wpt/sp17_w1n_targets.json, tools/wpt/sp17_w1n_focused_ids.json, tools/wpt/test_sp17_closure.py, tools/accountability/data/pixel_comparison/results/summary.json
---

W1N closes four existing column-wrap failures: `align-content-wrap-004` and
`flex-wrap-002` through 004. Its 19-ID proof also guards `flex-wrap-005`, the
writing-mode 002/003/005/006 test-reference pairs, gap-003 LTR/RTL pairs, and
flex fragmentation 010/011; all 19 are exact with zero mismatched pixels or
errors.

When a flex item's cross axis maps to its logical inline axis, auto cross size
now uses fit-content against available container cross space after specified
margins. Indefinite space selects max-content and auto margins contribute zero
during hypothetical sizing. Border/padding and cross min/max constraints apply
once, and final layout reuses the border-box size that established the line.
The row-flex logical-block intrinsic path is unchanged, with no public API or
target-specific branch.

Parameterized regressions cover intrinsic clamp bounds, definite/indefinite
space, one and two wrapped lines, centered align-content, specified and auto
margins, single constraint application, reversal, wrapping, direction, and
horizontal-tb/vertical-lr/vertical-rl projection.

The authoritative complete run is 3,746 runnable, 3,464 exact, 282 failures,
and zero errors. There are 3,927 unported rows, 672 live
`needs_writing_mode` rows, 976 unported `sp13_multicol` rows, and 871
text-manifest IDs. All 3,267 kickoff exact IDs remain exact. The validator
requires exactly 170 promotions; SP13-R's compatibility allowlist remains 42.
The committed `summary.json` SHA-256 is
`3fa96c5a653460785bbec889ece0aa6bbadcdf410fb7edacd8d2c9c194105263`.

Keep dynamic JavaScript, images, generated content, dotted-border parity,
sticky positioning, trailing-break height, and unrelated frozen runnable
failures outside W1N. W2 retains writing-mode 010–016 and glyph work.
