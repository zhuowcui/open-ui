---
id: 0007
title: Font strategy: use DejaVu Sans fallback, do NOT install Ahem
tags: text, font, gotcha, sp14
status: active
created: 2026-07-08
updated: 2026-07-08
---

Use the shared DejaVu Sans fallback for both our renderer and the Chromium reference. Do NOT install the Ahem font: installing it would re-render the 245 Ahem-using reference images and regress ~119 currently-passing tests. DejaVu (same font + same Skia as headless-Chromium reference) makes real-text parity achievable. ~245 corpus tests use Ahem (exact square glyphs, most deterministic once metrics resolve identically on both sides).
