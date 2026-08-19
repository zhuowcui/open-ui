---
id: 0032
title: CI landing gate and SP17 from-main handoff
tags: ci, sp17, handoff, next-steps
status: active
created: 2026-08-19
updated: 2026-08-19
refs: .github/workflows/ci.yml, .github/workflows/format-check.yml, docs/CI.md, docs/SP17-HANDOFF.md
---

Before SP17, make PR #1 hosted checks green and merge it; start agent/sp17-advanced-text from fresh main, never continue the accumulated branch. Hosted CI covers portable GN Debug/Release smoke, native formatting, Rust style/text/layout/paint tests, 99 Python closure tests, immutable SP13-R ledgers, and audit 7/7. SP17 begins with 842 needs_writing_mode rows: 19 runnable and 823 unported; 337 unported rows stop directly on writing-mode, but transactional probing determines the real target set. Preserve 3,267 exact starting passes and follow docs/SP17-HANDOFF.md.
