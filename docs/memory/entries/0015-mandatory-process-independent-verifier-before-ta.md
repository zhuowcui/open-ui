---
id: 0015
title: MANDATORY process: independent verifier before task_complete; no escape buckets
tags: process, standard
status: active
created: 2026-07-08
updated: 2026-07-08
refs: .github/copilot-instructions.md
---

Before task_complete on any task with user-stated exit conditions ("don't stop until X"): launch an independent verifier sub-agent (rubber-duck/general-purpose) that objectively checks EVERY condition with authority to run audits/counts/diffs; only complete after it explicitly concurs, and quote its verdict. Execution standard: do not use "too hard / too big / edge case / out of scope" as escape buckets. Every active-SP failing test must either pass Chromium pixel comparison or be explicitly assigned to another named SP/owning category with rationale. Full rules in .github/copilot-instructions.md.
