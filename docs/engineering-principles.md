# Open UI Engineering Principles

This project is built around Chromium parity. These principles capture the working
style that has proven necessary to make progress safely.

## North Star

Open UI should render the same UI as Chromium, pixel-for-pixel, while exposing a
standalone UI engine rather than a browser.

## Completion Standard

A task is complete only when the expected state is durable in source and
accountability artifacts.

For WPT parity work, that means:

- focused comparisons pass for the touched tests and guards,
- the full summary is regenerated when global status changes,
- mapping/deferred artifacts are regenerated,
- `audit.py` passes,
- remaining failures are explicitly classified,
- and no generic bucket hides unknown work.

## No Escape Buckets

Do not use "too hard", "too large", "out of scope", or generic `not_ported` as a way
to avoid owning work. If a test cannot be fixed in the current sprint, assign it to a
named dependency with a clear owning category.

Examples:

- `sp13_fragmentation` for block fragmentation ownership,
- `sp13_multicol` for multi-column layout ownership,
- `needs_font_metrics` for real-font measurement dependencies,
- `needs_javascript` for harness/runtime dependencies,
- `needs_grid` for CSS Grid dependencies,
- `needs_complex_border` for paint-quality cases outside layout.

## Prefer Production Fixes

Implement real layout, paint, and style behavior first. Pixel nudges are acceptable
only as a last resort when:

- the affected Chromium behavior is an antialiasing/compositing edge,
- the gate is narrow and geometry/style-based,
- nearby guard tests are validated,
- and the workaround is documented by the surrounding code and accountability results.

## Preserve User and Repo State

The working tree may contain unrelated user or generated changes. Do not revert files
or commits you did not create unless explicitly asked. When validating or probing,
avoid destructive commands and restore authoritative artifacts after focused runs.

## Classification Is Code

Failure classification is part of the product. `shared_detectors.py` is the single
source of truth for mapping and deferred artifacts. When a new dependency bucket is
needed, update the detector once and regenerate both CSVs.

## Evidence Before Claims

Every progress claim should point to at least one of:

- a full `summary.json` count,
- a focused `result.json`,
- `wpt_mapping.csv` counts,
- `sp12_5_deferred.csv`,
- `audit.py` output,
- code-review verdicts,
- or a reproducible command.

Do not rely on stale focused results. The runner caches per-test `result.json` files,
and filtered runs overwrite `summary.json`.

## Review Expectations

Major exit conditions require independent verification. For sprint completion, run an
independent verifier with the exact stated conditions and evidence. If it rejects any
condition, continue work or clarify the condition; do not self-certify completion.

## Coding Practices

- Make surgical changes that address the root cause.
- Reuse existing helpers and classification logic.
- Avoid broad catches and silent fallbacks.
- Keep type safety; avoid unnecessary casts.
- Add comments only when the logic is not self-explanatory.
- Validate with existing build/test/audit tools; do not invent parallel validation
  systems unless needed.
- For Rust layout work, prefer deterministic geometry helpers and focused unit tests
  before pixel-level tuning.

## Recommended Next Phase

Start SP15 functional text-layout follow-up using the completed SP14 ledgers. Prioritize
inline construction, line breaking, root/body propagation, and the named layout owners
exposed by deterministic text ports.
