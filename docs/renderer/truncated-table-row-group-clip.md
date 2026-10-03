# Truncated table row-group clip coverage

Chromium 147 remains the sole pixel target. The pinned WPT source
`css/css-break/table/repeated-section/multiple-row-groups.tentative.html`
contains four 25 CSS-pixel columns, a repeated 10-pixel green footer, and
four green body row groups over a red background. The 1.25× comparison at
1280×720 was the one residual after the shared repeated-table body-slice
repair. Its 94 wrong pixels formed one physical row, x=25 through x=118 at
y=137. At `(50,137)`, Open UI painted solid green `(0,128,0,255)`, while
Chromium painted `(63,96,0,255)`.

In the first three columns, a row-group child extends past the visible body
slice reserved ahead of the repeated footer. The layout tree clips those
children at 90 CSS pixels inside each column. The paint path had expanded
that fragmentainer clip outward to whole physical cells. At 1.25×, the
body/footer boundary lands at physical y=137.5. The expanded clip let the
body fill pixel row 137, covering the red that remains visible in Chromium.
The fourth column ends at the source row group's natural edge and was already
exact. Chromium's color at the clipped boundary is the result of two
half-covered green paints over red.

The paint repair keeps the logical clip and antialiased coverage when a
table row-group fragment has a block-axis clip and a child extends past the
fragment's visible end. Other fragmentainer clips retain their existing
physical closure. This condition follows fragment structure; it has no test
ID, color, viewport, or output-pixel branch.

The dirty diagnostic covered all 44 repeated-section cases and the wider
1,920-case `css_break` plus `css_multicol` selection at all four required
profiles. In 7,680 comparisons, only this 1.25× image changed, from 94 wrong
pixels to exact. No previously exact case changed, and every Chromium oracle
identity and decoded hash stayed fixed.

At clean source checkpoint `814a2005`, the complete
[v30 census](generated/four-profile-census-v30.json) confirmed
**21,239/22,924 exact**, 1,685 different, and zero errors. Across all
22,924 comparisons, only that one Open UI image changed. The
[40-profile viewport and scale sweep](generated/table-row-group-cross-v1.json)
of the repaired case is 40/40 exact. The clean
[v31 focused and primitive index](generated/focused-primitive-raster-v31.json)
is 640/640 and 960/960 exact, with all 1,600 Open UI and Chromium decoded
images unchanged. The clean
[v16 expanded requalification](generated/expanded-requalification-v16.json)
is 22,036/23,724 exact; all 800 native-addition images are unchanged. The
[v5 pending-case index](generated/pending-mutation-candidates-v5.json)
remains 22/144 exact. The renderer release gate still fails with 931
unowned residual test IDs. Neither the historical Open UI archive nor any
Chromium oracle byte was rewritten, and no pixel tolerance was added.
