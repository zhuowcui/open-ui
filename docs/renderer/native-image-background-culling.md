# Opaque image backgrounds

Open UI never runs JavaScript. Applications load images, change styles and
handle input through public native Rust methods and Rust callbacks. Chromium
147.0.7727.50 is the separate rendering reference, with zero pixel tolerance.

## What differs

The retained fieldset diagnostic has 128/128 matching bounds and 104/128 exact
images. All 24 differences occur at scale 1.25; repeated native runs and all
512 Chromium captures are stable. The
[v41 review](generated/native-scroll-insets-v41.json) preserves the input
identities, channel pairs, bounds and connected regions.

- Eight zero-padding cases paint the parent's red background under the edge
  of an opaque green image. Chromium skips that obscured background. Away
  from the corner, native pixels are `(126,128,63,255)` where Chromium gives
  `(126,191,126,255)`.
- Sixteen padding cases differ by one green channel value at image edges:
  native `(126,65,0,255)`, Chromium `(126,64,0,255)`. Image coverage packing
  and destination blending need further investigation.

Both groups belong to `openui-paint`. Matching bounds does not satisfy the
pixel gate.

## Candidate and verification

Pinned Blink checks opaque image foreground in
[LayoutImage](https://chromium.googlesource.com/chromium/src/+/refs/tags/147.0.7727.50/third_party/blink/renderer/core/layout/layout_image.cc)
and uses it to skip an obscured background in
[LayoutBox](https://chromium.googlesource.com/chromium/src/+/refs/tags/147.0.7727.50/third_party/blink/renderer/core/layout/layout_box.cc).
Our prior covering-child check only recognizes an opaque child background.

Private source `d913041d` also recognizes a decoded opaque image covering the
whole box. It preserves backgrounds when coverage cannot be established,
including partial size, padding, borders, transparency, effects and object
position. This uses document resources and shared paint logic; it contains no
test-ID selection or output-pixel patch.

An Engine regression and neighboring guards cover that proof. Two consuming
native Rust applications use real PNG bytes, typed setters, layout barriers,
Rust click callbacks, owned bounds and teardown checks. The generated field
inventory records the new `shape-outside` consumer exactly. Nine initial
read-only checks passed and one caught the stale inventory; all ten pass
after regeneration.

All seven manual hardening jobs pass on the culling source `d913041d`, with
zero skips. Its local Engine, application and pixel verification remains
queued. That hosted result does not qualify a later source.

Builds and image sweeps are queued after every command in all earlier whole
pipelines. Required verification includes the baseline assertion failure,
fixed and neighboring guards, clean workspace and ABI consumers, repeated
Chromium captures, focused and primitive matrices, and complete original and
expanded runs. The rounding difference remains open. The candidate is
unapplied and unqualified; no image improvement or release admission is claimed.

## Image-edge candidate

The [v42 evidence](generated/native-scroll-insets-v42.json) preserves a separate
candidate at `2eacae2c`. It samples the image without packing geometric coverage
into its colors, preserves opaque-source metadata, and draws the visible
destination with Skia antialiasing. A rectangular content clip and the image
destination share one contour. Color-managed samples are converted before
that draw. Existing background and broken-image patch policies are retained.

Pinned Chromium's [software renderer](https://chromium.googlesource.com/chromium/src/+/refs/tags/147.0.7727.50/components/viz/service/display/software_renderer.cc)
draws the sampled image with destination geometry and paint blending.
Five relevant Skia files, including the packed-color interpolation routines,
are byte-identical between Chromium's pinned `abbe599f` source and our existing
rust-skia checkout. This is a comparison of those five files, not proof that
the entire Skia source or build is identical.

Applying edge coverage during blending is a source-supported explanation
for the one-channel differences. Native execution must confirm it. The Engine
baseline preserves exact Chromium edge and corner values and requires an
actual assertion failure; compilation errors cannot stand in for that proof.
The fixed source must also preserve white surfaces and translucent backing.

Ten read-only checks pass. Required execution covers 128 retained fieldset
images, 720 public Rust callback images at five scales, clean workspace and
ABI checks, and all four renderer matrices. The public app includes opaque,
white, translucent, transparent, clipped and effect cases, with repeated
native processes, repeated Chromium captures, owned bounds and teardown.
The source is unapplied and unqualified. No exact gain is claimed.
