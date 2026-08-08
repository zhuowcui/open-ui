# SP14 — Deterministic Text Porting

## Status

SP14 W0–W2 is complete for every runnable test that was classified with
`needs_text`. The authoritative full run contains 3,406 tests: 2,715 exact passes,
691 named-owner failures, and zero errors. All 2,700 exact W1 baseline IDs remain
exact, and `audit.py` passes 7/7.

| Wave | Scope | Exact | Alternate owner | `needs_text` remaining |
|---|---:|---:|---:|---:|
| W0/W1 | 48 sole-owner runnable tests | 26 | 22 | 0 |
| W2 | 286 co-blocked runnable tests | 15 | 271 | 0 |
| Total manifest | 334 text-retained tests | 41 | 293 | 0 |

Exact means zero mismatched pixels. `near_miss_aa` is diagnostic only and is not
accepted for these deterministic ports.

## Implemented infrastructure

- `port_wpt.py` has an opt-in text mode that preserves text, entities, meaningful
  whitespace, inline inheritance, and forced breaks symmetrically in Rust builders
  and Chromium templates. Ordinary box mode remains unchanged.
- Text ports use the repo-vendored Ahem face with an explicit DejaVu Sans fallback
  for the small verified glyph repertoire Ahem lacks. Font hinting and antialiasing
  are disabled on both renderers only for IDs in `text_ported_tests.json`; the
  historical Ahem corpus retains its prior environment.
- `splice_text_port.py` validates all Rust/template replacements before writing,
  preserves registry identities, rejects ambiguous or unsupported input, rolls back
  failed commits, makes dry runs side-effect free, and is idempotent.
- `run_all_pixel_comparisons.py --ids-file <json>` provides validated exact-ID
  selection. Focused runs remain non-authoritative because they overwrite
  `summary.json`.
- Failure ownership for normalized text ports is derived from the original Chromium
  HTML. Mapping and deferred generation use the same evidence and the audit rejects
  stale `needs_text` or metadata-only ownership.

## W2 result

The immutable W2 ledger is `tools/accountability/data/wpt_ported/sp14_w2_targets.json`.
All 286 entries were surgically re-ported without batch-regenerating the divergent WPT
modules. Fifteen became exact. The remaining 271 have detector-backed functional
owners, led by font metrics (145), multicol (106), fragmentation (55), inline-block
layout (46), and positioned-inline layout (36). The focused float experiment that
collapsed clearing-break struts was rejected because it damaged subsequent float rows;
the deterministic 16px break strut remains.

## Acceptance evidence

- Release `pixel-compare` build succeeds.
- SP14 Python suite: 33/33.
- Full `wpt/` run without resume: 2,715/3,406 exact, 691 fail, 0 errors.
- Baseline preservation: 2,700/2,700 prior exact IDs remain exact.
- Generated `wpt_mapping.csv`, `sp12_5_deferred.csv`, and `SP12.5-PLAN.md` are
  deterministic.
- Audit: 7/7, with zero generic `not_ported`, blank unported ownership,
  `sp12_layout_bug`, or runnable `needs_text` rows.

## Next handoff

W3/W4 should process the 4,045 unported Chromium tests still classified with
`needs_text`. Work in dependency-aware batches, extend the conservative text repertoire
only with symmetric evidence, and keep `text_ported_tests.json` as the rendering-scope
boundary. Do not retire the global text detector until those unported rows are either
ported or have objective non-text ownership. Real-font metric parity remains SP16 work.
