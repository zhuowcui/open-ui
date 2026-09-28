# Relative continuations and column clips

Pinned Chromium is the pixel target. The original WPT fixtures, complete
manifest, and Chromium oracle captures were not changed for this repair.

## Cause and reduced evidence

`wpt/css_break/out-of-flow-in-multicolumn-042`, `-043`, and `-045` each had a
125-pixel seam at 1280×720@1.25. Chromium painted `(63,111,15)` at physical
x=87, y=25–149, while Open UI painted `(63,159,63)`. A relative red box is
laid out in one column and visually moved into another. Its green absolute
child is promoted into the later column. The shared-decoration prepaint had
kept the red box's synthetic column clip at the original coordinates, so the
red decoration was entirely clipped before the green child painted.

The [reduced HTML](reproducers/multicol-relative-042-no-absolute-fill.html)
makes the absolute child transparent in both renderers. The
[Chromium image](reproducers/multicol-relative-042-no-absolute-fill-chromium.png)
shows a 125×125 red area; the
[old Open UI image](reproducers/multicol-relative-042-no-absolute-fill-before.png)
is white there. They differ in 15,625 pixels, with bounds x=25–149,
y=25–149. The [repaired Open UI image](reproducers/multicol-relative-042-no-absolute-fill-after.png)
matches the diagnostic Chromium image exactly. The HTML SHA-256 is
`f2793e9fd63ae47f509b44875631e7f878b45ce130a4cfb7c370c12749416dc8`;
the three image SHA-256 values, in the order linked above, are
`a77e3e86170c9030081349d56bddb4362ca50cf2571e1874515ca1ce7a9132fe`,
`34ec92058865d7278b51db487ccc51b37cc61d8f4e1f1b491df6a6829c6b1226`,
and `2e4b1ff58feac253fc4c7118d21f40adc70f050c1934866606cdc5de1fffa114`.
These modified-fixture captures are diagnostic and do not replace the original
Chromium oracle.

The painter now moves inherited synthetic fragmentainer clips with the
relative continuation's visual offset when it hoists decorations. It leaves
authored overflow clips attached to their owning ancestor. Fragmented inline
ancestors retain their separate row paint owner, whose stored offset already
includes the inline translation. This fixes the shared fragmentation paint
path rather than selecting fixtures or changing finished pixels.

## Verification and remaining gap

At clean source checkpoint `17952772`, the 161 `openui-paint` library tests
passed. The complete `out-of-flow-in-multicolumn-*` family ran at all four
profiles: 485/524 exact, 39 different, zero errors. Only the three original
images named above changed from the prior clean census, each from 125
different pixels to exact. The complete `css_overflow` family ran at all four
profiles: 1,857/2,028 exact, 171 different, zero errors, with all 2,028
decoded Open UI images unchanged. The overflow guard includes the authored
`overflow-clip-margin-mul-column-content-box-ref` case.

Eight complete, disjoint clean shards produced the
[v19 four-profile census](generated/four-profile-census-v19.json):
**21,213/22,924 exact, 1,711 different, zero errors**. Against the v18 clean
census, exactly the three intended Open UI decoded images and statuses
changed. No previously exact comparison regressed. All 22,924 Chromium oracle
identities and decoded hashes stayed unchanged. The 944 remaining residual
test IDs have no reviewed root cause and owner, so this is a nonqualifying
diagnostic result, not a v0.2 release pass.

At the following clean checkpoint `ef7214b3`, the
[v20 raster index](generated/focused-primitive-raster-v20.json) stayed
640/640 focused and 960/960 primitive exact across 40 profiles in each
suite. All 1,600 decoded Open UI images and Chromium oracle hashes matched
the preceding clean raster run.
