# Open UI — Technical Learnings & Institutional Knowledge

This document captures hard-won insights from building Open UI. These are not theoretical
best practices — they are lessons learned through thousands of tests, dozens of review
rounds, and hundreds of bug fixes.

---

## 1. Porting Chromium Code

### 1.1 Don't Approximate — Port

**Lesson**: Every time we tried to "simplify" a Chromium algorithm, we introduced bugs.
Chromium's code handles edge cases that seem unnecessary until you hit them in WPT tests.

**Example**: CSS margin collapsing (§8.3.1) has ~15 interacting rules. Our first
implementation covered 10. The remaining 5 caused failures in ~200 WPT tests. Porting
Chromium's exact logic fixed them all.

**Rule**: Read the Blink source. Port it faithfully. Optimize later if profiling shows a need.

### 1.2 Follow Chromium's Order of Operations

**Lesson**: CSS spec allows implementation flexibility, but Chromium has specific ordering
decisions that WPT tests depend on. When we deviated from Chromium's ordering (even with
spec-compliant alternatives), pixel tests failed.

**Example**: Block layout processes children in DOM order, resolving BFC offsets lazily.
We tried eager BFC resolution — it was spec-compliant but produced different float positions
because the exclusion space state differed at the point of resolution.

### 1.3 Chromium's Interpretation Is Our Spec

**Lesson**: When the CSS spec is ambiguous (and it often is), Chromium's behavior is the
reference. WPT tests encode Chromium's interpretation. Fighting it produces failures.

**Example**: CSS 2.1 §10.6.4 (vertical auto-margin centering for absolute elements with
negative remaining space) is under-specified. Chromium assigns zero to one margin and the
deficit to the other. Our initial "equal split" implementation was arguably more correct
per spec text but failed Chromium's WPT tests.

---

## 2. Margin Collapsing (CSS 2.1 §8.3.1)

### 2.1 The Hardest Algorithm in CSS Layout

**Lesson**: Margin collapsing is deceptively complex. It interacts with:
- Adjacent siblings, parent/first-child, parent/last-child
- Self-collapsing blocks (zero height, no border/padding/content)
- Clearance (prevents collapsing)
- New BFC (prevents parent/child collapsing)
- min-height (prevents parent/last-child collapsing)
- Percentage heights with indefinite containing blocks

**Discovery**: A "self-collapsing" block's position can only be finalized when the NEXT
non-self-collapsing sibling is encountered. Until then, its position is tentative and
must be tracked in a pending list.

### 2.2 Pending Self-Collapsing Architecture

**Lesson**: Self-collapsing blocks accumulate in a pending list. They are NOT positioned
when encountered — only when a non-self-collapsing sibling or the parent's end triggers
finalization.

**Critical detail**: The flush offset must be saved BEFORE the non-self-collapsing child's
height is added to the block offset. Using `margin_boundary_offset` (the offset at the
point where the margin strut was resolved) rather than the current `block_offset` (which
already includes the child's height).

### 2.3 Parent/Last-Child Collapse Prevention

**What prevents parent/last-child margin collapsing**:
1. `height` is not `auto` (any explicit height prevents it)
2. `min-height > 0` (even 1px prevents it)
3. Bottom border or padding exists
4. Element establishes a new BFC
5. `is_fixed_block_size` or `stretch_block_size` is set (viewport, flex cross-size)

**Bug we found**: Using `available_block_size` instead of `percentage_resolution_block_size`
to check min-height percentage indefiniteness. These differ when the parent has explicit
height but the percentage resolution context is different.

---

## 3. Float Layout

### 3.1 Two-Phase Float Model

**Lesson**: Blink uses a two-phase model for floats:
1. **Unpositioned**: Float is parsed but its BFC block offset is unknown (lazy BFC resolution)
2. **Positioned**: BFC offset is resolved, float is placed in the exclusion space

This matters because floats before BFC resolution are accumulated but not placed. When BFC
offset resolves, all pending floats are placed at once.

### 3.2 Exclusion Space Queries

**Lesson**: The exclusion space tracks rectangles where floats occupy space. Layout
opportunities are the gaps between float exclusions. Shelf-based algorithm provides O(n)
queries.

**Key insight for intrinsic sizing**: Float max-content inline size is the SUM of all
float widths (they sit side-by-side), while non-float max-content is the MAX of all
non-float widths. The overall max-content is `max(non_float_max, float_sum)`.

### 3.3 Clearance

**Lesson**: Clearance moves an element below floats, but it also:
- Resets the margin strut (prevents propagation through cleared elements)
- Must use `max_of` with intrinsic_block_size (not plain assignment) to prevent regression
  from negative margins

---

## 4. Out-of-Flow (Absolute/Fixed) Positioning

### 4.1 Auto-Height Relayout

**Lesson**: Absolutely positioned elements with `height:auto` and both `top`/`bottom`
specified need a two-pass approach:
1. First pass: layout with available height from containing block
2. If the resolved height differs from the first pass, relayout with the final height

**Critical bug**: The original_unclamped height must be saved BEFORE the relayout. After
relayout, `child_fragment.size.height` already equals the final height, making the
comparison always succeed (never triggering the relayout).

### 4.2 OOF Candidates and Float Offsets

**Lesson**: When a child is shifted by a float inline offset, its OOF (out-of-flow)
candidates have already been extracted with the unshifted position. The candidates must
be post-shifted to account for the float offset.

**Pattern**: Track `oof_count_before` and `bubbled_count_before` before layout, then
shift new entries after layout.

---

## 5. Intrinsic Sizing

### 5.1 Separate Min and Max Accumulators

**Lesson**: Min-content and max-content block sizes differ because wrapping content at
min-content width produces taller layouts than at max-content width. Using max-content
for both modes is wrong.

### 5.2 Float Contribution

**Lesson**: Floats contribute to intrinsic inline size differently:
- **Min-content**: Each float could be the widest → use MAX
- **Max-content**: All floats sit side-by-side → use SUM

---

## 6. Testing Strategy

### 6.1 WPT Tests Are the Gold Standard

**Lesson**: Chromium's WPT (Web Platform Tests) encode the "correct" behavior for every
CSS feature, including edge cases the spec leaves ambiguous. Translating and passing these
tests is the most reliable way to validate correctness.

**Our approach**: Translate WPT test HTML into our test builder API, then validate layout
tree positions and sizes match expected values.

### 6.2 Pixel Tests Catch What Unit Tests Miss

**Lesson**: Unit tests validate positions and sizes. Pixel tests validate rendering —
anti-aliasing, subpixel positioning, color blending, border rendering, paint order. Many
bugs only manifest in pixel output.

**Specific examples**:
- Border corner pixels have anti-aliasing/blending — check mid-edge pixels for reliable assertions
- Skia N32 on Linux is BGRA; use `ColorType::RGBA8888` in `read_pixels` for correct RGB ordering
- Margin collapsing pixel tests caught a bug where adjacent siblings used sum instead of max

### 6.3 Test Infrastructure Matters

**Lesson**: `BlockTestBuilder` with fluent API significantly reduces test authoring friction.
Key helpers:
- `container()` returns the wrapper div
- `child(n)` returns the nth child
- `overflow_hidden()` is set on child, not container
- `with_style(|s| {...})` for properties not on the builder

**Gotcha**: `NestedChildBuilder` lacks `height_pct`, `width_pct`, `min_*`, `max_*`, and
`add_child` methods. Use `with_style(|s| {...})` for those on nested children.

---

## 7. Dual-Model Review

### 7.1 Different Models Find Different Bugs

**Lesson**: GPT 5.4 and Opus 4.6 have complementary strengths:
- **GPT**: Finds more issues, including some false positives. Good at structural analysis.
- **Opus**: Fewer findings but higher precision. Good at spec compliance analysis.

Using both catches bugs that either alone would miss.

### 7.2 Verify Every Finding

**Lesson**: Never blindly implement a review finding. Always verify against:
1. The actual code (is the described behavior really there?)
2. The CSS specification (is the "correct" behavior actually correct?)
3. Chromium's implementation (what does Blink actually do?)

**Statistics from SP12**: Of 102 findings, 83 were real bugs, ~12 were false positives,
and ~7 were deferred (architectural, not actionable without major changes).

### 7.3 Convergence Is Exponential

**Lesson**: Review rounds converge roughly exponentially:
- SP12: R1-R10 averaged 6 fixes/round → R14: 5 → R15: 4 → R16: 1 → R17: 3 → R18: 0
- SP11: 31 rounds needed (more complex domain — text/inline has more edge cases)
- Expect 15-30 rounds for complex features

### 7.4 False Positives Are Valuable

**Lesson**: False positives from AI reviewers often reveal areas where the code's intent
is unclear, even if the implementation is correct. They're opportunities to improve
documentation or refactor for clarity.

---

## 8. Build Environment

### 8.1 Rust Workspace Setup

```
bindings/rust/
├── Cargo.toml           (workspace root)
├── openui-geometry/     (core geometry types)
├── openui-style/        (CSS style system)
├── openui-text/         (font + text shaping)
├── openui-layout/       (layout algorithms)
├── openui-paint/        (Skia painting)
└── openui/              (framework + view! macro)
```

**Build**: `cd bindings/rust && cargo test --workspace`
**Quick check**: `cd bindings/rust && cargo check --workspace`
**PATH**: `$HOME/.cargo/bin:$HOME/local/bin:$HOME/depot_tools:$PATH`

### 8.2 Chromium Build

The C/C++ layer builds within Chromium's build system:
```bash
cd ~/chromium/src
./third_party/ninja/ninja -C out/Release openui_lib -j24
```

Tests use the Chromium test infrastructure:
```bash
./out/Release/openui_api_test       # 78 tests
./out/Release/openui_c_test         # 32 tests
./out/Release/openui_render_test    # 20 tests
./out/Release/openui_c_render_test  # 46 tests
```

### 8.3 Known Issues

- `sp12_i1_block_pixel_tests.rs` can get corrupted by background agents and git stash
  operations. Always run `git restore` on this file before testing.
- Rust 1.94.1 is the current toolchain version.
- GPT 5.4 review agents can get stuck (36+ minutes with no progress) — relaunch if stuck.
- Background review agents typically take 400-900 seconds.

---

## 9. CSS Specification Reference Map

Key CSS specifications and where they're implemented:

| Spec | Section | Our Implementation |
|------|---------|-------------------|
| CSS 2.1 §8.3.1 | Margin collapsing | `margin_collapsing.rs` |
| CSS 2.1 §9.4.1 | Block formatting context | `block.rs` |
| CSS 2.1 §9.5 | Floats | `float_handler.rs`, `exclusion_space.rs` |
| CSS 2.1 §9.5.2 | Clearance | `clearance.rs` |
| CSS 2.1 §9.6 | Absolute positioning | `out_of_flow.rs` |
| CSS 2.1 §9.7 | Fixed positioning | `out_of_flow.rs` |
| CSS 2.1 §10.3.3 | Block width | `block.rs` |
| CSS 2.1 §10.3.7 | Abs pos width constraint | `out_of_flow.rs` |
| CSS 2.1 §10.6.3 | Block height | `block.rs` |
| CSS 2.1 §10.6.4 | Abs pos height constraint | `out_of_flow.rs` |
| CSS Sizing 3 | Intrinsic sizing | `intrinsic_sizing.rs` |
| CSS Overflow 3 | Overflow handling | `overflow.rs`, paint clipping |
| CSS Position 3 | Sticky positioning | `out_of_flow.rs` |
| CSS Flexbox 1 | Flex layout | `flex/` |
| CSS Break 3 | Fragmentation | `fragmentation.rs` |
| CSS Multi-column 1 | Column layout | multicol integration |
| CSS Text 3 | Text processing | `openui-text/` |
| CSS Inline 3 | Inline layout | `openui-layout/src/inline/` |
| CSS Writing Modes 3 | Writing modes | `openui-geometry/`, `openui-layout/` |
