# Native text style inheritance

Applications set fonts, colors and text through public native Rust methods and
Rust callbacks. C calls the same Engine. Open UI never executes JavaScript;
pinned Chromium remains the only pixel target.

## Measured API gap

The text replacement source `90310e15` creates authored Text children for both
Rust and C. Its clean twelve-stage build passes 8,538 workspace tests and the
C/C++ consumers. The subsequent font matrix is interrupted after 306 images,
with no exact Chromium image or geometry result. All 204 actual C/C++ images
match Rust; the 102 Rust self-comparisons are excluded from that parity count.
These are incomplete results, not qualification.

The shared path has a second problem: new text children retain initial font
settings instead of inheriting the container's authored family, size, color
and line height. Updating the parent style also leaves descendants stale.
The container's position is correct, but its natural text dimensions are not.
Matching Rust and C output therefore does not establish Chromium correctness.

The [preserved evidence](generated/native-text-inheritance-v1.json) includes
the incomplete matrix, original receipts and a separate process-absence
witness. The old owner receipts are unchanged. No interrupted result is
rewritten as a pass.

## Shared native correction

Tested source `41b616c3` combines native text replacement with the existing authored
style resolution work from `0ccc37da`. It refreshes inheritable properties on
attachment, parent mutation, detachment, reparenting, cloning and animation.
Ordered declarations and owned computed-style snapshots are preserved.
Generated property metadata is regenerated from its source generator.

The correction changes no C header, export or struct layout: all 113 exports
and 30 layouts remain. It changes no text-raster, paint or compositor body.
The correction is integrated into the umbrella branch at `6def29f8`. All Rust,
style-generator, C example, ABI and workflow bytes match the tested source.

## Verified scope and remaining gates

A new consuming Rust test requests Ahem at 20px with line height 1. Its label
must have a natural 20×20 box. A Rust click callback changes the font to 24px
and text to `XX`; the box must become 48×24. The test also verifies callback
count, owned style snapshots and document teardown at five scales.

Test-only baseline `25322be8` fails the initial font assertion with exit 101.
The fixed source passes. Its C text parity guard, 10,000-update storage guard,
all nine named inherited-style guards and all 58 native conformance scenarios
pass. Thirteen read-only checks pass. The clean workspace passes 8,551 tests,
zero failures and 13 ignored; all thirteen build stages, ABI consumers and
C/C++ smoke applications pass.

The [complete native app matrix](generated/native-text-inheritance-v2.json)
finishes with exit 1: 0/600 images and 34,560/38,400 geometry states match
Chromium. All 400 actual C/C++ images and 25,600 geometry states match Rust;
the 200 Rust self-comparisons are excluded. All repeat runs agree, and every
pinned Chromium reference stays unchanged. This fixes the shared API's
inheritance gap without establishing pixel equality.

The remaining 3,840 geometry differences are widths at Ahem sizes 12 and 16.
Native widths are exactly 1/64 CSS pixel short; position and height agree.
The earlier Fontations metric investigation supports a shared font-metric
and layout-rounding discrepancy. Default native font rasterization also
remains different from Chromium. These require general implementation fixes,
not adjustments for a particular family, size or fixture.
The [narrow intrinsic-width follow-up](native-intrinsic-snap.md) preserves
shaped fractions through the layout grid ceiling. Its thirteen source checks
pass. The old source fails the new Rust callback guard; the correction passes
at all five scales, along with neighboring native guards. Its workspace,
complete native app and pixel matrices remain unexecuted.

Whole owner `1678` finishes all seven stages. The focused and primitive suites
pass 640/640 and 960/960. The original census is 21,334/22,924 exact and the
expanded census is 22,137/23,728 exact; both exit 1, with zero render errors.
The [complete matrix audit](generated/native-text-inheritance-v5.json) verifies
all 48,252 rows: nine recorded invariants per row and every Chromium input
remain unchanged, with zero exact gains or losses. No residual ownership is
inferred from unchanged results.
The [own-source hosted evidence](generated/native-text-inheritance-v3.json)
records hardening run `37366409715` passing all seven jobs, with zero skips. The
remaining gates do not qualify the renderer or any new release state; the
strict native app gate has already failed.

The [clean integration build](generated/native-text-inheritance-v6.json) at
`6def29f8` passes all thirteen stages, 8,551 workspace tests, zero failures and
13 ignored, plus ABI and C/C++ consumers. The complete pixel results retain
their actual `41b616c3` source identity; they are not relabelled as a new census.
Integration fixes the native API behavior without claiming release qualification.

Two unlaunched preparations and one build-harness failure are also preserved.
Preflight catches an incomplete test-name filter before execution; a later
build probe refers to uppercase `RAW` where only lowercase `raw` exists and
stops before Cargo. Fresh probes check the binding and reuse the actual
source-identical terminal guard logs before the complete build.

The accepted original renderer remains 21,334/22,924 exact, and the expanded
renderer remains 22,137/23,728 exact. Both full gates remain open.
