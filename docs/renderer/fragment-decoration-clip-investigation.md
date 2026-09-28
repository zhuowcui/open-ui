# Fractional fragment decoration clips

Chromium is the pixel target. The old Open UI screenshot archive is historical
evidence, not an expected image. This investigation keeps the pinned Chromium
capture and zero pixel tolerance unchanged.

## Cause and repair

In `wpt/css_break/fieldset-001-ref` at 1920×1080@1.5, the previous renderer
missed 56 pixels in one vertical line. The fragment's inline edge fell at
physical x=499.5. A hard decoration clip stopped border ink at that half-pixel
edge, so Open UI painted gray where Chromium retained fractional black border
coverage. The reduced fragment inspection located the edge in the continued
paragraph decoration, rather than in the fieldset's own border.

The shared paint path now extends a truncated fragment's decoration clip to
the next physical pixel boundary **only when the fragment paints an inline
border**. Its block-axis cutoff stays hard. A background without inline border
keeps the original clip because adjacent column continuations can otherwise
paint the same fractional background edge twice. The rule depends on fragment
geometry and border ink, not a test name or a post-raster pixel correction.

Two dirty experiments changed solid-border coverage directly, including its
suppressed-side path; neither changed the decoded pixels in the fieldset or
235 nearby border cases. A first broad decoration-clip experiment made
three comparisons exact, but changed the previously exact
`wpt/css_break/monolithic-overflow-005.tentative` at 1280×720@1.25 by 50
pixels. Its adjacent green background continuations painted the same edge
twice. That version was rejected. The guarded rule restored that test's exact
result in all four targeted profiles.

## Clean verification

At clean code checkpoint `4b89fd05`, eight complete disjoint shards produced
the [v16 census](generated/four-profile-census-v16.json): **21,208/22,924
exact, 1,716 different, zero errors**. Against the previous v15 clean census,
five Open UI decoded images changed. Only the 1.5× fieldset reference became
exact, from 56 differing pixels to zero. Four already-different fieldset and
flex-content comparisons became smaller. No previously exact comparison
regressed. All 22,924 Chromium oracle identities and decoded hashes stayed
fixed. The census still has 949 unowned residual test IDs and is not a
qualifying release result.

The clean [v17 raster index](generated/focused-primitive-raster-v17.json)
records 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded hashes unchanged from v16. The complete expanded run
measured 22,005/23,724 exact and zero errors. All 200 native final-state
additions retained their four-profile statuses and all 800 Open UI and
Chromium decoded hashes; 197 remain exact and three demoted. Its
[v7 ledger](generated/expanded-requalification-v7.json) leaves the original
manifest and pending candidates unchanged.

The remaining 1.25× fieldset reference still differs by 1,522 pixels; the
fieldset test itself differs at both fractional desktop scales. Those are
open renderer residuals, not passes inferred from the repaired edge.
