# Open UI Engineering Principles

This project is built around Chromium parity. These principles capture the working
style that has proven necessary to make progress safely.

## North Star

Open UI should render the same UI as Chromium, pixel-for-pixel, while exposing a
standalone native UI engine. Applications implement interaction in Rust through
public `Document` and `Element` APIs. Open UI never executes JavaScript, in this
or future versions.

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

For application interaction, completion requires a public native Rust API
over the shared retained engine and verification from a consuming Rust app.
That verification must cover the operation's state changes and applicable
events, geometry, or rendering. A needed element operation remains unfinished
until this public application path works, even if an internal Engine fixture
already produces exact Chromium pixels.

## No Escape Buckets

Do not use "too hard", "too large", "out of scope", or generic `not_ported` as a way
to avoid owning work. If a test cannot be fixed in the current sprint, assign it to a
named dependency with a clear owning category.

Examples:

- `sp13_fragmentation` for block fragmentation ownership,
- `sp13_multicol` for multi-column layout ownership,
- `needs_writing_mode` for vertical-flow and bidi dependencies,
- `needs_javascript` is a legacy label for Chromium WPT files containing
  scripts. Review their deterministic final visual states against Chromium
  using native Rust fixtures, and expose any application-needed interaction
  through the public Rust API. The label never calls for an Open UI JavaScript
  runtime or waives a native API gap,
- `needs_grid` for CSS Grid dependencies,
- `needs_complex_border` for paint-quality cases outside layout.

## Prefer Production Fixes

Implement shared layout, paint, style, and raster behavior that matches the
pinned Chromium oracle. The v0.2 pixel gate uses exact decoded RGBA equality at
every required profile, with zero tolerance. Do not add fixture-specific pixel
corrections, post-raster output edits, exception lists, or rewritten reference
images. Keep a residual open with measured bounds, channel deltas, a minimized
reproducer, and reviewed ownership until its exact gate passes.

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

## Current Work

The active contract is v0.2 Linux and headless release closure. Use
[`docs/renderer/contract.md`](renderer/contract.md) for the pixel gate and
[`docs/progress/current-status.md`](progress/current-status.md) for measured
progress. SP13–SP20 sprint documents are historical evidence. Chromium test
files that contain scripts may be assessed for deterministic visual final
states through native Rust fixtures; they do not create a JavaScript runtime
requirement. Application interactions belong in the public native Rust API.
