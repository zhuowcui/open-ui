---
id: 0034
title: SP17 W0B transactional porter probe is frozen at 311 actionable rows
tags: sp17, writing-mode, porter, cascade, accountability
status: active
created: 2026-08-19
updated: 2026-08-19
refs: docs/SP17-PLAN.md, tools/wpt/port_wpt.py, tools/wpt/generate_sp17_closure.py, tools/accountability/data/wpt_ported/sp17_actionable_targets.json, tools/accountability/data/wpt_ported/sp17_residual_dispositions.json
---

SP17 W0B adds transactional handling for writing mode, direction, bidi,
orientation, text combine, and corpus-used logical box properties. Cascade
metadata preserves importance, specificity, source order, inline declarations,
and `dir` hints until logical and physical aliases are resolved against the
computed writing direction. Inherited writing properties are materialized on
elements and generated structural/text nodes; `unicode-bidi` stays
non-inherited.

A faithful retained-text deterministic-Ahem probe ran the actual builder over
all 823 kickoff-unported SP17 rows. It found 292 newly generatable rows and 531
reason-backed residuals. With the 19 kickoff-runnable tests, the immutable
actionable ledger has 311 IDs; both ledgers are sorted, disjoint, and cover the
original 842 rows. A prospective atomic splice of all 292 builders succeeded in
memory across 23 files, but no Rust builder, mapping row, runner profile, or
pixel result was changed.

Only `wpt/css_flexbox/css-flexbox-test1` and its `-ref` retain SP17 ownership in
the residual ledger. Their fullwidth digits are unavailable in pinned Ahem and
vendored DejaVu Sans, so they remain rejected as `text_non_ascii` until W2 adds
a pinned glyph/fallback path. W1 starts by threading complete writing direction
through `ConstraintSpace` with horizontal-no-op and orthogonal-boundary tests;
do not batch-splice the 292 builders before shared geometry is ready.
