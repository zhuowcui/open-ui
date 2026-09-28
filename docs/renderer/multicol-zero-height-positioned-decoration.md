# Zero-height positioned container in a multicol continuation

Chromium is the pixel target. The original WPT fixture, complete manifest,
historical Open UI archive, and pinned Chromium oracle have not been changed.

`wpt/css_break/out-of-flow-in-multicolumn-046` has an auto-height relative
container with a red background and only one absolutely positioned, 200px-tall
green child. The container's own block size is zero. The outer and inner
multicolumn contexts need a visual continuation in the next column so the
child can be painted, but that continuation does not create a red background
area for its zero-height parent.

At 1280×720@1.25, the original Open UI image differed from Chromium in 125
pixels at physical x=87, y=25–149. At (87,50), Chromium stored
`(63,159,63)` and Open UI stored `(63,127,31)`. The difference comes from a
red parent continuation painted under the green child at the fractional
column edge.

The [reduced HTML](reproducers/multicol-zero-height-positioned-no-green.html)
makes the green child transparent in both renderers. In the
[Chromium image](reproducers/multicol-zero-height-positioned-no-green-chromium.png),
the visual continuation is white. The [old Open UI
image](reproducers/multicol-zero-height-positioned-no-green-before.png)
has red from x=88–149 and a half-covered red edge at x=87; it differs from
Chromium in 7,875 pixels within x=87–149, y=25–149. The
[repaired Open UI image](reproducers/multicol-zero-height-positioned-no-green-after.png)
is exact against that diagnostic Chromium image. These modified-fixture images
do not replace the oracle for the original test.

The overflow-continuation layout path now gives a cloned zero-height source
box a zero block decoration budget while retaining the continuation and its
descendants. This is an element geometry rule; paint still uses the normal
background path. The neighboring `-047` fixture has a real in-flow height,
so its parent decoration remains eligible and its image is unchanged.

The dirty diagnostic run of all 133 generated `out-of-flow-in-multicolumn-*`
cases at four profiles measured 490/532 exact, 42 different, zero errors.
Among the 131 cases in the original complete manifest, that is 486/524 exact
and 38 different, up from 485/524. Only the original `-046` image at
1280×720@1.25 changed, and it became exact. All other original Open UI
decoded hashes and all 524 Chromium oracle identities and decoded hashes
stayed fixed. The 38 remaining family comparisons retain their failing status;
this diagnostic is not a release qualification.
