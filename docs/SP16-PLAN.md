# SP16 — Real-Font Metrics and Raster Parity

## Status

SP16 is complete. Its immutable ledgers cover all 776 original
`needs_font_metrics` rows: 226 runnable targets and 550 reason-owned unported
residuals. The authoritative WPT snapshot contains 3,566 runnable tests, 2,823
exact passes, 743 functionally owned failures, 4,107 unported rows, and zero
render/diff errors. All 2,804 frozen baseline IDs remain exact.

| Ledger/scope | Count | Exact | Functional owner | Errors |
|---|---:|---:|---:|---:|
| Frozen exact baseline | 2,804 | 2,804 | — | 0 |
| Actionable SP16 targets | 226 | 19 | 207 | 0 |
| Sole-functional-owner targets | 20 | 5 | 15 | 0 |
| Unported residual dispositions | 550 | — | 550 | — |

Exact means zero mismatched pixels. Remaining actionable mismatches have concrete
non-font owners, including multicol/fragmentation, float line rewind, flex intrinsic
sizing and baselines, paint order, inline-block layout, images, and gradients.

## Implemented behavior

- Vendored DejaVu Sans, Sans Mono, and Serif regular/bold faces with their license
  and SHA-256 records. CSS generic families resolve deterministically to these files;
  explicitly authored Ahem remains isolated.
- Added shared primary-font metrics and font-relative length resolution for `ch`,
  `ex`, and `lh`, plus used line height for normal, numeric, percentage, and fixed
  values. Layout and paint now consume the same font description and line metrics.
- Added legacy box-only, deterministic Ahem, and real-font porter profiles. The
  real-font profile preserves author font family/shorthand/inheritance, resolves
  font-relative lengths after computed font properties, and retains text according
  to the unchanged 496-ID text manifest.
- Implemented transactional parsing of the corpus-used `font` shorthand, including
  style, variant, weight, stretch, size, optional line height, family lists, resets,
  `inherit`, and `initial`. Unsupported forms reject without partially changing a
  generated module.
- Added a sorted 226-ID real-font manifest with runner precedence real-font, Ahem,
  then legacy. The real-font path uses the vendored Fontconfig policy and the pinned
  Chromium 147 FreeType runtime with RGB horizontal LCD rendering, subpixel
  positioning, slight hinting, and autohinting disabled.
- Retired `needs_font_metrics` from the shared detector registry. Four narrow
  functional owners record newly exposed float-line-rewind, flex intrinsic-size,
  multiline-baseline, and flex paint-order gaps.

## Accountability closure

The frozen ledgers are:

- `sp16_baseline_exact.json`: 2,804 sorted unique IDs.
- `sp16_actionable_targets.json`: 226 sorted unique IDs.
- `sp16_residual_dispositions.json`: 550 sorted structured records with original
  Chromium path, actual porter rejection, and functional non-font owners.
- `sp16_real_font_tests.json`: the sorted 226-ID runner/profile manifest.

The actionable and residual ledgers are disjoint and cover all 776 original rows.
Historical SP14 and SP15 ledgers are unchanged; the audit recognizes SP16
supersession while enforcing their preserved baselines. No mapping row contains
`needs_font_metrics`, a generic fallback, an empty unported owner, or a
metadata-only runnable failure.

## Acceptance evidence

- Focused 20-ID sole-owner run: 5 exact, 15 functional failures, zero errors.
- Focused 226-ID run without resume: 19 exact, 207 functional failures, zero errors.
- Full `wpt/` run without resume: 2,823/3,566 exact, 743 fail, zero errors.
- Frozen baseline preservation: 2,804/2,804 remain exact.
- Template, summary, Rust registry, and function sets contain the same 3,566 IDs.
- The unchanged deterministic text manifest contains 496 IDs.
- Release `pixel-compare` build and relevant Python/Rust suites pass.
- Two accountability regeneration passes are byte-for-byte identical.
- `tools/accountability/audit.py` passes all 7 checks.

## Next handoff

SP17 owns advanced text such as bidi, vertical writing, and complex scripts. The
paused SP13 multicol/fragmentation clusters and the separately named JavaScript,
grid, table, image, generated-content, and paint-quality systems remain outside
SP16 ownership.
