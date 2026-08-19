---
id: 0024
title: CRITICAL: committed wpt modules are divergent from port_wpt.py; batch regen is UNSAFE
tags: gotcha, port_wpt, accountability, pilot, blocker
status: active
created: 2026-07-08
updated: 2026-07-08
refs: tools/wpt/batch_port_sp12.py, bindings/rust/pixel-compare/src/wpt/
---

Running `python3 tools/wpt/batch_port_sp12.py` (the "documented" regen) REWRITES all generated wpt modules with ~350K insertions / ~369K deletions vs the committed versions — even with EMIT_TEXT_NODES=False. The committed bindings/rust/pixel-compare/src/wpt/wpt_css_*.rs are STALE/divergent from the current porter (older porter version and/or hand-edits). The 2671/735 baseline was measured with the COMMITTED modules, so a full regen would shift results — DO NOT run batch_port_sp12.py to regen the corpus. (Contradicts the older "regen with batch_port_sp12.py" note — that is unsafe now.)
IMPLICATION for the SP14 pilot: do NOT regenerate existing modules. Port the Ahem pilot into a SEPARATE, NEW module (e.g. via port_wpt.py main with a dedicated module name / --output-dir) containing only the pilot tests, and register just that module in wpt/mod.rs, leaving the box-only corpus untouched. Also the HTML template side (generate_html_template strips text — entry 0023) still needs a text-retaining path for the pilot references.
