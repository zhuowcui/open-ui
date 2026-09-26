---
id: 0049
title: SP17 W1O closes auto-height flex basis and semantic breaks
tags: sp17, writing-mode, flex, flex-basis, semantic-break, porter, accountability, handoff
status: active
created: 2026-08-26
updated: 2026-08-26
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/flex/algorithm.rs, bindings/rust/openui-layout/src/inline/algorithm.rs, bindings/rust/openui-layout/tests/sp17_auto_height_flex_basis_tests.rs, tools/wpt/port_wpt.py, tools/wpt/sp17_w1o_targets.json, tools/wpt/sp17_w1o_focused_ids.json, tools/accountability/data/pixel_comparison/results/summary.json
---

W1O closes the sole remaining SP17-owned runnable failure,
`wpt/css_flexbox/auto-height-with-flex`. The one-ID target and 15-ID focused
proof are exact with zero mismatched pixels or errors.

The porter now lowers omitted one- and two-value flex shorthand bases, and an
accepted unitless third zero, to `Length::percent(0.0)`. Explicit `0px`, `0%`,
`auto`, and other bases remain distinct. Percentage bases resolve only against
definite main space; indefinite percentages use a content basis that replaces
the main-size property, while fixed zero stays definite.

Retained `<br>` elements are semantic `ElementTag::Break` controls rather than
synthetic pre-line text. Their inherited font, line-height, writing-mode,
direction, and orientation establish the forced-line strut. Semantic-break
intrinsic sizing preserves each real forced line without producing a phantom
trailing line through a min-inline relayout. Display suppression and clearing
break behavior remain intact.

Only the target Rust builder was regenerated. Two no-write generations and two
surgical splices are byte-identical; the 871-ID text manifest and 42-ID SP13-R
compatibility allowlist are unchanged. The frozen 3,267-ID kickoff baseline and
every W1N invariant remain exact.

The authoritative complete run is 3,746 runnable, 3,465 exact, 281 failures,
and zero errors. There are 3,927 unported rows, 671 live
`needs_writing_mode` rows, and 976 unported `sp13_multicol` rows. The validator
requires exactly 171 promotions. The committed `summary.json` SHA-256 is
`d182ff44328df0a6c711990830a62a3e9e6199d2c13bad583360efd68702b2a9`.

Keep dynamic JavaScript, tables, grid, transforms, generated content, images,
and unrelated frozen runnable failures outside W1O. W2 retains authoritative
bidi, upright/sideways glyph work, fallback shaping, and writing-mode 010–016.
