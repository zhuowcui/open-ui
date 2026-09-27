# Rounded border and content-background coverage

The CPU primitive test `wpt/css_backgrounds/background-clip-content-box-with-border-radius-002`
still differs from Chromium at eight 1.25-scale profiles. The clean
[v10 primitive evidence](generated/focused-primitive-raster-v10.json) records
896 differing pixels across those profiles. The prior
[v9 evidence](generated/focused-primitive-raster-v9.json) recorded 6,728
differing pixels across all 40 profiles. Chromium remains the expected image.

The [reduced fixture](../../tools/qualification/reproducers/rounded-content-border-seam.html)
is a 50 × 50 CSS pixel black content background inside a
25 pixel solid black border, with `background-clip: content-box` and a 100%
top-left radius. The sibling `-003` fixture has an exact four-profile result,
so the failure is specific to how this border and background meet, not a
general inability to draw a rounded content clip.
Captured with the pinned Chromium at 320 × 240 CSS pixels and 1.5 scale,
the reduced fixture produced the same PNG bytes as the matrix fixture
(`4017832a100689bb0b46ff0dd878a4d6179ad23d40b834836025ea286a5dea08`).

At the v9 checkpoint, with a 320 × 240 CSS pixel viewport and 1.5 device
scale, the right and bottom content edges fell at physical coordinate 142.5.
At `(142, 80)`, Chromium's gray channel was 63 and Open UI's was 127. With the
border paint temporarily disabled, Open UI remained at 127; with the
background paint disabled, it became 255. This isolated a missing fractional **border**
contribution. The nonrenderable rounded-border path first applies a hard
polygon clip to each side, then subtracts an antialiased inner rounded
contour. The hard side clip discards the half-covered device cell at the
content edge before the inner contour can contribute to it. The same
straight-edge pattern appears along the right and bottom seams.

In a dirty diagnostic build, extending the polygon's interior vertices by
one CSS pixel and using a float16 layer restored that border contribution.
The test improved from 6,728 to 6,224 differing pixels across the same 40
profiles. At `(142, 80)` the channel became 64, still one above Chromium;
the shared corner also overpainted. All 40 profiles remained different.
Chromium oracle identities and decoded hashes matched the clean run in
all 40 profiles. The experimental paint change was reverted because it did
not satisfy the exact gate. These diagnostics do not qualify a renderer build.

Pinned Chromium 147's `BoxBorderPainter::ClipBorderSidePolygonCloseToEdges`
uses antialiased side clipping and retains shared corner coverage for this
complex border path. Open UI's v9 nonrenderable border path used a hard
side polygon before subtracting the antialiased inner contour. A read-only
composition check on the 320×240@1.5 PNG shows why geometry alone is
insufficient: adding half-covered black at `(142, 80)` moves Open UI's gray
channel from 127 to 64, while Chromium stores 63. At 1.25 scale, the analogous
edge moves from 63 to 47, while Chromium stores 48. That check changes no
renderer or oracle pixels; it identifies the remaining coverage-packing and
blend-rounding work.

A second dirty diagnostic replaced the hard side polygon with a round-corner
path based on Chromium's `ClipBorderSidePolygonCloseToEdges` geometry, while
keeping Open UI's existing antialiased inner-contour subtraction. It remained
different in all 40 profiles and increased the aggregate differing pixels
from 6,728 to 9,392. At `(142, 80)` in the 320×240@1.5 profile, the gray
channel moved from 127 to 96; Chromium is 63. The two antialiased clips appear
to multiply coverage in this CPU replay path. This diagnostic does not prove
that Chromium's clip geometry is wrong or that a different raster backend
would behave the same way. The code was reverted. A qualifying fix needs to
match the combined clip and border coverage, including its final channel
rounding, across the neighboring cases.

Two further dirty diagnostics combined the side polygon and adjusted inner
contour with Skia PathOps, then rasterized their difference as one
antialiased path. Applying that to every side made `(142, 80)` exact at
320×240@1.5 but increased the 40-profile total to 14,104 differing pixels.
Restricting it to sides with two straight inner corners preserved the curved
side pixels and reduced that 1.5-scale profile from 243 to 141 differences,
but worsened the other four scales; the 40-profile total was 7,176. All 40
profiles remained different, and all Chromium decoded hashes stayed
unchanged. Both changes were reverted. A single antialiased path can recover
the missing seam coverage, but it also changes corner coverage and needs a
shared blend-rounding solution before it can qualify.

Pinned Chromium's `BoxBorderPainter::PaintSide` sends sides with a curved inner
edge through its side polygon and adjusted inner contour, but fills straight
sides as complete side rectangles. Open UI had sent all four sides through the
polygon path. Following Chromium's split repaired the missing fractional border
contribution at `(142, 80)` and reduced the 40-profile total from 6,728 to
5,624 differing pixels. No previously exact primitive comparison changed.

Chromium's `BoxDecorationData::ComputeBleedAvoidance` shrinks the background
under an opaque rounded border instead of adding an outer clip layer. Open UI
had allowed that layer for `background-clip: content-box` because its layer
guard checked only whether a *border-box* background was shrunk. Skipping the
layer when the border obscures the background edge reduced the same residual
to 2,640 differing pixels. Removing an older two-strip repaint for a single
saturated corner then made 32 of its 40 profiles exact. In the remaining eight
1.25-scale profiles, 112 pixels per profile differ by one gray level along the
right and bottom content edges: Open UI stores 47, Chromium 48. The combined
dirty diagnostic produced 952/960 exact primitive comparisons and 640/640
exact focused comparisons, with zero errors. Only this primitive ID changed;
none of the Chromium decoded hashes or previously exact Open UI comparisons
changed. The neighboring `-003` fixture stayed exact at all four contract
profiles, and the three `background-origin_origin-*-box_with_radius` residuals
kept their prior mismatch counts. A filtered four-profile check of this ID
improved from zero to three exact results. Those first runs were diagnostics
from a dirty source tree. A subsequent clean run at `aa2d4d24` confirmed
640/640 focused and 952/960 primitive exact comparisons with zero errors and
the same eight one-level residuals. The primitive release gate remains open.

The three full-suite `background-origin_origin-{border,padding,content}-box_with_radius`
cases have identical diff signatures within each of the four required
profiles in the [v8 census](generated/four-profile-census-v8.json). Their
shared rounded border is therefore a useful neighboring guard for the next
paint change; the evidence does not assign every pixel in those cases to the
border path.

**Candidate owner:** `openui-paint` rounded-border coverage and layer compositing.
Ownership remains unreviewed in the qualification ledger because the remaining
1.25-scale rounding difference and full-census effects have not been closed.
The next fix must match Chromium's shared content/border edge rounding without
a fixture-specific pixel correction. It must pass all 40 primitive profiles,
the exact neighboring fixtures, and the full four-profile census without
regression before the residual can be closed.
