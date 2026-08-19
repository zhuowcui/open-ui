# SP17 — Advanced Text and Writing-Mode Parity

## Status

SP17 is in kickoff/planning on `agent/sp17-advanced-text`, branched from
`main` commit `2c1fe78c142c8b83896cd51b1b6d580496611b5e` on 2026-08-19.
No production or generated WPT code has changed yet.

The committed starting evidence passes the full local 7/7 audit, including PNG
proof. A new no-resume WPT run from this branch is still required before the
SP17 ledgers are frozen; the existing committed summary is evidence of the
landed state, not a substitute for that run.

| Starting metric | Value |
|---|---:|
| Chromium inventory | 7,673 |
| Runnable WPTs | 3,566 |
| Exact passes | 3,267 |
| Functional failures | 299 |
| Render/diff errors | 0 |
| Unported rows | 4,107 |
| `needs_writing_mode` inventory | 842 |
| Runnable `needs_writing_mode` | 19 |
| Unported `needs_writing_mode` | 823 |

Exact means zero mismatched pixels. SP17 must preserve all 3,267 starting exact
IDs, including the immutable 2,823-ID SP13-R baseline and all 351 exact SP13-R
targets. It must not change the SP16 real-font/Ahem/legacy runner precedence or
introduce an SP17-specific raster override.

## Frozen scope and operational partition

The original 842-row `needs_writing_mode` inventory is the accountability
boundary. At kickoff it partitions as follows:

| Partition | Rows | Meaning |
|---|---:|---|
| Already runnable | 19 | Initial comparison slice; one sole-owned, 18 co-owned |
| Direct `writing-mode` rejection | 337 | 255 style-block and 82 inline-declaration rejections |
| Direct `unicode-bidi` rejection | 3 | SP17 declarations hidden inside the handoff's 486-row remainder |
| A different first rejection | 483 | Cannot be made runnable by accepting SP17 properties alone |

This preserves the handoff's 337/486 split while identifying the operational
340-row SP17-property probe pool. All 90 sole-owned unported rows are in that
pool: 62 are `css_flexbox`, 28 are `css_sizing`, and all stop directly on
`writing-mode` (67 style-block, 23 inline). They are the first functional
implementation cohort after the parser and ledgers are trustworthy.

Freeze four sorted immutable artifacts after the new no-resume baseline run
and transactional probe:

1. `sp17_baseline_exact.json`: every exact starting ID, expected 3,267;
2. `sp17_writing_mode_inventory.json`: all original 842 rows and their kickoff
   state, Chromium path, first rejection, and complete owner set;
3. `sp17_actionable_targets.json`: the 19 starting runnable IDs plus every
   original inventory row that becomes runnable through transactional probing;
4. `sp17_residual_dispositions.json`: every still-unported original row with
   Chromium path, actual first rejection, and complete non-SP17 owners.

The actionable and residual IDs must be disjoint and cover exactly the frozen
842-row inventory. Actionable does not mean guaranteed exact: after SP17
behavior is corrected, a runnable row may remain a functional failure only if
its remaining pixels have specific, detector-backed non-SP17 owners.

## Kickoff source assessment

The existing foundations are real but mostly disconnected from production
geometry:

- Style already defines `WritingMode`, `Direction`, `UnicodeBidi`,
  `TextOrientation`, `FontOrientation`, text-combine, and emphasis values.
- Geometry already has logical sizes/offsets/rects and an exhaustively tested
  `WritingModeConverter`.
- Inline item construction injects bidi controls, runs UAX #9, splits runs by
  level, shapes each level in its visual direction, and reorders line items.
- Font descriptions derive `FontOrientation`, and paint has emphasis and
  text-combine helpers.

The production gaps determine the implementation order:

- `ConstraintSpace` has logical-looking inline/block sizes but no
  `WritingDirectionMode`. Its root, block-child, and flex-child constructors
  default implicitly to horizontal LTR. There are currently 92 direct
  constructor call sites in layout source.
- Production layout does not use `WritingModeConverter`. Block, inline,
  out-of-flow, fragmentation, and multicol code construct physical sizes and
  offsets directly; flex explicitly documents horizontal writing as its only
  mode.
- `OutOfFlowCandidate` stores horizontal `Direction` for static-position and
  containing-block decisions, not the complete writing direction needed for
  vertical axes and orthogonal containing blocks.
- The porter rejects `writing-mode` and `unicode-bidi`, emits only `direction`,
  and does not thread the complete SP17 inherited/computed property set.
- The porter currently lowers logical sizes, insets, margins, padding, and
  borders directly to horizontal physical fields. That destroys vertical
  semantics and cross-property cascade order before layout sees the style.
- `FontDescription.orientation` is populated but not consumed by shaping or
  glyph paint. Vertical and sideways advances/transforms are therefore helper
  APIs rather than an end-to-end path.

## Execution plan

### W0A — Reproduce and freeze the starting evidence

1. Build `pixel-compare` in release mode in the pinned environment.
2. Run the complete `wpt/` suite without resume, then run the unflagged 7/7
   audit. Stop on any drift from 3,566 runnable, 3,267 exact, 299 functional
   failures, zero errors, or 4,107 unported rows; investigate and document the
   delta rather than changing expected constants.
3. Snapshot the authoritative full summary, run the 19-ID initial slice without
   resume, and commit a sorted initial-result evidence artifact. Restore the
   full summary afterward so focused evidence never masquerades as the
   authoritative suite.
4. Add `generate_sp17_closure.py` and focused Python tests. Preserve historical
   SP14-SP16 and SP13-R ledger bytes by digest, and make generation/check mode
   idempotent.

Exit: the branch contains immutable proof of the exact baseline, original
inventory, and initial 19 outcomes without changing the runnable inventory.

### W0B — Transactional SP17 CSS and porter probing

1. Validate and emit the corpus-used values and CSS-wide keywords for
   `writing-mode`, `direction`, `unicode-bidi`, `text-orientation`,
   `text-combine-upright`, emphasis/decorations, and other target-required
   advanced-text properties. Invalid declarations must leave the prior valid
   declaration intact.
2. Materialize the correct computed inheritance boundary for every SP17
   property on elements, `display:contents`, anonymous text nodes, generated
   structural text, `html`, and body/viewport builders.
3. Resolve logical/physical property conflicts transactionally using the
   element's computed writing direction and declaration order. Remove the
   current unconditional horizontal lowering of logical sizes and sides.
4. Remove only `writing-mode` and `unicode-bidi` from porter rejection after
   unit coverage is green. Probe all 340 direct SP17-property rejections into a
   temporary output, recording the next real rejection when a row is still not
   portable.
5. Freeze the actionable and residual ledgers from probe results. Do not
   batch-regenerate historical WPT modules; use the surgical splice workflow
   and committed upstream fixtures for idempotence tests.

Exit: generated builders represent the source cascade faithfully, the frozen
842 rows have a disjoint reason-backed disposition, and non-SP17 builders plus
runner profiles are byte-stable.

### W1 — Authoritative logical geometry

Implement this as small horizontal-no-op steps so the 3,267 baseline is checked
after every boundary change.

1. Add the resolved writing direction to `ConstraintSpace` and its builder.
   Root layout derives it from the root computed style; child spaces explicitly
   carry the child's writing direction and convert orthogonal available and
   percentage sizes at the parent/child boundary.
2. Introduce shared internal helpers for resolved logical border/padding/margin,
   logical min/max sizes, and logical-to-physical fragment conversion. Keep
   `Fragment.size` and `Fragment.offset` physical and authoritative for paint.
3. Convert normal block flow and intrinsic sizing first, including orthogonal
   children, margin collapse, floats/clearance, overflow, fragmentation, and
   multicol handoff. Convert exactly once when a physical fragment is created.
4. Convert flex axis selection, item sizing, alignment, baselines, wrapping,
   gaps, and static positions using the container writing direction plus
   `flex-direction`; remove the horizontal-only axis assumption.
5. Carry complete containing-block and static-position writing directions
   through relative, sticky, absolute, fixed, inline containing-block, and
   fragmented out-of-flow layout.
6. Add vertical-rl, vertical-lr, sideways-rl, sideways-lr, RTL, nested
   orthogonal-flow, and round-trip tests at constraint, block, flex, inline,
   fragmentation, multicol, and positioned boundaries.

The first pixel cohort is the 90 sole-owned direct rows, with the 62 flex rows
ahead of the 28 sizing rows. A row is promoted only at 0.0%; otherwise continue
the shared implementation or assign a proven non-SP17 owner.

### W2 — Bidi and vertical text integration

1. Make the existing bidi analysis authoritative through line breaking,
   visual-run placement, atomic inlines, inline decorations, baselines, and
   positioned-inline static positions. Cover normal/embed/override/isolate/
   isolate-override/plaintext and inherited `direction` end to end.
2. Consume `FontOrientation` during shaping. Produce upright, mixed, and
   sideways run advances and glyph positions, including script segmentation,
   fallback fonts, complex scripts, and emoji.
3. Store the resolved run orientation/transform and decoration/emphasis
   geometry on text fragments. Paint consumes that metadata and physical
   fragment geometry; it does not infer layout axes again from CSS.
4. Integrate text-combine-upright, text orientation, decoration placement,
   emphasis marks, ruby interaction, and target-required transforms through the
   shared text path. Keep SP18 generated content, first-line/first-letter,
   counters/quotes, text shadow, and ellipsis outside this closure unless a
   frozen target needs only a narrow supporting primitive.

### W3 — Actionable ports and co-owner cleanup

Process targets in this order:

1. 90 sole-owned direct rows (62 flex, then 28 sizing);
2. remaining direct-property rows with co-owners, clustered by flex/sizing,
   positioned layout, fragmentation/multicol, inline-block, and paint;
3. the 19 starting runnable rows, with the sole-owned
   `auto-height-with-flex` first and the other 18 evaluated against their
   concrete co-owners;
4. rows whose first rejection is another feature only when that dependency has
   become representable through shared work already required by SP17.

Run exact-ID manifests without resume. Promote only exact results; preserve
specific non-SP17 ownership for remaining functional failures, and allow no
render/diff error.

### W4 — Accountability closure

1. Require all 3,267 starting exact IDs to remain exact and every SP17
   actionable row to have a zero-error exact or reason-backed functional
   disposition.
2. Require the frozen 842-row inventory to remain a complete disjoint cover and
   no runnable row to contain `needs_writing_mode`.
3. Run focused Rust style/text/layout/paint tests, all Python closure/porter
   tests, release `pixel-compare`, the exact-ID actionable manifest, and the
   complete WPT suite without resume.
4. Regenerate mapping, deferred data, reports, status, and documentation twice;
   require byte-identical artifacts, a clean full 7/7 audit, formatting, and
   `git diff --check`.
5. Update this plan's status/evidence, `docs/progress/current-status.md`, the
   sprint history, roadmap, CI test list, and a memory entry before handoff.

## Verification discipline

- Focused runs overwrite `summary.json`; always snapshot and restore the full
  authoritative summary unless intentionally replacing it.
- Use the pinned Chromium 147 binary and 800x600 environment. No per-test
  substitutions, pixel offsets, geometry exceptions, or raster overrides.
- Keep production fragment geometry physical for paint, while layout decisions
  are made in resolved logical coordinates.
- Add a focused regression before each behavior change, then run the relevant
  crate and porter suites before pixel comparison.
- Do not retire or weaken `needs_writing_mode` until the frozen inventory and
  all runnable ownership invariants pass in the audit.

## Progress log

### 2026-08-19 — kickoff

- Fetched `origin`, confirmed `main` at `2c1fe78c`, and created
  `agent/sp17-advanced-text` with a clean worktree.
- Found no repository `AGENTS.md`; the landed handoff and repository engineering
  docs are the active local instructions.
- Ran `audit.py --repository-only` and the full unflagged `audit.py`; both pass.
  The full audit verifies all 3,267 pass claims at 0.0% with PNG proof and all
  seven accountability checks.
- Recounted the mapping: 842 owned rows = 19 runnable + 823 unported; 337 stop
  on `writing-mode`, three on `unicode-bidi`, 483 on another first rejection;
  90 unported rows are sole-owned.
- Traced style, geometry, block, inline, flex, out-of-flow, text, paint, porter,
  runner, and detector entry points. The concrete integration gaps are recorded
  above.
- Next command sequence: release-build `pixel-compare`, run the complete WPT
  suite without resume, audit it, capture the 19-ID initial evidence, then
  implement the SP17 ledger generator/tests before changing porter or layout.
