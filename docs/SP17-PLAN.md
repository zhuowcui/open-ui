# SP17 — Advanced Text and Writing-Mode Parity

## Status

SP17 W1O is the current checkpoint on `agent/sp17-advanced-text`, branched
from `main` commit `2c1fe78c142c8b83896cd51b1b6d580496611b5e` on
2026-08-19.

The branch has reproduced the complete suite without resume, frozen the kickoff
ledgers and 19-ID pixel evidence, added transactional SP17 CSS handling, and
frozen a faithful no-write porter probe over all 823 kickoff-unported rows. The
W1A established the constraint/logical-geometry foundation and preserved the
complete 3,267-ID exact baseline. W1B routes normal block, atomic-inline,
and final flex-item child spaces through the shared writing-direction boundary,
uses logical flex axes through final physical fragment conversion, and admits
the first exact actionable target. W1C fixes final flex placement to consume
the resolved container direction, proves all horizontal-tb/RTL flex-flow
combinations, and admits its exact companion target. W1D closes the four
vertical-container companions by projecting resolved flex-item main/cross sizes
back to physical fragment width/height at the vertical writing boundary. W1E
centralizes container main/cross to child inline/block mapping, closes
orthogonal intrinsic, stretch, percentage, aspect-ratio, wrapping, and overflow
padding paths, and admits the next 14 exact targets. W1F closes the vertical
flex-flow, logical-gap, and vertical atomic-inline cohort, makes normal block
and intrinsic flex sizing consume logical axes, and adds the first production
clockwise rotated Latin/Ahem paint path for homogeneous mixed-orientation runs.
W1G closes the logical out-of-flow core by keeping containing-block,
static-position, and child writing directions distinct, transposing abspos
constraints and intrinsic inputs at explicit boundaries, and routing flex
static positions through the existing main/cross mapping. W1H closes logical
multicol sizing and projection, axis-aware fragmented decoration and clipping,
vertical float overflow propagation, and three vertical flex continuation
shapes. W1I closes positioned-inline static geometry in direct block flow and
vertical multicol, including first/last continuation containing blocks and
logical out-of-flow projection. W1J closes the six vertical multicol
positioned-fragmentation shapes through one logical source-interval mapper for
inline, block, and flex containing blocks. W1K closes safe overflow alignment
for flex abspos static positions through the existing content/item alignment
resolvers while retaining signed free space and one-time logical projection.
W1L closes absolute flex static-position centering by carrying physical-axis
edge affinity through bubbling, sizing, and fragmented reconstruction.
W1M closes the 39-ID flex abspos alignment matrix through assertion-only
check-layout admission, single logical/physical edge resolution, complete
margin-box alignment, and clearance-only anonymous-wrapper extent.
W1N closes four existing column-wrap failures through logical-inline
fit-content cross sizing and reuses the hypothetical cross size in final child
layout. W1O closes the remaining sole-owned auto-height flex failure by
preserving percentage flex bases and semantic break controls. The authoritative
live full-suite result is 3,746 runnable, 3,465 exact, 281
functional failures, and zero errors.

| Metric | Kickoff | Live W1O |
|---|---:|---:|
| Chromium inventory | 7,673 | 7,673 |
| Runnable WPTs | 3,566 | 3,746 |
| Exact passes | 3,267 | 3,465 |
| Functional failures | 299 | 281 |
| Render/diff errors | 0 | 0 |
| Unported rows | 4,107 | 3,927 |
| `needs_writing_mode` owner rows | 842 | 671 |
| Frozen W0B actionable targets | 311 | 311 |
| Frozen W0B residual dispositions | 531 | 531 |

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
`writing-mode` (67 style-block, 23 inline). W0B found that 85 are generatable;
five reveal image dependencies and remain residuals. Together with the one
sole-owned kickoff-runnable row, the first functional cohort is therefore 86.

W0A freezes four sorted immutable kickoff artifacts:

1. `sp17_baseline_exact.json`: every exact starting ID, expected 3,267;
2. `sp17_writing_mode_inventory.json`: all original 842 rows and their kickoff
   state, Chromium path, first rejection, and complete owner set;
3. `sp17_initial_runnable_targets.json`: the sorted 19-ID kickoff manifest; and
4. `sp17_initial_runnable_results.json`: the exact per-pixel outcome and pinned
   runner provenance from its no-resume focused run.

W0B adds `sp17_actionable_targets.json` and
`sp17_residual_dispositions.json`. The actionable ledger contains the 19
starting runnable IDs plus 292 kickoff-unported rows that the real deterministic
Ahem builder can generate. The residual ledger preserves the other 531 rows'
Chromium paths, actual first rejections, rejection owners, and complete owner
sets. These ledgers are sorted, disjoint, and cover exactly the frozen 842-row
inventory.

Actionable does not mean guaranteed exact: after SP17
behavior is corrected, a runnable row may remain a functional failure only if
its remaining pixels have specific, detector-backed non-SP17 owners.

Two residuals intentionally retain SP17 ownership:
`wpt/css_flexbox/css-flexbox-test1` and its `-ref`. Both contain fullwidth digits
that neither pinned Ahem nor vendored DejaVu Sans provides, so the deterministic
font guard rejects them as `text_non_ascii`. W2 must supersede that guard with a
pinned glyph/fallback path before either row can move. The probe does not strip
text or depend on ambient fonts to make them appear portable.

The 311 actionable rows are distributed as follows:

| Area | Rows |
|---|---:|
| `css2_floats` | 1 |
| `css_backgrounds` | 3 |
| `css_break` | 51 |
| `css_flexbox` | 145 |
| `css_multicol` | 11 |
| `css_overflow` | 22 |
| `css_position` | 35 |
| `css_sizing` | 43 |

The largest residual first-rejection groups are 310 JavaScript rows, 49 image
rows, 25 `contain` rows, 20 transform rows, 19 canvas rows, 14 table-display
rows, and 12 `border-spacing` rows. The ledger, rather than this abbreviated
table, is authoritative.

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

Completed on 2026-08-19; see the progress log and immutable artifacts below.

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

Completed on 2026-08-19; see the progress log and frozen W0B artifacts below.

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
   unit coverage is green. Probe all 823 kickoff-unported inventory rows in
   memory with the real deterministic-Ahem builder, recording the next real
   rejection when a row is still not portable. This includes and verifies the
   complete 340-row direct-property pool rather than assuming all other kickoff
   rejections remain current.
5. Freeze the actionable and residual ledgers from probe results. Do not
   batch-regenerate historical WPT modules; use the surgical splice workflow
   and committed upstream fixtures for idempotence tests.

Exit met: prospective generated builders represent the source cascade
faithfully, the frozen 842 rows have a disjoint reason-backed disposition, and
historical builders plus runner profiles remain byte-stable. The two explicit
font-guard residuals above remain SP17-owned until W2 provides their glyphs.

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

The first pixel cohort is the 86 sole-SP17 actionable rows: the original
runnable `auto-height-with-flex` case plus 85 of the 90 kickoff-unported sole
owners. Five kickoff sole owners reveal image dependencies and remain in the
residual ledger. Process the flex-heavy portion before sizing. A row is
promoted only at 0.0%; otherwise continue the shared implementation or assign a
proven non-SP17 owner.

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

1. 86 sole-SP17 actionable rows (the original runnable case plus 85 newly
   generatable rows), flex first and then sizing;
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

### 2026-08-19 — W0A evidence frozen

- Built release `pixel-compare` successfully, then ran all 3,566 WPTs without
  resume in the pinned Chromium 147 / 800x600 environment. The result exactly
  reproduced 3,267 passes, 299 functional failures, and zero errors.
- Ran the unflagged audit after the full suite and again after focused evidence
  capture/restoration. Both runs passed 7/7 and verified PNG proof for all 3,267
  exact IDs. The restored full `summary.json` SHA-256 is
  `67d50eb1ee54fcf7121df5c1f186d2468a24b2f0310303606e0a689e4d2581a4`.
- Ran the sorted 19-ID initial manifest without resume: 0 pass, 19 functional
  failures, zero errors. Every mismatch reproduced the full-run value, from the
  16-pixel `flex-basis-011-ref` result through the 15,000-pixel flex-wrap cases.
- Added `generate_sp17_closure.py` with generate, check, and focused-capture
  modes. It freezes 3,267 baseline IDs, all 842 original owner rows, the 19-ID
  manifest, and structured per-pixel evidence. It also pins every historical
  SP13-R/SP14/SP15/SP16 ledger by SHA-256 and rejects inventory/count drift.
- Added six focused tests for ledger shape and disjointness, the 19/823 and
  337/3 partitions, exact initial pixel evidence, byte-idempotent generation,
  and historical-ledger compatibility. Hosted accountability CI now runs the
  tests and the SP17 generator check.
- Frozen artifact SHA-256 values:
  - baseline: `59a514d3b76b83ecc44efd43dda5a16ec2a0203803849407f3dce035d9e9fc20`;
  - inventory: `b72a0b0b4e74f4c1cb912ab65642dd6f5bef76a570f0a9b219f68729de6d10ad`;
  - initial targets: `f82d99182bf276ddd524b2894e2de6b21e8f1f5bf7d8f3bd984c8ef0e2c040c4`;
  - initial results: `1fcc02a0e742df04cec80e06750f74e6859ef11552d5488300c590095432d00d`.
- W0B starts in `tools/wpt/port_wpt.py`: add transactional parsing and cascade
  tests before changing rejection policy, then probe exactly the frozen 340
  direct-property rows into temporary output. Do not alter committed builders,
  mapping ownership, or the detector until the probe produces a reviewed
  disjoint actionable/residual disposition.

### 2026-08-19 — W0B transactional probe frozen

- Added transactional parsing for all corpus-used values and CSS-wide keywords
  of `writing-mode`, `direction`, `unicode-bidi`, `text-orientation`, and
  `text-combine-upright`. Invalid declarations preserve the prior valid value;
  shorthand/longhand, specificity, source order, inline declarations,
  `!important`, and `dir` presentational hints retain cascade priority.
- Materialized the inherited writing properties across elements,
  `display:contents`, anonymous text, generated structural text, `html`, and
  body builders. `unicode-bidi` correctly remains non-inherited.
- Added late logical-to-physical resolution for corpus-used sizes, insets,
  margins, padding, border sides/components, and logical corner radii. It uses
  the computed writing mode and direction and keeps logical/physical aliases in
  their true cascade order. This is explicitly a porter boundary until W1 makes
  logical geometry authoritative in layout.
- Removed only `writing-mode` and `unicode-bidi` from porter rejection and ran
  the actual deterministic-Ahem builder path over all 823 kickoff-unported
  rows. It generated 292 and recorded 531 reason-backed residuals. An atomic
  in-memory splice preparation for all 292 succeeded across 23 prospective
  files, retained text in every builder, and made no repository writes.
- Froze 311 actionable IDs (19 kickoff runnable plus 292 newly generatable) and
  531 residual dispositions. The ledgers are an exact disjoint cover of all
  842 kickoff rows. Their SHA-256 values are
  `9d2b53070cf206b6c37a5c66bd0d37ea757e12b7ca6d4a19a5eb98ee4579f96c`
  and
  `314a7a27f250ef1fb5f65b86e69f48771e5116a4b6592bce2190d474a9338776`.
- Preserved the deterministic font guard. The only residuals still carrying
  `needs_writing_mode` are the fullwidth-digit flex test/reference pair, both
  rejected as `text_non_ascii`; W2 owns the pinned fallback-font solution.
- Added eleven W0B ledger and transactional CSS tests (17 SP17 tests total).
  The combined SP13-R/SP14/SP15/SP16/SP17 closure/porter suite passes all 117 tests,
  both closure generators check cleanly, and historical builder idempotence is
  unchanged.
- Next: W1 begins with `WritingDirectionMode` in `ConstraintSpace`, explicit
  parent/child orthogonal conversion, and horizontal-no-op Rust tests. Do not
  splice the 292 builders before that shared geometry is ready.

### 2026-08-19 — W1A logical-geometry foundation

- Added an authoritative `WritingDirectionMode` to `ConstraintSpace`. Legacy
  root/block/flex constructors remain horizontal-LTR; explicit root and child
  constructors convert physical viewport or parent-logical size pairs exactly
  once when the axes are orthogonal. `ConstraintSpaceBuilder` inherits the
  direction and exposes a tested parent-to-child conversion boundary.
- Root paint/render layout now derives the constraint direction from the root's
  computed `writing-mode` plus `direction`; the pixel comparator's diagnostic
  layout path uses the same entry point.
- Added `LogicalBoxStrut` with physical/logical edge conversion and exhaustive
  round trips across horizontal LTR/RTL, vertical-rl/lr, and sideways flags.
  Added `ResolvedLogicalBox` as the shared computed-style view for logical
  min/preferred/max sizes, margins, padding, insets, used borders, and final
  logical-to-physical fragment sizes.
- The new APIs have focused unit coverage in geometry, constraint, logical
  style projection, and paint-root integration. Existing writing-mode tests,
  geometry tests, and compile checks remain green.
- Rebuilt `pixel-compare` in release mode and ran the immutable 3,267-ID
  baseline manifest without resume: 3,267 exact at `0.0%`, zero failures, and
  zero errors. Restored the authoritative full summary byte-for-byte at
  SHA-256
  `67d50eb1ee54fcf7121df5c1f186d2468a24b2f0310303606e0a689e4d2581a4`.
- This checkpoint is plumbing, not a vertical-layout completion claim.
  Production normal-block, flex, fragmentation, multicol, and out-of-flow
  child construction still needs to consume the explicit child direction and
  `ResolvedLogicalBox`, then convert completed fragments to physical geometry
  once. No actionable builders have been spliced yet.

### 2026-08-19 — W1B shared child geometry and first exact promotion

- Added shared block- and flex-child constraint helpers that derive the child's
  computed writing direction and transpose both available and percentage size
  pairs exactly once. Normal-flow block children, floats, atomic inline flex
  children, block-in-inline children, and final flex-item layout now use this
  boundary.
- Converted the flex container's axis selection, logical size constraints,
  gaps, margins, wrapping, alignment, child placement, final fragment size, and
  containing-block size to the container writing direction. Direction and
  block-flow flips are applied by `WritingModeConverter`; only authored
  `*-reverse` values reverse logical item order. Focused tests cover vertical-lr,
  vertical-rl, RTL inline starts, row wrapping, and gap/reference geometry.
- Fixed overflowing right/RTL-start inline alignment so the aligned edge is
  preserved and overflow extends toward the logical end. This was required for
  retained-text RTL flex gap references and is covered by direct inline tests.
- Surgically admitted `wpt/css_flexbox/flexbox-writing-mode-001`. It is an
  authored `horizontal-tb` case and proves the SP17 porter/cascade plus shared
  flex path without claiming vertical text closure. Its 800x600 result has zero
  mismatched pixels. The live full no-resume run is **3,567 total / 3,268 exact /
  299 fail / 0 errors**, and all 3,267 frozen baseline IDs remain exact.
- Regenerated four already-runnable RTL reference builders
  (`gap-001-rtl-ref`, `gap-003-rtl-ref`, `gap-006-rtl`, and
  `gap-006-rtl-ref`) because the old generated forms encoded logical margins
  in the wrong physical direction or omitted retained text. The current porter
  output and runtime fixes make the full `gap-00` slice 32/32 exact. This is a
  standards-based builder correction, not a test-specific geometry exception.
- Strengthened the live SP17 check and historical SP13-R through SP16 audit
  validators so immutable ledgers remain byte-pinned while later promotions
  are accepted only when their mapping/template/summary identity is complete,
  the result is exact at `0.0%`, and the applicable historical runnable floor
  is preserved. SP13-R's 1,018 residual rows remain strictly unported and
  ownership-identical.
- The five-builder splice is byte-idempotent. Mapping, deferred CSV, and HTML
  report generation were each run twice with identical hashes. The combined
  closure/porter suite passes 122 tests, the full style/text/layout/paint Rust
  matrix passes, release `pixel-compare` builds, both ledger checks pass, and
  the unflagged PNG-backed audit is clean 7/7. The live summary SHA-256 is
  `6f99e956b4a3eed7a2c4427c429ccff54baaeb7fa5af3f161c81354ae80c5df4`.
- W1 is not complete. Normal block layout still makes substantial physical-axis
  decisions; flex intrinsic/content-based and aspect-ratio branches need an
  orthogonal audit; out-of-flow/static-position propagation, fragmentation,
  multicol, and vertical/sideways glyph shaping and paint remain open. The next
  lowest-risk admission probe is the horizontal-RTL companion
  `wpt/css_flexbox/flexbox-writing-mode-004`; then use 002/003/005/006 and
  007/008 to drive true vertical and mixed orthogonal geometry. Promote none of
  them without a focused no-resume `0.0%` result and baseline preservation.

### 2026-08-19 — W1C horizontal RTL flex-flow closure

- Added a parameterized Rust regression for the eight combinations of
  row/row-reverse/column/column-reverse with wrap/wrap-reverse under
  horizontal-tb/RTL. It asserts physical offsets by original cyan, magenta,
  yellow, and black item identity, so ordering and mirroring cannot cancel out.
- Diagnosed the failure in shared geometry: flex resolved the container's
  writing direction for axis construction, but final item placement converted
  logical offsets with the parent `ConstraintSpace` direction. Final placement
  now receives the already-resolved container `WritingDirectionMode` directly.
  No per-test offsets, substitutions, raster changes, or vertical behavior were
  added.
- Surgically admitted only
  `wpt/css_flexbox/flexbox-writing-mode-004`. Its exact-ID no-resume run is
  1/1 exact with zero mismatched pixels and zero errors; repeating the splice
  produces byte-identical builder, template, report, and manifest artifacts.
- Rebuilt release `pixel-compare`, then ran the frozen 3,267-ID baseline without
  resume: 3,267/3,267 exact. The authoritative complete no-resume run is
  **3,568 runnable / 3,269 exact / 299 fail / 0 errors**. The full summary
  SHA-256 is
  `50c8a52b36c479357f9e89b5e8ea520608a69974d04616d36a7099639d25f0d1`.
- Mapping, deferred CSV, and HTML report generation were run twice with
  byte-identical hashes. The live inventory is 4,105 unported rows and 840
  `needs_writing_mode` rows. Every kickoff and historical ledger remains
  byte-pinned, and the SP17 live check now requires both 001 and 004 in the
  exact actionable promotion set.
- Verification passes: 178 focused flex/logical-writing Rust tests, the full
  locked style/text/layout/paint matrix, 122 SP13-R through SP17 Python tests,
  both closure-generator checks, release comparator build, surgical-splice
  idempotence, `cargo fmt --check`, and the unflagged PNG-backed audit 7/7.
- W1 remains open. Drive genuine vertical and orthogonal geometry next in this
  order: 002, 003, 005, 006, then 007 and 008. Flex intrinsic/aspect-ratio
  auditing, out-of-flow/static positions, fragmentation, multicol, and vertical
  glyph shaping/paint remain outside W1C.

### 2026-08-20 — W1D vertical-container flex-flow matrix closure

- Added one parameterized regression for vertical-rl/LTR, vertical-lr/LTR,
  vertical-rl/RTL, and vertical-lr/RTL across all eight flex-direction and
  wrap-reversal combinations. It asserts physical offsets and 20×15 physical
  item sizes by original cyan, magenta, yellow, and black identity.
- Diagnosed a shared fragment-boundary defect: vertical flex resolved logical
  main/cross sizes correctly, but the provisional child fragment retained the
  transposed fixed child-space pair. Non-horizontal flex placement now projects
  the resolved main/cross border-box sizes into physical width/height before
  logical placement. Horizontal fragment sizes remain untouched so
  fragmentation-reduced block sizes are preserved.
- Surgically admitted only `flexbox-writing-mode-002`, 003, 005, and 006 in one
  transaction. Their exact-ID no-resume run is 4/4 exact with zero mismatched
  pixels and zero errors. All four already-exact reference builders remain
  byte-identical, repeating the splice is byte-idempotent, and no 007/008
  builder was generated.
- Rebuilt release `pixel-compare`, then ran the frozen 3,267-ID baseline without
  resume: 3,267/3,267 exact. The authoritative complete no-resume run is
  **3,572 runnable / 3,273 exact / 299 fail / 0 errors**. Its `summary.json`
  SHA-256 is
  `2c041fc33000a20ea899bf2be6bf53354dd10530a072acaa35fbf29d9aecf88d`.
- The live inventory is 4,101 unported rows, 836 `needs_writing_mode` rows, and
  697 text-manifest IDs. The SP17 live check now requires exact actionable
  promotions 001–006 while every kickoff and historical ledger remains
  byte-pinned.
- Mapping, deferred CSV, and HTML report generation were run twice with
  byte-identical hashes. Verification passes 179 focused flex/logical-writing
  Rust tests, the full locked style/text/layout/paint matrix, all 122 SP13-R
  through SP17 Python tests, both closure-generator checks, formatter and diff
  checks, release comparator, splice/reference idempotence, and the unflagged
  PNG-backed audit 7/7.
- W1 remains open. Drive orthogonal-child sizing next with 007, then 008. Flex
  intrinsic/aspect-ratio auditing, out-of-flow/static positions, fragmentation,
  multicol, and vertical glyph shaping/paint remain outside W1D.

### 2026-08-20 — W1E orthogonal flex-item sizing closure

- Added a private `FlexItemAxisMapping` that converts container main/cross
  sizes to each child's logical inline/block axes once. Available sizes,
  percentage bases, fixed/stretch flags, intrinsic measurement spaces,
  aspect-ratio transfer, and final physical fragment projection now share that
  mapping. Horizontal fragments remain child-layout-owned so fragmentation
  reductions are not overwritten.
- Added parameterized Rust coverage for horizontal-tb, vertical-lr, and
  vertical-rl containers with horizontal and vertical children; orthogonal
  center/stretch alignment and percentage padding; min/max/fit-content;
  border-box aspect-ratio transfer; wrapped flexing; and overflow padding.
- Surgically admitted these 14 targets in one transaction:
  `flexbox-writing-mode-007` through 009,
  `aspect-ratio-intrinsic-size-009`, `fit-content-item-002` through 004,
  `flex-item-min-width-min-content`, `flex-item-max-width-min-content`,
  `flexbox_align-items-center-3`, `flexbox_align-items-stretch-3`,
  `stretching-orthogonal-flows`, `flexbox-flex-wrap-flexing-003`, and
  `flexbox-overflow-padding-002`. Their no-resume manifest is 14/14 exact with
  zero mismatched pixels and errors. The 007–009 reference builder spans remain
  byte-identical, the splice is byte-idempotent, and no 010–016 builder exists.
- The frozen 3,267-ID baseline remains 3,267/3,267 exact. The authoritative
  complete no-resume run is **3,586 runnable / 3,287 exact / 299 fail / 0
  errors**. Its `summary.json` SHA-256 is
  `bb87ab04fdc9fad9b4c6cd935ed4220ffd2fc413bec62ff123a9eede5ab6429d`.
- The live inventory is 4,087 unported rows, 822 `needs_writing_mode` rows, and
  711 text-manifest IDs. The live validator requires all six earlier 001–006
  promotions plus the complete 14-ID W1E cohort while every frozen kickoff and
  historical ledger remains byte-pinned.
- Mapping, deferred CSV, and HTML report generation were run twice with
  byte-identical hashes. Verification passes the focused flex/logical-writing
  and fragmentation regressions, full locked style/text/layout/paint matrix,
  all 122 SP13-R through SP17 Python tests, both closure-generator checks,
  formatter and diff checks, release comparator, splice/reference idempotence,
  and the unflagged PNG-backed audit 7/7.
- W1 remains open for the remaining vertical flex families, followed by
  out-of-flow/static positions, fragmentation, and multicol. Keep
  `flexbox-writing-mode-010` through 015 in W2 for vertical-text shaping and
  paint; do not generate them as geometry-only substitutes.

### 2026-08-22 — W1F vertical flex flow, gap, and atomic-inline closure

- Added private logical-axis boundaries for normal block layout, atomic inline
  layout, flex content/intrinsic sizing, gaps, wrapping, reverse flow, child
  constraints, and final physical fragments. Explicit physical min/max
  keywords remain resolved at the physical-property boundary, while automatic
  flex minima use the child's logical main axis.
- Vertical atomic inline layout now measures and positions fragments in the
  parent's logical axes before one writing-mode projection. Homogeneous
  Latin/Ahem runs in vertical mixed orientation are shaped horizontally and
  their complete paint stack is rotated clockwise; upright CJK and general
  mixed-script splitting remain W2 work.
- Added regressions for vertical-lr/vertical-rl flex direction and wrapping,
  logical gaps, atomic-inline transposition and shrink-to-fit sizing, logical
  intrinsic contributions, rotated Ahem classification/painting, and the
  column-wrap intrinsic crash case.
- Surgically admitted the complete 44-ID W1F cohort in one transaction: six
  `css-flexbox-row*` variants; `flex-direction-row-vertical` and its reference;
  five `flexbox-flex-direction-*` targets; two `flexbox-flex-wrap-*` targets;
  `gap-001` through `gap-007` in lr/rl target/reference forms; and
  `intrinsic-size_col-wrap-crash`. The focused no-resume result is 44/44 exact
  with zero mismatched pixels and zero errors. Shared references are reused,
  the splice is byte-idempotent, and no writing-mode 010–016 builder exists.
- The frozen 3,267-ID kickoff baseline remains 3,267/3,267 exact. The
  authoritative complete no-resume run is **3,630 runnable / 3,331 exact / 299
  fail / 0 errors**. Its `summary.json` SHA-256 is
  `76b70d2d6b32e5899de6b03a76df28c1111643f0307475e386cbbb822bc00a55`.
- The live inventory is 4,043 unported rows, 778 `needs_writing_mode` owner
  rows, and 755 text-manifest IDs. The W1F validator required all 64 exact
  SP17 promotions while every frozen kickoff and historical ledger remains
  byte-pinned.
- Mapping, deferred CSV, and HTML report generation were run twice with
  byte-identical hashes. Verification passes the full locked
  style/text/layout/paint matrix, all 122 SP13-R through SP17 Python tests,
  both closure-generator checks, formatter and diff checks, release comparator,
  splice/reference idempotence, and the unflagged PNG-backed audit 7/7.
- Continue W1 with out-of-flow/static-position, fragmentation, and multicol
  logical geometry. W2 owns upright CJK and mixed-script run splitting,
  sideways modes, and writing-mode 010–015.

### 2026-08-22 — W1G logical out-of-flow core closure

- Added a private out-of-flow axis mapping without changing
  `OutOfFlowCandidate` or any public API. The containing block,
  static-position parent, and child keep distinct complete writing directions;
  physical sizes, insets, and margins remain physical while intrinsic inputs,
  child constraints, static anchors, and fragments transpose at explicit
  logical/physical boundaries.
- Physical start/end polarity now follows horizontal-tb LTR/RTL, vertical-lr
  versus vertical-rl block flow, and vertical inline progression plus RTL.
  Child available sizes, percentage bases, fixed flags, clamped relayout, and
  aspect-ratio inputs enter the abspos child's writing mode before block layout.
- Flex abspos static positioning now consumes the existing flex main/cross
  mapping, uses the padding-box containing block, respects reversal and
  asymmetric borders/padding, and projects its final start-edge anchor exactly
  once.
- Added parameterized regressions for the six writing-mode/direction physical
  polarities, flex static positions, vertical min/max/fit-content sizing,
  physical auto and over-constrained margins, vertical aspect-ratio transfer,
  percentage descendants, child constraint transposition and clamped relayout,
  and final physical fragments. A baseline-discovered vertical negative-margin
  regression is independently pinned to Chromium's symmetric behavior.
- Surgically admitted the complete 27-ID W1G cohort in one transaction: six
  flex abspos auto-position cases; three vertical abspos aspect-ratio cases;
  twelve orthogonal min/max/fit-content sizing targets and their four shared
  references; and the orthogonal over-constrained margin target/reference pair.
  The focused result is 27/27 exact with zero mismatched pixels or errors.
- The frozen 3,267-ID kickoff baseline remains 3,267/3,267 exact. The
  authoritative complete no-resume run is **3,657 runnable / 3,358 exact / 299
  fail / 0 errors**. Its `summary.json` SHA-256 is
  `3974f4cbda275e0f8a63ea5b2aedf589c611e2dac600f39b6ffa193abb8c056c`.
- The live inventory is 4,016 unported rows, 751 `needs_writing_mode` owner
  rows, and 782 text-manifest IDs. The live validator requires all 91 exact
  SP17 promotions while every frozen kickoff and historical ledger remains
  byte-pinned.
- Mapping, deferred CSV, and HTML report generation were run twice with
  byte-identical hashes. Verification passes the full locked
  style/text/layout/paint matrix, all 122 SP13-R through SP17 Python tests,
  both closure-generator checks, formatter and diff checks, release comparator,
  27-ID splice idempotence, the writing-mode 010–016 exclusion, and the
  unflagged PNG-backed audit 7/7.
- Continue W1 with fragmentation and multicol logical geometry. Positioned
  inline static positions, flex safe-alignment abspos behavior,
  fragmented/multicol out-of-flow layout, upright/mixed vertical text,
  sideways modes, and writing-mode 010–015 remain later scoped work.

### 2026-08-22 — W1H logical multicol and vertical fragmentation closure

- Added one private multicol axis mapping and one shared finalizer. Column
  resolution, balancing, spanners, break progress, flex continuations, and
  in-flow geometry remain logical until the final physical projection;
  positioned fragments retain their existing physical contract.
- Child, probe, spanner, flex, and balance-relayout spaces consume complete
  computed writing directions. Available sizes, percentage bases, intrinsic
  contributions, fixed/stretch flags, and fragmentainer capacity transpose at
  the parent/child boundary; orthogonal children do not acquire general
  fragmentation support.
- Extended fragments with optional fragmentation writing-direction metadata.
  Paint maps clips, decoration slices, physical border suppression,
  backgrounds, shadows, radii, and ink overflow to horizontal Y,
  vertical-lr X-from-left, or vertical-rl X-from-right, with independent RTL
  inline progression.
- Normalized flex continuation inputs and vertical in-flow descendant extents
  into the multicol logical block axis. Authored overflow clips now reject
  negative block-start ink owned by a later fragment, and direct or nested
  vertical-rl floats do not enlarge the multicol scrollable-overflow union.
- Surgically admitted the complete 16-ID W1H cohort in one transaction. The
  focused proof is 18/18 exact, including existing `borders-006-ref` and
  `borders-007-ref`, with zero mismatched pixels or errors. Repeat splicing is
  byte-identical and writing-mode 010–016 remain absent.
- The frozen 3,267-ID kickoff baseline remains 3,267/3,267 exact. The
  authoritative complete no-resume run is **3,673 runnable / 3,374 exact / 299
  fail / 0 errors**. Its committed `summary.json` SHA-256 is
  `ed78d9c2fd09c64a52c5de47ece6e7077a8fbab34e3eff6a12d6814ede1c6474`.
  The live inventory is 4,000 unported rows, 735 `needs_writing_mode` rows,
  and 798 text-manifest IDs; the validator requires exactly 107 promotions
  without changing frozen ledgers.
- Verification covers parameterized layout/paint regressions, the locked Rust
  matrix, all 122 closure/porter tests, both closure-generator checks, double
  deterministic accountability generation, splice idempotence, formatter and
  diff checks, the unflagged audit at 7/7, and one local checkpoint commit.
- Keep positioned-inline containing blocks, fragmented/multicol out-of-flow
  layout, neighboring flex-abspos cases, tables, images/print-only cases,
  sideways modes, material vertical text/bidi, extreme column-rule geometry,
  and writing-mode 010–016 in later scoped work.

### 2026-08-23 — W1I positioned-inline static geometry closure

- Added one private positioned-inline geometry path shared by both inline
  layout entry points. Static anchors and first/last continuation containing
  blocks stay in logical inline/block coordinates until their owning block or
  multicol boundary, with direction-aware continuation affinity and synthetic
  empty-continuation filtering.
- Text indent, asymmetric edges, relative inline translation, atomic-inline
  bubbling, and block-in-inline interruption now contribute exactly once.
  Logical-to-physical out-of-flow projection carries each candidate's static
  anchor, containing-block offset, and containing-block size together while
  retaining the inline containing-block node, direction, and zero-border
  contract.
- Vertical multicol maps the positioned inline's first and last endpoints
  through the W1H column index/remainder and vertical-lr/vertical-rl projection.
  The physical containing block is constructed before out-of-flow layout, and
  the returned positioned fragment remains outside multicol's final in-flow
  projection with single descendant ownership.
- Surgically admitted the complete 30-ID cohort: five direct-flow
  `static-position_*` cases, five `static-position_*-in-multicol` cases, and
  five `*-in-multicols` cases for each of vertical-lr and vertical-rl. The two
  existing horizontal-tb failures `static-position_htb-rtl-ltr.tentative` and
  `static-position_htb-rtl-rtl` were repaired by the same shared path. The
  required 35-ID family proof is 35/35 exact with zero mismatched pixels or
  errors.
- The frozen 3,267-ID kickoff baseline remains 3,267/3,267 exact. The
  authoritative complete no-resume run is **3,703 runnable / 3,406 exact / 297
  fail / 0 errors**. Its committed `summary.json` SHA-256 is
  `13a7c9181f86b213a81a183eb164d0d05d1bd03911850bdc22d57e42c65cf416`.
  The live inventory is 3,970 unported rows, 703 `needs_writing_mode` rows,
  and 828 text-manifest IDs; the validator requires exactly 139 promotions
  without changing frozen ledgers.
- All 30 builders dry-run deterministically, two transactional splices are
  byte-identical, and the SP13-R later-promotion allowlist now recognizes the
  20 admitted multicol-position IDs without modifying its frozen ledger.
  Mapping, deferred CSV/plan, and HTML generation are byte-identical across
  two runs.
- Verification covers the 35-ID proof, three frozen-regression guards, the
  full release pixel suite, parameterized layout regressions, the locked Rust
  matrix, all 122 closure/porter tests, both closure-generator checks,
  formatter and diff checks, the writing-mode 010–016 exclusion, and the
  unflagged PNG-backed audit at 7/7.
- Keep the six true out-of-flow multicol fragmentation cases, fragmented
  abspos boxes, seven safe-alignment flex cases, absolute centering, tables,
  transforms, generated content, images/print cases, and tolerance/reference
  changes outside W1I. W2 retains authoritative mixed-script bidi, upright and
  sideways glyph work, fallback shaping, and writing-mode 010–016.

### 2026-08-23 — W1J vertical multicol out-of-flow fragmentation closure

- Added one private logical positioned-fragment record containing the source
  block interval, static anchor, containing-block offset and size, visual
  translation, resolved logical insets and margins, and writing direction.
  Continuations are the non-empty intersections with column-flow intervals;
  each retains its authoritative source offset, first/last flags,
  fragmentainer index, block-axis clip, and decoration slice before one W1H
  physical projection.
- Reconstructed block-in-inline relative translations now cross the physical
  to logical vector boundary once. Positioned descendants remain physical at
  their completed layout boundary, retain source-local children, and have one
  owning continuation. The same mapper covers relative inline containing
  blocks (including RTL boundary affinity), fragmented positioned blocks, and
  fragmented flex containing blocks with percentage inline sizing and
  asymmetric logical borders.
- Surgically admitted only `out-of-flow-in-multicolumn-063`, 064, 066, 067,
  118, and 119. The required 17-ID proof adds exact guards 001, 050, 057, 062,
  117, and 121–126 and is 17/17 exact with zero mismatched pixels or errors.
  Two no-write porter runs and the repeated transactional splice are
  byte-identical.
- The frozen 3,267-ID kickoff baseline remains exact. The authoritative
  complete no-resume run is **3,709 runnable / 3,412 exact / 297 fail / 0
  errors**. The live inventory is 3,964 unported rows, 697
  `needs_writing_mode` rows, 976 unported `sp13_multicol` rows, and 834
  text-manifest IDs. The live validator requires exactly 145 promotions; the
  SP13-R later-promotion allowlist contains 42 IDs without changing frozen
  ledgers. The committed `summary.json` SHA-256 is
  `be9c87dcb549fd3566b288749cd278e8430c6a2ad2cf8ec560d6996b2de996b1`.
- Verification covers the 17-ID proof, parameterized vertical-lr/vertical-rl
  × LTR/RTL fixed and stretched geometry, the complete no-resume release
  suite, the full locked Rust matrix, all 122 closure/porter tests, both
  closure-generator checks, deterministic mapping/deferred/HTML generation,
  formatter and diff checks, the unflagged audit at 7/7, and one local
  checkpoint commit.
- Keep safe-alignment flex abspos, `position-absolute-center-*`, transforms,
  filters, containment, JavaScript, generated content, images/print, and all
  other out-of-flow multicol cases outside W1J. W2 retains authoritative bidi,
  upright and sideways glyph work, fallback shaping, and writing-mode 010–016.

### 2026-08-23 — W1K safe flex overflow alignment closure

- Refactored the private flex abspos static-position calculation to retain
  signed main/cross free space and complete overflow-alignment values. The
  existing content-alignment resolver now owns main-axis placement and the
  existing item-alignment resolver owns cross-axis placement, including
  `align-self:auto` inheritance, safe fallback, unsafe signed offsets, reverse
  flow, and wrap reversal before one logical-to-physical projection.
- Kept the child margin box authoritative for alignment, the flex padding box
  authoritative for the containing/static rectangle, and the out-of-flow
  constraint solver authoritative for applying margins once. No public style,
  fragment, candidate, constraint-space, or layout API changed.
- Surgically admitted the three `flex-abspos-staticpos-align-self-safe-*`
  test/reference pairs and `flexbox-safe-overflow-position-005`. The required
  18-ID proof includes safe overflow 001–004 plus all six abspos auto-position
  writing-mode/direction guards and is 18/18 exact with zero mismatched pixels
  or errors. Two no-write generations and two transactional splices are
  byte-identical.
- Parameterized Rust regressions cover row, row-reverse, column, and
  column-reverse under horizontal-tb, vertical-lr, and vertical-rl with LTR and
  RTL. They prove oversized safe center fallback, signed default/unsafe center,
  fitting safe end, margin/border/padding geometry, `align-items` overflow
  inheritance, wrap-reverse safe flex-start, physical projection, and single
  margin application.
- The frozen 3,267-ID kickoff baseline remains exact. The authoritative
  complete no-resume run is **3,716 runnable / 3,419 exact / 297 fail / 0
  errors**. The live inventory is 3,957 unported rows, 690
  `needs_writing_mode` rows, 976 unported `sp13_multicol` rows, and 841
  text-manifest IDs. The live validator requires exactly 152 promotions; the
  SP13-R later-promotion allowlist remains 42 IDs without changing frozen
  ledgers. The committed `summary.json` SHA-256 is
  `a3a9be11bd242c330e9de765bbbb7917df4f95ea714c3672f97f6df5567f10df`.
- Verification covers the 18-ID proof, the complete no-resume release suite,
  the full locked Rust matrix, all 122 closure/porter tests, both
  closure-generator checks, deterministic mapping/deferred/HTML generation,
  splice idempotence, formatter and diff checks, writing-mode 010–016 exclusion,
  the unflagged audit at 7/7, and one local checkpoint commit.
- Keep `position-absolute-center-*`, JavaScript-backed abspos alignment,
  `flexbox-safe-overflow-position-006`, existing `flexbox-align-self-vert-*`
  failures, transforms, tables, generated content, and unrelated paint work
  outside W1K. W2 retains authoritative bidi, upright and sideways glyph work,
  fallback shaping, and writing-mode 010–016.

### 2026-08-24 — W1L absolute flex static-position centering closure

- Extended the public out-of-flow candidate carrier with horizontal and
  vertical start/center/end static-edge affinity and retained it in positioned
  fragmentation metadata. Flex now emits padding-box edge or center anchors;
  safe overflow falls back to start before the single writing-mode/direction
  projection.
- The generic out-of-flow solver derives the available interval from the
  retained edge. Start grows forward, end grows backward, and center grows
  symmetrically to the nearest containing-block edge. Known-size and
  shrink-to-fit boxes align their complete margin box, and auto physical height
  is recentered after vertical layout supplies its final size.
- Fragmented column flex reconstructs center/end margin-box starts before
  projecting unfragmented block flow into columns. This preserves the exact
  `flexbox_flex-container-fragmentation-010` and 011 guards without a
  test-specific branch.
- Surgically admitted only `position-absolute-center-002`; the shared fix also
  promotes existing runnable 001. The 17-ID proof covers center 001–004, all
  seven W1K targets, and six writing-mode/direction abspos auto-position guards.
  It is 17/17 exact with zero mismatched pixels or errors. Both porter dry-runs
  and both transactional splices are byte-identical.
- The frozen 3,267-ID baseline remains exact. The authoritative full suite is
  **3,717 runnable / 3,421 exact / 296 fail / 0 errors**; live unported
  inventory is 3,956, writing-mode ownership is 688, unported SP13-R multicol
  ownership is 976, the text manifest is 842, and the live validator requires
  exactly 154 promotions. The SP13-R later-promotion compatibility set remains
  42. The committed `summary.json` SHA-256 is
  `365354dae47ca97f6370a2dcd4edc3e69286de0866ee7e9169e8f7ee3e4f1cd5`.
- Verification covers the 17-ID proof, parameterized edge/sizing/reversal/
  bubbling/fragmentation regressions, the complete no-resume release suite,
  locked Rust matrix, all 122 closure/porter tests, both ledger checks,
  deterministic mapping/deferred/HTML generation, splice idempotence,
  formatter and diff checks, writing-mode 010–016 exclusion, and audit 7/7.
- Keep fallback/justify-self/JavaScript/table/transform/generated-content
  cases, `flexbox-safe-overflow-position-006`, and unrelated paint/layout work
  outside W1L. W2 retains material bidi and vertical glyph shaping.

### 2026-08-24 — W1M flex abspos alignment matrix closure

- Admitted exactly 29 static assertion-only layouts whose only scripting is
  the ordered `testharness.js`, `testharnessreport.js`, and
  `check-layout-th.js` harness plus one body `checkLayout(...)` hook. Inline
  code, unknown scripts, mixed handlers, and dynamic alignment remain hard
  porter rejections. Quoted `>` selectors are stripped without leaking body
  attributes into the deterministic comparison template.
- Flex abspos static positioning now resolves distribution fallbacks,
  physical left/right, logical and flex edges, reverse flow, wrap reversal,
  writing mode, and direction once. `align-self:auto`, baseline variants,
  safe overflow, ignored `justify-self`, auto margins, and specified complete
  margin-box alignment retain the W1K/W1L contracts.
- Mixed block/inline flow carries a clearing break's anonymous-wrapper extent
  to its parent exactly once. The clearing line stays zero-height while the
  wrapper advances by the greater of its computed clearance and strut; later
  no-op clearing breaks retain their normal line height.
- The atomic cohort is the 29 newly runnable IDs plus ten existing
  fallback/justify-self/margin test-reference failures. The 58-ID proof adds
  all 17 W1L guards and flex fragmentation 010/011 and is 58/58 exact with
  zero mismatched pixels or errors. Both no-write generations and both
  transactional splices are byte-identical.
- The frozen 3,267-ID baseline remains exact. The authoritative full suite is
  **3,746 runnable / 3,460 exact / 286 fail / 0 errors**; live unported
  inventory is 3,927, writing-mode ownership is 676, unported SP13-R multicol
  ownership is 976, the text manifest is 871, and the validator requires
  exactly 166 SP17 promotions. The SP13-R later-promotion set remains 42. The
  committed `summary.json` SHA-256 is `94574e79d0c0f5bbf979e57c6168e62aab35e56fed62f5977c2f3ed4784817df`.
- Verification covers the exact 58-ID proof, the complete no-resume release
  suite, parameterized float-clearance and flex-edge regressions, the locked
  Rust matrix, all 125 closure/porter tests, both closure generators,
  deterministic mapping/deferred/HTML generation, formatting and diff checks,
  writing-mode 010–016 exclusion, audit 7/7, and one local checkpoint commit.
- Keep dynamic JavaScript and mutation cases, tables, grid, transforms,
  generated content, `flexbox-safe-overflow-position-006`, and unrelated W2
  glyph work outside W1M.

### 2026-08-26 — W1N column-wrap fit-content cross sizing closure

- Generalized the private flex cross-size path so items whose cross axis maps
  to their logical inline axis use fit-content for horizontal and vertical
  children. Available container cross space is reduced by specified margins,
  auto margins remain zero, and indefinite space selects max-content.
- Border/padding and cross min/max constraints are applied once while computing
  the hypothetical line contribution. Final child layout reuses that resolved
  border-box size; row-flex items whose cross axis maps to child block size stay
  on the existing layout-based intrinsic path.
- The four target failures are exact, and the 19-ID proof covering nearby
  wrapping, writing-mode, gap, and fragmentation guards is 19/19 exact with
  zero mismatched pixels or errors.
- The frozen 3,267-ID kickoff baseline and W1M cohort remain exact. The full
  no-resume result is **3,746 runnable / 3,464 exact / 282 fail / 0 errors**;
  unported inventory is 3,927, live writing-mode ownership is 672, unported
  SP13-R multicol ownership is 976, and the text manifest stays at 871. The
  validator requires exactly 170 promotions and keeps the SP13-R compatibility
  allowlist at 42. The committed `summary.json` SHA-256 is
  `3fa96c5a653460785bbec889ece0aa6bbadcdf410fb7edacd8d2c9c194105263`.
- Verification covers the release focused proof, complete no-resume suite,
  parameterized Rust regressions, locked Rust and Python matrices, both closure
  generators, deterministic accountability generation, formatting, 010–016
  exclusion, audit 7/7, and one local checkpoint commit.

### 2026-08-26 — W1O auto-height flex basis and semantic break closure

- Flex shorthand lowering now preserves the computed `0%` basis for omitted
  one- and two-value bases and an accepted unitless third zero. Explicit
  `0px`, `0%`, `auto`, and other bases remain distinct.
- Percentage flex bases, including zero, resolve only against definite main
  space. Indefinite percentage bases use content sizing without allowing a
  specified main-size property to replace the content basis; fixed zero stays
  definite.
- Retained `<br>` elements are semantic break controls with inherited font and
  writing metrics. Their strut establishes each forced line without a
  synthetic pre-line text node or phantom trailing line.
- Surgically regenerated only `auto-height-with-flex`. The one-ID target and
  15-ID proof are exact with zero mismatched pixels or errors; two no-write
  generations and two splices are byte-identical, and the 871-ID text manifest
  remains unchanged.
- The frozen 3,267-ID baseline and every W1N cohort remain exact. The full
  no-resume result is **3,746 runnable / 3,465 exact / 281 fail / 0 errors**;
  unported inventory is 3,927, live writing-mode ownership is 671, and the
  validator requires exactly 171 promotions. The SP13-R compatibility
  allowlist remains 42. The committed `summary.json` SHA-256 is
  `d182ff44328df0a6c711990830a62a3e9e6199d2c13bad583360efd68702b2a9`.
- Verification covers the focused proof, complete no-resume suite, flex-basis
  and break regressions, locked Rust and 128-test Python matrices, both closure
  generators, deterministic mapping/deferred/HTML output, formatting, splice
  idempotence, writing-mode 010–016 exclusion, and audit 7/7.
