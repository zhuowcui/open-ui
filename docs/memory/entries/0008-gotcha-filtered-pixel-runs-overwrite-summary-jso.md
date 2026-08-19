---
id: 0008
title: GOTCHA: filtered pixel runs OVERWRITE summary.json
tags: gotcha, accountability, pipeline
status: active
created: 2026-07-08
updated: 2026-07-08
refs: tools/accountability/run_all_pixel_comparisons.py
---

run_all_pixel_comparisons.py OVERWRITES tools/accountability/data/pixel_comparison/results/summary.json on EVERY filtered run. summary.json is authoritative ONLY immediately after a full 'wpt/' run. Before any filtered/partial run: snapshot summary.json (e.g. to /tmp). After focused work: restore it or re-run full 'wpt/' before regenerating mapping/deferred artifacts and running audit.
