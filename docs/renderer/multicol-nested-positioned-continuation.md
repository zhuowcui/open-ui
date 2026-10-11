# Nested positioned continuations in multicolumn layout

Chromium is the pixel target. The archived Open UI screenshots are historical
evidence, not expected output. The original WPT fixture and pinned Chromium
oracle were not changed for this repair.

## Reduced evidence and cause

`wpt/css_break/out-of-flow-in-multicolumn-032` has a positioned parent inside
a multicolumn container and a nested absolute red child offset into later
columns. At 1280×720@1.25, the old Open UI fragment tree left that child
inside the parent's earlier clipped continuations. A fixed green sibling did
reach the later columns. The original fixture exposed the error as a
one-pixel seam at physical x=87, y=25–149: Open UI painted `(63,159,63)`
where Chromium painted `(63,111,15)`.

The [reduced HTML](reproducers/multicol-nested-032-no-green.html) makes the
green sibling transparent in both renderers, leaving the nested red child
visible. Its SHA-256 is
`054b519b859c613471c06044689be9715745c0baf9db9647cefe3f38c197237f`.
The diagnostic [Chromium image](reproducers/multicol-nested-032-no-green-chromium.png)
contains a 125×125 red region at x=25–149, y=25–149; the
[old Open UI image](reproducers/multicol-nested-032-no-green-before.png) is
white throughout that region. Their PNG SHA-256 values are
`a77e3e86170c9030081349d56bddb4362ca50cf2571e1874515ca1ce7a9132fe`
and `34ec92058865d7278b51db487ccc51b37cc61d8f4e1f1b491df6a6829c6b1226`.
These modified-fixture images are diagnostic; neither replaces the immutable
Chromium oracle for the original WPT case.

`extract_nested_positioned_fragments` previously descended into a positioned
parent only when that parent had a nonidentity transform. The parent here has
an identity transform, but its child still extends past every slice of the
parent's border box. The shared layout path now promotes such descendants
independently into the later fragmentainers. It installs a new transform
source only for a nonidentity local transform and otherwise keeps the
inherited source. The repair is owned by layout/fragmentation and does not
select a test ID or alter raster output after painting.

## Clean verification and open residual

At clean source checkpoint `9f983df9`, eight disjoint release-binary shards
produced the [v17 census](generated/four-profile-census-v17.json):
**21,209/22,924 exact, 1,715 different, zero errors**. Against the v16 clean
census, only two Open UI decoded images changed. The original `-032` fixture
at 1280×720@1.25 became exact from 125 differing pixels. No previously exact
comparison regressed, and all 22,924 Chromium oracle identities and decoded
hashes stayed fixed. The clean
[v18 raster index](generated/focused-primitive-raster-v18.json) remained
640/640 focused and 960/960 primitive exact; all 1,600 decoded images matched
the preceding clean raster run. The complete
[expanded requalification](generated/expanded-requalification-v8.json) kept
all 200 native additions at their prior four-profile statuses.

The other changed image, `out-of-flow-in-multicolumn-060` at the same profile,
still differs in 125 pixels at x=87, y=25–149. At (87,25), its Open UI color
moved from `(75,93,3)` to `(43,109,3)` while Chromium stayed `(51,105,3)`.
Its cause and owner are not yet reviewed; it remains a failing residual. The
overall census has 948 unowned residual test IDs and is not a qualifying
release result.

## Positioned continuation paint order

The `-060` residual was a paint-order error. Its fragment tree already
contained both 50×100 continuations of the outer green positioned box, the
in-flow red child in each continuation, and the promoted inner green
positioned child. At the shared fractional edge, Chromium paints the outer
green decorations together, then the red child content, then the inner green
positioned child. Open UI painted each outer green decoration immediately
before that continuation's red child. The resulting source-over orders are
`green, green, red, red, green, green` and
`green, red, green, red, green, green`, respectively. At physical (87,25),
those orders produced Chromium `(51,105,3)` and Open UI `(43,109,3)`.

The [reduced HTML](reproducers/multicol-positioned-060-no-inner-green.html)
turns off the inner green child's fill while retaining the two outer
continuations and red children. Its SHA-256 is
`b38bf9ae2de298a54571a432b71a13341ecb59c4c61cb8cdc9c61e0d6cec2f0e`.
The diagnostic [Chromium image](reproducers/multicol-positioned-060-no-inner-green-chromium.png)
and [old Open UI image](reproducers/multicol-positioned-060-no-inner-green-before.png)
still differ at exactly the 125 seam pixels: `(207,39,15)` versus
`(175,55,15)` at (87,25). Their PNG SHA-256 values are
`fdf1df2ae2853f0cbb55336de94823def4f8ec5f237a2af9162fbe0e18f83840` and
`8390031fd2bcd03e679acb14028496fd48e486bcb5f96123a46b87be10a411e4`.
These diagnostic inputs and images do not replace the immutable Chromium
oracle for the original case.

The painter now groups simple continuations of a positioned box at the
stacking phase that actually owns them. It paints their decorations first,
then traverses the continuations for child content. Hoisted continuations
receive this treatment at the ancestor stacking phase; their skipped copies
are never prepainted. Effects, transforms, authored overflow clips, and
atomic stacking contexts retain their existing paths. This is a shared
fragment paint-order rule, not a test-ID or post-raster pixel correction.

The targeted four-profile check found the original `-060` and neighboring
`-032` cases exact. In the diagnostic 131-case
`out-of-flow-in-multicolumn-*` family, only the former's 1280×720@1.25 image
changed: its 125 differing pixels became exact. All other 523 Open UI decoded
images were byte-identical to the preceding clean census. The paint crate's
161 unit tests passed.

At clean source checkpoint `8269ea09`, eight disjoint debug-runner shards
produced the [v18 census](generated/four-profile-census-v18.json):
**21,210/22,924 exact, 1,714 different, zero errors**. Compared with v17,
only the `-060` 1280×720@1.25 Open UI decoded image changed; it became exact.
No previously exact comparison regressed. All 22,924 Chromium oracle
identities and decoded hashes remained unchanged. The 947 residual test IDs
remain unowned, so this is a nonqualifying diagnostic result. The clean
[v19 raster index](generated/focused-primitive-raster-v19.json) remained
640/640 focused and 960/960 primitive exact, with all 1,600 decoded
Open UI and Chromium images unchanged from the prior clean raster run.
