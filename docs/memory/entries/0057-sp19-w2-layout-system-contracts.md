---
id: 0057
title: SP19 W2 freezes shared layout-system contracts without baseline drift
tags: sp19 table grid containment porter resources controls pseudos accountability handoff
status: active
created: 2026-08-31
updated: 2026-08-31
refs: bindings/rust/openui-style/src/layout_systems.rs, bindings/rust/openui-style/src/computed.rs, bindings/rust/openui-dom/src/tree.rs, tools/wpt/port_wpt.py, tools/wpt/splice_text_port.py, tools/wpt/test_sp19_closure.py
---

SP19 W2 adds typed public contracts for table structure and spans, Grid tracks and placement, containment and static container queries, replaced resources, form controls, marker pseudos, masks, transforms, shapes, and fixed-time animation snapshots. The transactional porter now accepts and emits the complete frozen 823-row syntax surface, including HTML table fixup and packaged deterministic metadata, without installing target rows yet.

Two independent no-write projections produce the same 823-test, 32-output byte set with SHA-256 `d99b51d41b09073b1b530af1ac6ca02291ae9d77963d2bf9c1d8091619eceff5`. Focused pixel runs can write a separate `--summary-file`, so they no longer replace the authoritative complete-run record.

The authoritative no-resume W2 proof remains 4,139/4,139 exact with zero failures and zero errors. Verification passes all 174 Python tests, including exactly 24 SP19 tests; the locked style/DOM/text/layout/paint Rust matrix; release `pixel_compare`; two SP19 closure checks; formatting and diff checks; and the PNG-backed audit 7/7.
