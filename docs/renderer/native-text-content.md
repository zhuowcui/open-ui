# Native text replacement

Open UI runs no JavaScript. A consuming app changes elements through public
native Rust methods and Rust callbacks. C uses the same Rust Engine. Pinned
Chromium is the pixel target; old Open UI screenshots preserve history.

## Measured failure

The raster trial at `e0dc491e` finishes its Rust/C/C++ font consumer with exit 1:
73 of 1,200 images match Chromium. Only 25,600 of 76,800 geometry states match.
All correct geometry comes from Rust; the C and C++ consumers match none of
their 51,200 geometry states or 800 images. All 800 C/C++ images are blank white,
and every text element has a zero-width, zero-height box.

The C element text setter calls the low-level Engine operation that stores
text data on the container itself. Layout reads authored Text children instead.
The public Rust `Element::set_text` already creates such a child. These paths
therefore give different results for the same application operation.

Two repeated geometry-only probes against the verified existing C library
confirm the cause. Changing size, line height or family does not repair the
empty box. Appending a real Text child through the public C API does: `XX` at
20px has a 40×20 CSS-pixel box with Ahem and a 27.40625×20 box with DejaVu Sans.
These probes run no Cargo or raster commands and generate no screenshots.
An earlier probe fails before creating a document because its enum lookup
does not include the ABI version macro; that failed evidence is preserved.

The [reviewed evidence](generated/native-text-content-v1.json) assigns all
800 C/C++ image failures to native text replacement. The 327 different Rust
images still need separate pixel root-cause review. Comparing Rust images with
themselves is not a C/Rust parity pass. The original receipt, images and
Chromium references remain unchanged.

## Shared native operation

Source `90310e15` adds `Engine::set_text_content`. Both public Rust
`Element::set_text` and C `oui_element_set_text` call that operation:

- A container replaces its children with one authored Text child.
- A Text handle updates its existing data.
- Replaced child handles become stale, following the existing removal rules.
- Empty container text retains an empty Text child, preserving existing Rust
  behavior.

The low-level `Engine::set_text` keeps its existing behavior. The correction
changes no C export or struct layout: all 113 exports and 30 layouts remain.
It is now integrated into the umbrella branch with authored text-style
inheritance at `6def29f8`.

The new Rust, C and C++ consumers change `X` to `XX` from native click callbacks,
check owned geometry and destroy their documents. A separate Engine guard
requires stable node and handle storage after 10,000 text replacements.

## Verification and remaining pixel gaps

All thirteen source, generated-contract, formatting and C/C++ syntax checks
pass. These checks do not establish runtime or pixel correctness.

The first source, `d5bd17d7`, has a bad viewport setup in its new parity test:
it changes scale while retaining physical dimensions from scale 1. Its hosted
run finishes with four passing jobs and three failures, with zero skips. Address
and leak sanitizer jobs each stop on `InvalidArgument`, with 31 tests passing
and the new test failing. The Linux C application job reaches the same failure,
with 32 tests passing. Their logs and source are preserved. The fresh source
derives physical dimensions from its authoritative logical viewport. Its
production correction is otherwise identical. Its own-source hosted run
[37355859674](https://github.com/zhuowcui/open-ui/actions/runs/37355859674)
finishes with all seven hardening jobs passing, zero skips and actual exit 0.
The [terminal evidence](generated/native-review-v4.json) preserves both runs.

Whole owner `1636` stops before Cargo: its probe repeats the `viewport` prefix
in the source path and cannot import the qualification module. The failed
probe, log and clean source remain preserved. No native result comes from it.

The fresh owner `1650` checks all four probe paths, the import and the restore
branch before starting. All 34 predecessors are terminal. Test-only baseline
`9fe1665d` fails the named C text assertion with exit 101; corrected source
`90310e15` passes it. The Engine storage guard passes 10,000 replacements, and
all 58 public native conformance scenarios pass. The clean build passes 8,538
workspace tests, zero failures, 13 ignored, and ABI consumers. Its C compile
and link pass, but the smoke exits 127: the loader needs `libopenui.so.0`, while
the harness installs only `libopenui_ffi.so`. The C++ build, application images
and matrices do not execute. This is a terminal harness failure.
The [native guard evidence](generated/native-text-content-v2.json) preserves
every actual exit and both baseline and fixed logs.

Fresh owner `1656` uses unchanged source `90310e15` on a new root. All 35 prior
owners are terminal. The new build installs the ABI's required SONAME filename
and verifies its bytes against the built library before linking or running C
and C++ applications. All probe paths and the restore branch are preflighted.
The [loader retry evidence](generated/native-review-v5.json) preserves the
previous stop and the new probes. Named baseline/fixed guards and the clean
build are repeated; 600 Rust/C/C++ image comparisons with 38,400 geometry states
and all four renderer matrices remain required. Native pixel comparisons use
the preserved pinned Chromium Fontations references and the app's immutable
default Engine options.

That owner is interrupted after its successful twelve-stage build and 306
recorded app images. Its processes are absent; the incomplete receipt and
images remain unchanged. All 204 actual C/C++ images match Rust, but none of
the 306 images or 19,584 geometry states matches Chromium. The
[native style follow-up](native-text-style-inheritance.md) fixes shared font
inheritance at tested `41b616c3`, passes its named baseline/fixed public Rust
guard and a clean thirteen-stage build. Its
[complete app matrix](generated/native-text-inheritance-v2.json) matches
34,560/38,400 Chromium geometry states but 0/600 images. All 400 actual C/C++
images match Rust; 200 Rust self-comparisons are excluded. The 3,840 remaining
geometry differences are text widths 1/64 CSS pixel short at Ahem sizes 12
and 16. All repeats and pinned Chromium inputs agree. All four renderer
matrices finish on that clean source: 21,334/22,924 original,
22,137/23,728 expanded, 640/640 focused and 960/960 primitive exact, with
zero errors. All 48,252 comparison rows retain their nine recorded invariants.
Matching C and Rust alone does not qualify the framework.

The [integration evidence](generated/native-text-inheritance-v6.json) records
the shared operation and style correction on umbrella `6def29f8`, with code
identical to tested `41b616c3`. Its fresh clean build passes all thirteen
stages and 8,551 workspace tests, zero failures and 13 ignored. Native app
font pixels and the full Chromium release gates still fail.

Two unlaunched preparations are also preserved: one would copy linked C/C++
outputs onto themselves; another incorrectly compared diff headers after the
test setup changed. Fresh roots and probes correct those harness issues before
verification starts. Neither preparation ran native or pixel stages. All
failed evidence remains available; it is not replaced by a later pass.

The accepted renderer remains 21,334/22,924 exact. No new release case is
admitted, and the full Chromium and release gates remain open.
