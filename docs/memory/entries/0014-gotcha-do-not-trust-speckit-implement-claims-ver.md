---
id: 0014
title: GOTCHA: do not trust speckit.implement claims; verify manually
tags: gotcha, speckit, process
status: active
created: 2026-07-08
updated: 2026-07-08
---

speckit.implement previously produced an OVERBROAD/misleading completion claim (huge commit 0d007bb, many files) that did not reflect real audited state. Always verify actual status/guard outputs/authoritative WPT+audit state manually; don't trust agent completion claims. Speckit prerequisites currently FAIL: .specify/scripts/bash/check-prerequisites.sh --json --require-tasks --include-tasks errors with 'Feature directory not found: specs/001-complete-sp12-parity' (that dir does not exist).
