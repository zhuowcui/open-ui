# Open UI Current Status

This document is the handoff snapshot for the current Rust WPT/accountability phase.
It records what is complete, what remains, and how to interpret the tracking data.

## Goal

Open UI's long-term goal is Chromium pixel parity for UI rendering without shipping a
browser. The project ports Chromium/Blink rendering behavior into a standalone UI
engine with a stable API and Rust implementation layers for DOM, style, layout, and
paint.

The working standard is strict:

- Compare Open UI output against headless Chromium.
- Treat unexplained pixel differences as bugs.
- Do not claim completion without generated artifacts and audit output.
- Do not hide failures in generic buckets.
- If a failing test depends on another sprint/system, classify it with an explicit
  owning dependency.

## Verified WPT Snapshot

Latest authoritative accountability snapshot (after SP13-R closure):

| Metric | Value |
|---|---:|
| Chromium inventory rows | 7673 |
| Ported/runnable WPT tests | 3566 |
| Unported but explicitly tracked tests | 4107 |
| Runnable passes | 3267 |
| Runnable failures | 299 |
| Runnable render/diff errors | 0 |
| Generic `not_ported` bucket rows | 0 |
| Empty unported dependency rows | 0 |
| `sp12_layout_bug` rows | 0 |
| `needs_font_metrics` rows | 0 |
| Runnable `sp13_multicol` rows | 0 |
| Unported `sp13_multicol` residuals | 1018 |

`python3 tools/accountability/audit.py` passes all 7 checks for this snapshot.
The full `wpt/` run was executed without resume. All 2823 frozen SP13-R baseline
IDs and all 351 runnable multicol targets remain exact.

## SP13-R Closure

SP13-R is complete. Its immutable ledgers contain 2823 frozen exact passes, 351
runnable multicol targets, and 1018 reason-owned unported residuals. The target
run finishes 351 exact, zero failed, and zero errors; no runnable mapping row
retains `sp13_multicol`.

Multicol used geometry, fragmentation, spanners and nesting, flex and positioned
interactions, rules, and fragmented paint now consume shared resolved layout and
continuation state. The residual ledger preserves Chromium paths, porter rejection
reasons, and complete owner sets. Vertical and sideways writing remain deferred.
See `docs/SP13-R-PLAN.md` for the frozen scope and evidence.

## SP16 Closure

SP16 is complete. Its immutable ledgers contain 2,804 frozen exact passes, 226
actionable real-font tests, and 550 reason-owned unported residuals. The actionable
set finishes with 19 exact and 207 detector-backed functional failures; the 20
sole-owner targets finish 5 exact and 15 functionally reclassified. All 776 original
`needs_font_metrics` rows are covered and the category is retired globally.

DejaVu Sans, Sans Mono, and Serif regular/bold faces, Fontconfig policy, and the
pinned Chromium 147 FreeType runtime make real-font selection, metrics, and LCD
rasterization deterministic without changing the historical Ahem profile. Shared
line metrics now drive `ch`/`ex`/`lh`, used line-height, layout, and paint. See
`docs/SP16-PLAN.md` for the frozen scope and evidence.

## SP15 Closure

SP15 is complete. Its immutable ledgers contain 2767 frozen exact passes, 76 actionable
tests, and 54 reason-owned unported residuals. All 49 root/body targets became runnable;
the 76-test actionable set finishes with 34 exact and 42 detector-backed functional
failures. The deterministic text manifest now contains 496 tests: 128 exact and 368
with functional non-text owners.

No mapping row retains any of the five retired SP15 categories. Root/body canvas and
overflow propagation, semantic clearing breaks, real decorated-inline continuation
fragments, and `display:contents` inheritance/style handling are implemented. See
`docs/SP15-PLAN.md` for the frozen scope and evidence.

## What "SP12 Complete" Means

SP12 is complete in the accountability sense: no remaining failure is classified as
an SP12-owned layout bug. It does **not** mean every test file in the SP12-scope WPT
directories passes.

The SP12-scope WPT directories include tests whose visible output depends on other
systems such as text shaping, inline layout, fragmentation, multicol, images,
gradients, JavaScript harness behavior, writing modes, grid/table layout, generated
content, and native form controls. Those rows are tracked under explicit dependency
categories rather than `sp12_layout_bug`.

## Current Runnable Failure Ownership

The 299 non-passing runnable tests are ported tests classified by the feature that owns the
remaining gap. Categories can overlap because one test may depend on multiple systems.

Top runnable failure categories:

| Category | Count |
|---|---:|
| `reference_test` | 90 |
| `needs_gradient` | 80 |
| `needs_image` | 66 |
| `needs_inline_block` | 55 |
| `needs_complex_border` | 52 |
| `needs_empty_block_margin_collapse` | 42 |
| `needs_body_canvas_background_extent` | 24 |
| `needs_generated_content` | 23 |
| `needs_rounded_border_paint` | 22 |
| `needs_writing_mode` | 19 |
| `needs_box_shadow` | 18 |
| `sp13_fragmentation` | 17 |

## Current Unported Inventory Ownership

The 4107 unported rows are Chromium WPT files that the current porter or renderer cannot
represent yet. They are still tracked with explicit dependency categories.

Top unported categories:

| Category | Count |
|---|---:|
| `needs_javascript` | 1945 |
| `sp13_multicol` | 1018 |
| `needs_writing_mode` | 823 |
| `sp13_fragmentation` | 651 |
| `needs_table_layout` | 462 |
| `needs_generated_content` | 443 |
| `reference_test` | 441 |
| `needs_inline_block` | 410 |
| `needs_containment` | 335 |
| `needs_image` | 325 |
| `needs_advanced_selectors` | 318 |
| `needs_grid` | 304 |
| `needs_empty_block_margin_collapse` | 304 |
| `needs_form_controls` | 264 |

## Recommended Next Work

After PR #1 is green and merged, start SP17 advanced text from fresh `main`.
The handoff freezes the expected 3,267-pass starting state, enumerates all 19
runnable writing-mode rows, and accounts for the 823 unported writing-mode rows,
including the 337 that currently stop directly on porter writing-mode rejection.
Follow `docs/SP17-HANDOFF.md` and preserve the exact-pixel standard, frozen
SP13-R baseline and target ledgers, and upstream-evidence ownership rules.

Hosted pre-merge checks and the separate pinned Chromium parity gate are
documented in `docs/CI.md`.

## Authoritative Commands

Build the comparison binary:

```bash
cd bindings/rust
cargo build --release --package pixel-compare
```

Run the full WPT pixel comparison:

```bash
LD_LIBRARY_PATH="$PWD/chrome/linux-147.0.7727.50/chrome-linux64" \
  python3 -u tools/accountability/run_all_pixel_comparisons.py 'wpt/'
```

Regenerate tracking:

```bash
python3 tools/accountability/generate_wpt_mapping.py
python3 tools/accountability/generate_sp12_5_csv.py
```

Audit:

```bash
python3 tools/accountability/audit.py
```

Focused WPT runs overwrite `summary.json`; snapshot it before running filtered
comparisons and restore it afterward unless the focused run is intentionally replacing
the authoritative summary.
