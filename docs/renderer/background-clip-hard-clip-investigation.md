# Square background-clip color coverage

Chromium is the sole pixel target. This investigation uses the pinned Chromium
147 capture and zero pixel tolerance. The historical Open UI archive is not an
expected image.

## Reduced paint rule

`wpt/css_backgrounds/background-clip-color` paints three green squares using
`border-box`, `padding-box`, and `content-box` color backgrounds. In the v14
clean census, its 1280×720@1.25 comparison still differed by 150 pixels in
two 38×38 physical regions, starting at (37, 100) and (37, 162). The changed
pixels occupied the leading and trailing edges of the inset color boxes. For
example, Open UI blended green into `(191,223,191,255)` where Chromium kept
white `(255,255,255,255)` at the leading edge.

Chromium's `BoxPainterBase::PaintFillLayer` clips a square padding/content
background with `ToPixelSnappedRect(clip_rect)` and fills the snapped outer
paint rectangle. Those rectangles are selected in layout CSS pixels before
the device-scale transform. Open UI instead drew the inset background
rectangle with analytic antialiasing, which leaked fractional color across
the clip and compounded coverage when a dashed border painted afterward.

The shared paint path now fills the outer decoration rectangle through a hard
padding/content clip for square color backgrounds. It retains layout-space
coordinates so the outer fill can still have fractional device coverage.
Rounded backgrounds continue through their rounded contour path. No WPT ID,
fixture byte, Chromium capture, comparator tolerance, or post-raster pixel is
special-cased.

An initial dirty diagnostic snapped both rectangles directly to the physical
grid. It made many background-clip cases exact, but moved the 125-pixel
`margin-trim_block-container-non-adjoining-item` difference from the top edge
at y=87 to the bottom edge at y=462. Chromium has half coverage at that
bottom edge, while physical snapping filled it completely. That version was
rejected and was never counted as qualifying evidence. The CSS-space rule
made the margin-trim comparison exact. A dirty three-case 40-profile sweep
for the original target, a fractional flex content box, and the margin-trim
case measured 120/120 exact and zero errors.

## Clean verification

At clean checkpoint `8ca66ffd`, eight complete disjoint shards produced the
[v15 census index](generated/four-profile-census-v15.json):
**21,207/22,924 exact, 1,717 different, zero errors**. Relative to the v14
clean run, 28 Open UI decoded images changed. Twenty-two comparisons became
exact; six already-different comparisons remained different with fewer
mismatched pixels. No previously exact comparison regressed. All 22,924
Chromium oracle identities and decoded hashes stayed fixed. The residual
inventory is 949 unowned test IDs, so the complete release gate still fails.

The gains include `background-clip-color`, `background-clip-content-box-002`,
`margin-trim_block-container-non-adjoining-item`, padding/content clip tests,
and local-scroll clip references at fractional scales. The fieldset and
border-image-outset residuals changed but did not become exact; their remaining
pixels need separate root causes and owners.

The clean [v16 raster index](generated/focused-primitive-raster-v16.json)
records 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded hashes unchanged from v15. The clean expanded run
measured 22,004/23,724 exact; all 200 native final-state additions retained
their four-profile statuses and decoded hashes. The
[v6 requalification ledger](generated/expanded-requalification-v6.json)
retains 197 exact additions and demotes three. The historical archive and
Chromium oracle were not rewritten.
