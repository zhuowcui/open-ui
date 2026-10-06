# Native intrinsic text widths

Open UI applications change elements through public Rust methods and Rust
callbacks. Open UI never runs JavaScript. Pinned Chromium defines the expected
geometry and pixels; older Open UI output does not define correctness.

## Measured gap

The [complete native text matrix](generated/native-text-inheritance-v2.json)
at `41b616c3` matches 34,560/38,400 Chromium geometry states. The remaining
3,840 differences affect width alone: Ahem text at sizes 12 and 16 is exactly
1/64 CSS pixel short. Rust, C and C++ agree with each other. All repeated runs
and pinned Chromium inputs remain unchanged.

For a single `X` at 12px, Chromium's natural width is 12.015625px; native
width is 12px. For `XX` at 16px, the widths are 32.015625px and 32px. These
measurements agree at all five scales. No width is explicitly authored.

The shared `intrinsic_text_width` helper discards small positive fractions
before converting an advance to the 1/64px layout grid. Chromium's
`ShapeResult::SnappedWidth` instead ceil-converts the original shaped advance.
The [preserved source](generated/native-intrinsic-snap-v9.json) is Chromium
`147.0.7727.50`, matching the measured reference binary. Source analysis does
not replace runtime verification.

This helper receives both shaped advances and the fallback's simpler
`SkFont::measure_str` widths. That fallback bypasses HarfBuzz. The pinned
Chromium and SkShaper sources also use different float-to-integer conversions
for HarfBuzz glyph advances: Chromium clamps the 16.16 value to an integer;
SkShaper rounds it. These source differences identify measurement paths to
test. They do not establish the runtime cause of the 21 exact pixel losses.

## Narrow native correction

Private `727da10e` changes that one production helper to preserve the shaped
fraction through the grid ceiling. It adds no family, size, fixture or test-ID
condition. It changes no font assets, reference bytes, paint implementation,
C export or struct layout. All 113 exports and 30 layouts remain.

A new consuming Rust guard creates one naturally sized label at 12px. A Rust
click callback changes it to 16px and replaces `X` with `XX`. It checks both
measured widths, position, height, callback count, text content, the earlier
owned bounds and teardown at five scales. Test-only baseline `e995e52f` and
the corrected source contain identical guard bytes. The existing helper test
now expects the original positive fraction to survive rounding.

The [preserved source and probes](generated/native-intrinsic-snap-v1.json)
record thirteen passing read-only checks. The
[completed native guard evidence](generated/native-intrinsic-snap-v2.json)
reproduces the named baseline failure with exit 101; the fixed source passes
the consuming Rust callback test at all five scales. C text parity, the
10,000-update storage guard, the earlier five-scale text-style callback,
58 native conformance scenarios and all nine inherited-style guards also pass.

The [completed native stages](generated/native-intrinsic-snap-v5.json) pass
all thirteen clean build stages and 8,552 workspace tests, zero failed and
13 ignored. Chromium geometry is now 38,400/38,400 exact: all 3,840 measured
width differences close. Images remain 0/600 exact and the native app gate
exits 1. All 400 actual C/C++ images match Rust; 200 Rust self-rows are excluded.
The focused 640/640 and primitive 960/960 image suites pass. The
[complete matrices](generated/native-intrinsic-snap-v7.json) finish at
21,313/22,924 original and 22,116/23,728 expanded exact, zero errors, actual
exits 1. Each loses 21 exact comparisons and gains none. The source is rejected
and remains unapplied. Own-source hosted run
`37373688613` initially passes five jobs and cancels two before their steps
execute. Its [completed retry](generated/native-intrinsic-snap-v4.json) passes
all seven jobs with zero skips. Both attempts and all available logs remain
preserved. Hosted hardening does not waive the measured pixel losses.

The [native width-only audit](generated/native-intrinsic-snap-v6.json) confirms
that all 600 native image hashes, all Chromium image hashes and all image
difference analyses remain unchanged. All 38,400 Chromium bounds stay fixed.
The only native geometry changes are the 3,840 widths, each increased by
exactly 1/64 CSS pixel. The glyph pixel discrepancy remains open.

Whole guard owner `1703` and text/style integration owner `1710` are terminal.
The earlier full owner `1697` and glyph owner `1690` are unlaunched. The
[fresh full preparation](generated/native-intrinsic-snap-v3.json) includes
all 42 prior owners and reuses the unchanged, source-identical actual guard
logs. New owner `1717` has finished its guards, clean build, native app,
focused and primitive matrices and both complete censuses. All seven stages
are terminal. The exclusive lock covered every stage and intervening gap.
The subsequent glyph descriptor guard runs on its fresh corrected source.

The first preparation has a Python metadata syntax error before execution;
it creates no source root and runs no Cargo or raster command. That failed
script remains preserved alongside the fresh, checked preparation. A separate
guard preparation also has a metadata syntax error before execution; its
failed script is preserved and a fresh preparation supplies the actual guard run.

The parent text/style API correction is now integrated into the umbrella
branch. This narrow width source remains rejected and unapplied. It does
not establish a fix for the 600 differing native images or assign formal WPT
residual ownership. Accepted
totals remain 21,334/22,924 original and 22,137/23,728 expanded exact, with both
full pixel gates open. All 48,252 Chromium input rows remain fixed, and all
22,924 original rows agree with expanded. The 804 additions stay unchanged,
with 200/201 exact at all four profiles. The trial changes 27 original rows
across 20 test IDs; 21 previously exact comparisons become different, while
six already-different rows change. Their reduced root causes still need
review; no formal WPT ownership is assigned. Correct native geometry does
not waive these pixel losses. No release state is admitted.

The [complete image audit](generated/native-intrinsic-snap-v8.json) verifies
57 existing PNG files against their recorded PNG and decoded RGBA hashes.
Every changed comparison has mismatch bounds, connected regions and channel
deltas against Chromium and against the prior native render. All 20 affected
tests have complete four-profile summaries: one change occurs at scale 1,
nine at 1.25 and seventeen at 1.5; scale 2 is unchanged. These are image
observations. Reduced Engine-backed reproductions and reviewed causes are
still required before changing the shared text measurement or allocation path.
