---
id: 0001
title: Project north star: Chromium pixel parity under strict accountability
tags: project, standard
status: active
created: 2026-07-08
updated: 2026-07-08
refs: docs/progress/current-status.md, docs/engineering-principles.md
---

Open UI ports Chromium/Blink rendering into a standalone engine (Rust DOM/style/layout/paint) WITHOUT shipping a browser. The bar is EXACT (0.0%) pixel parity vs headless Chromium. Strict accountability rules: unexplained pixel diffs are bugs; no completion claims without regenerated artifacts + audit; never hide failures in generic buckets; every failing test must have an explicit owning dependency (SP/category).
