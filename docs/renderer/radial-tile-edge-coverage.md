# Repeating radial-gradient tile edge coverage

The pinned Chromium raster is the pixel oracle. Historical Open UI screenshots
are not expected output. This investigation uses zero pixel tolerance.

The reduced case is a 200×150 CSS-pixel `div` with a 10px solid border and a
repeating elliptical radial background. Its two stops are blue and green at
20px. The `background-image-centered` source and reference builders retain the
same visual state. At 1.25×, their tile edge falls between physical pixels.

The direct radial-gradient path kept gradient evaluation in destination
coordinates. It then applied both an antialiased tile clip and an antialiased
tile draw. At a fractional edge, the two coverage masks multiplied. For
example, Open UI produced `[95, 96, 126, 255]` at physical pixel `(38, 37)`
while Chromium produced `[63, 65, 125, 255]`.

An initial diagnostic disabled antialiasing on the inner draw. That reduced
the count of different pixels from 626 to 439 per case, but increased total
absolute channel error from 43,910 to 60,963. At `(38, 37)` it removed the
gradient entirely. This was rejected and reverted.

The retained repair removes the outer antialiased tile clip. The direct radial
draw now owns fractional coverage once. In the [clean four-profile nine-case
guard](generated/radial-single-coverage-v1.json) at `d0860d3f`, only the source
and reference at 1.25× changed: each fell
from 626 wrong pixels to one, and total absolute channel error fell to one.
The other 34 images and all 36 Chromium oracle images were unchanged. There
were no exact-status regressions. The remaining difference is a one-channel,
one-level green rounding difference at `(186, 216)` in each image; both cases
still fail the exact gate.

The clean 40-profile focused and primitive matrices at the same checkpoint
remain 640/640 and 960/960 exact. This guard does not replace the full original
or expanded census. The residual pixel requires a separately reviewed shared
gradient color-rounding explanation before any further paint change.
