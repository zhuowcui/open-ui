---
id: 0016
title: Memory provenance: seeded from recovered session 4ab79aa9 'Read Repo Documentation'
tags: meta, provenance, recovery
status: active
created: 2026-07-08
updated: 2026-07-08
---

This memory store was created 2026-07-08 and initially seeded by recovering context from session 4ab79aa9-a7c9-4439-9d40-89c6e5d77d9a ("Read Repo Documentation"). That session's events.jsonl had grown to ~1.2GB (the corruption/slowness cause). Recoverable sources were its 41 checkpoints (256-296) and session-state/.../plan.md (newest, dated 2026-06-30). Going forward, prefer this store over resurrecting giant session logs. Local session store: ~/.copilot/session-store.db (tables: sessions, turns, checkpoints with rich fields).
