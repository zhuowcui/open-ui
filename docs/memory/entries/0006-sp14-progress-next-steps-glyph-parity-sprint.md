---
id: 0006
title: SP14 progress + next steps (glyph parity sprint)
tags: sp14, text, progress, next-steps
status: active
created: 2026-07-08
updated: 2026-07-08
refs: commit:343109c, commit:6d2d6de
---

DONE: SP14 docs committed (roadmap + SP14-PLAN + status refresh). Gated text emission built in tools/wpt/port_wpt.py behind EMIT_TEXT_NODES (default OFF; commit 343109c) — off=byte-identical output, on=emits escaped ElementTag::Text. Smoke test: text shaped + bbox-aligned but mismatch 0.356% (~900 large-delta edge px = sub-pixel advance/positioning + hinting/edging).
NEXT: (1) port single-line pilot with EMIT_TEXT_NODES on via allowlist; wire builders+templates. (2) Glyph parity core: tune SkFont edging/hinting/subpixel + advance rounding (openui-paint/src/text_painter.rs + shaping) to headless Chromium -> 0.0%. (3) Guard slices (zero regression to 2671) -> full wpt -> regen artifacts -> audit 7/7 -> reclassify out of needs_text -> verifier -> commit.
