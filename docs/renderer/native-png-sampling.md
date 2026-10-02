# Native PNG application and decoder investigation

Open UI never executes JavaScript. The consuming
[Rust application](../../bindings/rust/openui/examples/native_png_sampling.rs)
loads PNG bytes, registers them with `Document::register_image_resource`, and
uses public typed methods to display a replaced image, background image, or
border image. A Rust click callback changes width, height, and opacity after
the first rendered frame. The app reads owned bounds and verifies that its
weak element handle cannot keep the document alive after teardown.

The current branch builds this consumer under the checked-in Chromium Cargo
configuration. All 540 runs execute the callback exactly once, release the
document, and match Chromium's element bounds. After the raster opacity
correction, 148 images are pixel-exact.
Working application APIs do not establish pixel qualification.

For example, from `bindings/rust`, the retained green fixture can be rendered
and mutated through the native callback with:

```sh
cargo --config .cargo/config.chromium.toml run --locked -p openui \
  --example native_png_sampling -- /tmp/openui-native-png 1 \
  ../../docs/renderer/evidence/native-png-sampling-v1/assets/green.png \
  38a9a0ea560a60b9ce79be68126b1e57bbbbcab0c013b9893f4f43fce7ebc3c4 \
  60 60 60 60 0 replaced 1
```

## Measurements

The [manifest](evidence/native-png-sampling-v1/cases.json) contains 108
script-free states at scales 1, 1.25, 1.5, 2, and 3. It covers source-size,
downscaled, and enlarged PNGs, two origins, opacity, and three image roles.
The five source PNGs retain their original bytes, including gamma, ICC, and
alpha metadata. Each separate Chromium capture and geometry query repeats
identically. Open UI creates every state with Rust operations.

| Diagnostic implementation | Exact pixels | Exact owned bounds and native callback checks |
|---|---:|---:|
| Previous renderer `9b158cda` | 74/540 | 540/540 |
| Current branch renderer `42cce619` | 148/540 | 540/540 |
| Earlier unapplied renderer prototypes, N32 image patches | 98/540 | 540/540 |
| RGBA image patch format experiment, rejected | 95/540 | 540/540 |
| Neutral PNG gamma interpretation prototype | 146/540 | 540/540 |
| Raster decoration opacity with the earlier prototypes | 196/540 | 540/540 |
| Clean isolated raster opacity qualification source `fccbcccb` | 148/540 | 540/540 |

These are diagnostic cases; none has been admitted to the release manifest.
The [evidence index](generated/native-png-sampling-v1.json) records source,
executable, input, image, and report hashes, plus observed process exits.

## Reviewed decoder difference

Chromium uses `SkPngRustDecoder`. Its pinned `SkPngRustCodec` ignores a neutral
gamma value when no higher-precedence color profile or chromaticity is
present: `0.95 < gamma * 2.2 < 1.05`. Blink then treats the source as sRGB.
The libpng codec used by the current rust-skia build instead creates a custom
gamma profile. For the unscaled blue/orange PNG, this changes the orange green
channel from Chromium's 165 to Open UI's 166.
See the pinned [PNG decoder](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/platform/image-decoders/png/png_image_decoder.cc),
[Skia profile selection](https://skia.googlesource.com/skia/+/abbe599fb3c0ef2fa82bfadbb0ddcd321f22faf0/src/codec/SkPngRustCodec.cpp),
and [Blink image color tagging](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/platform/image-decoders/image_decoder.cc).

The [unapplied decoder patch](evidence/native-png-sampling-v1/png-neutral-gamma-prototype-v2.patch)
implements this shared metadata rule. It bounds PNG chunk reads, checks the
gamma chunk's CRC, preserves higher-precedence metadata, and reinterprets
the original decoded channels as sRGB. It does not change encoded resources
or reference pixels and has no test-ID or filename condition.

Against the preceding N32 prototype, all 120 changed native images improve,
48 become exact, and no previously exact image regresses. All 540 bounds
remain exact. The two metadata tests cover profile precedence, invalid gamma,
corrupt CRCs, and truncated or oversized chunks. The 44 existing image guards
are unchanged, at 20/44 exact; four original comparisons that use a neutral
gamma resource remain exact and byte-identical to the accepted C9 renderer.

The RGBA patch format experiment is rejected. It improves 13 existing failing
comparisons but worsens ten others at a different scale. It also regresses
three previously exact native images. Changing the stored image representation
does not establish correct sampling and compositing behavior.

## Reviewed opacity difference

Raster backgrounds and border images were missing from the sampled-image
opacity classification. The resulting F16 intermediate changes fully covered
PNG colors. For the green source at opacity 0.5 over white, Chromium produces
`#7ebf7e`; the F16 path produces `#80bf80` at all five measured scales.

The [isolated patch](evidence/native-png-sampling-v1/raster-image-opacity-standalone-v6.patch)
uses the ordinary packed N32 intermediate for replaced images, raster
backgrounds, and raster border images, including inside an ancestor's opacity
group. It changes shared paint behavior, adds no build flags, and has no
filename or test-ID condition. Its regression test paints a raster background
inside a translucent parent at five scales.

The [completed measurements](generated/native-png-sampling-v3.json) improve
all 270 changed PNG comparisons, make 74 more exact, and preserve all 540
native callbacks, owned bounds, and teardown checks. The existing image-opacity
consumer improves all 160 comparisons, making 109 exact. The 180 failed-image
and clipping controls gain 27 exact results, with no loss of an exact result.
One existing failure gets worse, from 83 to 105 differing pixels, and remains
open under `openui-paint` ownership.

All 8,492 workspace tests pass, with 13 ignored. The focused and primitive
40-profile gates pass at 640/640 and 960/960, with every PNG and decoded pixel
unchanged from C9. The 1,256 original image and opacity guards are also
unchanged. Both complete clean censuses have finished: 21,308/22,924 original
and 22,111/23,728 expanded comparisons are exact, with zero errors. Every
Open UI and Chromium image and oracle identity remains unchanged from C9;
all original rows agree between the two suites. The same 200 of 201 additions
are exact across four profiles. The failing fieldset addition remains in the
contract.

The opacity correction is committed on the umbrella branch at `42cce619`.
The full matrices ran on clean `fccbcccb`; its code and build inputs match
the umbrella checkpoint, with three documentation files differing. A fresh
clean umbrella build reproduces all 880 native images and owned bounds.
This attribution does not claim a full matrix ran on the umbrella source
identity. The older [v2 evidence](generated/native-png-sampling-v2.json) and
patch remain historical records. The original pixel gate and the worsened
failed-image edge remain open; this checkpoint is not release qualification.

Two separate edge trials are rejected: adding an analytic AA clip worsens
51 native comparisons, and drawing an AA shader rectangle worsens 33. Neither
increases the native suite's exact count. Their measurements are preserved
in the earlier evidence index. A later RGBA root-surface trial loses 215 exact
native results and is rejected. An opacity-bounds hint and a packed silver
stroke-color trial change no native pixels and are also left unapplied.

The four audited Skia clipping, canvas, and raster-blitter source files have
identical bytes at the Chromium pin and installed rust-skia revision. This
rules out a source-version difference in those files; it does not establish
the remaining edge's cause. Their pinned source hashes are in the v3 index.

## Remaining work

The decoder patch was measured together with other unapplied renderer and
build changes. Its standalone release build and complete clean renderer
matrices remain unqualified. The isolated opacity change still has 392 native
PNG failures; the combined diagnostic has 344. Remaining sampling, color
conversion, coverage, and compositing
causes need minimized evidence and review in `openui-paint`; not every cause
has been identified. The authoritative original census remains
21,308/22,924 exact. Needed native APIs and exact pixels remain separate
implementation obligations.
