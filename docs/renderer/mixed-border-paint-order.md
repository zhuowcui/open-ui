# Mixed border paint order

Chromium is the pixel target for this investigation. The compact
`ttwf-css3background-border-style-shorthand-missing-bottom` fixture uses a
5 px black border with `border-style: solid dotted`: the horizontal sides are
solid and the vertical sides are dotted. The neighboring `border-style`,
`border-style-shorthand`, and `border-style-shorthand-missing-left` fixtures
use solid, dotted, dashed, and double sides. All four belong to the immutable
original manifest; their HTML and Chromium captures were left unchanged.

The pinned Blink `box_border_painter.cc` sorts visible sides by alpha, then
style, then side. Dotted, dashed, and double sides paint before solid sides.
Its `ComputeMiter` skips a corner clip when a later, filled adjacent side
will overdraw that corner. Open UI had used the fixed top, bottom, right,
left order for every style and clipped the dotted right side regardless of
the adjacent solid sides. A diagnostic that only removed antialiasing from
the dotted clip improved the main cases but worsened the solid-bottom case
at fractional scales. The retained fix orders sides by the Blink rule and
lets a dotted right side use the later solid sides for corner coverage when
both are opaque. It changes shared border painting, with no test-ID branch or
pixel rewrite.

The clean [v24 census](generated/four-profile-census-v24.json) at
`2f560e46` compared all 22,924 images against the same Chromium oracle as
the [v23 census](generated/four-profile-census-v23.json). Exactly 16 Open UI
images changed, all in these four fixtures. Four comparisons became exact,
none regressed, and the remaining twelve changed comparisons moved closer
to Chromium. All 22,924 Chromium oracle identities and decoded hashes stayed
fixed. The `border` name guard covered 370 tests at four profiles and had no
regressions. The clean focused and primitive matrices remain 640/640 and
960/960 exact, with all 1,600 decoded images unchanged. The 200 native
additions also retain their prior statuses and decoded images.

| Fixture | 800×600@1 | 375×667@2 | 1280×720@1.25 | 1920×1080@1.5 |
|---|---:|---:|---:|---:|
| `border-style` and `border-style-shorthand` | 24→4 | 93→0 | 387→196 | 480→235 |
| `border-style-shorthand-missing-left` | 24→4 | 93→0 | 382→191 | 477→232 |
| `border-style-shorthand-missing-bottom` | 24→0 | 132→20 | 641→572 | 588→472 |

Values in the table are differing pixel counts before and after the fix.
The remaining fractional-scale pixels still require a separate raster and
corner-coverage investigation. No residual in this family is counted as an
exact pass unless its complete image equals Chromium at that profile. The
full release gate remains red at 21,223/22,924 exact, with 1,701 differences
and 939 residual test IDs lacking reviewed ownership.
