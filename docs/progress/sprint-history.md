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

**Current accountability snapshot: 7,673 SP12-scope Chromium WPT inventory rows, 3,566 ported/runnable tests, 2,823 runnable passes, 0 errors, 0 `sp12_layout_bug` rows, and 0 retired SP16 font-metric owner rows.**

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
| Current runnable WPT tests | 3,566 |
| Current runnable WPT passes | 3,267 |
| Current SP12-owned layout bugs | 0 |
| Generic unported bucket rows | 0 |
| Pixel comparison tests | 3,566 generated WPT comparisons + earlier SP pages/apps |
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
  source-built Skia, the 99-test Python closure/porter suite, immutable SP13-R
  ledger verification, and the 7/7 accountability audit.
- Formatted the tracked C/C++ and GN sources once so the native format gates
  enforce a clean baseline rather than failing on historical drift.
- Added `docs/CI.md` with hosted-versus-pinned validation boundaries and exact
  local equivalents.
- Added `docs/SP17-HANDOFF.md` with a fresh-`main` branch procedure, the
  3,267-pass guard, all 19 runnable writing-mode IDs, the full 842-row owner
  inventory, the 337 direct porter opportunities, implementation waves,
  architecture hotspots, and closure commands.
- Local pre-push evidence: Rust style/text/layout/paint suites pass; all 99
  Python tests pass; SP13-R ledgers remain 2,823/351/1,018; audit passes 7/7;
  clang-format 18, GN formatting, workflow syntax, and `git diff --check` pass.
