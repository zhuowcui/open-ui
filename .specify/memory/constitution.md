# Open UI Constitution

## Mission

Extract Chromium's rendering pipeline as a standalone, language-agnostic UI framework that
achieves 100% pixel-perfect rendering parity with Chromium/Blink. Provide the world's most
capable native UI rendering engine as modular, independently-usable libraries with a stable
C ABI and idiomatic Rust bindings.

## Core Principles

### I. Chromium Parity — NON-NEGOTIABLE

Every rendering algorithm must be ported directly from Chromium/Blink source code. No
approximations, no alternative implementations, no "good enough" shortcuts. The logic must
be completely identical — same control flow, same edge case handling, same numerical results.
If Chromium handles a case, we handle it. If Chromium's code has a specific order of operations,
we preserve that order. The only acceptable deviation is skipping features explicitly marked
as deprecated in Chromium.

**What this means in practice:**
- Read the Blink source. Understand the algorithm. Port it faithfully.
- When CSS specs are ambiguous, follow Chromium's interpretation — they are our reference implementation.
- Performance must be within 2x of Chromium's native performance for equivalent operations.
- Every feature must pass Chromium's own WPT (Web Platform Tests) and Blink layout tests.

### II. 100% Pixel Perfection — NON-NEGOTIABLE

Every rendering feature must produce output that is pixel-identical to headless Chromium.
Not "visually similar." Not "close enough." Identical. Zero tolerance pixel comparison is
the standard.

**What this means in practice:**
- Every feature gets exhaustive pixel comparison tests against actual Chromium output.
- All variants of all cases must be tested. No skipped cases. Test suites can be large — thoroughness matters more than brevity.
- Anti-aliasing, subpixel positioning, color blending, text hinting — all must match exactly.
- If a pixel differs, it's a bug. Period.

### III. No Mocks, No Temporary Code, No TODOs

Every line of code is production-ready. There are no placeholder implementations, no "TODO:
implement later" comments, no mock objects standing in for real functionality, no temporary
scaffolding left behind. Code is either fully implemented or it doesn't exist yet.

**What this means in practice:**
- If a feature can't be fully implemented yet due to architectural dependencies, it is deferred — not stubbed.
- Deferred items are documented with clear rationale (e.g., "requires inline layout integration").
- Every function does exactly what its documentation says. No silent no-ops.
- Dead code is removed, not commented out.

### IV. Exhaustive Testing

Testing is not a phase — it is inseparable from implementation. Every feature is validated
through multiple complementary strategies: unit tests for individual functions, integration
tests for system behavior, WPT tests for spec compliance, and pixel comparison tests for
visual correctness.

**What this means in practice:**
- Chromium's own test suites (WPT, Blink layout tests) are the primary validation source.
- Pixel comparison tests run against actual headless Chromium — not golden files from our own renderer.
- Test counts are expected to be large (7,505+ and growing). This is a feature, not a problem.
- All tests must pass before any commit. Zero tolerance for regressions.

### V. Dual-Model Review Until Convergence

Code quality is validated through iterative dual-model review: two independent AI models
(currently Opus 4.6 and GPT 5.4) review the same code, each finding different classes of
issues. Reviews continue in rounds until both models return zero issues in the same round.

**What this means in practice:**
- Every sprint's code goes through dual-model review as the final quality gate.
- Each finding is verified against the CSS specification and actual Chromium source before implementation.
- False positives are explicitly documented with rationale for rejection.
- The convergence criterion (0 issues from both models) is the exit condition, not a target.
- Historical data: SP11 required 31 rounds. SP12 required 18 rounds. This is expected.

### VI. Production-Grade Documentation

Documentation is a first-class artifact, not an afterthought. Every module, every public API,
every algorithm has documentation that explains not just what the code does, but why it does
it that way and which CSS specification section governs the behavior.

**What this means in practice:**
- Inline documentation references specific CSS specification sections (e.g., "CSS 2.1 §8.3.1").
- Architecture decisions are recorded with rationale.
- Sprint progress, learnings, and technical decisions are preserved as institutional knowledge.
- New contributors should be able to understand any module by reading its documentation alone.

### VII. Modular Architecture with Stable ABI

The framework is decomposed into independently-usable libraries, each with a stable C ABI.
Applications can use the full stack or any individual layer. The Rust bindings provide
idiomatic, safe wrappers over the C API.

**What this means in practice:**
- Four core libraries: Skia (2D graphics), Compositor (GPU compositing), Layout (CSS layout), Style (CSS cascade).
- Each library is independently compilable and testable.
- The C API is the integration boundary — language bindings are built on top.
- Breaking ABI changes require explicit versioning and migration paths.

## Quality Standards

### Code Quality Gates

All code must pass these gates before merge:

1. **Compilation**: `cargo check --workspace` with zero warnings
2. **Test Suite**: `cargo test --workspace` with 100% pass rate (currently 7,505 tests)
3. **Pixel Comparison**: All pixel tests match headless Chromium at 0% tolerance
4. **Dual-Model Review**: Both Opus 4.6 and GPT 5.4 return 0 issues in the same round
5. **No Artifacts**: No mocks, no TODOs, no temporary code, no dead code, no commented-out code

### CSS Specification Compliance

When implementing CSS features:

1. Read the relevant CSS specification section first
2. Read Chromium/Blink's implementation of that section
3. Port the algorithm faithfully
4. Validate against Chromium's WPT tests for that feature
5. Add pixel comparison tests against headless Chromium
6. Document which spec sections govern the behavior

### Performance Standards

- Layout algorithms must be within 2x of Chromium's native performance
- No unnecessary allocations in hot paths
- Data structures match Chromium's choices unless a Rust-idiomatic alternative is provably better

## Development Workflow

### Sprint-Based Development

Work is organized into numbered sprints (SP1, SP2, ...) with clear scope and exit conditions.
Each sprint follows this lifecycle:

1. **Planning**: Define scope, architecture, and exit conditions
2. **Investigation**: Research Chromium source, CSS specs, and test suites
3. **Implementation**: Port algorithms, write tests, iterate
4. **Testing**: Run full test suite, pixel comparisons, WPT validation
5. **Review**: Dual-model review rounds until convergence (0 issues from both models)
6. **Documentation**: Record progress, decisions, and learnings

### Fleet Deployment Pattern

For large implementation sprints, work is executed in "fleet deployment" mode:

1. Investigate → Research → Implement → Build → Test → Multi-agent review → Loop
2. Don't stop until all exit conditions are met
3. Exit conditions are explicit and measurable (not subjective)

### Commit Standards

- Each commit represents a logically complete unit of work
- Commit messages follow conventional commits format
- Review round fixes are committed individually (e.g., "R17: 3 correctness fixes")
- All tests must pass at every commit point

## Governance

This constitution supersedes all other development practices. Any proposed change to these
principles requires:

1. Explicit justification with concrete examples of why the change is necessary
2. Demonstration that the change does not compromise pixel-perfect rendering
3. Documentation of the amendment with before/after comparison
4. Approval through the dual-model review process

**Version**: 1.0.0 | **Ratified**: 2026-04-06 | **Last Amended**: 2026-04-06
