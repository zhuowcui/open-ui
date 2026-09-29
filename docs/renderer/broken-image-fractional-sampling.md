# Broken-image icon at fractional device scale

Chromium 147 remains the sole pixel target. This investigation began with the
clean [v31 census](generated/four-profile-census-v31.json) at `9534c9f6`.
The two diagnostic changes below were reverted. A subsequent shared sampler
fix was verified at clean checkpoint `09683348`.

Five original cases share the same 1.5× mismatch:
`css_sizing/contain-intrinsic-size-012`, `-017`, `-020`, `-027`, and
`grid-item-image-percentage-min-height-computes-as-0`. Each differs by 100
pixels in the physical rectangle x=31..52, y=30..53. The largest channel
differences are R=10, G=8, B=11; alpha is unchanged. The same five are exact
at 1×, 1.25×, and 2×. Both captured images show Chromium's broken-image icon
in this rectangle, despite the fixture referring to `resources/dice.png`.
This is a fallback-image paint mismatch; sampling the dice PNG cannot repair
these captured pixels.

`LayoutImageResource::BrokenImage` in the pinned Chromium source chooses its
100% image below device scale 2 and its 200% image at or above 2. Open UI's
`broken_image_resource` uses pinned copies of those same resources. The
remaining difference was in drawing the 100% resource into the 16 CSS-pixel
fallback slot at 1.5×. That narrowed ownership to
`openui-paint::paint_missing_image` and its physical image sampler.

As a diagnostic, `paint_missing_image` encoded its 1/16 destination phase in
source coordinates at fractional scales as it already does at integral scales.
The 53 nearby sizing test IDs were compared at all four required profiles.
The five 1.5× differences shrank from 100 to 89 pixels each, but those same
five regressed from exact to 142 differing pixels each at 1.25×. Selected
exact comparisons fell from 204/212 to 199/212. No Chromium oracle identity
or decoded hash changed. The [diagnostic index](generated/broken-image-fractional-phase-diagnostic-v1.json)
records all ten changed Open UI comparisons and their decoded hashes.

Drawing the same resource directly through CPU Skia instead of the physical
sampler was a second diagnostic. It changed only these five images in each
profile, but made every one different: 85 pixels at 1×, 273 at 2×, 255 at
1.25×, and 386 at 1.5×. The selection fell to 189/212 exact, with no Chromium
oracle change. The [direct-draw index](generated/broken-image-direct-skia-diagnostic-v1.json)
records the 20 changed comparisons. This edit was also reverted.

## Shared sampler repair

Chromium's pinned Skia code maps the first sample through a 32-bit float
inverse matrix, converts it to signed 32.32 fixed point, and advances each
horizontal sample by a fixed step. The former Open UI sampler recalculated
each sample from a higher-precision ratio. At 1.5× the 14-pixel image spans
24 physical pixels; the difference puts every third column on the wrong side
of a 1/16 filter-weight boundary. At physical columns 1, 4, and 7 relative
to the icon, Chromium's weights are 5, 1, and 13; Open UI previously used
6, 2, and 14. At 1.25×, the checked column 7 retains weight 12.

The fractional-scale fallback-image path now uses that fixed-point coordinate
sequence before the existing packed bilinear filter. Integral-scale sampling,
the pinned image bytes, and the Chromium oracle remain unchanged. A focused
unit test guards those filter phases. This is a resource-class raster rule,
not a test-ID or post-raster pixel replacement.

The [clean v32 full census](generated/four-profile-census-v32.json) is
21,244/22,924 exact, five more than v31, with zero errors. All five original
1.5× cases became exact. Across the entire corpus, 24 Open UI decoded images
changed, no previously exact image regressed, and all 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[change index](generated/broken-image-fixed-matrix-v1.json) records every
changed image and its before/after hash and pixel count. Nineteen already
failing comparisons also changed: most improved, while the two
`overflow-img-scroll-non-replaced` comparisons grew by four mismatched pixels
at 1.25× and by 86 and 85 at 1.5×. Their scroll, clipping, and alt-text paint
path remains a separate residual to investigate.

The clean [v33 focused/primitive matrices](generated/focused-primitive-raster-v33.json)
remain 640/640 and 960/960 exact. The complete
[v18 expanded requalification](generated/expanded-requalification-v18.json)
is 22,045/23,728 exact, with the same 198 of 201 additions exact across all
four profiles. The renderer release gate remains open: 1,680 original
comparisons still differ, across 926 unowned test IDs.

## Scroll-host diagnostic

The `overflow-img-scroll-non-replaced` pair still differs at fractional
scales. In the source case the broken image itself is the scroll container;
in the reference case the broken image is a child of a scroll container.
At 1.5×, the two Chromium captures differ within the icon, while Open UI
currently renders the same icon bytes for both. That narrows the remaining
question to the image's paint context or local sampling origin; it does not
establish a reviewed root cause.

A dirty diagnostic used the ordinary floating sampler only when the broken
image host had `overflow: scroll`. Across 55 selected cases and four profiles,
the exact count stayed **212/220**. Only the source scroll-host case changed:
its mismatched pixels fell from 67 to 63 at 1.25× and from 156 to 70 at
1.5×. No Chromium oracle hash changed. The reference case and the five sizing
cases repaired above were unchanged. The
[diagnostic index](generated/broken-image-scroll-sampler-diagnostic-v1.json)
records the report identities and changed comparisons. The experiment was
reverted because neither scroll comparison became exact. The next review
needs to distinguish the image's own paint from the ancestor scroll clip and
scroll backing before changing shared sampling again.

## Scroll clip edge evidence

The exact generated Chromium documents place both icons at a content-box
origin of (33, 33) CSS px. A CDP `DOM.getBoxModel` probe at 1920×1080@1.5
found the source image's content box at (33, 33)..(183, 81) and the
reference scroll container's content box at the same coordinates. In the
reference, the image child starts at (33, 33). Thus a different content-box
origin does not explain the icon pixels.

In the 30×30 physical crop x=45..74, y=45..74, the two pinned Chromium
captures differ at **66 pixels**, all on the icon's top, left, and bottom
edge: their inclusive difference bounds are (49, 49)..(74, 73). The two
Open UI captures are identical in that crop. For example, at (50, 49),
Chromium records (209, 209, 209) for the image that is itself a scroll host,
but (232, 232, 232) for the image inside a scroll host. Open UI paints the
same edge for both. This points to scroll clip or backing composition at a
fractional edge; the evidence does not yet distinguish the exact coverage
stage. The [scripted comparison index](generated/scroll-broken-image-clip-trials-v1.json)
pins the clean report, both diagnostic reports, Chromium identities, and all
eight profile results.

Two shared scroll-clip hypotheses were tested with dirty builds and reverted.
Allowing an analytic clip on every explicit scroll box with an empty range
reduced each pair member's wrong pixels by 20 at 1.25×, but increased each by
24 at 1.5×. Giving every fractional explicit scroll box a software backing
made the pair much worse: 590 and 1,049 wrong pixels at 1.25×, then 792 and
1,263 at 1.5×. Neither change made a comparison exact, and neither is
qualification evidence. The original results remain 67/193 wrong pixels at
1.25× and 156/305 at 1.5×. All eight Chromium oracle hashes stayed fixed.
The next implementation needs to reproduce Chromium's distinction between
painting replaced content in its own scroll host and compositing replaced
content through an ancestor scroll clip, then verify neighboring scroll
cases and the full census.
