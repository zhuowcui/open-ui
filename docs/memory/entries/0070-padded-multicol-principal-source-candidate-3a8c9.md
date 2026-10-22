---
id: 0070
title: Padded multicol principal source: candidate 3a8c9fbe layout adopted, painter deferred
tags: layout, multicol, fragmentation, positioned, checkpoint
status: active
created: 2026-10-21
updated: 2026-10-21
refs: docs/progress/multicol-principal-source-2026-10-21.md, bindings/rust/openui-layout/src/block.rs, bindings/rust/openui-layout/src/authored_projection.rs
---

Since 21 October 2026 main carries the archived candidate 3a8c9fbe's layout producer (finite_principal_source split portions, project_completed_split_principal_rows, authored_projection records, engine projection ownership) plus four repairs: final split portion distributes max(principal border box incl. block-end edges via paint_height, in-flow content); projection row membership uses the owned piece, not column capacity; finite principal source needs a positioned descendant of the container; nested relayout re-linearizes only line-box-only columns already linearized. The candidate's painter.rs (decoration/positioned column paint phases) and fragmented_column_paint_phases test remain in the archive: painter-only bisect showed gains (fieldset-002, oof-018, overflow-clip-008) and losses (oof-053, fieldset-001, overflow-clip-014..017). Bisect method: out/hunk_bisect.py splits a file's diff into hunks; Windows pixel_compare renders reproduce Linux census diffs for text-free tests. Never share CARGO_TARGET_DIR between two source copies of the workspace: cargo hashes path packages by relative path and older mtimes look fresh.
