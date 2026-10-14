# Resized one-axis bitmap pattern at fractional scale

Chromium is the pixel target. In the `background-repeat-round-2` and
`background-repeat-round-3` test/reference pairs, a 32×32 px PNG is painted
as a 36×36 px background tile and repeated along one axis. A separate linear
gradient in each fixture was addressed in the preceding
[picture-shader investigation](one-axis-gradient-picture-shader.md). The
remaining difference was on the PNG image.

Pinned Chromium's `third_party/blink/renderer/platform/graphics/image.cc`
uses `Image::DrawPattern` to paint a tiled bitmap with one image shader over
the destination. Open UI had resolved each repeated tile into a separate
physical patch. At fractional device scales, those per-tile draws lost the
shared pattern's edge and sampling phase. The retained change draws a resized,
one-axis repeated bitmap with one image shader over the whole background
destination. It applies the destination's physical edge coverage once. Native
size repeats retain the packed-image path: a broad diagnostic of the shared
shader worsened three native-size repeat cases. Integer device scales retain
their existing path, which already matched the affected PNG at 1× and 2×.
The selection uses image geometry and device scale, not test IDs or reference
pixels.

The clean [v26 four-profile census](generated/four-profile-census-v26.json) at
`954648fb` compared all 22,924 original images with the same pinned Chromium
oracle as [v25](generated/four-profile-census-v25.json). Exactly eight Open UI
decoded images changed, all in these four test/reference pairs at 1.25× and
1.5×. Every changed image moved closer to Chromium, two comparisons became
exact, and none regressed. All 22,924 Chromium oracle identities and decoded
hashes stayed fixed. A diagnostic guard covered 845 `css_backgrounds` IDs at
all four profiles and found no regression. The clean focused and primitive
40-profile matrices remain 640/640 and 960/960 exact, with all 1,600 Open UI
and Chromium decoded images unchanged. The selected 200 native additions
remain 797/800 exact, with all 800 Open UI and Chromium decoded images
unchanged.

| Fixture pair | 1280×720@1.25 wrong pixels | 1920×1080@1.5 wrong pixels |
|---|---:|---:|
| `background-repeat-round-2` and `-2-ref` | 93 → 0 | 98 → 3 |
| `background-repeat-round-3` and `-3-ref` | 94 → 1 | 98 → 2 |

The 1.25× `-3` images retain one red-channel difference in the gradient.
The 1.5× images retain two or three one-level bitmap color differences at
sampling and edge positions. Those comparisons remain failures. The full
renderer gate is now 21,225/22,924 exact, 1,699 different, zero errors, with 939
residual test IDs lacking reviewed ownership. No frozen reference, Chromium
oracle, tolerance, or post-raster pixel output was changed.
