---
id: 0026
title: SP14 100% strategy: 4381 needs_text = 48 sole + 288 co-blocked ported + 4045 unported
tags: sp14, text, strategy, plan
status: active
created: 2026-07-14
updated: 2026-07-14
refs: tools/accountability/data/wpt_mapping.csv, tools/accountability/shared_detectors.py
---

needs_text(4381): 336 ported+failing (48 with needs_text as SOLE category; 288 with co-blockers incl sp13_multicol/fragmentation/font_metrics), 4045 unported ALL with non-text porter deferral reasons (JS 1858, img 222, writing-mode 293, contain, table, grid...). Plan: W0 deterministic text infra (fontconfig kill AA for Ahem in render_chrome, scoped so committed corpus unaffected; porter text-retaining template mode + Ahem forcing both sides; surgical per-test splice tool honoring mem 0024). W1 fix 48 sole to pass. W2 re-port 288, fix text-caused failures, others owned by real co-categories. W3/W4 retire text_rendering (+font_metrics if warranted) from shared_detectors DEPENDENCY_DEFS, regen wpt_mapping+sp12_5_deferred (must update SP12.5-PLAN.md deferred count for audit), full-suite guard, audit 7/7. Classifications derive from UPSTREAM chromium HTML via classify_failure_categories.
