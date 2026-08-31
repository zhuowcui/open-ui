---
id: 0052
title: SP19 W0 freezes the 904-ID layout-systems closure
tags: sp19 accountability layout table grid containment handoff
status: active
created: 2026-08-30
updated: 2026-08-30
refs: tools/wpt/generate_sp19_closure.py, tools/wpt/sp19_targets.json, tools/accountability/data/wpt_ported/sp19_sp18_summary.json
---

SP19 W0 independently freezes 81 runnable repairs plus 823 static table/Grid/containment ports (341 table-only, 208 Grid-only, 231 containment-only, 43 intersections), excludes 221 executable-script rows, and projects 4,962 exact with 2,711 unported. All supplied manifest hashes reproduce exactly. Transactional generation and two no-write checks pass at wave w0; existing Python suite is 150/150, locked Rust style/text/layout/paint matrix passes, release pixel_compare builds, and audit passes 7/7 with 4,058 exact + 81 failures.
