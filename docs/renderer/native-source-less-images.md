# Native source-less image drawing

Open UI executes no JavaScript. The consuming Rust application constructs
these states through public `Document` and `Element` methods, changes typed
styles and attributes, and reads owned geometry. Chromium is the separate
pixel and geometry reference.

The [native app](../../bindings/rust/openui/examples/native_opacity_clip.rs)
accepts a child kind, clipping, opacity, position, size and decoration. An
image with no `src` attribute and an image with `src=""` are separate controls.
The checked-in [case manifest](evidence/native-source-less-images-v1/cases.json)
contains 360 script-free minimized inputs and their native arguments.

## Drawing differences

Chromium's `ImagePainter` first tests the logical content dimensions. A
source-less image with either dimension at most 2 CSS pixels has no fallback
frame. For larger content, it snaps the four content edges independently,
then strokes the frame inside those bounds. Rounding width separately changes
the right edge for fractional sizes such as 2.5 CSS pixels.

Chromium paints the image background and authored border under the decoration
clip, restores that clip, then paints the foreground frame. A rounded image
still clips its foreground to the inner rounded rectangle. A failed image
with an empty `src` has a different fallback glyph and clipping path; it must
retain those operations.

The [diagnostic index](generated/native-source-less-images-v1.json) records
the pinned Chromium sources and picture command traces. Every traced capture
matches its retained Chromium reference. The
[unapplied V3 patch](evidence/native-source-less-images-v1/renderer-prototype-v3.patch)
implements the shared image drawing rules, paired edge snapping and the
earlier opacity/clip prototype. It contains no test-ID or fixture-specific
pixel correction.

## Measured prototype results

These measurements use the same public native application and immutable
Chromium inputs at 320×240 CSS pixels and scales 1, 1.25, 1.5, 2 and 3.
The baseline is the preceding opacity prototype, not the clean release
renderer.

| Diagnostic suite | Before | V3 exact | Owned bounds exact |
|---|---:|---:|---:|
| Decorations, clipping and opacity | 836/1,200 | 1,118/1,200 | 1,200/1,200 |
| Small and fractional content dimensions | 255/600 | 555/600 | 600/600 |
| Combined | 1,091/1,800 | 1,673/1,800 | 1,800/1,800 |

The prototype improves 591 comparisons, makes 582 comparisons exact and
worsens none of these reduced cases. Source-less images are 891/900 exact;
the remaining nine involve transparent-frame or rounded-clip opacity. The
other 118 failures are empty-source fallback images. All Chromium inputs and
reference pixels are unchanged. The V3 locked workspace passes 8,491 tests,
with zero failures and 13 ignored tests. Its paint test output was preserved
and the tracked historical image restored before later source snapshots.

The subsequent generated-tile format correction leaves all 1,800 native PNGs
and owned bounds unchanged. It restores 18 of the earlier global prototype's
23 exact regressions in the 304 affected-profile guards. Five exact
regressions remain at that checkpoint. This does not establish a new complete
original census.

The later visible-quad opacity correction follows Chromium's choice of `Src`
for opaque compositor quads and `SrcOver` for translucent quads. It checks
the exposed source region and neighboring sampling taps without changing
pixels. Together with the generated-tile format correction, it restores
22 of the V2 prototype's 23 exact regressions in 312 affected guards. Those
guards are 178/312 exact, with zero errors and unchanged Chromium references.
One formerly exact SVG decoration still differs by one level in each RGB
channel of one pixel; 13
previously failing comparisons also worsen against the clean C9 renderer.
The source and executable checks pass, but this is rejected development
evidence and the renderer remains unapplied.

The SVG decoration path also contains an existing `49/50` opacity adjustment.
Its comment alone does not establish a Chromium paint rule. That adjustment
needs source and command-trace review and a shared geometry/compositing repair;
a coincidentally exact screenshot cannot justify a pixel correction.

## Release status

The renderer patch remains unapplied. Its Skia build flag and opacity changes
must pass the complete Chromium matrix before adoption. The
[global regression report](generated/native-opacity-full-regression-v1.json)
records the original prototype's failed census. The separate complete
source-less V2 census is 21,302/22,924 exact, with 1,622 differences, zero
errors and 23 formerly exact regressions. Its executable source check passes
and all Chromium references are unchanged, but those regressions prevent
adoption. That run does not establish V3 qualification.
These reduced cases are not admitted release cases. The clean `9b158cda`
renderer remains 21,308/22,924 exact, with 1,616 differences and zero errors.

To construct a native diagnostic state with the current renderer:

```bash
cd bindings/rust
cargo run --locked -p openui --example native_opacity_clip -- \
  /tmp/openui-native-image 1.25 no-source parent self50 0.5 24 80 rounded
```

The application writes `geometry.json` and `openui.png`. A build of the
current renderer is not a build of the unapplied prototype; qualify each
executable against its recorded source identity.
