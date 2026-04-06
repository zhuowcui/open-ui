# Open UI — Architecture Decision Records

## ADR-001: Use Chromium's Embedded Skia, Not Upstream

**Date**: SP1
**Status**: Accepted

**Context**: Skia exists as both an independent Google project and as a component embedded
within Chromium. The embedded version has Chromium-specific patches for performance and
correctness.

**Decision**: Use Chromium's embedded Skia rather than upstream Skia.

**Rationale**: Chromium patches Skia for specific rendering behaviors (text hinting, subpixel
positioning, GPU backend optimizations). Using upstream Skia would produce pixel differences
in text rendering and anti-aliasing.

**Consequences**: We depend on Chromium's build system for the C/C++ layer. This is acceptable
because pixel-perfect parity is our primary goal.

---

## ADR-002: C ABI as Integration Boundary

**Date**: SP4
**Status**: Accepted

**Context**: We needed to choose how to expose the rendering pipeline to consuming applications.
Options: C++ headers, C ABI, or Rust-only.

**Decision**: Stable C ABI (`openui.h`) as the primary integration boundary.

**Rationale**:
- C ABI is callable from every major programming language via FFI
- ABI stability is achievable (unlike C++ name mangling)
- Rust bindings are built as a safe wrapper over the C API
- Enables future Go, Python, Swift, etc. bindings without additional work

**Consequences**: Every public API function goes through C ABI. Some ergonomic cost, but
maximum portability and stability.

---

## ADR-003: Hybrid C++/Rust Architecture

**Date**: SP9
**Status**: Accepted

**Context**: SP1-SP8 used Chromium's C++ pipeline via C ABI. Starting SP9, we began porting
algorithms to pure Rust for the native rendering engine.

**Decision**: Two-layer architecture:
1. **C/C++ layer**: Wraps actual Chromium code for full pipeline access (SP1-SP8)
2. **Rust layer**: Ports Chromium algorithms to pure Rust (SP9+)

**Rationale**:
- C/C++ layer provides ground truth for pixel comparison
- Rust layer provides portability, safety, and eliminates Chromium build dependency for users
- The Rust layer is validated against the C/C++ layer through pixel comparison
- Over time, the Rust layer becomes the primary distribution mechanism

**Consequences**: We maintain two codepaths temporarily. The C/C++ layer becomes a testing
reference, and the Rust layer becomes the production distribution.

---

## ADR-004: LayoutUnit Fixed-Point Arithmetic

**Date**: SP9
**Status**: Accepted

**Context**: CSS layout requires sub-pixel precision but floating-point accumulation errors
cause layout drift. Chromium uses fixed-point arithmetic.

**Decision**: Port Chromium's `LayoutUnit` — a 32-bit fixed-point type with 6 fractional
bits (1/64th pixel precision).

**Rationale**:
- Matches Chromium's exact numerical behavior
- No floating-point accumulation errors
- Deterministic across platforms
- Efficient (integer arithmetic on hot paths)

**Consequences**: All layout dimensions use `LayoutUnit`, not `f32`/`f64`. Conversion
from CSS pixel values to LayoutUnit is explicit.

---

## ADR-005: Lazy BFC Offset Resolution

**Date**: SP12
**Status**: Accepted

**Context**: Block Formatting Context (BFC) offset resolution in Chromium is lazy — the
offset is not known until content, border, or padding forces resolution.

**Decision**: Port Chromium's lazy BFC resolution with abort-and-relayout pattern.

**Rationale**:
- Exact match to Chromium's behavior is required for float positioning correctness
- Eager resolution (simpler to implement) produces different exclusion space state at
  resolution point, causing float position differences
- WPT tests depend on Chromium's specific resolution timing

**Consequences**: The block layout algorithm includes an abort-and-relayout path when
BFC offset changes. This adds complexity but is essential for correctness.

---

## ADR-006: Dual-Model Review as Quality Gate

**Date**: SP11
**Status**: Accepted

**Context**: CSS layout algorithms have subtle correctness requirements that unit tests
alone don't catch. Code review by humans is limited by reviewer CSS expertise.

**Decision**: Use dual-model AI review (Opus 4.6 + GPT 5.4) with convergence criterion
(0 issues from both models in the same round) as a mandatory quality gate.

**Rationale**:
- Two models find complementary classes of bugs
- Iterative rounds with verification against spec catches both implementation bugs and
  spec misunderstandings
- Convergence criterion is objective and measurable
- Historical effectiveness: SP11 found 150 real bugs in 31 rounds; SP12 found 83 in 18 rounds

**Consequences**: Sprints take longer to complete (15-30 review rounds for complex features).
This is acceptable because correctness is non-negotiable.

---

## ADR-007: WPT Tests as Primary Validation

**Date**: SP12
**Status**: Accepted

**Context**: We needed a comprehensive test suite for CSS block layout that covers edge
cases, spec compliance, and cross-browser interoperability.

**Decision**: Translate Chromium's WPT (Web Platform Tests) into our test builder API
as the primary validation strategy.

**Rationale**:
- WPT tests are the industry standard for CSS compliance testing
- They cover edge cases that are hard to enumerate manually
- They encode Chromium's specific interpretation of ambiguous spec areas
- Passing WPT gives high confidence in spec compliance

**Consequences**: Large test suites (7,505 tests for block layout alone). Translation
effort is significant but produces comprehensive coverage.

---

## ADR-008: Pixel Comparison Against Live Chromium

**Date**: SP5
**Status**: Accepted

**Context**: We needed to validate that our rendering output matches Chromium exactly.
Options: golden file comparison vs live Chromium comparison.

**Decision**: Compare against live headless Chromium output, not golden files.

**Rationale**:
- Golden files become stale when Chromium updates
- Live comparison catches regressions in both our code and Chromium tracking
- 0% tolerance pixel comparison is only meaningful against the actual reference implementation
- We render the same content through both our engine and headless Chromium, then compare

**Consequences**: Test infrastructure is more complex (needs headless Chromium available).
Pixel tests are more meaningful.

---

## ADR-009: Sprint-Based Development with Explicit Exit Conditions

**Date**: SP1
**Status**: Accepted

**Context**: The project scope is enormous (porting Chromium's rendering pipeline). We
needed a way to decompose work into manageable units with clear completion criteria.

**Decision**: Sprint-based development where each sprint has:
- Explicit scope (what features to implement)
- Measurable exit conditions (test pass rates, pixel match rates, review convergence)
- "Fleet deployment" execution (investigate → implement → test → review → loop)

**Rationale**:
- Exit conditions prevent premature declaration of "done"
- Sprint boundaries allow architectural assessment and course correction
- Each sprint builds on the previous sprint's foundation and test infrastructure

**Consequences**: Sprints are completed to full quality before moving on. No partially
implemented features across sprints.
