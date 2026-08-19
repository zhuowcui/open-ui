---
id: 0028
title: SP14 W0/W1 closes 48 sole-needs_text tests with 26 exact and 22 named residuals
tags: sp14, text, accountability, milestone, handoff
status: superseded
created: 2026-08-07
updated: 2026-08-07
refs: commit:eb92a21, commit:7e80669, tools/accountability/data/wpt_ported/text_ported_tests.json, tools/accountability/data/pixel_comparison/results/summary.json
---

Verified full `wpt/` state: 3406 total, 2700 exact pass, 706 fail, 0 errors. All 2671 prior exact-pass IDs remain exact; 29 tests became exact (the 26 SP14 targets plus 3 neighboring float/fragmentation tests). Audit passes 7/7 with 0 generic `not_ported`, 0 blank unported owners, and 0 `sp12_layout_bug`.

The 48 runnable tests formerly owned solely by `needs_text` are authoritative in `text_ported_tests.json`: 26 exact at 0 mismatched pixels and 22 detector-backed residuals (12 rounded-border paint, 4 float descendants of inline, 3 abspos flex static position, 2 positioned-inline layout, 1 float/BFC phantom-margin separation). None retain `needs_text`, including `sp12_5_deferred.csv`.

Infrastructure: deterministic retained text/Ahem on both renderers, scoped no-AA environments, transactional surgical splice with dry-run/idempotency checks, display:contents flattening, flex whitespace suppression, HTML body block semantics, heading UA font-size geometry, entities/whitespace/`br`/inherited text styles, and computed inherited radii. Inline layout retained breaker checkpoint/restore, successive float-bottom queries, leading-space collapse, forced-break struts, and shift-below-float behavior with focused tests.

Acceptance commands: `cargo build --release -p pixel-compare`; `python3 -m unittest -q tools/wpt/test_sp14_text_port.py`; focused openui-layout tests; `python3 tools/accountability/run_all_pixel_comparisons.py wpt/` (no resume); mapping/deferred generators; `python3 tools/accountability/audit.py`.

Next SP14 handoff: process the 288 co-blocked ported text tests by genuine dependency, then the 4045 unported text-classified tests. Keep `text_ported_tests.json` as the opt-in boundary and require exact 0-pixel parity for deterministic Ahem ports.

Superseded by 0029: W2 closed the 286 runnable co-blocked targets.
