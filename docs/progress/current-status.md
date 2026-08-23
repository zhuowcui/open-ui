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

Latest authoritative accountability snapshot (SP17 W1H full no-resume run):

| Metric | Value |
|---|---:|
| Chromium inventory rows | 7673 |
| Ported/runnable WPT tests | 3673 |
| Unported but explicitly tracked tests | 4000 |
| Runnable passes | 3374 |
| Runnable failures | 299 |
| Runnable render/diff errors | 0 |
| Generic `not_ported` bucket rows | 0 |
| Empty unported dependency rows | 0 |
| `sp12_layout_bug` rows | 0 |
| `needs_font_metrics` rows | 0 |
| Runnable `sp13_multicol` rows | 0 |
| Unported `sp13_multicol` residuals | 1002 |

`python3 tools/accountability/audit.py` passes all 7 checks for this snapshot.
The full `wpt/` run was executed without resume on 2026-08-22. Its committed
`summary.json` SHA-256 is
`ed78d9c2fd09c64a52c5de47ece6e7077a8fbab34e3eff6a12d6814ede1c6474`.
All 3267 frozen SP17 baseline IDs, including all 2823 frozen SP13-R baseline
IDs and all 351 runnable multicol targets, remain exact.

## SP17 W1H Logical Multicol and Vertical Fragmentation Closure

SP17 is active on `agent/sp17-advanced-text`. W0A freezes all 3,267 starting
exact IDs, the complete 842-row `needs_writing_mode` inventory, the 19 runnable
kickoff IDs, and their exact per-pixel results. At kickoff the inventory was 19
runnable plus 823 unported; 337 rows stopped directly on `writing-mode`, three
on `unicode-bidi`, and 483 first stopped elsewhere. Historical SP13-R through
SP16 ledgers are byte-pinned and unchanged.

The kickoff 19-ID no-resume run produced 19 expected functional failures and
zero errors. Its evidence remains immutable; live full-suite evidence now
supersedes the kickoff summary for current accountability.

W0B now accepts and transactionally computes the corpus-used SP17 declarations,
preserves importance/specificity/source-order conflicts between logical and
physical aliases, propagates inherited writing properties, and emits the
existing Rust style enums. A faithful deterministic-Ahem builder probe over all
823 kickoff-unported rows found 292 newly generatable rows and 531 actual
residuals. Together with the 19 kickoff-runnable IDs, the frozen actionable
ledger contains 311 rows. The two ledgers are sorted, disjoint, and cover all
842 original owner rows.

No generated Rust WPT module, runner profile, mapping row, or authoritative
result changed in W0B. The only frozen residual dispositions still owned by
SP17 are the `css-flexbox-test1` test/reference pair: their fullwidth digits are
absent from the pinned fonts and remain guarded as `text_non_ascii` until W2
adds a pinned glyph path. See `docs/SP17-PLAN.md` and the six
`tools/accountability/data/wpt_ported/sp17_*` artifacts.

W1A gives every `ConstraintSpace` an authoritative writing direction,
provides one-time physical-root and parent/child orthogonal size conversion,
and adds shared logical edge and computed-style projections. The production
render root derives its direction from computed style. A release no-resume run
of all 3,267 frozen exact IDs remained 3,267 exact at `0.0%` with zero errors,
and the full 3,566-ID summary was restored byte-identically afterward.

W1B adds shared child-space helpers and routes normal block children, floats,
atomic inline/block-in-inline children, and final flex-item layout through the
computed child writing direction. Flex now chooses and converts its logical
axes with the container writing direction, including wrapping, direction,
gaps, margins, placement, and final physical fragments. Overflowing
right/RTL-start inline alignment also preserves the aligned edge.

The first actionable admission,
`wpt/css_flexbox/flexbox-writing-mode-001`, is exact at zero mismatched pixels.
It is a horizontal-tb porter/cascade and shared-flex proof, not a vertical-text
closure claim. Four retained-text RTL gap reference builders were regenerated
to correct previously invalid logical-margin lowering; the complete `gap-00`
slice is 32/32 exact.

W1C fixes a shared direction-propagation defect in final flex item placement:
the resolved container direction now reaches logical-to-physical conversion
instead of falling back to the parent constraint direction. A parameterized
regression covers every flex-direction and wrap reversal under
horizontal-tb/RTL by original item identity. The sole surgical admission is
`wpt/css_flexbox/flexbox-writing-mode-004`, exact at zero mismatched pixels.

W1D covers vertical-rl/LTR, vertical-lr/LTR, vertical-rl/RTL, and
vertical-lr/RTL across all eight flex-direction/wrap combinations by original
CMYK item identity. It fixes physical flex-item size projection at the vertical
fragment boundary while preserving fragmentation-reduced horizontal sizes.
Only `flexbox-writing-mode-002`, 003, 005, and 006 were admitted; each is exact
at zero mismatched pixels and its existing reference builder is byte-unchanged.
W1E adds a single private flex-axis mapping boundary for container main/cross
and child logical inline/block sizes. Available sizes, percentage bases,
fixed/stretch flags, intrinsic measurements, aspect-ratio transfer, and final
physical fragments now use that mapping consistently, while horizontal
fragmentation-owned reductions remain authoritative. Fourteen orthogonal flex
targets covering writing modes, intrinsic and fit-content sizing, alignment,
wrapping, overflow padding, and aspect ratio were admitted in one transaction.
All are exact, and the existing 007–009 reference builders are byte-identical.

W1F converts normal block and vertical atomic-inline layout through logical
coordinates, extends flex content/intrinsic sizing through the same axis
mapping, and closes vertical row/column flow, wrapping, reverse flow, and all
seven logical gap patterns. Homogeneous Latin/Ahem runs in vertical mixed
orientation now shape horizontally and rotate their complete paint stack
clockwise. Upright CJK and general mixed-script splitting remain deferred.

The complete 44-ID cohort was spliced atomically and is 44/44 exact with zero
mismatched pixels or errors. Shared reference builders remain byte-identical,
the repeated splice is byte-idempotent, and writing-mode 010–016 were not
generated. The W1F validator required all 64 exact SP17 promotions.

W1G separates the containing-block, static-position parent, and abspos child's
complete writing directions. Physical insets, margins, and authored sizes keep
their physical semantics while intrinsic contributions, child constraints,
static anchors, and fragments cross explicit logical/physical boundaries.
Flex abspos static positions now use the flex main/cross mapping and the
padding-box containing block, including direction reversal and asymmetric
borders and padding.

The atomic 27-ID W1G cohort covers six flex abspos auto-position cases, three
vertical aspect-ratio transfers, twelve orthogonal intrinsic-sizing targets,
their four shared references, and the orthogonal over-constrained margin pair.
It is 27/27 exact with zero mismatched pixels or errors. The live validator now
required all 91 exact SP17 promotions.

W1H keeps multicol sizing, balancing, spanner placement, break progress, and
continuation geometry in logical inline/block coordinates, then projects the
container, columns, rules, in-flow fragments, decoration slices, baselines,
and overflow metadata through one shared finalizer. Child and relayout spaces
now use the complete computed writing direction; orthogonal children transpose
sizing inputs without incorrectly inheriting general fragmentation support.

Fragment paint records the fragmentation writing direction. Column and
overflow clips, first/interior/last decoration edges, border radii, background
sources, shadows, and ink overflow map to Y for horizontal-tb, left-origin X
for vertical-lr, and right-origin X for vertical-rl. Vertical inline
progression applies RTL independently. The same logical continuation boundary
now covers wrapping row flex, growing column flex, and break-before cases, and
vertical-rl float descendants no longer enlarge multicol scrollable overflow.

The atomic 16-ID W1H cohort spans eight css-break fragmentation targets, six
css-multicol sizing/scrolling targets, and two css-overflow float targets. The
18-ID proof, including the existing exact `borders-006-ref` and
`borders-007-ref`, is 18/18 exact with zero mismatched pixels or errors. The
live validator requires all 107 exact SP17 promotions, and writing-mode
010–016 remain absent.

The current full result is 3673 runnable, 3374 exact, 299 functional failures,
and zero errors. Live `needs_writing_mode` ownership is 735 rows; the frozen
842-row kickoff inventory and 311/531 disposition remain immutable. The text
manifest contains 798 IDs.

W1 remains incomplete for positioned-inline static positions, flex
safe-alignment abspos behavior, and fragmented/multicol out-of-flow layout.
Tables, images/print-specific cases, sideways modes, and extreme column-rule
geometry remain later work. W2 retains upright
CJK and mixed-script run splitting, sideways text, and writing-mode 010–015.

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

The 4000 unported rows are Chromium WPT files that the current porter or renderer cannot
represent yet. They are still tracked with explicit dependency categories.

Top unported categories:

| Category | Count |
|---|---:|
| `needs_javascript` | 1945 |
| `sp13_multicol` | 1002 |
| `needs_writing_mode` | 716 |
| `sp13_fragmentation` | 643 |
| `needs_table_layout` | 462 |
| `needs_generated_content` | 443 |
| `reference_test` | 421 |
| `needs_inline_block` | 408 |
| `needs_containment` | 335 |
| `needs_image` | 325 |
| `needs_advanced_selectors` | 306 |
| `needs_grid` | 304 |
| `needs_empty_block_margin_collapse` | 304 |
| `needs_form_controls` | 264 |

## Recommended Next Work

Continue SP17 W1 with positioned-inline and fragmented/multicol out-of-flow
behavior in their explicit later cohorts. Keep the neighboring flex-abspos,
table, image/print-only, and extreme column-rule cases out of the closed W1H
cohort. Reserve upright CJK and mixed-script
run splitting, sideways text, and `flexbox-writing-mode-010` through 015 for W2;
do not substitute geometry-only builders for their glyph requirements.
Keep all 3267 kickoff exact IDs green, splice surgically, do not batch-regenerate
the remaining actionable builders, and do not retire the detector. Follow
`docs/SP17-PLAN.md` for the exact boundary and recorded commands.

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
python3 tools/wpt/generate_sp17_closure.py --check
```

Audit:

```bash
python3 tools/accountability/audit.py
```

Focused WPT runs overwrite `summary.json`; snapshot it before running filtered
comparisons and restore it afterward unless the focused run is intentionally replacing
the authoritative summary.
