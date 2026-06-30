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

Latest authoritative accountability snapshot (after SP12/SP12.5/SP13 layout work):

| Metric | Value |
|---|---:|
| Chromium inventory rows | 7673 |
| Ported/runnable WPT tests | 3406 |
| Unported but explicitly tracked tests | 4267 |
| Runnable passes | 2671 |
| Runnable failures | 735 |
| Runnable render/diff errors | 0 |
| Generic `not_ported` bucket rows | 0 |
| Empty unported dependency rows | 0 |
| `sp12_layout_bug` rows | 0 |

`python3 tools/accountability/audit.py` passes all 7 checks for this snapshot
(commits `b66e0df`, `4780f71` on `001-complete-sp12-parity`).

## Current Direction: layout paused, Text next (SP14+)

Layout (SP12) is complete in-scope and SP13 (fragmentation/multicol) has been driven to
~48 hard, heterogeneous residuals. **We are pausing further layout work** and pivoting to
**text**, which is the single largest unlock: `needs_text` alone blocks **4045 unported**
tests plus **336 runnable failures** (and `needs_font_metrics` another 230/553).

Important: text is **not** a from-scratch build. The text engine already exists from SP11
(`openui-text`: font metrics, HarfBuzz/Skia shaping, bidi, hyphenation, emoji, emphasis),
inline layout already shapes text, and the glyph painter already exists. The gap is the
**accountability pixel pipeline**: the WPT porting tool (`tools/wpt/port_wpt.py`) emits
box-only builders, so there are currently **0 text nodes** across ported WPT tests and text
is never compared against Chromium. The text track (SP14+) is therefore a **porting + parity**
effort. See `docs/plan/10-text-rendering-parity.md` and `docs/SP14-PLAN.md`.

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

The 735 non-passing runnable tests are ported tests classified by the feature that owns the
remaining gap. Categories can overlap because one test may depend on multiple systems.

Top runnable failure categories:

| Category | Count |
|---|---:|
| `needs_text` | 336 |
| `needs_font_metrics` | 230 |
| `sp13_fragmentation` | 217 |
| `reference_test` | 190 |
| `sp13_multicol` | 170 |
| `needs_inline_block` | 108 |
| `needs_writing_mode` | 79 |
| `needs_image` | 70 |
| `needs_gradient` | 70 |
| `needs_complex_border` | 46 |
| `needs_advanced_selectors` | 42 |
| `needs_generated_content` | 31 |

## Current Unported Inventory Ownership

The 4267 unported rows are Chromium WPT files that the current porter or renderer cannot
represent yet. They are still tracked with explicit dependency categories.

Top unported categories:

| Category | Count |
|---|---:|
| `needs_text` | 4045 |
| `needs_advanced_selectors` | 3867 |
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

## Recommended Next Work

**SP14+ text track (this is the current direction).** Text is the largest single unlock of
the unported inventory (`needs_text` 4045) and a large block of runnable failures
(`needs_text` 336 + `needs_font_metrics` 230 + `needs_inline_block` 108). Because the SP11
engine, inline layout, and glyph painter already exist, the work is to wire text through the
WPT pixel pipeline and reach Chromium parity, starting with the deterministic Ahem subset.
See `docs/plan/10-text-rendering-parity.md` (roadmap) and `docs/SP14-PLAN.md` (first SP).

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
