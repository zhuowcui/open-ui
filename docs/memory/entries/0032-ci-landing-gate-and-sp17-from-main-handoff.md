---
id: 0032
title: CI landing gate and SP17 from-main handoff
tags: ci, sp17, handoff, next-steps
status: active
created: 2026-08-19
updated: 2026-08-19
refs: .github/workflows/ci.yml, .github/workflows/format-check.yml, docs/CI.md, docs/SP17-HANDOFF.md
---

Before SP17, make PR #1 hosted checks green and merge it; start agent/sp17-advanced-text from fresh main, never continue the accumulated branch. Hosted CI covers portable GN Debug/Release smoke, native formatting, Rust style/text/layout/paint tests, 100 Python closure tests, immutable SP13-R ledgers, and repository-contained accountability. Porter tests must use the committed `tools/wpt/fixtures/upstream` snapshots rather than an implicit sibling Chromium checkout; the first hosted run caught and removed that dependency. Standalone native builds must keep Chromium-only diagnostic switches behind `chromium_src`, because distro Clang 18 treats `-Wno-gcc-install-dir-libstdcxx` as an error. Hosted CI uses `audit.py --repository-only` because PNGs are intentionally ignored; it keeps exact committed result proof and checks 2–7 strict, while only unflagged local `audit.py` counts as full 7/7 pixel verification. SP17 begins with 842 needs_writing_mode rows: 19 runnable and 823 unported; 337 unported rows stop directly on writing-mode, but transactional probing determines the real target set. Preserve 3,267 exact starting passes and follow docs/SP17-HANDOFF.md.
