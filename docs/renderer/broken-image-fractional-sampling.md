# Broken-image icon at fractional device scale

Chromium 147 remains the sole pixel target. This investigation compares the
clean [v31 census](generated/four-profile-census-v31.json) at `9534c9f6`
with an uncommitted diagnostic. The diagnostic change was reverted and is not
release qualification evidence.

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
remaining difference is in drawing the 100% resource into the 16 CSS-pixel
fallback slot at 1.5×. That narrows ownership to
`openui-paint::paint_missing_image` and its physical image sampler; the exact
Chromium filter phase and coverage order at this scale are still unresolved.

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

Neither diagnostic can be used. Next, derive the 1.5× sample coordinates
and filter weights from the pinned Chromium image draw, create a minimal
Engine-backed broken-image fixture with neighboring scale and position phases,
and check the shared sampler against already exact missing-image cases. The
five original comparisons and the complete renderer gate remain open.
