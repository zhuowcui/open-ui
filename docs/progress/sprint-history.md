# Open UI — Sprint Progress Record

## Sprint Overview

| Sprint | Title | Tests | Review Rounds | Status |
|--------|-------|-------|---------------|--------|
| SP1 | Research & Infrastructure | — | — | ✅ Complete |
| SP2 | Skia Extraction | — | — | ✅ Complete (deprecated) |
| SP3 | Rendering Pipeline | 20 | — | ✅ Complete |
| SP4 | DOM Adapter & C API | 130 | — | ✅ Complete |
| SP5 | Offscreen Rendering | 196 | — | ✅ Complete |
| SP6 | Widget Coverage & SVG | 39 pages | — | ✅ Complete |
| SP7 | Events & Animations | — | — | ✅ Complete |
| SP8 | React-like Rust API | 100 | — | ✅ Complete |
| SP9 | Native Rendering Foundation | 617+ | 3 | ✅ Complete |
| SP10 | Full CSS Flexbox | 617 | — | ✅ Complete |
| SP11 | Text & Inline Layout | 1,902 | 31 | ✅ Complete |
| SP11.5 | Full Chromium Text Parity | 3,371 | 6 | ✅ Complete |
| SP12 | CSS Block/Layout WPT Accountability | 7,673 inventory rows | multi-wave | ✅ Complete by ownership |
| SP14 | Deterministic Text Porting | 4,045 owner rows | W0–W4 | ✅ Complete by ownership |
| SP15 | Inline/Layout + Root/Body Closure | 130 owner rows | closure | ✅ Complete by ownership |
| SP16 | Real-Font Metrics + Raster Parity | 776 owner rows | closure | ✅ Complete by ownership |
| SP17 | Advanced Text + Writing Modes | 842 frozen owner rows | W1M | 🟡 Active; flex abspos alignment matrix closure, 166 exact promotions |

**Current accountability snapshot: 7,673 SP12-scope Chromium WPT inventory rows, 3,746 ported/runnable tests, 3,460 runnable passes, 286 functional failures, 0 errors, 0 `sp12_layout_bug` rows, 842 frozen SP17 kickoff owner rows, and 676 live writing-mode owner rows.**

---

## SP1: Research & Infrastructure

**Goal**: Investigate Chromium's architecture, set up build system, verify compilation.

**What we did**:
- Analyzed Chromium's rendering pipeline architecture
- Set up sparse Chromium checkout (only rendering-relevant directories)
- Configured GN build system for our targets
- Verified Chromium compilation with our integration points
- Compared upstream Skia vs Chromium's embedded Skia

**Key decision**: Use Chromium's embedded Skia (not upstream) because Chromium patches Skia
for performance and correctness in the rendering pipeline.

---

## SP2: Skia Extraction (Deprecated)

**Goal**: Extract Skia as a standalone library with C API.

**What happened**: Completed a standalone Skia wrapper, but later realized we needed the full
Blink integration (not just Skia). SP3 replaced this approach with direct Blink pipeline
integration. The standalone Skia work was deprecated but informed our understanding.

**Lesson**: Don't extract layers in isolation — understand the full pipeline first.

---

## SP3: Rendering Pipeline Integration

**Goal**: Integrate Blink's style→layout→paint pipeline.

**What we did**:
- Integrated `DummyPageHolder` for headless Blink rendering
- Wired up style computation → layout tree → paint artifacts
- Created first pixel-accurate renders
- 20 tests passing

**Key insight**: Blink's rendering pipeline is tightly coupled internally but has clean
boundaries at the API level. `DummyPageHolder` provides the minimal surface needed.

---

## SP4: DOM Adapter & C API

**Goal**: Create a stable C ABI wrapping the Blink rendering pipeline.

**What we did**:
- Designed 65-function C API (`include/openui/openui.h`)
- Document creation, element manipulation, style setting, layout, rendering
- Comprehensive error handling and resource management
- 130 tests passing

**Key decision**: C ABI as the integration boundary. This makes the library usable from
any language with C FFI, while keeping Blink's C++ internals completely hidden.

---

## SP5: Offscreen Rendering

**Goal**: Rasterize Blink's paint output to pixels and PNG files.

**What we did**:
- Implemented `oui_render_to_pixels` and `oui_render_to_png`
- 14 pixel-perfect test pages comparing our output vs headless Chromium
- Established pixel comparison testing methodology
- 196 tests passing

**Methodology established**: Render the same content through both our API and headless
Chromium. Compare pixel-by-pixel at 0% tolerance. Any difference is a bug.

---

## SP6: Widget Coverage & SVG

**Goal**: Support all standard HTML elements and SVG rendering.

**What we did**:
- Added 117 HTML elements to the rendering pipeline
- Implemented SVG shape rendering (rect, circle, ellipse, line, polygon, path)
- Advanced SVG (gradients, filters, clip-path, masks, text paths)
- Resource provider for images and external resources
- 39 pixel-perfect test pages
- 15 element test sheets + 10 rich website integration tests + 14 core pages

**Scale milestone**: First time we validated complex, real-world layouts (e-commerce,
dashboard, blog, documentation site) against Chromium.

---

## SP7: Events & Animations

**Goal**: Event handling, CSS animations, and interactivity.

**What we did**:
- Implemented event dispatch system (click, hover, keyboard, etc.)
- CSS animations and transitions
- Hit-testing for interactive elements
- Animation frame timing

---

## SP8: React-like Rust API

**Goal**: Provide an ergonomic Rust developer experience with reactive primitives.

**What we did**:
- `view!` proc macro for JSX-like UI declaration
- `#[component]` attribute for reusable components
- Reactive runtime: `create_signal`, `create_memo`, `create_effect`
- Scope-based resource management
- `App` shell with render loop
- 100 Rust tests, 99.1% pixel match to Chromium

**Pixel comparison (10 web apps built identically in HTML and `view!` macro)**:
- Framework vs Web (headless Chromium): **99.11% average**
- Remaining differences: text anti-aliasing between DummyPageHolder and full compositor

---

## SP9: Native Rendering Foundation

**Goal**: Build a pure-Rust rendering engine foundation — geometry, style, DOM, layout, paint.

**What we did**:
- `openui-geometry`: LayoutUnit (fixed-point arithmetic), logical/physical types, writing modes
- `openui-style`: CSS property system, computed values, cascade
- `openui-dom`: Lightweight DOM tree for layout
- Basic block and flex layout algorithms
- Skia-based paint backend
- 3 rounds of dual-model review, 617+ tests

**Architecture shift**: From wrapping Chromium's C++ to porting algorithms into pure Rust.
This gives us control, portability, and eliminates the Chromium build dependency for users.

---

## SP10: Full CSS Flexbox Layout

**Goal**: Complete CSS Flexible Box Layout Level 1 implementation.

**What we did**:
- Ported Chromium's `FlexLayoutAlgorithm` to Rust (3,221 LOC)
- All flex properties: direction, wrap, grow/shrink/basis, alignment, gap, order
- Definite/indefinite main size handling
- Min/max constraint interaction
- 617 tests passing

---

## SP11: Text & Inline Layout

**Goal**: Full Chromium text rendering parity — fonts, shaping, line breaking, inline layout.

**What we did**:
- HarfBuzz text shaping via Skia's SkShaper
- Unicode BiDi algorithm (UAX #9)
- Line breaking (UAX #14) with CSS `line-break` property support
- Inline formatting context: line height, vertical-align, text-align
- Text painting: glyphs, decorations (underline, overline, line-through), shadows
- Font variant properties with OpenType feature mapping
- CSS hyphenation, text-emphasis, text-combine-upright
- Writing modes (horizontal-tb, vertical-rl, vertical-lr)
- Ruby annotation layout
- Color font and emoji rendering
- **31 rounds of dual-model review, 150 issues fixed**
- 1,902 tests

---

## SP11.5: Full Chromium Text Parity

**Goal**: Close remaining gaps to 100% Chromium text parity.

**What we did**:
- Locale-aware text-transform (Blink's CaseMap equivalent)
- Font variant ligatures, numeric, caps, east-asian
- Comprehensive WPT-equivalent text test suite (724 new tests)
- Performance optimization (HashSet for O(1) lookup in line breaking)
- 6 rounds of dual-model review
- 3,371 cumulative tests

---

## SP12: CSS Block/Layout WPT Accountability (COMPLETE BY OWNERSHIP)

**Goal**: eliminate all current SP12-owned fixable residuals and make every remaining
SP12-scope WPT row accountable to an explicit owner.

### Verified status

| Metric | Value |
|---|---:|
| Chromium SP12-scope inventory rows | 7673 |
| Ported/runnable WPT tests | 3517 |
| Unported but explicitly tracked rows | 4156 |
| Runnable passes | 2767 |
| Runnable failures | 750 |
| Runnable render/diff errors | 0 |
| Generic `not_ported` bucket rows | 0 |
| `sp12_layout_bug` rows | 0 |
| `needs_text` rows | 0 |

`tools/accountability/audit.py` passes all checks for this state.

### What changed during the SP12 accountability push

- Built and used a full WPT accountability pipeline:
  - generated Rust WPT document builders,
  - generated Chromium HTML templates,
  - per-test OpenUI/Chromium/diff PNGs,
  - full `summary.json`,
  - full inventory `wpt_mapping.csv`,
  - deferred dependency CSV and plan.
- Fixed the final SP12-owned rounded background/border antialiasing residuals.
- Fixed the final rounded overflow clip-margin residuals:
  - `overflow-clip-margin-010` and ref,
  - `overflow-clip-margin-visual-box-and-value-with-border-radius` and ref.
- Tightened tracking so unported rows are no longer hidden in a generic
  `not_ported` bucket. Every unported row now has at least one explicit dependency
  category.
- Tightened `audit.py` so future generic/unclassified unported rows are audit
  failures.

### What remains outside SP12 ownership

The SP12-scope directories still contain non-passing and unported tests. They are not
classified as SP12-owned layout bugs. Top owners include:

- SP13 fragmentation and multicol,
- SP15 inline layout and root/body viewport propagation,
- SP11 font metrics,
- future JavaScript/test harness support,
- future advanced selectors, writing modes, table/grid layout, generated content,
  form controls, canvas/SVG, and paint-quality features.

See `docs/progress/current-status.md` and `docs/SP12.5-PLAN.md` for current counts.

---

## Cumulative Statistics

| Metric | Value |
|--------|-------|
| Current SP12-scope inventory | 7,673 Chromium WPT rows |
| Current runnable WPT tests | 3,717 |
| Current runnable WPT passes | 3,421 |
| Current SP12-owned layout bugs | 0 |
| Generic unported bucket rows | 0 |
| Pixel comparison tests | 3,717 generated WPT comparisons + earlier SP pages/apps |
| Dual-model review rounds | 55+ (31 SP11 + 6 SP11.5 + 18 SP12) |
| Total review findings | 250+ |
| Total real fixes from review | 230+ |
| CSS features implemented | Block, Flex, Inline, Text, Ruby |
| Chromium version | M147 (147.0.7727.50) |

---

## SP13 (partial) + Accountability Restore + Text Pivot

### What happened

- Continued SP13 fragmentation/multicol. Net layout state advanced to **2671 pass / 735
  fail / 0 errors**, `audit.py` 7/7 (commits `b66e0df`, `4780f71`).
  - `b66e0df`: restored a broken/stale `summary.json` left by an earlier commit (audit was
    failing 7/7) by re-running the full WPT suite and regenerating artifacts.
  - `4780f71`: SP13 fix — extended multicol overflow columns for abspos descendant overflow
    (`out-of-flow-in-multicolumn-002`, `-082`), zero regressions.
- SP13 has ~48 hard, heterogeneous fragmentation/multicol residuals remaining; documented
  per-cluster for later resumption.

### Decision: pause layout, pivot to text (SP14+)

Text is the largest single unlock (`needs_text` 4045 unported + 336 runnable;
`needs_font_metrics` 230/553). The SP11 text engine, inline layout, and glyph painter already
exist — the gap is that the WPT porting tool emits box-only builders, so text is never
compared against Chromium. The text track (SP14–SP18) is a porting + parity effort, starting
with the deterministic Ahem subset.

See `docs/plan/10-text-rendering-parity.md` (roadmap) and `docs/SP14-PLAN.md` (first SP).

### SP14 W3/W4 complete: global text accountability closed

- Froze a 2,715-ID exact baseline, a 111-ID deterministic W3 ledger, and a structured
  3,934-row W4 rejection/ownership ledger. W3 and W4 are a disjoint cover of all 4,045
  original unported `needs_text` rows.
- Added the 111 W3 tests transactionally across existing divergent modules: 52 exact,
  59 functional non-text failures, and zero render/diff errors.
- Retired the global `text_rendering`/`needs_text` category only after every W4 row had
  merged upstream-detector and actual-rejection ownership.
- Authoritative full state: **2,767 pass / 750 fail / 0 errors** across 3,517 runnable
  tests; all 2,715 frozen baseline IDs remain exact; audit passes 7/7.
- This handoff led to SP15 inline/layout and root/body propagation; the remaining text
  roadmap continues with SP16 real-font metrics, SP17 advanced text, and SP18 generated
  content/text effects.

### SP15 complete: inline/layout and root/body ownership closed

- Froze a 2,767-ID exact baseline, a 76-ID actionable ledger, and 54 structured
  unported residual dispositions covering all 130 original SP15 owner rows.
- Promoted all 49 deterministic root/body tests. Across all 76 actionable tests,
  34 are exact, 42 retain precise non-SP15 functional owners, and none error.
- Implemented real decorated-inline continuation fragments, semantic clearing breaks,
  `display:contents` inheritance/style handling, and root/body canvas/overflow propagation.
- Retired all five SP15 categories after proving complete coverage and preserved the
  immutable historical SP14 W4 ledger through explicit supersession rules.
- Authoritative full state: **2,804 pass / 762 fail / 0 errors** across 3,566 runnable
  tests; all 2,767 frozen baseline IDs remain exact; audit passes 7/7.
- Next: SP16 real-font metrics, or resume the explicitly owned SP13 multicol and
  fragmentation clusters.

### SP16 complete: real-font metrics and raster ownership closed

- Froze a 2,804-ID exact baseline, a 226-ID actionable real-font ledger, and 550
  structured unported residual dispositions covering all 776 original
  `needs_font_metrics` rows.
- Vendored deterministic DejaVu Sans, Sans Mono, and Serif faces; added shared primary
  metrics, `ch`/`ex`/`lh`, used line height, full corpus-used font shorthand parsing,
  and manifest-scoped Linux LCD rendering using the pinned Chromium FreeType runtime.
- The 20 sole-owner targets finish 5 exact and 15 functionally reclassified. Across
  all 226 actionable tests, 19 are exact, 207 retain precise non-font owners, and none
  error.
- Retired `needs_font_metrics` globally while preserving the immutable SP14/SP15
  ledgers through explicit supersession rules.
- Authoritative full state: **2,823 pass / 743 fail / 0 errors** across 3,566 runnable
  tests; all 2,804 frozen baseline IDs remain exact; audit passes 7/7.
- Next: SP17 advanced text, or resume the explicitly owned SP13 multicol and
  fragmentation clusters.

### SP13-R complete: runnable multicol exact closure

- Froze a 2823-ID exact baseline, a 351-ID runnable multicol target ledger, and
  1018 structured unported residual dispositions. The target and residual ledgers
  are a disjoint cover of the original 1369 multicol-owned rows.
- Implemented shared multicol used geometry, authoritative fragmentation and
  continuation state, spanners and nested rows, fragmented flex and positioned
  interactions, rule painting, and fragmented decoration/image behavior.
- The exact-ID target run finishes 351 pass / 0 fail / 0 errors at 0.0% mismatch.
  No runnable row retains `sp13_multicol`; every unported residual retains its
  Chromium path, porter rejection, and complete reason-backed ownership.
- Authoritative full state: **3267 pass / 299 fail / 0 errors** across 3566 runnable
  tests; all 2823 baseline IDs remain exact; audit passes 7/7.
- Vertical and sideways writing modes remain out of scope. Next: SP17 advanced
  text or another explicitly owned residual system.

### PR #1 landing gate: portable CI and SP17 handoff

- Replaced the unbootstrapped shallow `depot_tools` workflow with Ubuntu 24.04
  packages for standalone GN, Ninja, Clang, and clang-format. Native Debug and
  Release jobs now build and execute the portable `hello_world` target instead
  of implicitly entering the Chromium-dependent Skia POC.
- Added hosted Rust formatting plus style/text/layout/paint tests, cached
  source-built Skia, the 100-test Python closure/porter suite, immutable SP13-R
  ledger verification, and repository-contained accountability verification.
- Formatted the tracked C/C++ and GN sources once so the native format gates
  enforce a clean baseline rather than failing on historical drift.
- Added `docs/CI.md` with hosted-versus-pinned validation boundaries and exact
  local equivalents.
- Added `docs/SP17-HANDOFF.md` with a fresh-`main` branch procedure, the
  3,267-pass guard, all 19 runnable writing-mode IDs, the full 842-row owner
  inventory, the 337 direct porter opportunities, implementation waves,
  architecture hotspots, and closure commands.
- Local pre-push evidence: Rust style/text/layout/paint suites pass; all 100
  Python tests pass; SP13-R ledgers remain 2,823/351/1,018; audit passes 7/7;
  clang-format 18, GN formatting, workflow syntax, and `git diff --check` pass.
- The first hosted run exposed three porter tests that had silently relied on
  `/home/nero/chromium`. They now patch `WPT_ROOT` to two committed upstream
  snapshots from Chromium source commit `09d377d9438dc95267369f74a073acd81bdde38f`;
  the affected SP13-R and SP16 idempotence tests pass in isolation without a
  sibling checkout.
- The same run proved Ubuntu Clang 18 rejects Chromium's newer
  `-Wno-gcc-install-dir-libstdcxx` switch under `-Werror`. The compiler config
  now adds that switch only for `chromium_src` hermetic builds, preserving ABI
  behavior while making the standalone native smoke target portable.
- The clean runner also confirmed that comparison PNGs are deliberately
  ignored workstation artifacts. Hosted CI now uses an explicit
  `audit.py --repository-only` mode that keeps exact committed `result.json`
  proof and checks 2–7 strict; the unflagged local 7/7 audit remains the sole
  pixel-evidence gate and still requires both screenshots for every pass.
- A final-head Release matrix runner then spent its entire 20-minute budget in
  `apt-get` and was cancelled before GN ran, even though Release had compiled
  and executed on the preceding head. The native gate now provisions once and
  runs Debug plus Release sequentially in one job, preserving both builds while
  removing the duplicate network failure surface.

### SP17 W0A: writing-mode kickoff evidence frozen

- Started `agent/sp17-advanced-text` from landed `main` commit `2c1fe78c` and
  committed the execution plan before changing implementation code.
- Release-built `pixel-compare`, ran the complete 3,566-ID WPT suite without
  resume, and reproduced **3,267 pass / 299 fail / 0 errors** exactly. The
  unflagged local audit passes all 7 checks with PNG verification.
- Froze the complete 3,267-ID exact baseline and the original 842-row
  `needs_writing_mode` inventory: 19 runnable, 823 unported, 337 direct
  `writing-mode` rejections, three direct `unicode-bidi` rejections, and 483
  rows first blocked elsewhere.
- Ran the exact 19-ID kickoff manifest without resume and retained structured
  evidence for all 19 functional failures and zero errors; restored the full
  summary before re-running audit.
- Added generation/check/capture tooling, six focused tests, SHA-256 guards for
  every historical SP13-R through SP16 ledger, and hosted CI coverage.
- Next: W0B transactional SP17 CSS handling and a temporary 340-row porter
  probe. No production behavior, committed WPT builder, mapping owner, or
  detector changed in W0A.

### SP17 W0B: transactional CSS and porter disposition frozen

- Implemented transactional parsing, CSS-wide keyword normalization, cascade
  priority including `!important` and `dir`, inherited writing properties, Rust
  enum emission, and computed-direction logical property resolution in the WPT
  porter.
- Probed all 823 kickoff-unported owner rows through the actual retained-text
  deterministic-Ahem builder path: 292 generate successfully and 531 retain
  their actual rejection plus complete detector-backed ownership.
- Froze 311 actionable IDs (19 existing plus 292 new) and 531 residual rows as a
  sorted, disjoint cover of the original 842-row scope. An atomic prospective
  splice of all 292 builders succeeded in memory across 23 files without
  changing committed builders.
- Preserved the non-ASCII font guard. Only the fullwidth-digit
  `css-flexbox-test1` test/reference pair retains `needs_writing_mode`; W2 must
  supply a pinned glyph/fallback path rather than ambient font behavior.
- The combined SP13-R through SP17 closure/porter suite passes all 117 tests and
  both ledger checks pass. Historical SP13-R/SP16 builders remain byte-stable.
- Next: W1 threads complete writing direction through `ConstraintSpace` and
  proves horizontal no-op behavior before block/flex geometry or surgical WPT
  admission.

### SP17 W1A: constraint and logical-geometry foundation

- Added authoritative writing direction to `ConstraintSpace`, explicit
  physical-root conversion, parent/child orthogonal size conversion, and
  horizontal-compatible legacy constructors.
- Root rendering derives its direction from computed style. Shared
  `LogicalBoxStrut` and `ResolvedLogicalBox` types now centralize edge, size,
  inset, and used-border projection while fragments remain physical for paint.
- Rebuilt the release comparator and ran all 3,267 frozen exact IDs without
  resume: all remain exact at `0.0%`, with zero failures and zero errors. The
  authoritative full summary was restored byte-for-byte afterward.
- No WPT builder, mapping row, runner profile, or committed result changed.
  Next: migrate normal block and flex child boundaries to the shared logical
  view, then admit a small sole-SP17 cohort surgically.

### SP17 W1B: shared child/flex geometry and first exact promotion

- Centralized normal block, float, atomic-inline, block-in-inline, and flex-item
  child constraint construction around computed child writing direction with
  one orthogonal conversion of available and percentage size pairs.
- Converted flex axis selection, logical sizing, wrapping, alignment, gaps,
  margins, item positions, and final fragment conversion to the container
  writing direction. Added vertical-lr/rl and RTL geometry regressions, plus
  correct overflowing RTL-start inline alignment.
- Surgically admitted `flexbox-writing-mode-001` at exactly `0.0%`. The
  authoritative no-resume full suite is now **3,567 runnable / 3,268 exact /
  299 fail / 0 errors**, with all 3,267 kickoff exact IDs
  preserved. Live unported inventory is 4,106 and `needs_writing_mode` is 841.
- Corrected four existing RTL gap reference builders whose historical generated
  CSS lowering was invalid or omitted retained text. All 32 `gap-00` cases are
  exact; the five-builder transaction is byte-idempotent.
- Historical ledgers remain byte-pinned. Standalone SP13-R and the audit accept
  later growth only when mapping/template/summary identities agree and the
  promoted result is exact. The 1,018 SP13-R multicol residuals remain frozen.
- Verification: 122 Python closure/porter tests; full locked Rust
  style/text/layout/paint tests; release comparator build; both ledger checks;
  double byte-identical mapping/deferred/report regeneration; and unflagged
  PNG-backed audit 7/7.
- Next: probe horizontal-RTL `flexbox-writing-mode-004`, then use 002/003/005/006
  and 007/008 to finish shared vertical/mixed geometry. W1 still needs physical
  normal-block decisions, flex intrinsic/aspect-ratio paths, out-of-flow/static
  positions, fragmentation/multicol, and vertical glyph shaping/paint.

### SP17 W1C: horizontal RTL flex-flow closure

- Added an identity-based regression for all eight horizontal-tb/RTL
  flex-direction and wrap-reversal combinations.
- Fixed final flex placement to use the resolved container writing direction
  during logical-to-physical conversion instead of the parent constraint
  direction.
- Surgically admitted only `flexbox-writing-mode-004` at exactly `0.0%`; its
  splice is byte-idempotent and the 001/004 promotion set is now required by
  the live closure test.
- The frozen 3,267-ID baseline remains 3,267/3,267 exact. The authoritative
  full no-resume suite is **3,568 runnable / 3,269 exact / 299 fail / 0
  errors**; live unported inventory is 4,105 and writing-mode ownership is 840.
- Verification: focused flex/logical-writing tests, full locked Rust matrix,
  122 Python closure/porter tests, both ledger checks, release comparator,
  double deterministic accountability generation, splice idempotence, and
  unflagged PNG-backed audit 7/7.
- Next: drive vertical and mixed orthogonal geometry with 002, 003, 005, 006,
  then 007 and 008. Intrinsic/aspect-ratio, out-of-flow, fragmentation,
  multicol, and vertical glyph work remain open.

### SP17 W1D: vertical-container flex-flow matrix closure

- Added an identity-based parameterized regression for the four vertical
  writing-mode/direction combinations and all eight flex-flow reversals,
  including physical item-size assertions.
- Fixed vertical flex items' resolved logical main/cross border-box projection
  into physical fragment width/height. Horizontal fragment sizes remain owned
  by child layout so fragmentation reductions are not overwritten.
- Surgically admitted only `flexbox-writing-mode-002`, 003, 005, and 006 at
  exactly `0.0%`. The exact actionable promotion set is now 001–006; all four
  existing references remain byte-identical and no 007/008 builder was added.
- The frozen 3,267-ID baseline remains 3,267/3,267 exact. The authoritative
  full no-resume suite is **3,572 runnable / 3,273 exact / 299 fail / 0
  errors**; live unported inventory is 4,101, writing-mode ownership is 836,
  and the text manifest contains 697 IDs.
- Verification: 179 focused flex/logical-writing tests, full locked Rust
  matrix, 122 Python closure/porter tests, both ledger checks, release
  comparator, double deterministic accountability generation, splice and
  reference idempotence, formatter/diff checks, and audit 7/7.
- Next: drive orthogonal-child sizing with 007, then 008. Intrinsic/aspect-ratio,
  out-of-flow, fragmentation, multicol, and vertical glyph work remain open.

### SP17 W1E: orthogonal flex-item sizing closure

- Added one private axis-mapping boundary for flex container main/cross sizes
  and child logical inline/block sizes. It now owns available and percentage
  sizes, fixed/stretch flags, intrinsic spaces, aspect-ratio transfer, and
  final physical projection without overriding horizontal fragmentation.
- Added parameterized regressions across horizontal and vertical containers and
  children, alignment/stretch and percentage padding, intrinsic keywords,
  border-box aspect ratio, wrapping, and overflow padding.
- Surgically admitted 14 exact targets: writing-mode 007–009 plus 11 intrinsic,
  alignment, wrap, aspect-ratio, and overflow-padding cases. Their manifest is
  14/14 exact; 007–009 reference builders remain byte-identical, the splice is
  byte-idempotent, and no 010–016 builder was generated.
- The frozen 3,267-ID baseline remains 3,267/3,267 exact. The authoritative
  full no-resume suite is **3,586 runnable / 3,287 exact / 299 fail / 0
  errors**; live unported inventory is 4,087, writing-mode ownership is 822,
  and the text manifest contains 711 IDs.
- Verification: focused flex/logical-writing and fragmentation regressions,
  full locked Rust matrix, 122 Python closure/porter tests, both ledger checks,
  release comparator, double deterministic accountability generation, splice
  and reference idempotence, formatter/diff checks, and audit 7/7.
- Next: close the remaining vertical flex families in W1, then out-of-flow,
  fragmentation, and multicol geometry. Writing-mode 010–015 remains W2
  vertical-text shaping and paint work.

### SP17 W1F: vertical flex flow, gaps, and atomic-inline closure

- Added private logical-axis mappings for normal block, atomic inline, and
  flex intrinsic/content sizing. Vertical fragments are transposed once at the
  writing-mode boundary; automatic flex minima use the child logical main axis
  while explicit physical min/max keywords keep physical-property semantics.
- Added the first vertical mixed-text production path: homogeneous Latin/Ahem
  runs shape horizontally and rotate their complete paint stack clockwise.
  Upright CJK, mixed-script splitting, and sideways modes remain W2 work.
- Added parameterized regressions for vertical-lr/rl direction, reverse flow,
  nowrap/wrap/wrap-reverse, all seven gap patterns, vertical atomic-inline
  sizing/offsets, logical intrinsic contributions, rotated Ahem paint, and the
  column-wrap intrinsic crash.
- Surgically admitted all 44 builders as one cohort. The no-resume target run
  is 44/44 exact with zero mismatched pixels or errors; shared references are
  byte-identical, repeat splicing is idempotent, and writing-mode 010–016 were
  not generated.
- The frozen 3,267-ID baseline remains 3,267/3,267 exact. The authoritative
  full no-resume suite is **3,630 runnable / 3,331 exact / 299 fail / 0
  errors**; live unported inventory is 4,043, writing-mode ownership is 778,
  and the text manifest contains 755 IDs.
- Verification: focused logical-writing/flex/text/paint regressions, full
  locked Rust matrix, 122 Python closure/porter tests, both ledger checks,
  release comparator, double deterministic accountability generation, splice
  and reference idempotence, formatter/diff checks, and audit 7/7.
- Next: continue logical geometry through out-of-flow/static positions,
  fragmentation, and multicol. W2 retains upright/mixed vertical text,
  sideways modes, and writing-mode 010–015.

### SP17 W1G: logical out-of-flow core closure

- Added a private out-of-flow axis mapping that preserves distinct complete
  writing directions for the containing block, static-position parent, and
  abspos child. Physical properties remain physical while child constraints,
  intrinsic inputs, static anchors, and fragments transpose only at explicit
  logical/physical boundaries.
- Reworked flex abspos static positioning through the existing main/cross
  mapping and padding-box containing block, including reverse direction,
  asymmetric borders and padding, and one final physical projection.
- Added parameterized regressions for all horizontal-tb/vertical-lr/vertical-rl
  × LTR/RTL physical polarities, orthogonal child constraints and relayout,
  flex static positions, intrinsic keywords, auto and over-constrained margins,
  vertical aspect-ratio transfer, percentage descendants, and physical
  fragments.
- Surgically admitted the complete 27-ID cohort in one transaction. Its focused
  comparison is 27/27 exact with zero mismatched pixels or errors, repeat
  splicing is byte-idempotent, and writing-mode 010–016 remain absent.
- The frozen 3,267-ID baseline remains 3,267/3,267 exact. The authoritative
  full no-resume suite is **3,657 runnable / 3,358 exact / 299 fail / 0
  errors**; live unported inventory is 4,016, writing-mode ownership is 751,
  and the text manifest contains 782 IDs.
- Verification: focused and full locked Rust matrices, all 122 Python
  closure/porter tests, both ledger checks, release comparator, double
  deterministic accountability generation, splice idempotence, formatter/diff
  checks, and audit 7/7.
- Next: continue fragmentation and multicol logical geometry. Positioned-inline
  static positions, flex safe-alignment abspos behavior, fragmented out-of-flow
  layout, upright/mixed vertical text, sideways modes, and writing-mode 010–015
  remain later scoped work.

### SP17 W1H: logical multicol and vertical fragmentation closure

- Kept multicol sizing, balancing, spanners, break progress, and in-flow
  continuations in logical coordinates, with complete writing-aware child and
  relayout constraints and one final physical projection.
- Added fragmentation writing-direction metadata and mapped column/overflow
  clips, decoration slices, border polarity, backgrounds, shadows, radii, and
  ink overflow across horizontal-tb, vertical-lr, and vertical-rl plus RTL.
- Closed wrapping-row, growing-column, and break-before flex continuation
  shapes, per-fragment negative-start clipping, and direct/nested vertical-rl
  float overflow propagation.
- Surgically admitted all 16 builders as one cohort. The 18-ID proof set is
  18/18 exact with zero mismatched pixels or errors, repeat splicing is
  byte-identical, and writing-mode 010–016 remain absent.
- The frozen 3,267-ID baseline remains exact. The authoritative full suite is
  **3,673 runnable / 3,374 exact / 299 fail / 0 errors**; live unported
  inventory is 4,000, writing-mode ownership is 735, the text manifest is 798,
  and the live validator requires exactly 107 promotions. The committed
  `summary.json` SHA-256 is
  `ed78d9c2fd09c64a52c5de47ece6e7077a8fbab34e3eff6a12d6814ede1c6474`.
- Verification: focused logical multicol/paint regressions, the full locked
  Rust matrix, all 122 closure/porter tests, both ledger checks, release
  comparator, double deterministic accountability generation, splice
  idempotence, formatter/diff checks, and audit 7/7.

### SP17 W1I: positioned-inline static geometry closure

- Added a private logical positioned-inline candidate path shared by both
  inline layout entry points. First/last continuation containing blocks now
  honor the inline ancestor direction and logical line positions while text
  indent, relative offsets, atomic-inline bubbling, block-in-inline
  interruption, and synthetic empty continuations are resolved once.
- Extended the out-of-flow projection boundary to carry logical static anchors
  and inline containing-block geometry together without losing the inline
  node, direction, or zero-border contract. Vertical multicol maps both
  endpoints through its existing column index/remainder and vertical-lr or
  vertical-rl projection before physical positioned layout.
- Surgically admitted all 30 selected builders and repaired both existing
  horizontal-tb RTL family failures. The 35-ID proof is 35/35 exact with zero
  mismatched pixels or errors; two dry-runs and two transactional splices are
  byte-identical.
- The frozen 3,267-ID baseline remains exact. The authoritative full suite is
  **3,703 runnable / 3,406 exact / 297 fail / 0 errors**; live unported
  inventory is 3,970, writing-mode ownership is 703, the text manifest is 828,
  and the live validator requires exactly 139 promotions. The committed
  `summary.json` SHA-256 is
  `13a7c9181f86b213a81a183eb164d0d05d1bd03911850bdc22d57e42c65cf416`.
- Verification: focused positioned-inline and regression-guard comparisons,
  the complete no-resume release suite, the full locked Rust matrix, all 122
  closure/porter tests, both ledger checks, release comparator, double
  deterministic mapping/deferred/HTML generation, splice idempotence,
  formatter/diff checks, the writing-mode 010–016 exclusion, and audit 7/7.
- Next: keep the six true multicol out-of-flow fragmentation cases,
  fragmented abspos boxes, flex safe alignment, absolute centering, tables,
  transforms, generated content, and image/print cases in later atomic W1
  cohorts. W2 retains material bidi and vertical glyph shaping.

### SP17 W1J: vertical multicol out-of-flow fragmentation closure

- Added one private logical positioned-fragment record for source interval,
  static anchor, containing-block geometry, visual translation, resolved
  logical insets/margins, and direction. Column-flow intersections now produce
  source-ordered continuations with authoritative slice metadata and one W1H
  physical projection.
- Unified relative inline, fragmented positioned block, and fragmented flex
  containing blocks, including RTL boundary affinity, relative logical
  offsets, percentage inline sizes, asymmetric logical borders, source-local
  descendants, and single ownership.
- Surgically admitted only out-of-flow multicol IDs 063, 064, 066, 067, 118,
  and 119. The 17-ID target/guard proof is 17/17 exact with zero mismatched
  pixels or errors; two dry-runs and two transactional splices are
  byte-identical.
- The frozen 3,267-ID baseline remains exact. The authoritative full suite is
  **3,709 runnable / 3,412 exact / 297 fail / 0 errors**; live unported
  inventory is 3,964, writing-mode ownership is 697, unported SP13-R multicol
  ownership is 976, the text manifest is 834, and the live validator requires
  exactly 145 promotions. The SP13-R later-promotion compatibility set is 42.
  The committed `summary.json` SHA-256 is
  `be9c87dcb549fd3566b288749cd278e8430c6a2ad2cf8ec560d6996b2de996b1`.
- Verification: the 17-ID proof, parameterized vertical writing/direction
  layout regressions, complete no-resume release suite, locked Rust matrix,
  all 122 closure/porter tests, both ledger checks, deterministic
  mapping/deferred/HTML generation, splice idempotence, formatter/diff checks,
  and audit 7/7.
- Next: keep flex safe alignment, absolute centering, tables, transforms,
  generated content, images/print cases, and other excluded out-of-flow cases
  in later cohorts. W2 retains material bidi and vertical glyph shaping.

### SP17 W1K: safe flex overflow alignment closure

- Refactored the private flex abspos static-position path to retain signed
  main/cross free space and complete safe/unsafe alignment values. It now uses
  the existing content- and item-alignment resolvers, inherits both fields for
  `align-self:auto`, and performs reverse-flow, wrap-reverse, writing-mode, and
  direction projection once.
- Kept the child margin box authoritative for alignment and preserved W1G's
  padding-box containing block and one-time out-of-flow margin application.
  Parameterized regressions cover row/column and both reversals across
  horizontal-tb, vertical-lr, and vertical-rl under LTR/RTL, including safe
  fallback, signed unsafe center, fitting safe end, asymmetric edges, inherited
  overflow alignment, and wrap-reverse.
- Surgically admitted all three safe align-self test/reference pairs and
  `flexbox-safe-overflow-position-005`. The 18-ID target/guard proof is 18/18
  exact with zero mismatched pixels or errors; two dry-runs and two
  transactional splices are byte-identical.
- The frozen 3,267-ID baseline remains exact. The authoritative full suite is
  **3,716 runnable / 3,419 exact / 297 fail / 0 errors**; live unported
  inventory is 3,957, writing-mode ownership is 690, unported SP13-R multicol
  ownership is 976, the text manifest is 841, and the live validator requires
  exactly 152 promotions. The SP13-R later-promotion compatibility set remains
  42. The committed `summary.json` SHA-256 is
  `a3a9be11bd242c330e9de765bbbb7917df4f95ea714c3672f97f6df5567f10df`.
- Verification: the 18-ID proof, full parameterized layout regressions,
  complete no-resume release suite, locked Rust matrix, all 122
  closure/porter tests, both ledger checks, deterministic
  mapping/deferred/HTML generation, splice idempotence, formatter/diff checks,
  writing-mode 010–016 exclusion, and audit 7/7.
- Next: keep absolute centering, tables, transforms, generated content,
  images/print cases, and unrelated paint/layout work outside W1K. W2 retains
  material bidi and vertical glyph shaping.

### SP17 W1L: absolute flex static-position centering closure

- Added retained start/center/end affinity for both physical static-position
  axes. Flex emits padding-box anchors plus edge bias; out-of-flow sizing turns
  that edge into a forward, backward, or symmetric available interval and
  aligns the complete margin box after final sizing.
- Auto physical height in vertical writing is recentered after content layout.
  Multicol reconstructs a fragmented flex candidate's hypothetical margin-box
  start before column projection, preserving the exact flex-container
  fragmentation 010/011 guards.
- Admitted only `position-absolute-center-002`; the shared fix promotes 001.
  The focused center/W1K/writing-direction proof is 17/17 exact. Two porter
  dry-runs and two transactional splices are byte-identical.
- The frozen 3,267-ID baseline remains exact. The authoritative full suite is
  **3,717 runnable / 3,421 exact / 296 fail / 0 errors**; live unported
  inventory is 3,956, writing-mode ownership is 688, unported SP13-R multicol
  ownership is 976, the text manifest is 842, and the live validator requires
  exactly 154 promotions. The SP13-R later-promotion set remains 42. The
  committed `summary.json` SHA-256 is
  `365354dae47ca97f6370a2dcd4edc3e69286de0866ee7e9169e8f7ee3e4f1cd5`.
- Verification: complete no-resume release suite, locked Rust matrix, all 122
  closure/porter tests, both ledger checks, deterministic generated artifacts,
  splice idempotence, formatter/diff checks, 010–016 exclusion, and audit 7/7.
- Next: keep fallback/justify-self/JavaScript/table/transform/generated-content
  cases and unrelated paint work outside W1L. W2 retains material bidi and
  vertical glyph shaping.

### SP17 W1M: flex abspos alignment matrix closure

- Restricted JavaScript admission to the exact inert check-layout harness and
  stripped it from comparison templates, including quoted `>` selectors.
  Inline mutation, unknown scripts, extra handlers, and dynamic alignment are
  still rejected.
- Completed one-time flex abspos resolution for distribution fallbacks,
  physical/logical and flex edges, reverse/wrap reversal, writing mode,
  direction, self alignment, safe overflow, margins, and ignored
  `justify-self`.
- Fixed mixed-flow float clearance so a zero-height clearing line contributes
  `max(clearance, strut)` to its anonymous wrapper and parent exactly once.
  Parameterized regressions cover left/right/both floats, margins, short
  floats, no-op clearing breaks, and complete main/cross edge matrices.
- The mandatory 39-ID cohort combines 29 new assertion-only layouts with ten
  existing fallback/justify-self/margin failures. The focused 58-ID proof adds
  all W1L guards plus fragmentation 010/011 and is 58/58 exact. Dry-run and
  transactional splice pairs are byte-identical.
- The frozen 3,267-ID baseline remains exact. The authoritative full suite is
  **3,746 runnable / 3,460 exact / 286 fail / 0 errors**; live unported
  inventory is 3,927, writing-mode ownership is 676, unported SP13-R multicol
  ownership is 976, the text manifest is 871, and the validator requires 166
  exact SP17 promotions. The SP13-R later-promotion set remains 42. The
  committed `summary.json` SHA-256 is `94574e79d0c0f5bbf979e57c6168e62aab35e56fed62f5977c2f3ed4784817df`.
- Verification: 58-ID zero-pixel proof, complete no-resume release suite,
  locked Rust matrix, all 125 closure/porter tests, both ledger checks,
  deterministic generated artifacts, splice idempotence, formatter/diff
  checks, writing-mode 010–016 exclusion, and audit 7/7.
- Next: keep dynamic JavaScript/mutation, tables, grid, transforms, generated
  content, safe-overflow 006, and W2 glyph work outside W1M.
