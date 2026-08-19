---
id: 0021
title: SP14 milestone: Ahem text pixel path proven end-to-end (0.04%)
tags: sp14, text, milestone, state
status: active
created: 2026-07-08
updated: 2026-07-08
refs: commit:50a43ab
---

The SP14 end-to-end text pixel path is PROVEN: port template -> DOM ElementTag::Text -> shape -> inline layout -> Ahem glyph paint -> pixel compare -> audit. sp13/inline_single_span renders real Ahem "Xpqg" with geometry identical to Chromium; mismatch 0.04% (200px), all on vertical glyph edges = sub-pixel AA/text-gamma only. Committed 50a43ab with zero wpt/ regression + audit 7/7.
NEXT focused step: the AA/text-gamma last mile to reach exact 0.0% (Chromium renders glyph edges at a fractional sub-pixel x ~19.87 with gamma-corrected AA; OpenUI positions at integer x -> crisp edges). If unreachable, classify as near_miss_aa per plan policy (0.0% stays the bar, never hidden). THEN: port the single-line Ahem pilot batch via EMIT_TEXT_NODES allowlist (entry 0006), setting font on the Text node.
DECISION 2026-07-08 (user): accept near_miss_aa for the smoke test (geometry exact, 8x under 0.5% threshold); do NOT pursue Chrome-raster replication now. NEXT: port the SP14 single-line Ahem pilot batch via port_wpt.py EMIT_TEXT_NODES (allowlist), ensuring font_family/font_size are set on the emitted ElementTag::Text node.
