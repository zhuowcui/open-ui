# Fractional multicolumn edge evidence

Chromium 147 is the sole pixel target. The [v37 clean census](generated/four-profile-census-v37.json)
at `dc451061` leaves several multicolumn comparisons different only at
fractional scale. The historical Open UI archive is provenance, not an
expected image. This note records two reduced, pinned WPT sources and their
pixel evidence; it does not qualify a renderer change.

| WPT source | 1280×720@1.25 difference | Other required profiles |
|---|---|---|
| `css/css-break/table/repeated-section/background-001.tentative.html` | 125 pixels, one physical column at x=87, y=25–149 | Exact at 1×, 1.5×, and 2× |
| `css/css-break/flexbox/multi-line-column-flex-fragmentation-015.html` | 125 pixels, one physical column at x=162, y=25–149 | Exact at 1×, 1.5×, and 2× |

In the table case, the two 50 CSS px columns meet at CSS x=70, which maps to
physical x=87.5 at 1.25×. At physical `(87,30)`, Chromium is RGB
`(63,160,63)` and Open UI is `(127,191,127)`. Against white, those colors
are approximately 75% and 50% coverage of CSS green `(0,128,0)`. The
Chromium value is consistent with two independently blended half-covered
green edges: `1 - (1 - 0.5)^2 = 0.75`. This is a coverage observation, not
proof of the source display-item order. The Open UI fragment tree contains a
100 CSS px green table header in each adjacent column, each 50 CSS px wide.

In the flex case, Chromium is RGB `(191,223,191)` at physical `(162,30)`,
approximately 25% green over white; Open UI is white there. The difference
continues for the full 125 physical px height of the 100 CSS px square. The
Open UI fragment tree has five 20 CSS px columns and a column-flex
continuation in each. Its fifth column ends at CSS x=120; the Chromium fringe
lies at physical x=162, near CSS x=130. A sixth or clipped continuation in
Chromium is a possible explanation, but the present fragment evidence does
not establish that cause.

The gradient paint path clipped each square gradient to its own identical
paint rectangle with a hard edge, even though the draw itself uses an
antialiased rectangle. Removing that redundant clip lets both repeated
header fragments contribute at their shared half-pixel edge. In a dirty
two-case diagnostic, the table pixel at `(87,30)` moved to `(64,159,64)`:
only one level per channel remains. The flex image did not change. In a
separate dirty four-profile diagnostic over all 120 frozen fixtures whose
source contains a linear gradient, seven formerly different comparisons
became exact, no formerly exact comparison regressed, 29 Open UI images
changed, and all 480 Chromium decoded images stayed fixed. The 44-case
repeated-table family changed only this table image across 176 comparisons.
These targeted results are promising but do not substitute for a clean full
census.

The clean [v38 full census](generated/four-profile-census-v38.json) at
`d39282e4` confirmed the result across all 22,924 comparisons: 21,255 exact,
1,669 different, zero errors, seven newly exact comparisons, no exact
regression, 29 changed Open UI images, and 5,957 fewer wrong pixels than v37.
All Chromium oracle identities and decoded images stayed fixed. The clean
[v39 focused/primitive index](generated/focused-primitive-raster-v39.json)
is 640/640 and 960/960 exact, with no decoded image changes. All 804 decoded
images in the clean [201-addition guard](generated/expanded-additions-gradient-guard-v1.json)
are unchanged from the prior complete expanded run; the same three additions
remain different. The guard does not replace a complete expanded-manifest
rerun at this checkpoint.

The remaining signatures point to fractional fragment painting or clipping,
not a general box-size error: both tests are exact at the three other required
scales, and the table source has adjacent repeated header fragments at the
seam. Further repair must follow fragment structure and reproduce Chromium's
coverage without a case ID, color, or viewport branch. The table and flex
comparisons remain different, and their last pixel mechanism remains unowned
for release qualification.
