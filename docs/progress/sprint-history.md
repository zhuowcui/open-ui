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

**Current accountability snapshot: 7,673 SP12-scope Chromium WPT inventory rows, 3,406 ported/runnable tests, 2,430 runnable passes, 0 `sp12_layout_bug` rows.**

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
| Ported/runnable WPT tests | 3406 |
| Unported but explicitly tracked rows | 4267 |
| Runnable passes | 2430 |
| Runnable failures | 974 |
| Runnable render/diff errors | 2 |
| Generic `not_ported` bucket rows | 0 |
| `sp12_layout_bug` rows | 0 |

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
- SP11/SP13 text and inline layout,
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
| Current runnable WPT tests | 3,406 |
| Current runnable WPT passes | 2,430 |
| Current SP12-owned layout bugs | 0 |
| Generic unported bucket rows | 0 |
| Pixel comparison tests | 3,406 generated WPT comparisons + earlier SP pages/apps |
| Dual-model review rounds | 55+ (31 SP11 + 6 SP11.5 + 18 SP12) |
| Total review findings | 250+ |
| Total real fixes from review | 230+ |
| CSS features implemented | Block, Flex, Inline, Text, Ruby |
| Chromium version | M147 (147.0.7727.24) |
