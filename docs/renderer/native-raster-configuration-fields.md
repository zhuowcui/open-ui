# Native text raster configuration

Applications configure and change their retained documents through public Rust
methods and Rust callbacks. Open UI executes no JavaScript. Chromium remains
the sole pixel target, with zero tolerance.

## Current validation checkpoint

The [original harness-failure review](generated/native-review-v2.json) records the
public-export source `e0dc491e`. Its harness executes five baseline clean/test
commands, then fails while restoring a nonexistent branch. The root remains
clean at the default-strike baseline. No fixed tests, consuming applications
or pixel matrices execute. Actual process exits and original receipts are
preserved; the failed harness is terminal and a fresh corrected run is required.
This establishes no raster qualification. Earlier results below belong to
their named sources.

The [fresh retry](generated/native-review-v3.json) preserves those failed bytes
and corrects the restore branch in new probes on a fresh source root. All
three named baseline assertions fail as required, then all fixed assertions,
neighboring guards and text, paint, engine, software and Ganesh test suites
pass. Ten read-only checks also pass. Its whole pipeline has now finished its
clean native build, consuming apps and pixel matrices. The complete results
below reject this source for application; a scoped guard pass does not qualify
the raster changes.

The [next preserved checkpoint](generated/native-inline-fallback-v1.json)
records all 17 clean build stages passing, including the locked workspace,
native Rust examples, C/C++ consumers and renderer executable source identity.
The native control stage also passes 60 cases, 120 images and 7,680 control
states. These are native contracts, not Chromium pixel qualification. The
application stage now finishes with exit 1 in the
[next evidence index](generated/native-keywords-v1.json): 828 of 840 native
contracts pass and twelve fail. All twelve failed cases have deterministic
images, unchanged logical geometry and successful native callbacks. Mask/phase
contracts remain incorrect. This is still not Chromium pixel qualification.
The [next measured evidence](generated/native-text-content-v1.json) records the
font consumer's exit 1: 73/1,200 images and 25,600/76,800 geometry states match
Chromium. Rust supplies all correct geometry and all 73 exact images. All 800
C/C++ images are blank white, with 51,200 incorrect zero-size text boxes. Their
reviewed cause is the C setter storing container data instead of creating a
Text child. The [shared native text correction](native-text-content.md) is
prepared separately; it is unapplied. The other 327 different Rust images
still need pixel root-cause review. Rust self-comparisons are not C/Rust passes.

The static consumer completes with 60/60 exact images under explicit Fontations;
default and FreeType each match 0/60. All 180 geometry states and repeated runs
agree. Its stage exits 0 for the explicit Fontations scope; this does not
qualify default native raster. The selection is 648/880 exact, 232 different,
zero errors, actual exit 1. Focused 640/640 and primitive 960/960 comparisons
are exact, actual exits 0. The [complete audit](generated/native-review-v4.json)
finishes original at 21,251/22,924 exact, 1,673 different and zero errors;
expanded is 22,050/23,728 exact, 1,678 different and zero errors. Both exit 1.
Against the accepted renderer, 83 original and 87 expanded comparisons lose
exactness; none gains exactness. Four lost comparisons belong to an addition,
leaving 199/201 addition cases exact at all four profiles. All Chromium inputs
remain unchanged across 46,652 original/expanded comparisons, and every
original row agrees between suites. Focused and primitive retain all 1,600
comparison invariants. The source is rejected for application. No accepted
renderer total or release admission changes.

The [authored glyph precision investigation](native-author-glyph-precision.md)
prepares a separate shared correction for premature rounding within text runs.
It has not executed its native guard or pixel matrices and inherits this
trial's regressions. It is also unapplied and unqualified.

The [v38 evidence](generated/native-scroll-insets-v38.json) preserves
`0e1f12ff`, which replaces two invalid test-only `fields_mut` calls with the
existing `update_derived` closure. Test assertions and renderer production
code are unchanged from `5ef50aac`. The corresponding corrected test-only
baselines are `b7284ac9` for phase/settings and `5390b683` for the default
physical strike. Their three named baseline failures and fixed passes now
reproduce at `3d4eea11`, alongside the full text, paint, engine and software/Ganesh
guard suites, in [v40](generated/native-scroll-insets-v40.json).

The parent hosted run finishes with six passing jobs and one failure. Strict
Miri passes the C version-prefix test. C UBSan stops while compiling an invalid
three-argument `oui_element_get_bounds` call in the new example. Private
`3d4eea11` uses the existing two-argument API; `OUI_OK` already requires a
layout box. Both old C/C++ consumers reproduce the compilation failure, and
both corrected consumers pass strict syntax checks. Public headers, exports,
Rust production code and workflows are unchanged by this consumer correction.

Both corrected sources pass ten read-only checks. All seven new-source hosted
checks pass, with zero skips, in [v39](generated/native-scroll-insets-v39.json).
Its local queue cleans all 18 workspace packages at every
source switch, requires all three named baseline failures, then runs a clean
17-stage build, native control/phase/geometry and Rust/C/C++ font consumers,
the 880-comparison selection and all four complete matrices. Builds and image
sweeps remain separate; all ten earlier whole pipelines must be terminal.
Five native C configuration boundary tests also pass. The workspace then
stops before tests or pixel comparisons: `native_control_raster` imports
`openui_geometry`, which is not a direct application dependency.
Private `fb284c54` exposes `RasterBackend`, `RasterPixelGeometry`, `TextEdging`,
`TextHinting` and `TextRasterConfiguration` through the shared Engine, public
`openui` API and prelude. The example now imports only the public API. No
renderer body, dependency, C export or C layout changes. Ten read-only checks
pass on this revision. Its clean native and pixel queue waits for every earlier
whole pipeline, and its own-source hosted checks remain pending.
Compilation and pixel failures remain required failures. This source is
unapplied and inherits the unqualified raster production changes below.

## Prepared shared renderer correction

Clean private `bcb7063f` follows the C configuration consumer correction
`ac08ec56`. Its [versioned evidence](generated/native-scroll-insets-v33.json)
preserves the [source patch](evidence/native-raster-fields-v1/native-raster-fields-v1.patch),
test-only baseline, native app, verification programs and waiting pipeline.
It is unapplied. Compilation, baseline/fixed regression tests, native execution
and fresh pixel comparisons remain pending.

The source review found that author and embedded phase fields were ignored,
native phase was largely cancelled by origin subtraction, and custom
Fontations outlines overrode requested text settings. This candidate changes
the shared renderer:

| Setting | Prepared behavior |
|---|---|
| LCD phase | Offset glyph ink horizontally by the requested 1/64 physical pixel units after fitting layout-origin positions. Logical advances, boxes and hit testing retain their layout coordinates. |
| Edging and positioning | Carry the configured mask, subpixel positioning and linear-metric flags into the custom Fontations font. |
| Hinting and autohint | Choose unhinted, light, normal, full or monochrome fitting from the configured font; select the interpreter or automatic hinter using Skia/Fontations rules. |
| LCD orientation | Carry surface pixel geometry to full LCD outline fitting. |
| Explicit Fontations backend | Preserve this selection for aliased and grayscale author text as well as LCD text. |

Phase is shared by horizontal, vertical, combined and emphasis painting. It
applies to subpixel-antialiased ink; aliased and grayscale masks ignore the
LCD-specific offset. Two private 10px-only phase/origin adjustments and their
calibration tests are removed. Physical-strike fitting and configured origins
must establish correct pixels through the unchanged Chromium gates.

The fitting logic follows the available local Skia Fontations source. That
checkout and the pinned executable are different patch versions; source review
does not establish agreement with the executable. All pixel comparisons still
use pinned Chromium `147.0.7727.50` and independently recorded inputs.

## Verification

The test-only baseline is `ee3d4134`. Its two new tests must fail by name, rather
than merely fail to compile. The correction must pass both tests and the
existing physical-outline neighbor. They check all three text roles, five
scales, integer physical phase shifts and requested mask/position flags.

The [native Rust application](evidence/native-raster-fields-v1/native_raster_fields.rs)
uses public immutable options, element setters and a Rust click callback. It
keeps owned geometry snapshots and verifies document teardown. Its planned
840-case application run checks repeatability, mask behavior, physical phase
distance and unchanged layout geometry. These API checks admit no Chromium
pixel cases.

Separate fresh Chromium comparisons cover 200 font cases, 400 images and
25,600 logical origin states, followed by the complete original, expanded,
focused and primitive matrices. Existing intrinsic and static native consumers
are retained. The new pipeline waits until every command in the earlier
clean-cache, fieldset, font and C configuration pipelines is terminal.

All ten read-only repository checks pass. The first check run found generated
author-style inventory drift after emphasis began using a shared settings
helper. Normal regeneration corrected it; both check receipts are preserved.
Rust formatting passes. No runtime or pixel pass is inferred from those checks.

## Prepared default native physical strike

Clean private `5ef50aac` extends the correction above. Its
[v34 evidence](generated/native-scroll-insets-v34.json) preserves the test-only
baseline `d01f9a17`, shared paint correction `e12bf951`, and a consuming
[native Rust control app](evidence/native-default-strike-v1/native_control_raster.rs).
Custom native and embedded outlines must be fitted at their physical font size
before painting. Scaling an outline already fitted at its logical size changes
the ink. The default Skia path must use the same physical-strike behavior as
the explicitly selected CPU paths.

The new guard compares all output bytes for 240 combinations of text role,
family, logical size, device scale and backend. The app creates native file-input
labels and changes opacity from a Rust callback. Its planned 60-case run checks
120 images and 7,680 control states, owned bounds, repeatability and teardown.
It adds no file-picker behavior. These native checks are not Chromium pixel
qualification.

Ten read-only checks and Rust formatting pass. Compilation, the named baseline
failure and fixed guards, native execution and Chromium comparisons remain
pending. Neither this source nor its raster-field parent is applied.

The original waiting queues had an incorrect expanded-summary filename. Both
owners were stopped with actual exit 143 before any stage ran. Their scripts
and published snapshots remain unchanged. Corrected queues `v1337` and `v1338`
retain the same sources and stage probes, and still wait for every command in
the earlier pipelines.

## Open work

The retry compiles and completes the consuming-app stages above, with required
native contract and pixel failures. Full renderer verification is pending.
Existing family/size hinting and FreeType aliased-profile overrides require review.
The default native physical-strike correction above still needs qualification.
Variable, synthetic, color-font and transformed/vertical text behavior also
need qualification. Copying configuration into the Engine does not prove those
behaviors complete.

The accepted umbrella renderer remains 21,334/22,924 exact, with 1,590
differences and zero errors. This candidate changes no accepted result, export
count or admitted release case. Every required native API and pixel gate
remains required.
