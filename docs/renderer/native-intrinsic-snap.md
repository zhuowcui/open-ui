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
before converting the shaped advance to the 1/64px layout grid. Chromium's
`ShapeResult::SnappedWidth` instead ceil-converts the original advance. The
supporting source is Chromium `147.0.7727.24`; the measured reference binary
is `147.0.7727.50`. Source analysis does not replace runtime verification.

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

Workspace, 600 Rust/C/C++ images, 38,400 geometry states and all four renderer
matrices have not executed on this width correction. Own-source hosted run
`37373688613` initially passes five jobs and cancels two before their steps
execute. Its [completed retry](generated/native-intrinsic-snap-v4.json) passes
all seven jobs with zero skips. Both attempts and all available logs remain
preserved. Hosted hardening does not replace the unexecuted full pixel checks.

Whole guard owner `1703` and text/style integration owner `1710` are terminal.
The earlier full owner `1697` and glyph owner `1690` are unlaunched. The
[fresh full preparation](generated/native-intrinsic-snap-v3.json) includes
all 42 prior owners and reuses the unchanged, source-identical actual guard
logs. New owner `1717` is prepared but has not executed its clean build,
native app or four matrices. The exclusive lock covers every stage and
intervening gap. The glyph queue still needs fresh preparation after that owner.

The first preparation has a Python metadata syntax error before execution;
it creates no source root and runs no Cargo or raster command. That failed
script remains preserved alongside the fresh, checked preparation. A separate
guard preparation also has a metadata syntax error before execution; its
failed script is preserved and a fresh preparation supplies the actual guard run.

The parent text/style API correction is now integrated into the umbrella
branch. This narrow width source remains unapplied and unqualified. It does
not establish a fix for the 600 differing native images or assign formal WPT
residual ownership. Accepted
totals remain 21,334/22,924 original and 22,137/23,728 expanded exact, with both
full pixel gates open. No release state is admitted.
