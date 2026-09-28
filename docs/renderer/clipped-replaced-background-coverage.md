# Clipped replaced background coverage

Chromium is the pixel target. A replaced element with an opaque background and
clipped overflow can apply fractional coverage twice: once at the box contour
and once at the replaced-content clip. At a half-device-pixel edge, the
resulting background coverage is one quarter. The shared physical paint path
had packed that quarter as 65/255 by adding one after multiplying by 256.
Chromium's output corresponds to 64/255. The paint path now packs the product
once, without that extra increment. This rule depends on geometry and clip
state, not a test ID or an expected screenshot.

The reduced visual state is a 60×60 green PNG in a green image element with
40 CSS pixels of right padding, inside a `margin-trim: block` container. At
1280×720 and 1.25× scale, the old render differed on the two 50-pixel
padding-edge rows: `(190, 223, 190, 255)` where Chromium had
`(191, 223, 191, 255)`. Both `margin-trim_block-container-replaced-block`
and `margin-trim_block-container-replaced-block-start` are now exact at all
four required profiles.

The clean [v28 census](generated/four-profile-census-v28.json) at `a0e3f4cd`
measured 21,227/22,924 exact, 1,697 different, and zero errors. Against v27,
exactly three Open UI decoded images changed: those two margin-trim images
became exact, and `background-margin-iframe-root-ref` at 1.25× fell from 342
wrong pixels to six. No previously exact comparison regressed. All 22,924
Chromium oracle identities and decoded hashes stayed fixed. The six remaining
iframe pixels are at inset-border corner intersections and remain a separate
unowned raster difference. The release census still fails, with 937 unowned
residual test IDs.

The clean [v29 raster index](generated/focused-primitive-raster-v29.json)
remains 640/640 focused and 960/960 primitive exact. All 1,600 Open UI and
Chromium decoded image hashes match the preceding clean raster run. The clean
[v14 expanded requalification](generated/expanded-requalification-v14.json)
measured 22,024/23,724 exact and zero errors. The 200 native additions kept
their prior pixels and statuses: 197 are exact at all four profiles and three
remain demoted. No historical Open UI archive byte, Chromium oracle,
tolerance, or post-raster output was changed.
