# Agent Memory (`docs/memory/`)

A tiny, in-repo, git-tracked memory store so an agent (or human) can **persist
durable project context across sessions** and retrieve it **without loading
everything into context at once**.

It exists because session context is large, ephemeral, and can be lost or
corrupted. This store is the durable, greppable, reviewable source of truth for
"what an agent needs to know to be productive here."

## Why it is shaped this way

| Requirement | How this store meets it |
|---|---|
| **Short per chunk** | One fact/decision/gotcha/status per file, kept small (aim ≤ ~30 lines). You read one entry, not a monolith. |
| **Indexable** | `index.tsv` is a flat, one-line-per-entry index (`id, status, tags, title, updated, path`). Grep it first. |
| **Quick search** | `mem.py search QUERY` scans titles, tags, refs, and bodies and prints compact one-line hits. |
| **Second nature** | One command to read, one to search, one to write. The index is maintained automatically — never by hand. |
| **Durable & reviewable** | Plain Markdown under git; changes show up in diffs/PRs and survive session loss. |

## Layout

```
docs/memory/
  README.md            # this file
  index.tsv            # auto-generated index (do not hand-edit; run `reindex`)
  entries/
    0001-<slug>.md     # one memory per file, small, single-topic
    0002-<slug>.md
    ...
tools/memory/mem.py    # the CLI (zero dependencies, Python 3.8+)
```

### Entry format

Each entry is Markdown with a simple `key: value` frontmatter block:

```markdown
---
id: 0007
title: summary.json is overwritten by filtered pixel runs
tags: gotcha, accountability, pipeline
status: active
created: 2026-07-08
updated: 2026-07-08
refs: tools/accountability/run_all_pixel_comparisons.py
---

Body: the actual fact/decision/gotcha/status in a few tight lines.
```

- `status`: `active` (current truth), `superseded` (kept for history, no longer
  true — point to the entry that replaces it), or `archived` (obsolete).
- `tags` / `refs`: comma-separated. `refs` point at commits (`commit:6d2d6de`),
  files, or other entry ids.

## CLI (`tools/memory/mem.py`)

```bash
# Read the whole index cheaply (or just `view docs/memory/index.tsv`)
python3 tools/memory/mem.py list [--tag T] [--status active]

# Search metadata + bodies; -v also prints matching body lines
python3 tools/memory/mem.py search "multicol" [--tag sp13] [--status active] [-v]

# Print one or more full entries
python3 tools/memory/mem.py get 0007 0012

# Add a new memory (index auto-updated). Body via --body, --body-file, or stdin
python3 tools/memory/mem.py add \
  --title "Short imperative title" \
  --tags "state,accountability" \
  --refs "commit:6d2d6de,docs/progress/current-status.md" \
  --body "One tight fact or decision."

# Update an existing entry (bumps `updated`, reindexes)
python3 tools/memory/mem.py update 0007 --status superseded --append "Replaced by 0021."

# Rebuild index.tsv from entry files (after manual edits)
python3 tools/memory/mem.py reindex
```

The store is plain files, so `grep`/`view`/`edit` work too — the CLI is just the
low-friction path that keeps the index consistent.

## When to read (do this at the start of work)

1. `view docs/memory/index.tsv` (or `mem.py list`) to see what exists.
2. `mem.py search <topic>` before touching a subsystem (e.g. `multicol`,
   `port_wpt`, `accountability`) and `mem.py get <id>` the relevant hits.
3. Prefer these entries over re-deriving state from scratch.

## When to write (keep it current)

Add or update an entry whenever you learn something durable and reusable:

- Authoritative state changes (pass/fail counts, audit status, HEAD/branch).
- A decision or pivot (with rationale).
- A gotcha / footgun / non-obvious constraint.
- A verified command or workflow.
- A code hotspot (file + region) worth remembering.
- Sprint progress and the concrete next steps.

Keep each entry small and single-topic. When a fact stops being true, mark the
entry `superseded`/`archived` (don't silently delete history) and add the new
truth as its own entry or via `update`.

Do **not** store secrets, credentials, or ephemeral one-off task instructions.
