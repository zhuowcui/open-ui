# Isolated native sizing and Rust font-unit queries

Open UI never executes JavaScript. Consuming applications create, mutate and
operate elements through native Rust methods and Rust callbacks. Pinned
Chromium defines expected pixels and geometry. Historical Open UI images are
provenance and are not release pixel gates.

Clean private `48af2cf4` completes all four rendering matrices, 8,563 workspace
tests, 17 read-only checks and actual Rust/C/C++ ABI consumers. The repeated
native consuming app performs 2,880 Rust callbacks per run. All 48,252 renderer
comparisons preserve their nine measured invariants, with no changed pixels
or Chromium inputs. The original result is 21,334/22,924 exact; expanded is
22,137/23,728, both zero errors and actual exit 1. Focused is 640/640 and
primitive 960/960 exact. This source includes no private Fontations or caption
changes and does not inherit their native font image results.

The paired public `1dd7a23e` consumer matches 1,420/7,380 full rectangles.
Private `48af2cf4` matches 5,770/7,380: 4,350 full-rectangle gains and no
full-rectangle losses. An individual dimension still loses exactness in ten
states: vertical Ahem `in the box` at 15px has minimum height 45.015625px,
where Chromium uses 45px. Correcting width from 15px to 45px leaves the full
rectangle different on both sources, so a full-rectangle loss count alone
misses that height regression. The separate dimension audit exits 1. Both
audits retain their actual results; this candidate remains unapplied.

Clean private `584ab9dd` measures text through the shared shaper and preserves
positive shaped fractions. It passes 8,563 workspace tests and both native
guards. Its repeated consuming app matches 7,360/7,380 bounds, with 5,940 full
rectangle gains, zero full-rectangle losses and zero inline dimension losses
against the paired public source. The remaining twenty states are vertical
DejaVu Sans `in the box` at 18.72px. Native pixels were not captured by this app.
The own-source twelve-row sizing diagnostic has six exact and six different
comparisons, zero errors, matrix exit 1. All six existing differences worsen:
57 to 114 wrong pixels at 1.25x and 384 to 406 at 1.5x. The whole diagnostic
owner exits 6. Complete matrices were not run on this source. The candidate
remains unapplied and unqualified.

A separate public Rust font-relative helper diagnostic on `48af2cf4` compares
2,400 repeated bounds against pinned Chromium at five scales: 1,810 exact and
590 different. `ch` matches 500/800, `ex` 750/800 and `lh` 560/800. The Rust
app runs twice, calls 1,200 Rust callbacks per run, checks owned snapshots and
teardown, and assigns manually resolved pixel values. It does not execute
JavaScript or qualify reactive `ch`/`ex`/`lh` length declarations; those typed
variants are not currently exposed by `LengthValue`.

Two Chromium captures fail because temporary-profile removal runs before the
browser is stopped. Both failed attempts are preserved. A diagnostic wrapper
stops Chromium before profile removal and completes all five repeated capture
profiles. Harness file bytes stay fixed; the wrapper and added
`--disable-dev-shm-usage` switch are recorded as diagnostic conditions.
The original pinned capture code has not been changed by this diagnosis.

Source review identifies shared metric differences: Chromium rounds glyph
advances for fonts without subpixel positioning, floors effective font-cache
sizes to two decimal places, and truncates the numeric line-height basis and
product to its layout grid for `lh`. Source review does not qualify a fix.
Native font-unit corrections have their own source identities and runtime
results; they do not inherit the earlier sizing or Fontations results.

The [completed evidence](generated/native-intrinsic-isolated-v1.json) preserves
both sizing sources, their source patches and exact reconstruction, all four
complete isolated matrices, the narrowed rejected shaping diagnostic, native
consumers and callback logs, every relative-unit residual, pinned source
reviews, failed captures and the successful diagnostic retry. All archive
members are freshly hash-verified. No oracle bytes or release state change.

Public `1dd7a23e` passes all sixteen own-source checks and its three hosted PR
workflows: six executed jobs pass, five skip and none fail. All nine current
native/comparison guards actually execute and pass. Hosted results belong to
that umbrella source and do not qualify private trials. Skips are not release
passes. Full Chromium parity, native API completion and the remaining release
gates stay open.
