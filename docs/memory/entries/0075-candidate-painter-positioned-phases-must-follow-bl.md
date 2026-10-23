---
id: 0075
title: Candidate painter positioned phases must follow Blink per-layer fragment order
tags: paint, multicol, fragmentation, positioned, census
status: active
created: 2026-10-22
updated: 2026-10-22
refs: docs/progress/painter-candidate-variants-2026-10-22.md, docs/progress/evidence/painter-candidate-variants-census-v1.json
---

Candidate 3a8c9fbe painter.rs against main de28b52c (paired Linux expanded censuses): main 21,686 exact; positioned-phases hunks alone 21,630 (+4/-60); decoration-replay hunks alone 21,683 (+8/-11); whole painter 21,690 (+13/-9). Groups are interdependent. The out-of-flow-in-multicolumn-053 loss: the candidate paints positioned phases grouped per outer fragmentainer (green decoration prepass, then column 1 red+span text, column 1 green, column 2 red+text), so column 2's LCD text fringe lands on column 1's green at x=69. Blink's PaintLayerPainter paints each layer phase across all fragments, then each child layer across all fragments, so the positioned green box always paints last. Reproduce on Windows with --raster-config chromium-linux-lcd (Windows aliased text hides it). Tools: out/hunk_bisect.py with BISECT_FILE, out/wsl-paint-census.sh.
