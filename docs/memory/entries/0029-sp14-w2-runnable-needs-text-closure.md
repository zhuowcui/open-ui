---
id: 0029
title: SP14 W2 closes all runnable needs_text ownership
tags: sp14, text, accountability, milestone, handoff
status: active
created: 2026-08-07
updated: 2026-08-07
refs: commit:f13f617, tools/accountability/data/wpt_ported/sp14_w2_targets.json, tools/accountability/data/wpt_ported/text_ported_tests.json, tools/accountability/data/pixel_comparison/results/summary.json
---

Verified full `wpt/` state: 3406 total, 2715 exact, 691 fail, 0 errors. All 2700 W1 baseline exact IDs remain exact. The 286 immutable W2 targets split into 15 exact and 271 detector-backed alternate owners; the complete 334-ID text manifest is 41 exact and 293 alternate owners. No runnable failure retains `needs_text`, no residual has metadata-only ownership, and audit passes 7/7 with zero generic, blank, or `sp12_layout_bug` rows.

W2 added deterministic Latin-1/ellipsis/arrows/bidi-control handling, Ahem plus explicit DejaVu Sans fallback under manifest-scoped no-AA/no-hinting, exact-ID focused runs, an author-proof runner reset style, precise detectors for clearing breaks and display-contents gaps, and transactional multi-module splice coverage. Deferred classification now uses original Chromium HTML for normalized text ports, matching mapping ownership without relying on altered comparison templates.

Acceptance: release `pixel-compare`; SP14 Python 33/33; line width 8/8; breaker rewind and leading-space 2/2; float inline 10/10; forced break 1/1; full comparison without resume; deterministic mapping/deferred regeneration; audit 7/7.

Next handoff: process the 4045 unported `needs_text` rows in dependency-aware W3/W4 batches. Keep `text_ported_tests.json` as the renderer scope and exact zero-pixel parity as the promotion bar. Do not retire the global text detector until the unported inventory is closed; real-font metrics remain SP16.
