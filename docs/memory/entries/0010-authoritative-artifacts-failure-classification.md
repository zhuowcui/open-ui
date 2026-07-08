---
id: 0010
title: Authoritative artifacts + failure classification
tags: accountability, artifacts
status: active
created: 2026-07-08
updated: 2026-07-08
refs: tools/accountability/shared_detectors.py, tools/accountability/audit.py
---

Authoritative tracking files: tools/accountability/data/wpt_mapping.csv (pass/fail/category inventory), sp12_5_deferred.csv (deferred failing tests), pixel_comparison/results/summary.json (full-run summary). audit.py must pass 7/7 (checks: 0 generic not_ported rows, 0 empty unported-dependency rows, 0 sp12_layout_bug rows, counts reconcile, etc.). Failure classification is centralized in shared_detectors.py (single source of truth for both CSVs). Category sp12_layout_bug is the FALLBACK for tests with no detected cross-SP dependency and must stay at 0.
