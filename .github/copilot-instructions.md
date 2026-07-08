# Copilot Instructions — open-ui

## Project memory (MANDATORY — read at start, write as you learn)

This repo has a durable, in-repo memory store at `docs/memory/` (CLI:
`tools/memory/mem.py`). It exists so you do NOT have to reconstruct project
state from giant session logs. Treat it as your working memory: it is
short per entry (one fact/decision/gotcha/status per file), indexed
(`docs/memory/index.tsv`), and searchable. Full spec: `docs/memory/README.md`.

**At the START of every task, before doing anything else:**

1. Read the index: `view docs/memory/index.tsv` (or
   `python3 tools/memory/mem.py list`). It is one line per entry — cheap.
2. Search before touching a subsystem, e.g.
   `python3 tools/memory/mem.py search "multicol"` (add `--tag`/`--status`,
   `-v` for body lines), then `python3 tools/memory/mem.py get <id>` the hits.
3. Prefer memory over re-deriving state. Entry `0002` holds the current
   authoritative WPT snapshot; re-run the pipeline to confirm before relying
   on any number.

**WRITE to memory whenever you learn something durable and reusable** — a
state change (pass/fail counts, audit status, HEAD/branch), a decision/pivot
with rationale, a gotcha/footgun, a verified command, a code hotspot
(file+region), or concrete sprint progress + next steps:

```bash
python3 tools/memory/mem.py add --title "…" --tags "a,b" \
  --refs "commit:<sha>,path/to/file" --body "one tight fact"
python3 tools/memory/mem.py update <id> --status superseded --append "Replaced by 00NN."
```

Rules: keep each entry small and single-topic; when a fact stops being true
mark it `superseded`/`archived` (don't silently delete history) and record the
new truth; the index is auto-maintained by the CLI — run `reindex` only after
hand-editing files; never store secrets or one-off ephemeral task notes.
Keeping this store current is part of finishing the work, not optional.

## Exit-condition enforcement (MANDATORY)

Before calling `task_complete` on any task that has user-stated exit
conditions (e.g. "don't stop until X", "exit conditions: 1) ..., 2) ...,
3) ..."), you MUST:

1. **Launch a sub-agent** (via the `task` tool, typically `rubber-duck`
   or `general-purpose`) whose sole job is to independently verify that
   every stated exit condition has been objectively met. Provide the
   sub-agent with:
   - The full list of exit conditions as stated by the user.
   - The concrete evidence you believe satisfies each (commit SHAs, test
     counts, CSV classifications, audit output, review transcripts, etc.).
   - Authority to run verification commands (audit, counts, diffs) itself.
2. **Wait for the sub-agent's verdict.** If it finds ANY condition
   unsatisfied — even partially — do NOT call `task_complete`. Resume
   work on the gap and re-verify.
3. Only call `task_complete` after the sub-agent explicitly concurs that
   every exit condition is fully met, and quote its verdict in your
   completion summary.

This rule exists because prior sessions have overclaimed completion
(e.g. calling `task_complete` while hundreds of residuals remained
misclassified or uninvestigated). Self-assessment is not sufficient;
an independent verification pass is required.

Do not reinterpret or narrow the user's exit conditions to make them
easier to satisfy. If a condition is ambiguous, ask via `ask_user`
rather than deciding unilaterally.

## Execution standard (MANDATORY)

This work is being done by an agentic coding agent. Do not use human
fatigue, task size, or "too big in scope" as a reason to stop, defer, or
declare work out of scope. When the user sets an objective, decompose it
into concrete engineering steps and keep executing until the objective is
met or a real external blocker requires user input.

Do not create informal escape buckets such as "too hard", "multi-day",
"edge case", or "out of scope" to avoid fixing failures. If a test is in
the active SP, it must either pass pixel comparison with Chromium or be
explicitly assigned to another named SP / tracked owning category with a
clear rationale. A remaining failure may not be treated as complete just
because the fix is large, architectural, or inconvenient.

All work should be production quality immediately: correct, validated,
tracked in the authoritative artifacts, and consistent with the user's
stated standard of 100% Chromium pixel parity.

## Standing repo facts

- WPT pipeline: regen templates with `python3 tools/wpt/batch_port_sp12.py`;
  build with `cd bindings/rust && cargo build --release --package pixel-compare`;
  run with `LD_LIBRARY_PATH="$PWD/chrome/linux-147.0.7727.50/chrome-linux64"
  python3 -u tools/accountability/run_all_pixel_comparisons.py 'wpt/'`.
- `run_all_pixel_comparisons.py` OVERWRITES `summary.json` on partial
  runs. Snapshot before running any filtered pipeline invocation.
- Authoritative tracking: `tools/accountability/data/wpt_mapping.csv`,
  `sp12_5_deferred.csv`, `pixel_comparison/results/summary.json`.
  Validate with `python3 tools/accountability/audit.py` — must pass 7/7.
- Failure classification lives in
  `tools/accountability/shared_detectors.py` (single source of truth
  for wpt_mapping.csv + sp12_5_deferred.csv). Category `sp12_layout_bug`
  is the fallback for tests with no detected cross-SP dependency.

## Active Technologies
- Rust workspace plus Python tooling; exact compiler/interpreter versions are whatever the repository's existing build and accountability scripts currently use successfully. + OpenUI Rust layout/rendering crates, `pixel-compare`, Skia/Chromium rendering assets already used by the repo, Chromium headless binary for reference rendering, WPT/Blink test corpus. (001-complete-sp12-parity)
- File-based authoritative artifacts: `tools/accountability/data/wpt_mapping.csv`, `tools/accountability/data/sp12_5_deferred.csv`, `tools/accountability/data/pixel_comparison/results/summary.json`, per-test result directories, generated Rust WPT files. (001-complete-sp12-parity)

## Recent Changes
- 001-complete-sp12-parity: Added Rust workspace plus Python tooling; exact compiler/interpreter versions are whatever the repository's existing build and accountability scripts currently use successfully. + OpenUI Rust layout/rendering crates, `pixel-compare`, Skia/Chromium rendering assets already used by the repo, Chromium headless binary for reference rendering, WPT/Blink test corpus.
