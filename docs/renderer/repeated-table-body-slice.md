# Repeated table body paint slice

Chromium is the sole pixel target. The reduced
[header and body example](reproducers/table-repeated-header-body-slice.html)
has four adjacent 25 CSS pixel columns, a 20-pixel repeated table header, and
a 320-pixel green body over a red background. The Engine-backed WPT fixture
`table_repeated-section_header.tentative` has the same visual structure.

At 1280×720 and 1.25× scale, the old Open UI render differed from Chromium
for all 20 physical pixels of the header at each of three column seams. At
one seam it painted `(11, 122, 0, 255)` where Chromium painted
`(47, 104, 0, 255)`: Open UI applied too much green coverage. Its fragment
tree placed the continued body row group at y = −60 CSS pixels in the second
column, so that green body painted behind the green repeated header. Their
coverage accumulated at fractional column edges. The existing body-slice
helper already restricted nested repeated tables; the ordinary multicolumn
continuation path had only translated the row group.

The ordinary path now gives a normal-flow, zero-spacing repeated table body
its own visible slice. In the reduced header case, the second-column body
fragment begins at y = 20 CSS pixels and is clipped to the 80-pixel interval
below the header. Positioned descendants, forced breaks, and nonzero border
spacing retain their previous continuation geometry. A broad first trial
without these guards made 16 comparisons exact but regressed 12 previously
exact comparisons. The guarded 44-case repeated-section diagnostic made 11
exact with no regressions; a wider 1,920-case `css_break` and `css_multicol`
diagnostic across all four profiles changed only the same 12 images.

The clean [v29 four-profile census](generated/four-profile-census-v29.json) at
`7cd8e574` measured **21,238/22,924 exact**, 1,686 different, and zero
errors. Eleven comparisons became exact, including repeated header, footer,
header-and-footer, inline-block, and multicolumn table cases at the two
fractional desktop scales. The `multiple-row-groups` case became exact at
1.5× and improved from 130 to 94 wrong pixels at 1.25×. Exactly 12 Open UI
images changed; none regressed from exact, and all 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The release gate remains red with
932 unowned residual test IDs.

The clean [v30 raster index](generated/focused-primitive-raster-v30.json) is
640/640 focused and 960/960 primitive exact; all 1,600 Open UI and Chromium
image hashes match the prior clean run. The clean
[v15 expanded requalification](generated/expanded-requalification-v15.json)
measured 22,035/23,724 exact, with no change to any of the 800 native
addition images. The 200 earlier additions still include 197 four-profile
exact cases and three demoted cases. The
[v4 pending-candidate index](generated/pending-mutation-candidates-v4.json)
still has one of 36 cases exact at all four profiles. Open UI executes no
JavaScript. Neither historical Open UI images nor Chromium oracle bytes were
rewritten, and no tolerance or test-ID branch was added.
