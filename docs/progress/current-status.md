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

Latest authoritative SP12-scope accountability snapshot:

| Metric | Value |
|---|---:|
| Chromium SP12-scope inventory rows | 7673 |
| Ported/runnable WPT tests | 3406 |
| Unported but explicitly tracked tests | 4267 |
| Runnable passes | 2430 |
| Runnable failures | 974 |
| Runnable render/diff errors | 2 |
| Generic `not_ported` bucket rows | 0 |
| Empty unported dependency rows | 0 |
| `sp12_layout_bug` rows | 0 |

`python3 tools/accountability/audit.py` passes all checks for this snapshot.

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

The 976 non-passing runnable tests are ported tests from SP12-scope directories. They
are classified by the feature that owns the remaining gap. Categories can overlap
because one test may depend on multiple systems.

Top runnable failure categories:

| Category | Count | Owner |
|---|---:|---|
| `sp13_fragmentation` | 347 | SP13 |
| `needs_text` | 344 | SP11/SP13 |
| `sp13_multicol` | 280 | SP13 |
| `needs_font_metrics` | 233 | SP11 |
| `reference_test` | 208 | N/A |
| `needs_inline_block` | 111 | SP11 |
| `needs_writing_mode` | 90 | Future |
| `needs_image` | 70 | SP13 |
| `needs_gradient` | 70 | SP13 |
| `needs_advanced_selectors` | 48 | Future |
| `needs_complex_border` | 46 | Future paint quality |
| `needs_generated_content` | 35 | Future |

## Current Unported Inventory Ownership

The 4267 unported rows are SP12-scope Chromium WPT files that the current porter or
renderer cannot represent yet. They are still tracked with explicit dependency
categories.

Top unported categories:

| Category | Count | Owner |
|---|---:|---|
| `needs_text` | 4045 | SP11/SP13 |
| `needs_advanced_selectors` | 3867 | Future |
| `needs_javascript` | 1945 | Future harness/runtime |
| `needs_writing_mode` | 826 | Future |
| `sp13_fragmentation` | 654 | SP13 |
| `needs_font_metrics` | 553 | SP11 |
| `needs_table_layout` | 458 | Future |
| `needs_generated_content` | 456 | Future |
| `reference_test` | 455 | N/A |
| `needs_inline_block` | 424 | SP11 |
| `sp13_multicol` | 420 | SP13 |
| `needs_containment` | 325 | Future |

## Recommended Next Work

The best next phase is SP13 fragmentation and multicol.

Rationale:

- It owns the largest runnable failure blocks: `sp13_fragmentation` and
  `sp13_multicol`.
- It aligns with existing blocked todos around fragmented containing blocks, flex
  fragmentation, multicol balancing, and column-span behavior.
- It should convert many already-ported tests from fail/error to pass without first
  requiring a broader text or JavaScript harness implementation.

The biggest inventory unlock is SP11/text+inline, but that is a larger architecture
push because it affects thousands of unported rows and hundreds of runnable failures.

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
