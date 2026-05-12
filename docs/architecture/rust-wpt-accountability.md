# Rust WPT Accountability Architecture

This document describes the current Rust-side WPT comparison and tracking system.

## Purpose

The accountability system prevents false progress claims. It ties every status claim
to generated artifacts:

- a Chromium WPT inventory row,
- a generated Rust test and HTML template when ported,
- pixel comparison PNGs and `result.json`,
- a full `summary.json`,
- `wpt_mapping.csv` classification,
- `sp12_5_deferred.csv` dependency tracking,
- and `audit.py` verification.

## Pipeline

```text
Chromium WPT CSS files
        |
        v
tools/wpt/batch_port_sp12.py
        |
        +--> bindings/rust/pixel-compare/src/wpt/*.rs
        |    Generated Rust Document builders and registries.
        |
        +--> tools/accountability/data/wpt_ported/*.json
        |    HTML templates used for Chromium reference rendering.
        |
        +--> tools/accountability/data/wpt_ported/*_report.csv
             Porter status and rejection reasons.

pixel_compare + headless Chromium
        |
        v
tools/accountability/data/pixel_comparison/results/
        |
        +--> per-test openui.png, chromium.png, diff.png, result.json
        +--> summary.json

tracking generation
        |
        v
tools/accountability/data/wpt_mapping.csv
tools/accountability/data/sp12_5_deferred.csv
docs/SP12.5-PLAN.md

audit
        |
        v
tools/accountability/audit.py
```

## Key Artifacts

| Artifact | Role |
|---|---|
| `bindings/rust/pixel-compare/src/wpt/*.rs` | Generated Rust WPT builders. |
| `tools/accountability/data/wpt_ported/all_wpt_templates.json` | HTML templates for Chromium reference rendering. |
| `tools/accountability/data/wpt_ported/*_report.csv` | Ported/not-portable report by area. |
| `tools/accountability/data/pixel_comparison/results/summary.json` | Authoritative runnable test status. |
| `tools/accountability/data/wpt_mapping.csv` | Full Chromium inventory and classification. |
| `tools/accountability/data/sp12_5_deferred.csv` | Runnable failing tests deferred to another dependency. |
| `docs/SP12.5-PLAN.md` | Human-readable deferred dependency plan. |
| `tools/accountability/shared_detectors.py` | Single source of truth for dependency classification. |
| `tools/accountability/audit.py` | Integrity verifier. |

## Classification Model

There are two different axes:

1. **Where the test file comes from**: the SP12-scope Chromium WPT directory, such as
   `css-flexbox`, `css-overflow`, `css-backgrounds`, `css-break`, or `css-multicol`.
2. **Who owns the remaining gap**: SP12, SP11, SP13, Future, or N/A.

A test can be in an SP12-scope directory while depending on SP11 text/font metrics,
SP13 fragmentation, or a future grid/table/JavaScript harness feature.

`sp12_layout_bug` is the fallback category for a failing runnable test when no other
dependency detector applies. A clean SP12 exit requires this count to be zero.

Unported tests must not use the generic `not_ported` failure bucket. They must have a
named dependency category derived from either their HTML content or the porter rejection
reason.

## Audit Invariants

`tools/accountability/audit.py` verifies:

1. Every pass has `result.json`, `openui.png`, `chromium.png`, and 0.0% mismatch.
2. Every generated template appears in `summary.json`.
3. Every summary test has a Rust registry entry and function definition.
4. `wpt_mapping.csv` has valid categories and matches summary accounting.
5. Deferred runnable failures exist in summary and are still failing.
6. There are no orphan result directories.
7. Mapping and deferred classification agree.
8. Every unported row has an explicit dependency category and no row uses generic
   `not_ported`.

## Focused Runs

`run_all_pixel_comparisons.py` accepts a prefix filter, but it always writes
`summary.json`. For focused probes:

```bash
cp tools/accountability/data/pixel_comparison/results/summary.json /tmp/summary.json
rm -f tools/accountability/data/pixel_comparison/results/<test-id>/result.json
LD_LIBRARY_PATH="$PWD/chrome/linux-147.0.7727.50/chrome-linux64" \
  python3 -u tools/accountability/run_all_pixel_comparisons.py '<prefix>'
cp /tmp/summary.json tools/accountability/data/pixel_comparison/results/summary.json
```

Deleting the target `result.json` is required because cached pass/fail results are
otherwise reused.
