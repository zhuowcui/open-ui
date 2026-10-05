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
record thirteen passing read-only checks. Native baseline/fixed guards,
workspace, 600 Rust/C/C++ images, 38,400 geometry states and all four renderer
matrices have not executed. Own-source hosted run `37373688613` is pending.
Whole owner `1697` is prepared but unlaunched; it waits for complete owner
`1678` and holds the build/raster lock throughout its stages and intervening
gaps. The unlaunched glyph queue must be prepared again after this owner.

The first preparation has a Python metadata syntax error before execution;
it creates no source root and runs no Cargo or raster command. That failed
script remains preserved alongside the fresh, checked preparation.

This source is unapplied and unqualified. It does not establish a fix for the
600 differing native images or assign formal WPT residual ownership. Accepted
totals remain 21,334/22,924 original and 22,137/23,728 expanded exact, with both
full pixel gates open. No release state is admitted.
