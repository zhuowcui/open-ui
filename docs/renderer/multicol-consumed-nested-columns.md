# Consumed nested columns in a positioned visual continuation

Chromium is the pixel target. The original WPT fixture, complete manifest,
historical Open UI archive, and pinned Chromium oracle were not changed.

`wpt/css_break/out-of-flow-in-multicolumn-047` has a 100px-tall outer
multicolumn box. Its inner relative multicolumn box has a red background, a
direct absolutely positioned green child offset below the first outer
column, and a 200px-tall in-flow relative child whose own absolutely
positioned green child fills the inner columns. Chromium gives the inner box
a 100px used block size. Its inner column content is consumed there; the
direct absolute child still needs a visual continuation in the next outer
column.

At 1280×720@1.25, the previous Open UI image had 125 differing pixels at
physical x=87, y=25–149. At (87,50), Open UI stored `(31,119,7)` and
Chromium stored `(63,127,31)`. Two source-level errors overlapped at that
fractional column edge:

1. The visual continuation painted the red parent background beyond its
   100px used block size. The [no-green reduced
   HTML](reproducers/multicol-nested-047-no-green.html) makes both green
   children transparent. The [Chromium
   image](reproducers/multicol-nested-047-no-green-chromium.png) and
   [old Open UI image](reproducers/multicol-nested-047-no-green-before.png)
   differ in 7,875 pixels within x=87–149, y=25–149. Capping the positioned
   box's decoration at its remaining source interval makes the
   [diagnostic Open UI image](reproducers/multicol-nested-047-no-green-after-cap.png)
   exact to Chromium.
2. The outer continuation cloned the inner `ColumnBox` fragments after their
   in-flow source had already been consumed. The [direct green only
   variant](reproducers/multicol-nested-047-direct-green-only.html) was
   [exact in Open UI](reproducers/multicol-nested-047-direct-green-only-openui.png)
   against [Chromium](reproducers/multicol-nested-047-direct-green-only-chromium.png)
   after the decoration cap. In the [nested green only
   variant](reproducers/multicol-nested-047-nested-green-only.html), Chromium's
   [image](reproducers/multicol-nested-047-nested-green-only-chromium.png)
   is white after x=87, while [Open UI before the clone
   repair](reproducers/multicol-nested-047-nested-green-only-before.png)
   repainted green through x=149. That variant differs in 7,875 pixels.

The shared layout path now keeps the visual continuation for the direct
positioned descendant, limits the positioned parent's decoration to its
remaining own source, and omits already-consumed inner column boxes from the
outer continuation. It applies only after the nested multicolumn source is
fully consumed and has no own overflow columns. No test ID, post-raster
pixel edit, tolerance, or oracle change is involved. The modified-fixture
images above are diagnostic and do not replace the original Chromium oracle.

## Verification

The original `-046` and `-047` cases are exact at all four required profiles.
The dirty diagnostic run of all 133 generated `out-of-flow-in-multicolumn-*`
cases at four profiles measured 491/532 exact, 41 different, zero errors.
Compared with the preceding 490/532 family run, only `-047` at
1280×720@1.25 changed: its 125 differing pixels became exact. All 532
Chromium oracle identities and decoded hashes stayed fixed.

At clean source checkpoint `d0592ccd`, eight disjoint shards produced the
complete [v23 four-profile census](generated/four-profile-census-v23.json):
**21,219/22,924 exact, 1,705 different, zero errors**. Compared with v22,
only that `-047` Open UI decoded image changed. No previously exact
comparison regressed, and all 22,924 Chromium oracle identities and decoded
hashes remained unchanged. The 939 remaining residual test IDs still lack
reviewed owners, so the census does not pass the release gate.

The clean [v24 focused and primitive raster
index](generated/focused-primitive-raster-v24.json) remains 640/640 and
960/960 exact. All 1,600 Open UI decoded hashes and Chromium oracle
identities/hashes match v23. A clean selected check of the 200 expanded
native final-state additions retained 797/800 exact comparisons, three
differences, and zero errors; all 800 Open UI and Chromium hashes matched
the prior full expanded run. This selected check is diagnostic, not a full
expanded-manifest qualification.
