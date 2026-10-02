# Generated background image sampling

Chromium remains the sole pixel target. Open UI executes no JavaScript;
applications construct and change backgrounds through public native Rust APIs.
This investigation concerns the shared paint path after layout.

## Reviewed difference

Enabling Chromium's `SK_ENABLE_LEGACY_SHADERCONTEXT` setting in the private
renderer candidate repairs native scrollbar sampling but loses 23 previously
exact Chromium comparisons. The complete failed censuses and the
configuration-only reproduction are preserved in the
[earlier evidence](generated/native-viewport-full-v14.json).

Chromium records generated background patterns as pictures. Its pinned
[generated-image implementation](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/platform/graphics/generated_image.cc)
creates a picture shader. The pinned
[SkPictureShader cache](https://skia.googlesource.com/skia/+/abbe599fb3c0ef2fa82bfadbb0ddcd321f22faf0/src/shaders/SkPictureShader.cpp)
stores an eight-bit tile as premultiplied RGBA8888, with the destination color
space or sRGB when the destination is untagged. On the declared Linux profile,
that image format bypasses the native-N32-only legacy image shader context.

Open UI's concrete generated-tile path instead allocated native N32. Adding
the legacy setting therefore also changed its gradient sampler. The
[isolated format correction](evidence/native-viewport-scroll-v1/generated-tile-format-prototype-v150.patch)
changes that temporary allocation to premultiplied sRGB RGBA8888 on the
measured eight-bit untagged canvas. It retains the existing tile dimensions,
phase, gradient painting, and repeat mapping. Encoded resources and Chromium
reference images retain their original bytes.

## Measured result

Clean private source `16fc959d` repairs all 18 generated-background exact
regressions in the 76-ID, four-profile selection. The selection finishes with
168/304 exact, 136 different, zero errors, and observed exit 1. Twenty-eight
images change; no differing-pixel count worsens against the preceding private
candidate. All 850 native viewport images, geometry, callbacks, and document
teardown checks remain exact and unchanged.
The complete focused and primitive matrices also finish with observed exits 0:
640/640 and 960/960 exact over their 40 profiles. All 1,600 Open UI images,
Chromium images, oracle identities, and difference signatures are unchanged.

Five exact regressions against the accepted renderer still prohibit promotion:

- `css_flexbox/overflow-area-001` and `overflow-area-002` at scales 1.25 and 1.5;
- `css_backgrounds/tiny-foreignObject-double-border-radius-crash` at scale 1.

These remain owned by `openui-paint`. The global sampling setting is isolated
as their cause; correct operation-specific sampling and backing transforms
remain unresolved. Their bounds, connected regions, and signed channel deltas
are retained in the [new evidence](generated/native-viewport-full-v15.json).
The selected suite has incomplete release scope. It does not establish a new
full-census exact total or admit any release case.

A second clean source `312806f6` uses Chromium SoftwareRenderer's strict
image-rectangle draw for scroll backing replay. All 304 selected comparison
invariants and all 850 native controls remain identical to `16fc959d`. That
[draw-operation trial](evidence/native-viewport-scroll-v1/scroll-image-replay-prototype-v155.patch)
does not repair any of the five remaining regressions and stays unapplied.

Both trials remain outside the umbrella renderer. The sampling setting and
prior solid-tile/corner changes remain unapplied. Qualification still requires
complete original and expanded censuses without regressions, exact focused
and primitive matrices, and review of every changed image. The accepted
complete renderer evidence remains 21,308/22,924 original and 22,111/23,728
expanded exact, with zero errors. The original pixel gate still fails.
