# Nested row flex first-line forced break

Chromium is the sole pixel oracle for this investigation. The original
`wpt/css_break/flexbox_multi-line-row-flex-fragmentation-027` comparison at
1280×720@1.25 differed in 125 pixels at the edge of an absolute green box.
The [reduced HTML](reproducers/flex-row-first-line-break-no-abs.html) makes that
absolute box transparent. With the overlay removed, the old Open UI render
painted a 63×63 green region in the first column where
[Chromium](reproducers/flex-row-first-line-break-no-abs-chromium.png) painted
red: 3,969 differing pixels. The
[old Open UI image](reproducers/flex-row-first-line-break-no-abs-before.png)
shows the misplaced nested flex item. The archive of older Open UI screenshots
was not used as a target.

The nested row flex container has three children. Its first two children form
the first flex line; the second has `break-before: column`. Chromium moves the
containing flex item to the next column. A break on the third child, which is
on the next nested line, leaves the item in the first column. Changing the
first child's break also moves the item. These Chromium variants isolate
first-line break propagation. The [CSS flex fragmentation rules](https://www.w3.org/TR/css-flexbox-1/#pagination)
describe propagation from row flex items to their flex line and from the first
line to the container.

Open UI's DOM-only `propagated_break_before` followed the first in-flow child
and missed a forced break on another item in that first flex line. The row
flex fragmentation pass now inspects laid-out first-line fragments when it
decides whether the containing line must start in the next fragmentainer. The
new layout test checks both first-line and later-line breaks. No fixture or
paint output was changed to make the comparison pass.

In a diagnostic four-profile run of all 125
`flexbox_multi-line-row-flex-fragmentation-*` cases, the original 443/500 exact
comparisons became 444/500. Only `-027` at 1280×720@1.25 changed; it became
exact, and all 500 Chromium decoded hashes stayed unchanged. The reduced
no-absolute case also became [pixel exact](reproducers/flex-row-first-line-break-no-abs-after.png)
at that profile. The other 56 differences in this family remain open.

At clean checkpoint `89a0a1f3`, eight disjoint shards produced the complete
[v20 census](generated/four-profile-census-v20.json): **21,214/22,924 exact,
1,710 different, zero errors**. Against the prior v19 census, exactly this
one Open UI decoded image changed and became exact. No previously exact
comparison regressed, and all 22,924 Chromium oracle identities and decoded
hashes stayed fixed. The clean [v21 raster index](generated/focused-primitive-raster-v21.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 decoded
images unchanged from v20. The complete clean
[expanded v10 run](generated/expanded-requalification-v10.json) measured
22,011/23,724 exact; the 200 native additions retained their previous
statuses and pixels. The release renderer gate remains open with 943 unowned
residual test IDs.
