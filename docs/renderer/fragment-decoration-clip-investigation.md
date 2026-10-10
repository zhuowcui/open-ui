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

At that checkpoint, the 1.25× fieldset reference still differs by 1,522 pixels; the
fieldset test itself differs at both fractional desktop scales. Those are
open renderer residuals, not passes inferred from the repaired edge.


## Native fieldset paint follow-up

Open UI executes no JavaScript. The consuming
[native Rust app](../../bindings/rust/openui/examples/native_fragmented_fieldset.rs)
creates fieldsets and legends, changes opacity from Rust click callbacks, and
reads owned bounds and fragment rectangles. Chromium alone supplies expected
geometry and pixels.

The reduction exposed two shared paint errors. The column overflow pass
replayed decorations already owned by the column's prepaint pass, applying
fractional border coverage twice. Sliced padding/content background colors
also used the continuous source box as their fill and clip boundary. The
shared correction skips already-painted decorations and uses the local
fragment border box for its color fill and enclosing hard clip. Continuous
source geometry still supplies the content/padding insets. These changes
contain no test IDs, expected colors, scale thresholds, or pixel edits.

The original reduction reproduced 2,336 wrong pixels at 1.25 scale and 1,104
at 1.5. The corrected source is exact at all five scales: 1, 1.25, 1.5, 2 and
3. Removing the translucent fill left the same 1,284 background-edge errors
in the intermediate border-only candidate, isolating the second error.
All 143 owned bounds and 143 fragment-rectangle sets agree with Chromium;
callbacks, independent process repeats and handle teardown pass.
Four neighboring opacity/phase cases still differ by 322, 386, 386 and 849
pixels. Their edge causes remain open; the reduction is not universally exact.

Clean private source `0a5f5587` completes the official fresh CPU build and
all required matrices:

| Suite | Exact | Different | Errors |
|---|---:|---:|---:|
| Focused | 640/640 | 0 | 0 |
| Primitive | 960/960 | 0 | 0 |
| Original | 21,342/22,924 | 1,582 | 0 |
| Expanded | 22,145/23,728 | 1,583 | 0 |

Each complete census gains four exact comparisons against the last public
keyboard qualification, with no exact losses or worsened differences.
Seven original rows change; all 48,252 Chromium reference fields remain
unchanged. `fieldset-001` and `fieldset-001-ref` now match at all four required
profiles. `fieldset-004` still differs by 75 pixels at 1.5, and the nested
multicol fieldset remains different. There are 882 original residual IDs;
200/201 expanded additions are exact at every required profile.

The [append-only evidence](generated/native-fragmented-fieldset-v1.json)
preserves 348 hashed files, including repeated references, minimized inputs,
bounds, connected regions, channel deltas, build receipts, completed audits,
and strict actual exits. Earlier incorrect legend shorthand and unstable
capture attempts are retained. The failed audit filename lookup is retained
alongside its corrected successful audit. No frozen image, oracle, manifest,
or prior evidence is rewritten.

The umbrella branch adopts the measured shared paint logic with standard
Rust formatting. The public app receives formatting and an accurate
diagnostic header. Fresh combined
umbrella qualification is pending. Both full pixel gates remain failed with
actual exit 1. Remaining native APIs, residual ownership, compositor, hardware
and release qualification remain open.
