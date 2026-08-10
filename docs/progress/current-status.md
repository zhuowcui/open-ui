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

Latest authoritative accountability snapshot (after SP15 closure):

| Metric | Value |
|---|---:|
| Chromium inventory rows | 7673 |
| Ported/runnable WPT tests | 3566 |
| Unported but explicitly tracked tests | 4107 |
| Runnable passes | 2804 |
| Runnable failures | 762 |
| Runnable render/diff errors | 0 |
| Generic `not_ported` bucket rows | 0 |
| Empty unported dependency rows | 0 |
| `sp12_layout_bug` rows | 0 |

`python3 tools/accountability/audit.py` passes all 7 checks for this snapshot.
The full `wpt/` run was executed without resume, and all 2767 frozen SP15
exact-pass IDs remain exact.

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

The 762 non-passing runnable tests are ported tests classified by the feature that owns the
remaining gap. Categories can overlap because one test may depend on multiple systems.

Top runnable failure categories:

| Category | Count |
|---|---:|
| `sp13_multicol` | 363 |
| `needs_font_metrics` | 226 |
| `sp13_fragmentation` | 213 |
| `reference_test` | 180 |
| `needs_inline_block` | 101 |
| `needs_empty_block_margin_collapse` | 93 |
| `needs_image` | 87 |
| `needs_gradient` | 87 |
| `needs_writing_mode` | 81 |
| `needs_complex_border` | 77 |
| `needs_rounded_border_paint` | 74 |
| `needs_abspos_flex_static_position` | 50 |
| `needs_positioned_inline_layout` | 39 |

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
| `needs_font_metrics` | 550 |
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

Proceed to SP16 real-font metrics or resume the named SP13 multicol/fragmentation
residuals. Preserve the exact-pixel standard, frozen SP15 baseline, and upstream-evidence
ownership rules.

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
