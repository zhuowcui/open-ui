# Copilot Instructions — open-ui

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
