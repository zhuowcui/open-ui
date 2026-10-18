---
id: 0063
title: Use main as the experimental source of truth with one PR at a time
tags: workflow, git, checkpoint, user-preference
status: active
created: 2026-10-17
updated: 2026-10-17
refs: AGENTS.md, CONTRIBUTING.md, docs/progress/branch-checkpoint-2026-10-17.md, https://github.com/zhuowcui/open-ui/pull/4
---

On 17 October 2026 the user explicitly directed all pending latest implementation history to merge into main because the repository is experimental, with no PR stack or multiple pending PRs. This supersedes memory 0062 and its draft PR 3 prerequisite. PR 4 consolidates codex/native-final-linux-v4314 and its ancestor PR 3 directly into main with history preserved. Keep main as the source of truth, use this checkout, and start each later focused task from the latest main. Use one PR targeting main and merge it before starting another. Agents own integration; do not leave worktree merges for the user or create new implementation worktrees by default. Existing checkouts and borrowed Git objects remain preserved. The known native_resize_control_clip failure at scale 1.25 is explicitly accepted for this experimental integration and remains failed in the evidence; it is the first focused follow-up when feature work resumes. No release or renderer qualification is claimed. Existing evidence and unadopted candidate patches remain historical reference material. Keep feature work paused at this checkpoint and document every accepted change and unresolved failure.
