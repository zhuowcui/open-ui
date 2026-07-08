---
id: 0005
title: SP14 goal + scope: single-line LTR Ahem text to 0.0% parity
tags: sp14, text, plan
status: active
created: 2026-07-08
updated: 2026-07-08
refs: docs/SP14-PLAN.md
---

SP14 = Text Rendering Foundation. Prove the end-to-end path (port -> DOM text -> shape -> inline layout -> glyph paint -> pixel compare -> audit) on the DETERMINISTIC subset: single-line, LTR, horizontal text. Exit: port tool emits text nodes (documented); pilot single-line tests pass at 0.0%; ZERO regressions to existing box-only passes; audit 7/7; newly-passing tests reclassified out of needs_text; independent verifier concurs. Out of scope (later SPs): multi-line/wrapping (SP15), real-font metrics (SP16), bidi/vertical/transform (SP17), first-line/-letter/shadow/generated content (SP18).
