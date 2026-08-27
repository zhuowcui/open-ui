---
id: 0050
title: SP17 W2A closes rotated vertical and sideways text
tags: sp17, writing-mode, vertical-text, sideways, orientation, paint, flex, accountability, handoff
status: active
created: 2026-08-27
updated: 2026-08-27
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/fragment.rs, bindings/rust/openui-layout/src/inline/algorithm.rs, bindings/rust/openui-layout/tests/sp17_rotated_vertical_sideways_text_tests.rs, bindings/rust/openui-paint/src/painter.rs, tools/wpt/sp17_w2a_targets.json, tools/wpt/sp17_w2a_focused_ids.json, tools/accountability/data/pixel_comparison/results/summary.json
---

W2A closes the 22 formerly unported writing-mode 010–016 test/reference pairs
and sideways-lr/sideways-rl base, RTL, and row-mix builders. The target cohort
and reconstructed 49-ID focused proof are exact with zero mismatched pixels or
errors.

Inline layout now resolves and stores a public `TextRunOrientation` on each text
fragment. Paint consumes that metadata without re-reading CSS or Unicode text.
Clockwise and counterclockwise runs transform shadows, decorations, glyphs, and
emphasis in one balanced canvas stack; layout retains shaped advances and
exported baselines. `UnresolvedMixed` remains a compatibility sentinel until
W2B adds homogeneous mixed-script splitting.

Only the 22 target builders were surgically regenerated. The target manifest is
pinned at `4162bd75b614a81ab43c897aef201456660b35f6cdac8109c41b1aa126ec963f`.
The 49-ID proof includes the targets, writing-mode 001–009 pairs, `slr-ref`, six
row-flow guards, and the vertical-row pair; its replacement pin is
`b44b3ec2d5c1aea2e6e159f923858d66cdb24ff9f4a958a75ec8092771012511`.

The authoritative complete run is 3,768 runnable, 3,487 exact, 281 failures,
and zero errors. There are 3,905 unported rows, 649 live
`needs_writing_mode` rows, 976 unported `sp13_multicol` rows, and 893 text
manifest IDs. The validator requires 193 exact promotions and preserves the
42-ID SP13-R later-promotion allowlist. The committed `summary.json` SHA-256 is
`6b8c2d53561027fd561171b5642ef65fc281493d2729559b320871f21e48cbff`.
