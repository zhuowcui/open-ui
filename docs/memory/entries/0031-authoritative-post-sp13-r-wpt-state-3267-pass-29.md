---
id: 0031
title: Authoritative post-SP13-R WPT state: 3267 pass / 299 fail / 0 errors
tags: state, accountability, sp13r, handoff
status: active
created: 2026-08-19
updated: 2026-08-19
refs: tools/accountability/data/pixel_comparison/results/summary.json, tools/accountability/data/wpt_mapping.csv, docs/SP13-R-PLAN.md
---

PR #1 head records 3,566 runnable tests: 3,267 exact passes, 299 explicit-owner functional failures, and zero errors; 4,107 rows remain unported. All 2,823 frozen SP13-R baseline IDs and all 351 SP13-R targets are exact, no runnable row carries sp13_multicol, and audit.py passes 7/7. Re-run the complete no-resume WPT suite before freezing any later sprint baseline.
