---
id: 0033
title: SP17 W0A kickoff evidence is frozen before porter or layout changes
tags: sp17, accountability, writing-mode, handoff
status: active
created: 2026-08-19
updated: 2026-08-19
refs: docs/SP17-PLAN.md, tools/wpt/generate_sp17_closure.py, tools/accountability/data/wpt_ported/sp17_writing_mode_inventory.json
---

SP17 starts from main commit `2c1fe78c` on `agent/sp17-advanced-text`. A fresh
no-resume full run reproduced 3,267 pass / 299 functional fail / 0 errors across
3,566 runnable tests, and the unflagged audit passes 7/7. W0A freezes the 3,267
exact IDs, all 842 original `needs_writing_mode` rows, the 19 runnable IDs, and
their exact kickoff pixels. The inventory is 19 runnable + 823 unported; the
direct-property probe pool is 337 `writing-mode` + 3 `unicode-bidi`, with 483
rows first blocked elsewhere. W0B must add transactional parser/cascade tests,
probe those 340 rows only into temporary output, and freeze actionable/residual
dispositions before changing committed builders, mapping ownership, or the
detector. Historical SP13-R through SP16 ledger bytes are SHA-pinned.
