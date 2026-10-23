---
id: 0075
title: Candidate painter positioned phases must follow Blink per-layer fragment order
tags: paint, multicol, fragmentation, positioned, census
status: active
created: 2026-10-22
updated: 2026-10-23
refs: docs/progress/painter-candidate-variants-2026-10-22.md, docs/progress/evidence/painter-candidate-variants-census-v1.json
---

Candidate 3a8c9fbe painter.rs against main de28b52c (paired Linux expanded censuses): main 21,686 exact; positioned-phases hunks alone 21,630 (+4/-60); decoration-replay hunks alone 21,683 (+8/-11); whole painter 21,690 (+13/-9). Groups are interdependent. The out-of-flow-in-multicolumn-053 loss: the candidate paints positioned phases grouped per outer fragmentainer (green decoration prepass, then column 1 red+span text, column 1 green, column 2 red+text), so column 2's LCD text fringe lands on column 1's green at x=69. Blink's PaintLayerPainter paints each layer phase across all fragments, then each child layer across all fragments, so the positioned green box always paints last. Reproduce on Windows with --raster-config chromium-linux-lcd (Windows aliased text hides it). Tools: out/hunk_bisect.py with BISECT_FILE, out/wsl-paint-census.sh. Adopted 23 October 2026 (column-paint-phases record) with four repairs: owners with inline-level positioned descendants keep the ordinary path; out-of-flow subtrees leave the shared in-flow column prepass and positioned owners paint after in-flow (own background in every column, then descendants column-major); fieldsets with a rendered legend keep the shared decoration instead of the complete replay. Census 21,686 -> 21,699, 13 gains, 0 losses. Fourth repair: the phase admits a position:fixed box that has positioned_fragmentation (fragmented under a transform containing block, so it paints like an absolute one); unfragmented fixed boxes stay out; without it out-of-flow-in-multicolumn-037/039 worsened. Only spanner-fragmentation-002 @1.25 worsens by 1 px.
