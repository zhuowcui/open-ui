---
id: 0030
title: SP14 W3/W4 closes the global needs_text backlog
tags: sp14, text, accountability, milestone, handoff
status: active
created: 2026-08-08
updated: 2026-08-08
refs: tools/accountability/data/wpt_ported/sp14_w3_baseline_exact.json, tools/accountability/data/wpt_ported/sp14_w3_targets.json, tools/accountability/data/wpt_ported/sp14_w4_residuals.json, tools/accountability/data/pixel_comparison/results/summary.json
---

Verified authoritative state: 3,517 runnable tests, 2,767 exact, 750 named-owner
failures, and zero errors. All 2,715 frozen baseline exact IDs remain exact. W3 adds
111 deterministic text ports (52 exact, 59 functional residuals); W4 records actual
porter rejection reasons and merged non-text owners for the other 3,934 rows. The W3
and W4 ledgers are sorted, unique, disjoint, and cover all 4,045 original unported
`needs_text` rows.

The global `text_rendering`/`needs_text` category is retired. The 445-ID text manifest
contains 93 exact and 352 named-owner failures. Mapping accounts for 7,673 rows as
3,517 runnable plus 4,156 unported, with no generic, blank, metadata-only W4, or
`sp12_layout_bug` ownership. Mapping, deferred data, SP12.5 documentation, and the HTML
report regenerate deterministically; audit passes 7/7.

Next handoff: SP15 owns inline/layout and root/body viewport-propagation gaps exposed by
deterministic text. SP16 owns real-font metrics, SP17 advanced text/writing modes, and
SP18 generated content/text effects. Existing JavaScript, image, grid, table,
containment, form-control, and paint owners remain explicit in W4.
