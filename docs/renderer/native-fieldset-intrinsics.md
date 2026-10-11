# Native fieldset intrinsic sizing

Open UI runs no JavaScript. A consuming app creates retained elements, supplies
image bytes, and changes their typed styles through public Rust methods and
Rust callbacks. Chromium supplies the expected geometry and pixels.

## Measured browser behavior

The private intrinsic-sizing source `c170aa7e` loses 70 original exact
comparisons and four expanded comparisons. Two independent Chromium processes
now query every changed case at eight profiles: 544 queries over 34 cases.
Their observations agree. These queries produce no screenshots and establish
no native pixel passes. The [versioned evidence](generated/native-scroll-insets-v30.json)
preserves the measurements and the earlier failures.

A reduced case uses a natural 200 × 200 image with `height: 100%` inside a
`width: fit-content` container. Changing the container height through native
Rust must reproduce these Chromium results:

| Container | Container width at height 100 | Container width at height 150 | Displayed image width |
|---|---:|---:|---|
| Ordinary div | 100 | 150 | 100, then 150 |
| Fieldset | 200 | 200 | 100, then 150 |

Two independent Chromium runs agree on all 80 container/image bounds
observations at five scales. The unchanged reduced inputs and observations
are in the [query evidence](evidence/native-fieldset-intrinsics-v1/fieldset-percentage-height-queries-v1.json).
These are browser measurements; the native guard has not run yet.

## Prepared shared correction

Blink measures anonymous fieldset content with an indefinite available
block-size when computing intrinsic contributions. Used layout later resolves
the image's percentage height against the actual content box. Open UI's new
intrinsic collector reused the used-layout context, making these two operations
share the definite fieldset height.

Private `2518bdcd` adds a separate intrinsic collection context while preserving
normal used layout. Its [reviewable patch](evidence/native-fieldset-intrinsics-v1/native-fieldset-intrinsic-context-v1.patch)
and [native Rust consumer](evidence/native-fieldset-intrinsics-v1/native_fieldset_intrinsics.rs)
use shared layout behavior and public APIs. The app registers real image bytes,
changes height from a Rust click callback, reads owned bounds, renders both
states, and checks teardown. Layout owns this correction; no test-ID or
post-raster pixel branch is added.

Ten read-only repository checks pass on the clean source. Compilation, the
old-source failure, the fixed guard, and all native pixel comparisons remain
pending. The queued verification preserves the 1,000 existing sizing
measurements and checks 128 new native images, 280 affected original
comparisons, both 40-profile matrices, and complete original/expanded censuses.
It waits for the earlier pipelines to finish before running Cargo. Local
builds and image sweeps remain sequential.

This source inherits the other intrinsic regressions and remains unapplied.
No new case is admitted or qualified. Needed element behavior remains a
public native Rust API obligation even when a Chromium test uses a script.

## Completed corrected consumer source

`abed078d` corrects the native example's border argument without changing its
coordinates or production renderer. Its named baseline/fixed guards and ten
build stages pass. All 1,000 prior intrinsic geometry observations pass, and
the focused/primitive matrices are exact. The native app stops on unequal
consecutive Chromium captures after 62/128 images: 62 exact bounds and 50 exact
pixels, before reaching any fieldset case. That incomplete run is not a pass;
a separate investigation preserves both unstable captures.

The [complete diagnostic](generated/native-scroll-insets-v40.json) now covers
all 64 cases and 128 images, including the fieldsets. All 128 native bounds
match, 104/128 images are exact, and all 64 native repeats are deterministic.
Each reference state is captured twice in each of two independent Chromium
processes; no unequal pair occurs in this run. The 24 pixel failures occur at
1.25 scale, with bounds, connected regions and channel deltas preserved.
Their causes and ownership still require review. The earlier unequal pair
and complete census regressions remain open. This diagnostic does not close
compiled-source attribution or qualify the renderer.

Both complete censuses now finish with actual exit 1: 21,270/22,924 original
and 22,069/23,728 expanded exact, zero errors. Six comparisons become exact
against the intrinsic parent, but 64 original and four addition comparisons
remain exact losses against the accepted renderer. Only 199/201 addition
cases are exact at all four profiles. The
[complete audits and subsequent intrinsic investigation](native-intrinsic-constraints.md)
preserve these failures and identify two shared layout causes. This source
remains unapplied and admits no release state.

## Separate LCD trial

Private `52788b83` completes its build and native measurements. Explicit LCD
text fixes 24/60 static-position images; default text remains 0/60 exact. All
120 native bounds agree and all repeated outputs are identical. Its focused
suite is 600/640 exact: `multicol-block-no-clip-001` loses exactness in every
one of the 40 profiles. The primitive suite remains 960/960 exact. Chromium,
font, resource and oracle inputs are unchanged. Both full censuses are now
complete. The original suite is 20,771/22,924 exact and the expanded suite is
21,570/23,728 exact, both with zero errors and actual exit 1. It loses 493
previously exact comparisons, all using the explicit FreeType capture profile.
The [font engine investigation](native-font-engines.md) records the shared
routing discrepancy and prepared physical-outline correction. Native
causation and correction verification remain pending; this trial is unapplied.
