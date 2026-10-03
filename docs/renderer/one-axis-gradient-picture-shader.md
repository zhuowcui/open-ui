# One-axis repeated gradient picture shader

Chromium is the pixel target. The `background-repeat-round-2` and
`background-repeat-round-3` test/reference pairs each paint a 72×72 px box
with a 1 px solid border, one repeated PNG background, and one repeated
linear-gradient background. The gradient tile is 36×36 px after `round`
adjustment in the tests and is an explicit 36×36 px `repeat-x` or `repeat-y`
tile in the references. The other axis does not repeat. These are native
renderer fixtures; Open UI runs no JavaScript.

Pinned Chromium's `third_party/blink/renderer/platform/graphics/generated_image.cc`
records `DrawTile` in `GeneratedImage::CreateShader` into a paint picture and
creates a repeating paint-record shader. Open UI had
pre-rasterized the generated gradient into a physical bitmap before repeating
it. At fractional device scales, that changed the ordered color evaluation
throughout the gradient interior. The retained change records one linear
gradient tile as a picture shader for one-axis `repeat` or `round`, and applies
physical coverage at a fractional rectangular clip. It changes shared
background painting, with no test-ID branch, tolerance, oracle edit, or
post-raster pixel rewrite.

The clean [v25 four-profile census](generated/four-profile-census-v25.json) at
`5c7aaa9c` compares all 22,924 original images against the same pinned
Chromium oracle as [v24](generated/four-profile-census-v24.json). Exactly eight
Open UI decoded images changed: the four test/reference pairs at 1.25× and
1.5×. Each moved closer to Chromium; no comparison became exact and none
regressed. All 22,924 Chromium oracle identities and decoded hashes stayed
fixed. The complete `css_backgrounds` diagnostic covered 845 IDs at all four
profiles and found no regression. The clean focused and primitive gates remain
640/640 and 960/960 exact, with all 1,600 Open UI and Chromium decoded images
unchanged. The selected 200 native additions remain 797/800 exact, with all
800 Open UI and Chromium decoded images unchanged.

| Fixture pair | 1280×720@1.25 wrong pixels | 1920×1080@1.5 wrong pixels |
|---|---:|---:|
| `background-repeat-round-2` and `-2-ref` | 3,644 → 93 | 4,422 → 98 |
| `background-repeat-round-3` and `-3-ref` | 3,531 → 94 | 4,493 → 98 |

The gradient interior now matches Chromium in these images. The remaining
pixels are on the separate PNG image: at 1.25×, a partly covered terminal row
or column and a few neighboring edge pixels; at 1.5×, two side columns with a
one-level green-channel difference. A trial that removed the PNG patch's hard clip
improved the terminal row but worsened the 1.5× edges, so it was reverted.
This PNG raster/coverage residual remains open and unowned for qualification.
The release gate remains red at 21,223/22,924 exact, 1,701 different, zero
errors, and 939 residual test IDs without reviewed ownership.
