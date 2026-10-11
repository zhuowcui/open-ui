# Negative flex margins and intrinsic width

Chromium is the pixel target. The old Open UI archive is historical evidence,
not an expected image for this investigation. The source fixture is
`wpt/css_flexbox/negative-margins-001`; the complete clean comparison uses the
pinned Chromium 147 capture and zero pixel tolerance.

## Reduced case and root cause

The visual case is a 40px red container with a 3px black border. Its green
inline flex box has three items: a 40px first item, a 20px second item with
`margin-left: -40px`, and a 20px third item. Chromium's flex box contributes
40px of intrinsic width, so the red container does not show through. The
previous Open UI layout contributed 60px and painted an extra green strip.

The flex intrinsic-sizing path had clamped the item's **outer margin-box**
contribution to zero for its automatic minimum-size branch. That erased the
negative margin. The shared layout calculation now clamps the nonnegative
border-box contribution, then adds the signed margin. A unit test for this
fixture failed at 60px before the change and passes at 40px after it. This
follows Blink's separate border-box and margin accounting in
`FlexLayoutAlgorithm::ComputeMinMaxSizes`.

The first clean complete diagnostic at `8ea12c9f` was rejected. It measured
21,177/22,924 exact, 1,747 different, and zero errors: the target became exact
at 800×600@1 and 375×667@2, but
`wpt/css_flexbox/intrinsic-size_row-wrap-002.tentative` changed from exact to
different at all four profiles. Its first item contributes 100px of minimum
width; the second has a negative 10px margin. The summed max-content width
fell to 90px, below its 100px min-content width. Blink floors max-content at
min-content. The shared calculation now does the same; a second unit test
failed at 90px before that guard and passes at 100px after it.

## Clean verification

At clean checkpoint `2ff236ce`, eight complete disjoint shards covered all
5,731 cases at all four required profiles. The strict
[v13 census index](generated/four-profile-census-v13.json) verified the source,
runner, manifest, profile, backend, report, and oracle identities. It records
21,181/22,924 exact, 1,743 different, and zero render errors. Relative to the
prior clean v12 census, only the four Open UI images for the negative-margin
case changed. Its 800×600@1 and 375×667@2 comparisons became exact; no
previously exact comparison regressed. All 22,924 Chromium decoded hashes and
oracle identities remained fixed.

The 15-case neighboring guard set measured 58/60 exact, zero errors, and zero
guard regressions. The focused 40-profile gate remained 640/640 exact and the
primitive 40-profile gate remained 960/960 exact. All 1,600 Open UI decoded
hashes and Chromium hashes in those gates matched the preceding clean run.
The clean expanded matrix measured 21,978/23,724 exact and retained the same
197/200 additions exact at all four profiles. The 800 addition statuses and
Open UI/Chromium decoded hashes did not change.

At that v13 checkpoint, two fractional-scale comparisons in the
negative-margin case remained different:

| Profile | Different pixels | Bounds in physical pixels | Observed edge |
|---|---:|---|---|
| 1280×720@1.25 | 146 | x=28, y=25, width=55, height=20 | top/bottom coverage and right edge |
| 1920×1080@1.5 | 150 | x=34, y=34, width=61, height=16 | border/background edge |

At 1.25 scale, one right-edge Open UI pixel is RGBA `(127,63,63,255)` while
Chromium's is `(127,127,127,255)`, showing red parent color in Open UI's
coverage. This was a separate border/background compositing residual at v13.
The census then had 968 unowned residual test IDs overall. No fixture
bytes, Chromium capture, archive image, comparator tolerance, or release gate
was changed.

## Background coverage closure

The parent red background and its opaque green descendant occupied the same
physical edge. The green box was wrapped in an anonymous line fragment, which
the existing conservative paint occlusion check did not traverse. The green
box also had no border, so its opaque children did not suppress its own hidden
background. At fractional scales the extra coverage layers blended red into
edge pixels that Chromium rendered from the common backdrop.

The paint path now follows a single ordinary anonymous wrapper, includes its
offset, and culls a borderless background only when a fully covering opaque
in-flow child makes it invisible. It rejects clips, continuation decorations,
out-of-flow candidates, transformed ancestry, and other paint-changing wrapper
state. A unit regression covers the anonymous line and a clipped wrapper.

The first clean diagnostic after that change was rejected: three previously
exact comparisons in `css_break` and `css_multicol` regressed at fractional
scale. A child can cover a local box yet move or clip within a fragmentainer,
so the parent's background is still needed. The final rule preserves
borderless backgrounds for fragment continuations and boxes with a multicolumn
ancestor. A unit guard covers that ancestry. The diagnostic was not counted
as progress.

At clean checkpoint `2dce665c`, eight complete disjoint shards produced the
[v14 census index](generated/four-profile-census-v14.json):
**21,185/22,924 exact, 1,739 different, zero errors**. Relative to v13, only
five Open UI images changed. The two fractional negative-margin comparisons,
`flex-grow-006` at 1.25×, and `background-color-border-box` at 1.25× became
exact. `background-clip-color` at 1.25× remains different, but its mismatched
pixels fell from 225 to 150. No exact comparison regressed. All 22,924
Chromium decoded hashes and oracle identities stayed fixed. The residual
inventory is 965 unowned test IDs, so the complete release gate still fails.

The clean [v15 raster index](generated/focused-primitive-raster-v15.json)
records 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium hashes unchanged from v14. The clean expanded run measured
21,982/23,724 exact; all 200 additions retained their four-profile statuses
and decoded hashes. The [v5 requalification ledger](generated/expanded-requalification-v5.json)
retains 197 exact additions and demotes three. The historical archive and
Chromium oracle were not rewritten.
