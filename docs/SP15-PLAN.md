# SP15 — Inline/Layout and Root/Body Closure

## Status

SP15 is complete. All 130 originally SP15-owned mapping rows are covered by immutable
ledgers: 76 actionable tests and 54 reason-owned unported residuals. The authoritative
WPT snapshot contains 3,566 runnable tests, 2,804 exact passes, 762 named-owner
failures, 4,107 unported rows, and zero render/diff errors. All 2,767 frozen baseline
IDs remain exact.

| Ledger/scope | Count | Exact | Functional owner | Errors |
|---|---:|---:|---:|---:|
| Frozen exact baseline | 2,767 | 2,767 | — | 0 |
| Actionable SP15 targets | 76 | 34 | 42 | 0 |
| Root/body promotions within targets | 49 | included above | included above | 0 |
| Unported residual dispositions | 54 | — | 54 | — |

Exact means zero mismatched pixels. The 42 remaining actionable mismatches are owned
by concrete non-SP15 systems such as font metrics, multicol/fragmentation, absolute
flex positioning, gradients/images/shadows, complex root painting, special background
clips, and mixed inline/block layout.

## Implemented behavior

- Added an opt-in root-aware document path with distinct viewport, `html`, and `body`
  nodes while preserving the existing `base_doc()` structure for earlier builders.
- Preserved root/body rules, inline styles, inheritance, display state, and direct text
  in both Rust builders and Chromium templates.
- Implemented eligible root/body canvas-background propagation, source-box suppression,
  body overflow transfer/reset, root precedence, and hidden/unboxed suppression.
- Replaced synthetic clearing-break height with semantic forced-break clearance against
  floats, including the fragmentation path.
- Materialized decorated inline boxes as real per-line continuation fragments with
  first/last metadata and slice/clone margin, border, padding, background, intrinsic,
  forced-break, and soft-wrap behavior.
- Preserved computed inheritance through `display:contents` ancestors and retained
  renderable `style` element content when CSS changes its display state.
- Extended the transactional splice workflow to promote root-aware builders, templates,
  text-manifest entries, and porter reports atomically and idempotently.

## Accountability closure

The frozen ledgers are:

- `sp15_baseline_exact.json`: 2,767 sorted unique IDs.
- `sp15_actionable_targets.json`: 76 sorted unique IDs.
- `sp15_residual_dispositions.json`: 54 sorted structured records with Chromium path,
  actual porter rejection, and functional non-metadata owners.

The actionable and residual ledgers are disjoint and cover all 130 original SP15 rows.
Historical SP14 W4 data remains unchanged; the audit recognizes its 49 promoted entries
and the 54 residual supersessions. All five global SP15 categories are retired:

- `needs_inline_box_decoration_break`
- `needs_clearing_break_after_floats`
- `needs_display_contents_style_element`
- `needs_display_contents_list_layout`
- `needs_root_body_layout`

No mapping row retains a retired category, generic fallback, blank unported owner, or
metadata-only runnable failure.

## Acceptance evidence

- Focused 76-ID run: 34 exact, 42 functional failures, zero errors.
- Full `wpt/` run without resume: 2,804/3,566 exact, 762 fail, zero errors.
- Frozen baseline preservation: 2,767/2,767 remain exact.
- Template, summary, Rust registry, and function sets all contain the same 3,566 IDs.
- Deterministic text manifest: 496 tests, 128 exact and 368 functionally owned failures.
- Python SP14/SP15/porter suite: 54/54.
- DOM/layout/paint Rust suites pass under `--no-fail-fast`.
- Release `pixel-compare` build succeeds.
- The 76-ID post-closure splice is byte-identical on rerun.
- Two accountability regeneration passes are byte-for-byte identical.
- `tools/accountability/audit.py` passes all 7 checks.

## Next handoff

SP16 owns real-font metrics and raster parity. The paused SP13 multicol and fragmentation
clusters remain separately and explicitly owned, as do JavaScript, writing modes, grid,
tables, containment, generated content, images, and paint-quality systems.
