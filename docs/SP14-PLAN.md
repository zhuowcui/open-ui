# SP14 — Deterministic Text Porting

## Status

SP14 W0–W4 is complete. The full 4,045-row unported text backlog is closed:
111 deterministic tests became runnable and the other 3,934 have reason-backed
functional owners. The authoritative full run contains 3,517 tests: 2,767 exact
passes, 750 named-owner failures, and zero errors. All 2,715 frozen baseline exact
IDs remain exact, and `audit.py` passes 7/7.

| Wave | Scope | Exact | Alternate owner | `needs_text` remaining |
|---|---:|---:|---:|---:|
| W0/W1 | 48 sole-owner runnable tests | 26 | 22 | 0 |
| W2 | 286 co-blocked runnable tests | 15 | 271 | 0 |
| W3 | 111 formerly unported deterministic tests | 52 | 59 | 0 |
| Total manifest | 445 text-retained tests | 93 | 352 | 0 |
| W4 | 3,934 unported residual dispositions | — | 3,934 | 0 |

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
- `splice_text_port.py` accepts positional IDs or `--ids-file`, derives canonical IDs
  for unported mapping rows, and transactionally mixes new builders with replacements.
  It validates every Rust registry and module/global template change in memory,
  rejects collisions and partial state, rolls back failed commits, preserves dry-run
  safety, and is idempotent.
- `run_all_pixel_comparisons.py --ids-file <json>` provides validated exact-ID
  selection. Focused runs remain non-authoritative because they overwrite
  `summary.json`.
- Failure ownership for normalized text ports is derived from the original Chromium
  HTML. Mapping and deferred generation use the same evidence and the audit rejects
  stale `needs_text` or metadata-only ownership.

## W3/W4 result

The immutable W3 ledger contains 2 floats, 64 backgrounds, 25 flexbox, 15 multicol,
and 5 sizing tests. All 111 were inserted without regenerating the divergent modules.
Fifty-two became exact; the other 59 expose functional gaps led by complex-border,
image, rounded-border, gradient/generated-content, multicol, and writing-mode owners.
Eleven of the 14 former sole-`needs_text` rows are exact; the other three are
multi-value complex-border paint cases.

The structured W4 ledger records all 3,934 residual IDs with their Chromium paths,
actual deterministic rejection reasons, rejection owners, and merged upstream
detector ownership. W3 and W4 are unique, disjoint, and cover all 4,045 original rows.
The global `text_rendering`/`needs_text` dependency is retired. Root/body-only porter
rejections have explicit SP15 viewport-propagation ownership rather than metadata-only
or generic fallback ownership.

## Acceptance evidence

- Release `pixel-compare` build succeeds.
- SP14 Python suite: 43/43.
- Relevant Rust suite: 521/521; release `pixel-compare` build succeeds.
- Focused W3 run: 52 exact, 59 functional failures, 0 errors.
- Full `wpt/` run without resume: 2,767/3,517 exact, 750 fail, 0 errors.
- Baseline preservation: 2,715/2,715 prior exact IDs remain exact.
- Generated `wpt_mapping.csv`, `sp12_5_deferred.csv`, `SP12.5-PLAN.md`, and
  `report.html` are deterministic.
- Audit: 7/7, with zero generic `not_ported`, blank unported ownership,
  `sp12_layout_bug`, `needs_text`, metadata-only W4 ownership, or render errors.

## Next handoff

SP15 owns the inline/layout and root/body propagation gaps exposed by deterministic
text ports. SP16 remains the real-font metrics phase, SP17 owns writing modes/bidi and
advanced text, and SP18 owns generated content and text effects. The W4 ledger also
keeps existing JavaScript, image, grid, table, containment, and form-control systems
explicitly accountable.
