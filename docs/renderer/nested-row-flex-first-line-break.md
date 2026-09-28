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
at that profile. The other 56 differences in this family remain open. A clean
complete census is required before this repair can be counted in the release
matrix.
