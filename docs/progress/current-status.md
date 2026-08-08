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

Latest authoritative accountability snapshot (after SP14 W2 runnable-text closure):

| Metric | Value |
|---|---:|
| Chromium inventory rows | 7673 |
| Ported/runnable WPT tests | 3406 |
| Unported but explicitly tracked tests | 4267 |
| Runnable passes | 2715 |
| Runnable failures | 691 |
| Runnable render/diff errors | 0 |
| Generic `not_ported` bucket rows | 0 |
| Empty unported dependency rows | 0 |
| `sp12_layout_bug` rows | 0 |

`python3 tools/accountability/audit.py` passes all 7 checks for this snapshot.
The full run was executed without resume, and all 2700 prior exact-pass IDs remain exact.

## Current Direction: unported text closure (SP14 W3/W4)

SP14 now has a deterministic, manifest-scoped text pipeline. All 334 formerly runnable
`needs_text` tests retain text: 41 pass exactly and 293 have evidence-backed non-text
owners. No runnable failure retains `needs_text`. The next logical batch is the 4045
unported rows that still carry that category; see `docs/plan/10-text-rendering-parity.md`
and `docs/SP14-PLAN.md`.

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

The 691 non-passing runnable tests are ported tests classified by the feature that owns the
remaining gap. Categories can overlap because one test may depend on multiple systems.

Top runnable failure categories:

| Category | Count |
|---|---:|
| `needs_font_metrics` | 225 |
| `sp13_fragmentation` | 214 |
| `reference_test` | 179 |
| `sp13_multicol` | 165 |
| `needs_inline_block` | 99 |
| `needs_writing_mode` | 79 |
| `needs_image` | 70 |
| `needs_gradient` | 70 |
| `needs_rounded_border_paint` | 56 |
| `needs_abspos_flex_static_position` | 49 |
| `needs_complex_border` | 45 |
| `needs_positioned_inline_layout` | 38 |

## Current Unported Inventory Ownership

The 4267 unported rows are Chromium WPT files that the current porter or renderer cannot
represent yet. They are still tracked with explicit dependency categories.

Top unported categories:

| Category | Count |
|---|---:|
| `needs_text` | 4045 |
| `needs_javascript` | 1945 |
| `needs_writing_mode` | 826 |
| `sp13_fragmentation` | 654 |
| `needs_font_metrics` | 553 |
| `needs_table_layout` | 458 |
| `needs_generated_content` | 456 |
| `reference_test` | 455 |
| `needs_inline_block` | 424 |
| `sp13_multicol` | 420 |
| `needs_containment` | 325 |
| `needs_grid` | 304 |
| `needs_form_controls` | 264 |
| `needs_advanced_selectors` | 200 |

## Recommended Next Work

**SP14 W3/W4 unported text closure.** Process the 4045 unported `needs_text` rows in
dependency-aware batches using the now-verified surgical text porter. Preserve the manifest
scope, exact-pixel standard, and upstream-evidence ownership rules. Do not retire the global
text detector until the unported inventory is closed.

Remaining SP13 layout residuals (~48 actionable fragmentation/multicol tests) are paused but
tracked; they can be resumed after the text track or in parallel.

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
