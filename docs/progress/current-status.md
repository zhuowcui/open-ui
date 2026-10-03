# Open UI current status

Open UI is in v0.2 release-candidate closure for the pure-Rust Linux/headless
product. Waves W0 through W10 are locally committed. W11 source packaging and
documentation are implemented, but final external and hardware qualifications
remain open.

## Verified repository state

| Evidence | Result |
|---|---:|
| Historical frozen SP20 pass records | 5,731, using a tolerant comparator |
| Optional historical byte replay | 5,549 unchanged, 182 changed, 0 errors; not a gate |
| Latest complete clean census against cached Chromium captures | Native API checkpoint `9e0f0145`: 21,308/22,924 exact, 1,616 different, 0 errors; all original and expanded comparison invariants unchanged from the accepted viewport renderer. The full pixel gate remains failing |
| Chromium oracle consistency audit | One older cached capture differs from six fresh captures under the same recorded identity; reconciliation open |
| Focused / primitive 40-profile matrices | 640/640 / 960/960 exact on clean native style/RGBA source `9e0f0145` |
| Expanded native final-state additions | 200/201 exact at all four profiles in the latest clean run; one still fails |
| Pending native final-state candidates | 0/35 exact at all four profiles after the latest clean recheck |
| Full inventory | 7,673 |
| Explicitly unported | 1,942 |
| Accountability audit | 7/7 |
| Application conformance scenarios | 58 across 10 domains |
| Frozen / current C exports | 84 / 110 |
| C examples / C++ consumers | 9 / 4, including native C/C++ window consumers; eight C and three C++ headless consumers run on clean `9e0f0145` |
| Workspace tests | 8,515 pass; 13 ignored on clean native style/RGBA source `9e0f0145`, with the Linux C feature enabled |
| Python closure, qualification, accountability and packaging tests | 244 pass |
| Owned objects after 10,000 mutation soak | no growth/leak |
| Unchanged-frame lifecycle | zero layout, paint, and raster work |

The [native style evidence](../v02/generated/native-primitive-styles-v1.json)
records clean `9e0f0145`. Public Rust, C, and C++ consumers exercise 35 primitive
longhands at five scales, mutate retained state from callbacks, and check
owned snapshots and pixels. Safe/unsafe alignment, `currentcolor`, optional
colors, and full unsigned counts survive C transport without changing any
of the 110 exports or 30 existing layouts. The consumer also exposed raw
N32 bytes being interpreted as RGBA. Shared CPU readback now requests RGBA
explicitly, correcting red/blue order for native apps and Linux presenters.
The full workspace passes 8,515 tests with zero failures and 13 ignored.
Its complete focused and primitive matrices are 640/640 and 960/960 exact
at zero tolerance. All 1,600 Open UI and Chromium images, oracle identities,
and difference signatures remain unchanged against the previous exact raster
evidence. Its [own complete original and expanded matrices](../renderer/generated/native-viewport-full-v17.json)
now finish with observed exits 1: 21,308/22,924 and 22,111/23,728 exact, zero
errors. Every comparison invariant remains unchanged from the accepted
viewport renderer, and all original rows agree between the two suites.
The same 200 of 201 additions are exact at all four profiles; the original
1,616 differences and fieldset/legend addition still fail.
Own-checkpoint hosted CI has six successful jobs and five skipped hardening
jobs; skips remain open results. Every needed public native operation still
requires implementation and consuming-application verification. Open UI
executes no JavaScript, in any version.

The [clean private SVG checkpoints](../renderer/generated/native-svg-viewport-v2.json)
implement a public native Rust viewport constructor and replace the historical
SVG decoration alpha with fixed viewport layout and shared border painting.
Source `4daf1876` passes 480 owned-bounds, Rust callback and teardown checks,
its native API unit test, and both complete 40-profile pixel suites. It makes
the tiny SVG guard exact at all four required profiles. Later `169fc7fe`
corrects curved double-border stripes on all four physical sides: the original
native suite gains 112 exact images with no exact loss, and all 240 changed
images improve. Fresh neighboring controls bring the measured native SVG
surface to 940/1,920 exact rendered states and 1,920/1,920 exact owned bounds, with all
native callback and teardown checks passing. The other 980 pixel results
remain failures. Both complete original censuses and the later raster suites
are running after preserved disk-exhaustion failures; expanded runs and full
workspace verification remain pending. The patches stay unapplied, and the
accepted full census remains 21,308/22,924 exact.

The [neutral PNG gamma correction](../renderer/generated/native-png-sampling-v4.json)
is committed at `63aeb672`. Both complete clean matrices have observed exit 1
and retain 21,308/22,924 original and 22,111/23,728 expanded exact results;
Chromium bytes and identities remain unchanged.
Three original images change, with lower total channel error and no loss of
an exact result. Two still-failing images each gain nine differing green
channel cells; these remain failures. The native PNG consumer gains 35 exact
images, reaching 183/540, while all 880 native image, opacity and clipping
bounds and teardown checks pass. Fresh clean umbrella binaries reproduce
every image and difference signature from clean `ebbe2b6f`. The 40-profile
matrices remain exact. All eight generated-contract checks, accountability
and archive integrity checks pass. The 1,616 census differences and residual
ownership review still keep the release gate open.

The [native viewport implementation](../renderer/generated/native-viewport-scroll-v4.json)
now exposes public Rust
`Element::scroll_metrics` over the shared engine, reserves paired scrollbar
gutters, clamps programmatic/wheel/smooth scrolling, resolves pending layout,
and clamps retained offsets after content shrink or viewport resize. The
[consuming Rust application](../../bindings/rust/openui/examples/native_viewport_scroll.rs)
uses typed construction, a Rust callback, normalized wheel input, owned
snapshots and checked teardown. All 850 offset, bounds and dimension checks
match Chromium. At clean qualification source `8f45444e`, 196/250 original and
488/600 direction guards are pixel-exact; all 510 comparisons at scales 1,
2 and 3 are exact. The remaining 166 fractional-scale failures stay failures.

Compared with the earlier immutable-recording checkpoint, physical scroll
snapping, correct thumb enclosure, and the whole-track path for non-scrollable
viewports remove 32,877 mismatched pixels and make three more images exact.
No exact image is lost or differing-pixel count increased. The pinned Chromium
trace reproduces four immutable oracle captures byte-for-byte and shows why
scrollable and non-scrollable controls require different shared paint paths.
Explicit texture-piece replay and a separate untagged composition surface
produce identical pixels; the extra implementations are omitted. The previous
[recording evidence](../renderer/generated/native-viewport-scroll-v3.json)
remains historical and unchanged. Its rejected thumb proposal enabled
antialiasing on interior edges; the current enclosure keeps those edges crisp.

The latest clean implementation passes 8,498 locked workspace tests with
13 ignored. Recorded controls survive live style mutation and document drop.
Fresh complete focused and primitive gates pass 640/640 and 960/960 exact,
with all 1,600 native and Chromium image hashes and oracle identities unchanged.
Fresh clean umbrella binaries reproduce all 850 native images, geometry and
difference signatures. The [complete pre-repair original/expanded reruns](../renderer/generated/native-viewport-full-v4.json)
at `8f45444e` now have observed exit 1: 21,292/22,924 original and
22,095/23,728 expanded exact, zero errors. Canonical result hashes and counts
are verified. All original rows agree between the runs, and every original
native image, Chromium image and difference signature agrees with the earlier
`574864d0` result. These complete failures do not qualify the clipping repair.

The [earlier complete viewport runs](../renderer/generated/native-viewport-full-v1.json)
at `574864d0` have now finished with observed exit 1: 21,292/22,924 original
and 22,095/23,728 expanded exact, zero errors. Their original rows agree;
200/201 additions remain exact at every profile. Against neutral gamma,
30 comparisons change and 16 lose exactness; all Chromium images and oracle
identities remain fixed. The preceding `8f45444e` runtime reproduces every one
of those 16 failed PNGs. These are owned by `openui-paint` and keep the gate
failing.

A [shared clipping repair](../renderer/generated/native-viewport-full-v2.json)
at clean `1366b72f` defers the outer viewport clip
to final surface/tile assembly when no scrollbar gutter is reserved. It
restores all 16 exact results in the 96-comparison affected selection and
preserves all 850 native images, geometry and difference signatures. Reserved
gutters retain their smaller client clip. The repair is now implemented in
the umbrella, with identical renderer, build, harness, resources and inputs
to that clean qualification source; only documentation differs. It passes
8,498 locked workspace tests, 13 ignored, and complete 640/640 focused and
960/960 primitive exact matrices with observed exits 0. All 1,600 native and
Chromium images and oracle identities stay unchanged. The
[complete repair runs](../renderer/generated/native-viewport-full-v7.json)
have now finished with observed exits 1: 21,308/22,924 original and
22,111/23,728 expanded exact, zero errors. All 16 earlier exact results are
restored across the complete census, with no new exact loss or worsened pixel
count. All 22,924 Chromium images and oracle identities remain unchanged;
the original rows agree between both runs. All 804 addition comparisons remain
unchanged, with 200/201 additions exact at every profile. Source remains clean
and unchanged, and binary hashes stay fixed. These complete failures keep the pixel gate open.
All eight generator checks, immutable archive
integrity, formatting, release-source verification and 7/7 repository metadata
accountability checks pass. The ordinary audit without clean-checkout mode
first failed on absent ignored historical PNGs; that log is retained, and
the existing `--repository-only` mode passes without changing any audit rule.

The [fresh clean umbrella reproduction](../renderer/generated/native-viewport-full-v3.json)
at implementation checkpoint `28cde831` builds after cleaning the six
renderer crates and reproduces all 850 native images, geometry, callbacks,
teardowns and difference signatures from `1366b72f`. Source remains clean
and unchanged through build and both completed native runs, which have
observed exit 0. The fresh umbrella release-source verification also passes;
these checks do not close the 166 native fractional-scale failures or the
unfinished complete renderer gate.

Eight fresh Chromium traces reproduce the immutable oracle PNGs byte-for-byte.
They reject the unsnapped-scroll hypothesis: hidden viewport scrolling also
uses a snapped screen-space transform. A clean per-axis scrollbar candidate
at `cc056cee` instead selects ordinary picture rendering for a forced bar with
no range while the opposite axis uses a composited scrollbar. Its adjacent
corner shares the ordinary picture. All 850 native geometry/callback/teardown
records stay unchanged; eight already failing images improve by 1,898 pixels,
with zero exact loss and no new exact image. The combined scrollbar and client
clip corrections are now implemented in the umbrella after the complete
[census comparison](../renderer/generated/native-viewport-full-v9.json).
Its [completed raster checks](../renderer/generated/native-viewport-full-v5.json)
pass 640/640 focused and 960/960 primitive exact, with all 1,600 native and
Chromium images and difference signatures unchanged. The combined candidate's
workspace and complete censuses are recorded below. The 166 native fractional
failures remain failures.

A subsequent clean client-scissor candidate at `079208f8` encloses the reserved
viewport client clip in physical pixels, following Chromium's integer compositor
scissor. Four already failing images improve, removing 640 differing pixels;
two retain their differing-pixel counts but improve color values. No changed
color-channel cell worsens, no exact image is lost, and no new image becomes
exact. All 850 native geometry, oracle, callback and teardown records remain
unchanged. These corrections are now implemented in the umbrella. Its
[completed clean workspace](../renderer/generated/native-viewport-full-v6.json)
passes 8,498 tests with zero failures and 13 ignored, restoring the test's
tracked output before verifying unchanged source. Its
[completed focused and primitive matrices](../renderer/generated/native-viewport-full-v7.json)
pass 640/640 and 960/960 exact, with observed exits 0 and all 1,600 native and
Chromium images, oracle identities and differences unchanged. Its
[complete original and expanded runs](../renderer/generated/native-viewport-full-v9.json)
have finished with observed exits 1: 21,308/22,924 and 22,111/23,728 exact,
zero errors. Every original and addition image, oracle identity and difference
is unchanged from the accepted clipping repair. All 8,282 tracked code, build,
test, resource and workflow files match clean `079208f8`. The
[fresh clean umbrella build](../renderer/generated/native-viewport-full-v10.json)
at `a5547e7e` reproduces all 850 native images, geometry, oracle records,
callbacks, teardowns and differences from that source, with observed exits 0.
Source remains clean and unchanged. The evidence also
verifies all 804 expanded addition images unchanged from `574864d0`, retaining
200/201 additions exact at all four profiles in the complete pre-repair run.

A private [immutable content-layer prototype](../renderer/generated/native-viewport-full-v7.json)
at clean `cae25afc` records content bounds, the background-color hint and a
proven opaque rectangle before raster. Chromium's largest covered rectangle
can omit part of the layer even when the canvas background is white; fresh
traces and recorded paint confirm that decision. The prototype reaches
706/850 native images exact, making 22 additional images exact without losing
an exact result. All 850 geometry/oracle/callback/teardown records and all 510
integer-scale results are unchanged. However, six already failing direction
cases worsen beside the scrollbar, and 144 fractional failures remain.
`openui-paint` owns the unresolved layer raster/composition differences.
The prototype remains unapplied and unqualified. Its locked workspace passes
8,498 tests, zero failures and 13 ignored, with clean source unchanged after
restoring tracked test output. Its
[complete focused and primitive matrices](../renderer/generated/native-viewport-full-v8.json)
pass 640/640 and 960/960 exact, with observed exits 0 and all 1,600 native and
Chromium images, oracle identities and differences unchanged. Complete
censuses and general clip/effect/transform metadata remain required; the six
composition regressions prevented acceptance of that checkpoint.
The earlier `80dd2353` attempt lost three exact images and is preserved as
rejected evidence. No reference pixels or final output pixels are rewritten.
The [solid-tile investigation](../renderer/generated/native-viewport-full-v9.json)
confirms that Chromium draws the affected white edge tiles as solid-color
quads, bypassing the background-hint clear used by ordinary raster tiles.
Wider viewports expose their grey debug borders; switching debugging off
restores all 12 normal captures byte-for-byte, including six existing oracles.
The new private prototype retains owned paint operations and proves tile
colors before raster. An intermediate attempt omitted thin recorded bands,
losing improvements in 33 already failing comparisons. Clean `d64d3f7f`
selects visible tiles before recording that fractional clip and reaches
706/850 exact with 22 newly exact, zero exact losses, no worsened mismatch
count or color-channel cell against `079208f8`. All geometry, callbacks and
teardowns remain exact, and all 510 integer-scale images remain exact.
Its 8,498 workspace tests pass, zero failures and 13 ignored. Twelve viewport
guards retain exact geometry and callbacks, with four exact images; eight
still fail. The prototype remains unapplied, with 144 fractional failures.
Its [complete focused and primitive matrices](../renderer/generated/native-viewport-full-v10.json)
pass 640/640 and 960/960 exact with observed exits 0, clean source unchanged,
and all 1,600 native/Chromium images, oracle identities and differences
unchanged. Its later completed original and expanded censuses retain
21,308/22,924 and 22,111/23,728 exact, zero errors, with all images and
differences unchanged. Those complete failures do not qualify a release.

The [private sampling and corner investigation](../renderer/generated/native-viewport-full-v11.json)
identifies a missing Chromium Skia build setting: `SK_ENABLE_LEGACY_SHADERCONTEXT`.
Clean `8d103fb1` adds only that setting to the comparison configuration, retains
the existing Skia pin, and reaches 823/850 exact native viewport images without
an exact loss or worsened color-channel cell. Its complete focused and primitive
matrices pass 640/640 and 960/960 exact, with all 1,600 images and oracle identities
unchanged. Clean `64fa1a7c` then preserves coverage at the independent scrollbar
corner and reaches 850/850 exact, plus 12/12 exact wider-viewport guards. All
native geometry, callbacks and teardown checks remain exact; reference bytes
remain unchanged. These prototypes are unapplied. The corner candidate's
[completed workspace](../renderer/generated/native-viewport-full-v12.json)
passes 8,498 tests, zero failures and 13 ignored, with tracked output restored
and clean source unchanged. Its full renderer and application qualification
remain open. Its [completed replacement raster matrices](../renderer/generated/native-viewport-full-v13.json)
are 640/640 focused and 960/960 primitive exact, with all image hashes and oracle
identities unchanged against the configuration-only candidate. Its
[completed original and expanded censuses](../renderer/generated/native-viewport-full-v14.json)
fail with 21,299/22,924 and 22,102/23,728 exact, zero errors. Against the
preceding solid-tile/accepted renderer, 144 comparisons change: 23 lose
exactness, 14 become exact, and 46 have more differing pixels. Every Chromium
image and oracle identity stays fixed; all 804 additions are unchanged and
200/201 remain exact at all four profiles. A clean configuration-only
304-comparison sweep reproduces every changed row, including all 23 exact
losses. The sampling flag change causes the measured regressions without
requiring the corner change. Shared image sampling remains owned by
`openui-paint` and raster build configuration, with the correction unresolved.
The candidate is rejected for promotion and remains unapplied.
The subsequent [generated-tile format correction](../renderer/generated-image-sampling.md)
at clean private `16fc959d` repairs all 18 generated-background exact losses
in the affected 304-comparison selection. That selection is 168 exact,
136 different, zero errors, with observed exit 1. Twenty-eight images change;
no differing-pixel count worsens against the private sampling candidate.
All 850 native images, geometry, callbacks and teardown checks remain exact
and unchanged. Complete focused and primitive matrices pass 640/640 and
960/960 exact over all 40 profiles, with all 1,600 comparison invariants
unchanged. Five exact regressions against the accepted renderer remain
under `openui-paint` ownership. A separate strict image-rectangle scroll
replay at `312806f6` changes none of the 304 selected rows or 850 native
controls. Both trials remain unapplied and do not establish a new full-census
total. The accepted native API code `9e0f0145` has now completed both of its
own censuses from unchanged clean source and its pinned binary. Original and
expanded counts remain 21,308/22,924 and 22,111/23,728 exact, with zero errors;
all comparison invariants remain unchanged. The full pixel gate still fails.
A later opaque-layer composition trial at clean `b3c54ea8` restores the four
scrolling losses. Its affected selection is 172/304 exact, zero errors; all
other 300 comparisons and all 850 exact native controls remain unchanged.
One SVG pixel regression remains. Conservative layer-opacity metadata and
transparent/clipped/effected neighbors must be verified before adopting the
trial. The [review](../renderer/generated-image-sampling.md#scroll-layer-composition-follow-up)
rejects direct CPU scroll-content replay, which worsens all four failures.
Its [completed own-source raster checks](../renderer/generated/native-viewport-full-v16.json)
are 640/640 focused and 960/960 primitive exact, with observed exits 0 and all
1,600 comparison invariants unchanged. All trials remain unapplied, and the
accepted full-census counts remain unchanged.

The [coverage-region follow-up](../renderer/generated/native-viewport-full-v17.json)
at clean `6368057f` replaces the background-color shortcut with owned physical
coverage and the visible sampling footprint. A first single-rectangle trial
lost two matches by discarding a smaller covering background. Retaining the
region union restores them: all 304 selected invariants and all 850 existing
native controls match `b3c54ea8`, and its own complete focused/primitive gates
are 640/640 and 960/960 exact. New public Rust consumers exercise 180 states
without JavaScript. Their bounds, offsets, client dimensions, Rust callbacks
and teardown checks pass; every scroll extent fails, and only 95 pixels are
exact. Sixteen controls regain the opaque canvas underneath transparent
backing samples. The SVG decoration alpha remains an unmodeled effect in the
coverage proof. SVG/native API work, scroll extents/ranges, control pixels
and complete candidate qualification remain required. The patches stay
unapplied, and none of these new states is admitted to the release matrix.
All hosted workflows at `fdac09eb` have now completed: six jobs passed and
five hardening jobs were skipped. Skips remain open qualification work.
The older solid-tile candidate's complete censuses have finished with observed
exits 1: 21,308/22,924 original and 22,111/23,728 expanded exact, zero errors.
All native and Chromium images, oracle identities and difference signatures
remain unchanged against the preceding accepted renderer. These complete
failures keep the release gate open. Its first corner raster attempts were stopped
with observed exits 143 because an overlapping workspace test can temporarily
write a tracked PNG. Those partial outputs are preserved and do not qualify.

All six ordinary hosted jobs at umbrella checkpoint `427f7df4` pass; five
skipped hardening jobs remain unverified. Later checkpoints require their
own hosted results.
The strict generated inventory now accounts for the existing thumb-color
property: 203 consumed public fields and zero unclassified fields. All eight
generator checks, archive integrity and the 7/7 accountability audit pass.
All six ordinary hosted checks at `d5cd5f04` pass; five skipped hardening jobs
remain unverified. The earlier full runs at `574864d0` exclude the later
recording, transform and picture changes. Native
scrollbar input/accessibility, nested scrolling ranges, propagation,
remaining C property conversion and renderer/release qualification remain open.
Open UI runs no JavaScript; all needed application operations remain public native Rust API
obligations.

The [native scroll API evidence](../v02/native-scroll-metrics.md) records clean
`dfae5ae9`: the additive C metrics query, pending-layout offset reads, independent
overflow-axis values and shared inset length mutations pass 28 API tests,
seven C examples, two C++ consumers, and a public Rust callback/resize consumer
at five scales. All previous 109 exports and 29 layouts remain unchanged; the
ABI now has 110 exports. Eight generators, archive integrity and accountability
pass. Two earlier failed C consumer attempts are preserved. Needed native API
coverage and renderer/release qualification remain open; these checks do not
qualify new pixels or a later hosted head.

The complete clean checkpoint `9b158cda` is
[21,308/22,924 exact](../renderer/generated/four-profile-census-v49.json),
with 1,616 differences, zero errors and 892 unreviewed residual IDs. Its
[complete delta](../renderer/generated/native-image-full-delta-v1.json)
verifies 30 improved comparisons, including 17 newly exact, against
`056421db`. None worsen or regress from exact. Every Chromium image and
oracle identity remains unchanged in all four suites.
The complete [expanded run](../renderer/generated/expanded-requalification-v32.json)
is 22,111/23,728 exact, with 1,617 differences and zero errors; all 22,924
original rows agree between the runs. The
[v34 diagnostic selection](../../tools/qualification/manifests/expanded-v34.json)
retains 200 of the declared 201 additions. The clean
[v51 raster index](../renderer/generated/focused-primitive-raster-v51.json)
is 640/640 focused and 960/960 primitive exact, with every image and oracle
identity unchanged from `056421db`. The first original/expanded execution was
interrupted by an environment reset and remains incomplete evidence. Separate
replacement runs have complete terminal reports and pass count, source and
hash validation. Their process handles expired before exit codes were observed;
no exit code is inferred. The source identity remained clean and unchanged
from build through final audit. These results do not close residual ownership
or the required pixel gate.

The shared [raster image opacity correction](../renderer/native-png-sampling.md#reviewed-opacity-difference)
is now committed at umbrella checkpoint `42cce619`. Its
[completed clean evidence](../renderer/generated/native-png-sampling-v3.json)
retains 21,308/22,924 original and 22,111/23,728 expanded exact comparisons,
with every native image and Chromium oracle unchanged from `9b158cda`.
The full matrices ran on clean `fccbcccb`; all code and build inputs match
`42cce619`, with only three documentation files differing. Fresh clean umbrella
consumer builds reproduce all 880 native images and owned bounds. The 540 PNG,
160 image-opacity, and 180 clipping comparisons are respectively 148, 109,
and 143 exact, gaining 210 exact results and losing none. One previously
failing clipped-image edge worsens and remains open. The 40-profile matrices
remain 640/640 and 960/960 exact. This checkpoint does not close the release
gate or the remaining native API obligations.

The [native image fallback correction](../renderer/native-image-fallback.md)
matches all 120 isolated public Rust image states in pixels and owned bounds.
Its complete development selection is 7,039/7,680 exact, with zero errors,
five newly exact comparisons and no formerly exact regression; focused and
primitive matrices remain 640/640 and 960/960 exact. Column owned geometry,
source-less frames and opacity remain open. Its clean complete original,
expanded, focused and primitive results are indexed above.

The [native opacity investigation](../renderer/native-image-opacity.md) adds
two consuming Rust applications and 60 script-free minimized states. A shared
paint prototype matches all 320 reduced opacity comparisons and retains the
120 plain image guards. The expanded shadow/outline/transform/clip checks are
90/140 exact, with one comparison worsening from 83 to 105 wrong pixels.
The prototype remains unapplied. All measured public image bounds are exact;
the click consumer mutates opacity through a Rust callback. These diagnostic
results do not replace the clean census or close native API qualification.

The [renderer executable source check](../renderer/renderer-build-identity.md)
now rejects a comparison executable built from different source before using
cached results or rendering. It also checks source and binary stability after
the run. The [verification index](../renderer/generated/renderer-build-identity-v1.json)
records real executable acceptance/rejection and an explicitly nonqualifying
single-case integration. It also preserves an attribution error in the newer
opacity development runs: their copied comparison binary was still the earlier
prototype. Fresh replacement focused and primitive matrices are 640/640 and
960/960 exact with unchanged images. The
[complete original replacement census](../renderer/generated/native-opacity-full-regression-v1.json)
is 21,297/22,924 exact, with 1,627 differences and zero errors. Its 23
formerly exact regressions prevent applying that prototype; all Chromium
images and oracle identities remain unchanged. The clean renderer counts
above remain authoritative.

The [source-less image investigation](../renderer/native-source-less-images.md)
adds 360 script-free states constructed by a consuming Rust app. Its unapplied
V3 paint prototype is 1,673/1,800 exact, including 891/900 source-less image
comparisons, with all owned bounds exact. It makes 582 reduced comparisons
exact against the preceding opacity prototype and worsens none. Its locked
workspace passes 8,491 tests with 13 ignored. These reduced cases do not
replace the complete release census or waive the global regressions. The
separate complete V2 original census is 21,302/22,924 exact, with 1,622
differences, zero errors and 23 formerly exact regressions. Its source checks
pass and all Chromium references remain unchanged; the prototype is rejected.

The [native SVG viewport investigation](../renderer/native-svg-viewport.md)
adds 96 script-free states through a proposed public Rust element constructor.
Its latest unapplied prototype is 134/480 pixel-exact and 480/480 exact in
owned bounds. The tiny double-border control is exact at all five scales,
but 346 native comparisons still differ and 54 already failing comparisons
worsen against the first viewport prototype. The locked workspace passes
8,492 tests with zero failures and 13 ignored. The old `49/50` SVG opacity
adjustment is removed in the prototype in favor of fixed viewport geometry
and shared border drawing. This remains diagnostic work; the accepted
renderer and release counts are unchanged.
Its complete original diagnostic is now 21,328/22,924 exact, with 1,596
differences and zero errors. It adds 20 exact comparisons against C9 and
loses none, while 13 already failing image comparisons worsen. Every Chromium
image and oracle identity remains unchanged, and source and executable
identities stay fixed. The complete expanded diagnostic is 22,131/23,728
exact, with 1,597 differences and zero errors. All original rows agree and
all 804 addition images are unchanged, retaining 200/201 exact additions.
Both observed terminal exits are 1; clean release qualification remains open.

The [native PNG application](../renderer/native-png-sampling.md) supplies real
encoded resources through public Rust APIs and changes size and opacity from
a Rust callback. All 540 current-branch runs pass callback, owned-bounds, and
teardown checks; 74/540 images are pixel-exact. The unapplied neutral-gamma
decoder prototype reaches 146/540 against an earlier prototype's 98/540,
with 48 new exact images and no exact regression. All Chromium captures are
fixed. The separate RGBA patch format trial loses three exact native images
and is rejected. These new controls are not admitted; standalone decoder and
complete clean renderer qualification remain open.

The preceding `497e322d`
[delta index](../renderer/generated/native-column-flex-full-delta-v1.json)
retains the original command's exit-143 observation alongside its complete
terminal report. Its pixels remain unchanged, including the four fractional
failures that worsened at the earlier `e9211183` checkpoint.

The `9b158cda` [fresh hosted hardening run](https://github.com/zhuowcui/open-ui/actions/runs/36840619251)
has finished. Miri, Rust 1.85, C UBSan and application/X11/Wayland checks pass.
Both native sanitizer jobs fail after passing 25 FFI tests, reporting 10,476
Fontconfig bytes in 236 allocations at exit. Fuzzing stops at `tree_mutations`
with 2,606 Fontconfig bytes in 59 allocations; its three later targets remain
unverified. The [seven-job index](../v02/generated/hosted-hardening-9b158cda-v1.json)
preserves every outcome and all three complete failed-job log hashes. The
font-manager lifetime/root cause and leak gates remain open. Incomplete
renderer runs are not qualifying evidence.

The preceding complete clean checkpoint `547c7081` is
[21,291/22,924 exact](../renderer/generated/four-profile-census-v45.json),
with 1,633 differences, zero errors and 900 unowned residual IDs. The
[complete delta](../renderer/generated/native-constrained-box-full-delta-v1.json)
retains all 22,924 Open UI RGBA images and Chromium oracle identities from
`822e0462`; no exact comparison regresses. Its complete
[expanded run](../renderer/generated/expanded-requalification-v28.json) is
22,094/23,728 exact, with 1,634 differences and zero errors. Every original
row agrees with the separate census, and all 23,728 images and oracle
identities are unchanged. The
[v30 diagnostic selection](../../tools/qualification/manifests/expanded-v30.json)
retains 200 of the declared 201 additions exact at all four profiles.
The clean [v47 raster index](../renderer/generated/focused-primitive-raster-v47.json)
is 640/640 focused and 960/960 primitive exact. Complete renderer
qualification and ownership of the remaining differences stay open.

The preceding complete clean checkpoint `822e0462` is
[21,291/22,924 exact](../renderer/generated/four-profile-census-v44.json),
with 1,633 differences, zero errors, and 900 unowned residual test IDs. Its
[complete delta](../renderer/generated/native-spanner-boundary-full-delta-v1.json)
makes 13 comparisons exact, changes 26 Open UI images, and has no exact
regressions from `0ad4b12f`. Every Chromium image and oracle identity remains
fixed. Two existing fieldset differences at 1.25× increase by 87 pixels each.
This checkpoint is not renderer qualification. Its complete
[expanded run](../renderer/generated/expanded-requalification-v27.json) is
22,094/23,728 exact, with 1,634 differences and zero errors; all original rows
agree with the separate census, and all declared 201 additions remain included.
The [v29 diagnostic selection](../../tools/qualification/manifests/expanded-v29.json)
lists the same 200 additions exact at all four profiles. Its
[focused/primitive matrices](../renderer/generated/focused-primitive-raster-v46.json)
are 640/640 and 960/960 exact, with all 1,600 images unchanged from `0ad4b12f`.

The subsequent [native constrained-box geometry repair](../v02/native-element-geometry.md#constrained-boxes-and-visible-child-overflow)
separates a box's own size from the continuation space carrying child overflow.
The consuming Rust and C applications check dynamic size changes, owned
rectangles, child input, and accessibility through the shared engine. It also
retains independently sized child boxes after their atomic content ends.
Its complete clean renderer runs are indexed above; those remaining pixel
failures and native API qualification remain open.

The preceding [complete clean census](../renderer/generated/four-profile-census-v41.json)
at `15f9f12d` is 21,266/22,924 exact, 1,658 different, and zero errors
against the cached Chromium captures.
The [circular background one-axis clip repair](../renderer/circular-background-one-axis-clip.md)
makes the F16 circular fill apply the same horizontal overflow scissor as the
direct rounded fill. One original comparison became exact; only that Open UI
image changed, and no exact comparison regressed. All 22,924 Chromium oracle
identities and decoded images stayed fixed. The 914 residual test IDs remain
unowned. The clean
[v42 focused/primitive index](../renderer/generated/focused-primitive-raster-v42.json)
is 640/640 and 960/960 exact, with all 1,600 images unchanged from v41.
The complete [v24 expanded requalification](../renderer/generated/expanded-requalification-v24.json)
is 22,069/23,728 exact, 1,659 different, and zero errors. Every original
result matches the separate census; all 804 addition images stayed fixed,
leaving 200 of 201 additions exact at all four profiles. All 23,728 Chromium
oracle identities and decoded images stayed fixed. The
[v26 diagnostic selection](../../tools/qualification/manifests/expanded-v26.json)
lists those 200; the release contract retains all 201, including the fieldset
legend failure. The other 35 AST-lowered candidates remain pending.
The [fractional Ahem legend investigation](../renderer/fieldset-legend-ahem-raster-investigation.md)
records that case's 14 extra glyph rows and three rejected shared-raster
experiments; no renderer fix or qualification is claimed from that work.

The [scaled LCD font investigation](../renderer/scaled-lcd-hinting-oracle-investigation.md)
rejected a shared no-hinting change: among 904 real-font comparisons, 28 became
exact but 43 formerly exact comparisons regressed. The renderer was restored.
Of 12 fresh Chromium captures, 11 matched their cached images and one differed;
five additional fresh captures of that case all matched the first fresh one.
The [oracle audit](../renderer/generated/chromium-font-oracle-audit-v1.json)
records two decoded images under one identical recorded input identity. The
root cause is unreviewed, both variants remain preserved, and final renderer
qualification requires reconciliation. This diagnostic does not update the
clean census counts or qualify any renderer change.

At the prior clean checkpoint `e14e3e64`, the
[v39 census](../renderer/generated/four-profile-census-v39.json)
was 21,264/22,924 exact, 1,660 different, and zero errors.
The [vertical-lr Ahem rotation-anchor repair](../renderer/vertical-lr-ahem-rotation-anchor.md)
made nine original comparisons exact, changed only those nine Open UI images,
and reduced wrong pixels by 30,184. No formerly exact comparison regressed;
all 22,924 Chromium oracle identities and decoded images stayed fixed. The
916 remaining residual test IDs were unowned. The clean
[v40 focused/primitive index](../renderer/generated/focused-primitive-raster-v40.json)
was 640/640 and 960/960 exact. The complete
[v22 expanded requalification](../renderer/generated/expanded-requalification-v22.json)
was 22,066/23,728 exact, 1,662 different, and zero errors. The same nine
original comparisons plus one admitted native final-state comparison became
exact; 199 of 201 additions passed all four profiles. The
[v24 diagnostic selection](../../tools/qualification/manifests/expanded-v24.json)
listed those 199, while the release contract kept all 201 and their two
failures at that checkpoint. The other 35 AST-lowered candidates remained pending.

At the prior clean checkpoint `d39282e4`, the
[v38 census](../renderer/generated/four-profile-census-v38.json)
was 21,255/22,924 exact, 1,669 different, and zero errors.
The [analytic gradient-edge repair](../renderer/fractional-multicol-seam-investigation.md)
changed 29 Open UI images, made seven comparisons exact, and reduced the
remaining wrong-pixel count by 5,957. No formerly exact comparison regressed;
all 22,924 Chromium oracle identities and decoded images stayed fixed. The
923 remaining residual test IDs are unowned. The clean
[v39 focused/primitive index](../renderer/generated/focused-primitive-raster-v39.json)
is 640/640 and 960/960 exact, with all 1,600 Open UI and Chromium images
unchanged from the prior raster v38 index. The clean
[201-addition guard](../renderer/generated/expanded-additions-gradient-guard-v1.json)
is 801/804 exact, with the same three failures and all 804 Open UI and
Chromium images unchanged from the prior complete expanded run. The complete
[v21 expanded requalification](../renderer/generated/expanded-requalification-v21.json)
at clean checkpoint `e59ec07e` is 22,056/23,728 exact, 1,672 different,
and zero errors. All 22,924 original and 804 added images match their clean
v38 census and selected-additions guard respectively, including all Chromium
oracle identities and decoded image hashes. The
[v23 diagnostic selection](../../tools/qualification/manifests/expanded-v23.json)
lists the 198 four-profile-exact additions; the release contract retains all
201, including the same three failures.

At the prior clean checkpoint `dc451061`, the
[v37 census](../renderer/generated/four-profile-census-v37.json) was
21,248/22,924 exact, 1,676 different, and zero errors.
The [adjacent row-flex continuation repair](../renderer/adjacent-row-flex-fragmentation.md)
made three fractional-scale comparisons exact and reduced one other difference
from 132 to 14 pixels. Only four Open UI images changed across 22,924
comparisons; no formerly exact case regressed, and all Chromium oracle
identities and decoded images stayed fixed. The 924 remaining residual test
IDs are unowned. The clean
[v38 focused/primitive index](../renderer/generated/focused-primitive-raster-v38.json)
is 640/640 and 960/960 exact, with all 1,600 Open UI and Chromium images
unchanged from the prior raster v37 index. The complete clean
[v20 expanded requalification](../renderer/generated/expanded-requalification-v20.json)
is 22,049/23,728 exact, 1,679 different, and zero errors; 198 of 201
additions remain exact at all four profiles, with the same three failures.

At the prior clean checkpoint `128edc38`, the
[v36 census](../renderer/generated/four-profile-census-v36.json) was
21,245/22,924 exact, 1,679 different, and zero errors.
The fractional Ahem hinting repair changed 21 already failing Open UI images,
reduced their combined wrong-pixel count by 281, and introduced no exact
regression. All 22,924 Chromium oracle identities and decoded images stayed
fixed. Its [remaining edge mismatch](../renderer/fractional-ahem-stripe-edges.md)
still needs an exact source-level repair.
At the prior clean checkpoint `6676cf60`,
the [v35 image-delta audit](../renderer/generated/radial-ua-full-delta-v1.json)
found nine changed Open UI images, one newly exact comparison, and no
previously exact regression. All 22,924 Chromium oracle identities and decoded
hashes stayed fixed. The [radial tile repair](../renderer/radial-tile-edge-coverage.md)
reduced two already failing 1.25× cases from 626 wrong pixels to one each.
The [object fallback investigation](../renderer/object-fallback-host-clip.md)
records an exact-case regression in the intermediate v34 census and its scoped
repair; all six original `object` fixtures are exact at four profiles again.
The clean [v37 raster index](../renderer/generated/focused-primitive-raster-v37.json)
at `e1bdccc4` is 640/640 focused and 960/960 primitive exact with zero errors.
All 1,600 Open UI and 1,600 Chromium decoded images are unchanged from the
prior raster index.
The [201-addition clean guard](../renderer/generated/expanded-additions-object-deferred-v1.json)
at `6676cf60` retained 801 exact and three different profile comparisons;
none of its Open UI or Chromium decoded images changed from the prior guard.
This selected guard is not a complete expanded-manifest run.
The [new expanded admission](../renderer/adjoining-floats-native-admission.md)
retains all 200 prior additions and adds one case exact at all four required
profiles. The complete clean expanded run at `6b53a991` is
[22,046/23,728 exact](../renderer/generated/expanded-requalification-v19.json),
with 1,682 differences, zero errors, and 198 of 201 additions exact across
four profiles. The [v21 diagnostic selection](../../tools/qualification/manifests/expanded-v21.json)
lists those 198; the release contract still includes all 201. The same three
earlier additions remain failures. The remaining 35
[pending candidates](../renderer/generated/pending-mutation-candidates-v7.json)
produced 18/140 exact comparisons, 122 differences, and zero errors at clean
checkpoint `8324c6b0`; none is exact at all four profiles. All 140 comparison
statuses, mismatched-pixel counts, and diff signatures match the prior pending
report. At that prior checkpoint, the original release gate had 926 unowned
residual test IDs.
Open UI runs no JavaScript; the newly admitted interaction has a public native
Rust API path. A script in a Chromium test is a source-data fact, not an
application feature requirement or an excuse to omit a needed native Rust API.
The public `Element::detach` and `TextNode::detach` operations now keep authored
nodes available for reattachment while removing them from the presented
document. A new conformance scenario checks element lookup, focus,
reattachment, listener delivery, and eventual destruction; the text-node
scenario now checks detachment and reattachment too. That checkpoint's
conformance suite was 43/43. Seven additional native IME scenarios now check
final text after an empty preview, one-step undo, cancellation and selection
restoration, disabled/noneditable controls, and callbacks that cancel, redirect,
restart, or remove the target. The public `Document::dispatch_composition_cancel`
method exposes cancellation directly to native Rust applications; the suite
contains 50 scenarios. See [interaction and controls](../v02/interaction-controls.md).
At clean native-IME checkpoint `04394c86`, the
[v43 static raster guards](../renderer/generated/focused-primitive-raster-v43.json)
are 640/640 focused and 960/960 primitive exact, and all 1,600 Open UI images,
Chromium images, and oracle identities are
[unchanged](../renderer/generated/native-ime-raster-delta-v1.json). The locked
workspace passes 8,480 tests; Linux-enabled Rust/C/platform checks pass 185.
All six regular hosted checks and all six clean native C/C++ window processes
pass at that checkpoint. The five optional hosted hardening jobs were skipped.
This API work does not update the original or expanded census counts or close
the remaining renderer, native API review, or release-lab gates.
The subsequent [native empty-block border repair](../renderer/native-empty-block-fragment-slicing.md)
corrects child continuation geometry in ordinary block containers. The scoped
diagnostic guard makes 11 original comparisons exact with no exact regression
across its 112 comparisons; its Chromium images and identities stay fixed.
The later [complete clean census](../renderer/generated/four-profile-census-v42.json)
at `e51d88fd` is 21,278/22,924 exact, 1,646 different, and zero errors. It made
15 comparisons exact but regressed three previously exact profiles of one
nested multicol case. All Chromium images and oracle identities stayed fixed.
The source now preserves extracted-spanner row ownership; the 116-comparison
diagnostic guard restores those profiles and retains 12 new exact comparisons,
with no exact regression. A new clean full run is pending; the regressing
checkpoint is not accepted. The complete
[v25 expanded requalification](../renderer/generated/expanded-requalification-v25.json)
is 22,081/23,728 exact, 1,647 different, and zero errors, with 200/201 additions
exact at all profiles. The clean
[v44 focused/primitive index](../renderer/generated/focused-primitive-raster-v44.json)
remains 640/640 and 960/960 exact. These results do not close pixel qualification.

The [native geometry repair](../v02/native-element-geometry.md) exposes owned
`Element::client_rects`, corrects combined element bounds, and shares geometry
with accessibility and view timelines. Four new public Rust scenarios cover
columns, pointer eligibility, empty and singular boxes, scrolling, transforms,
and handle lifetime. The versioned C rectangle-copy API and a real C consumer
use the same engine. ABI verification preserves every existing struct layout
and all 84 frozen exports; its 107 exports, six headless C consumers, and
the C++ header consumer pass. The final locked workspace passes 8,486 tests with zero failures and 13 ignored.
All 54 public application conformance scenarios passed at that geometry checkpoint. Needed native API behavior
and the full renderer and release-lab qualifications remain open.
The subsequent [native text input repair](../v02/interaction-controls.md)
adds cancelable textarea Enter and read-only input enforcement across typing,
deletion, IME, and accessibility actions. Committed characters now emit one
input notification after the edit. The public suite has 55 scenarios, and the
Linux-enabled framework/engine check passes 162 tests with eight ignored.
The subsequent shared Rust/C/Linux-platform check passes 191 tests with eight
ignored; the full workspace passes 8,489 with 13 ignored, and all six C
consumers and the C++ header consumer pass. The 107-export ABI is unchanged.
These results do not update the clean renderer census or qualify release-lab
input and accessibility operation.
The public `Document::dispatch_key_input` now exposes the Linux adapter's
normalized input path to consuming headless and Linux Rust apps. Two additive
C input exports use the same document defaults, bringing the current ABI to
109 symbols without changing the frozen 84 symbols or any struct layout.
The updated Rust consumer scenarios and C checks verify separate logical key
names and committed text, cancellation, read-only state, owning-thread rules,
and callbacks inspecting the edited control. The local C window consumer
dispatches Enter and committed text from a presentation callback and observes
exactly two input notifications. Physical input and AT-SPI qualification remain
open.
The [normalized-input diagnostic](../v02/generated/native-keyboard-input-diagnostic-v1.json)
records 153 headless and 193 Linux Rust/C/platform passes, each with eight
ignored, 8,491 workspace passes with 13 ignored, 244 Python checks, and
all six headless C consumers plus the C++ header consumer. The subsequent
[clean window index](../v02/generated/native-keyboard-window-index-v1.json)
records six passing C/C++ processes on X11 software/OpenGL and pure Wayland
software at `07be54f9`. These checks
do not update the renderer census or complete physical input qualification.
The refined [spanner continuation repair](../renderer/native-empty-block-fragment-slicing.md)
recovers the three new ordinary-wrapper failures in a 20-comparison diagnostic
while retaining nested row overflow. The
[diagnostic index](../renderer/generated/native-spanner-boundary-diagnostic-v2.json)
records both layout guards, the reduced border sweep and all 1,600 unchanged,
exact focused/primitive images. Its completed
[wider diagnostic](../renderer/generated/native-spanner-boundary-cohort-diagnostic-v1.json)
is 7,034/7,680 exact, with 646 differences and zero errors: 13 comparisons
become exact and none regresses from exact, while two existing fieldset
differences worsen by 87 pixels each. Every Chromium image and oracle identity
is unchanged. Clean `822e0462` passes all
[1,600 focused/primitive comparisons](../renderer/generated/focused-primitive-raster-v46.json),
with every image unchanged from `0ad4b12f`. The complete
clean original/expanded censuses are indexed above. A larger start-margin candidate
remains held after two exact regressions; no reference or fixture was changed.
The append-only C `oui_element_detach` export uses the same engine operation
for element and text handles. `oui_document_element_by_id` now provides owned
handles for native C element lookup. That detach/lookup checkpoint has 107
symbols; the 84 frozen symbols and all C struct layouts remain intact.
`oui_app_run` and `oui_app_request_exit` now run native C applications through
Rust `App` and the same retained `Document`. Versioned platform callbacks can
mutate elements and request exit after engine and presentation borrows end.
[Clean native C/C++ consumers](../v02/native-c-lifecycle-evidence.md) exercise
X11 software/OpenGL and pure Wayland software windows. Shared Rust keyboard
handling now respects cancelled keydown events,
separates committed text from logical key names, and ignores text input on
noneditable controls. Physical release-lab and packaged application
qualification, including AT-SPI operation, remains open.
The locked Rust workspace suite passed with CI's 4 MiB libtest worker-thread
stack. The default 2 MiB worker stack still aborts in an existing fragmented
multicol integration test; its isolated rerun passes at 4 MiB.

A [fractional overflow-clip investigation](../renderer/fractional-rectangular-overflow-clip.md)
previously found a coincident button/child edge that differed by 125 pixels at
1.25×. A
dirty hard-clip experiment made that case exact but regressed 33 previously
exact comparisons in a 3,392-comparison diagnostic. A narrower contained-child
experiment still regressed four exact comparisons in 1,696 fractional-scale
comparisons. Both edits were reverted. The later scoped
[button repair](../renderer/button-content-clip-coverage.md) is the clean
qualification evidence above.

A [broken-image sampling investigation](../renderer/broken-image-fractional-sampling.md)
isolated five identical 1.5× differences to Chromium's fallback-image icon.
The pinned Skia fixed-point coordinate sequence made all five exact in the
clean census. No previously exact comparison regressed and every Chromium
oracle hash stayed fixed. Nineteen already failing comparisons changed size;
two scrolling/alt-text overflow cases worsened and remain open.

The [prior complete clean census](../renderer/generated/four-profile-census-v30.json)
at `814a2005` is 21,239/22,924 exact, 1,685 different, and zero errors.
The [table row-group clip repair](../renderer/truncated-table-row-group-clip.md)
made the remaining repeated-section comparison exact at 1.25×. Across all
22,924 comparisons, only that Open UI image changed; no exact image regressed
and every Chromium oracle identity and decoded hash stayed fixed. The clean
[v31 raster index](../renderer/generated/focused-primitive-raster-v31.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 decoded
images unchanged. The complete clean
[v16 expanded requalification](../renderer/generated/expanded-requalification-v16.json)
is 22,036/23,724 exact; all 800 addition images remain unchanged, with 197
of 200 additions exact across four profiles and three demoted. The
[v17 diagnostic selection](../../tools/qualification/manifests/expanded-v17.json)
retains those 197. The clean
[v5 pending-candidate index](../renderer/generated/pending-mutation-candidates-v5.json)
is unchanged at 22/144 exact, with one of 36 cases exact at all four profiles.
The repaired table case is exact in the
[40-profile sweep](../renderer/generated/table-row-group-cross-v1.json). The
release renderer gate still fails with 931 unowned residual test IDs.

The [prior complete clean census](../renderer/generated/four-profile-census-v29.json)
at `7cd8e574` is 21,238/22,924 exact, 1,686 different, and zero errors.
The shared [repeated table body-slice fix](../renderer/repeated-table-body-slice.md)
made 11 comparisons exact and reduced one other difference, with no
regression. Exactly 12 Open UI images changed and all 22,924 Chromium oracle
hashes stayed fixed. The [v30 raster index](../renderer/generated/focused-primitive-raster-v30.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v15 expanded requalification](../renderer/generated/expanded-requalification-v15.json)
is 22,035/23,724 exact, with all 200 additions unchanged: 197 exact, three
demoted. The [v16 diagnostic selection](../../tools/qualification/manifests/expanded-v16.json)
retains those 197. The clean
[v4 pending-candidate index](../renderer/generated/pending-mutation-candidates-v4.json)
again records one of 36 cases exact at all four profiles. The release renderer
gate still fails with 932 unowned residual test IDs.

The [prior complete clean census](../renderer/generated/four-profile-census-v28.json)
at `a0e3f4cd` is 21,227/22,924 exact, 1,697 different, and zero errors.
The shared [clipped replaced-background coverage fix](../renderer/clipped-replaced-background-coverage.md)
made two margin-trim comparisons exact and reduced a third iframe difference
from 342 pixels to six. Only those three Open UI images changed, all 22,924
Chromium oracle hashes stayed fixed, and no exact comparison regressed. The
[v29 raster index](../renderer/generated/focused-primitive-raster-v29.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v14 expanded requalification](../renderer/generated/expanded-requalification-v14.json)
is 22,024/23,724 exact; all 200 additions have the same pixels and statuses,
with 197 exact and three demoted. The
[v15 diagnostic selection](../../tools/qualification/manifests/expanded-v15.json)
retains those 197. The clean
[v3 pending-candidate index](../renderer/generated/pending-mutation-candidates-v3.json)
again records one of 36 cases exact at all four profiles. The release renderer
gate still fails with 937 unowned residual test IDs.

The [prior complete clean census](../renderer/generated/four-profile-census-v27.json)
at `8950b426` remains 21,225/22,924 exact, 1,699 different, and zero errors.
All 939 residual test IDs and their pixel diff signatures match the prior
clean census. The [v28 raster index](../renderer/generated/focused-primitive-raster-v28.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v13 expanded requalification](../renderer/generated/expanded-requalification-v13.json)
is 22,022/23,724 exact, 1,702 different, and zero errors; the same 197 of
200 additions are exact at all four profiles and the same three are demoted.
The [v14 diagnostic selection](../../tools/qualification/manifests/expanded-v14.json)
retains those 197. The clean
[v2 pending-candidate index](../renderer/generated/pending-mutation-candidates-v2.json)
records 22/144 exact comparisons. `adjoining-floats-dynamic` is now exact at
all four profiles and eligible for a future expanded-manifest admission; the
other 35 cases remain open. The original renderer release gate still fails.
The [nested float investigation](../renderer/nested-inline-float-propagation.md)
records the shared layout fix and the separate sticky-position residual.

The [prior complete clean census](../renderer/generated/four-profile-census-v26.json)
at `954648fb` changed exactly eight Open UI images in four resized one-axis
bitmap test/reference pairs at fractional scales. Two comparisons became
exact, six moved closer, and none regressed. All 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[shared bitmap-pattern investigation](../renderer/resized-one-axis-bitmap-pattern.md)
records the source rule and remaining one-to-three-pixel differences. The
clean [v27 raster index](../renderer/generated/focused-primitive-raster-v27.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded images unchanged. The clean
[v5 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v5.json)
remains 797/800 exact, with all 800 Open UI and Chromium decoded images
unchanged. The release census still fails exactness with 1,699 differences
across 939 unowned residual test IDs.

The [prior complete clean census](../renderer/generated/four-profile-census-v25.json)
at `5c7aaa9c` changed exactly eight Open UI images in four one-axis repeated
linear-gradient test/reference pairs. Their wrong-pixel counts fell sharply
at fractional scales, but none became exact because the separate PNG edge
still differs. No exact comparison regressed, and all 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[gradient picture-shader investigation](../renderer/one-axis-gradient-picture-shader.md)
records the source rule and remaining raster edge. The clean
[v26 raster index](../renderer/generated/focused-primitive-raster-v26.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded images unchanged. The clean
[v4 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v4.json)
remains 797/800 exact, with all 800 Open UI and Chromium decoded images
unchanged. The release census still fails exactness with 1,701 differences
across 939 unowned residual test IDs.

An [earlier complete clean census](../renderer/generated/four-profile-census-v24.json)
at `2f560e46` gained four exact comparisons from v23 by painting mixed border
styles in Chromium's alpha, style, and side order. Exactly 16 Open UI images
changed across four border fixtures; none regressed, and all 22,924 Chromium
oracle identities and decoded hashes stayed fixed. The
[border paint-order investigation](../renderer/mixed-border-paint-order.md)
records the compact case, source rule, rejected clip-only diagnostic, and
four-profile pixel counts. The clean
[v25 raster index](../renderer/generated/focused-primitive-raster-v25.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded hashes unchanged from v24. The clean
[v3 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v3.json)
remains 797/800 exact with three differences and zero errors; all 800 decoded
images and statuses are unchanged. The complete census still fails exactness
with 1,701 differences across 939 unowned residual test IDs.

An [earlier complete clean census](../renderer/generated/four-profile-census-v23.json)
at `d0592ccd` gained one exact comparison from v22:
`out-of-flow-in-multicolumn-047` at 1280×720@1.25. Its parent decoration
now ends at its used block size, and the outer visual continuation no longer
replays inner columns whose in-flow source was already consumed. Only that
Open UI decoded image changed; no previously exact comparison regressed.
All 22,924 Chromium oracle identities and decoded hashes stayed fixed. The
[consumed nested columns investigation](../renderer/multicol-consumed-nested-columns.md)
includes reduced reproductions of both masked errors. The clean
[v24 raster index](../renderer/generated/focused-primitive-raster-v24.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium hashes unchanged from v23. The census still fails exactness,
with 1,705 differences across 939 unowned residual test IDs.
The clean [v2 selected-additions
recheck](../renderer/generated/expanded-additions-recheck-v2.json) at
`db763999` measured 797/800 exact, three different, and zero errors. All
800 Open UI and Chromium hashes matched the prior full expanded run; this
selection does not replace complete expanded-manifest qualification.

The prior [v22 clean census](../renderer/generated/four-profile-census-v22.json)
at `6e21bd9b` gained one exact comparison from v21:
`out-of-flow-in-multicolumn-046` at 1280×720@1.25. Its zero-height
positioned parent no longer paints a background through the visual
continuation needed by its absolute child. Only that Open UI decoded image
changed; no previously exact comparison regressed. All 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[zero-height continuation investigation](../renderer/multicol-zero-height-positioned-decoration.md)
includes a reduced reproducer. The clean
[v23 raster index](../renderer/generated/focused-primitive-raster-v23.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium hashes unchanged from v22. The census still fails exactness,
with 1,706 differences across 940 unowned residual test IDs.

The prior [v21 clean census](../renderer/generated/four-profile-census-v21.json)
at `99fd4432` gained three exact comparisons from v20:
`flexbox_multi-line-row-flex-fragmentation-029` at 1280×720@1.25 and
`-030` at 1280×720@1.25 and 1920×1080@1.5. Only those three Open UI decoded
images changed; no previously exact comparison regressed. All 22,924 Chromium
oracle identities and decoded hashes stayed fixed. The
[final-decoration investigation](../renderer/flex-final-continuation-decoration.md)
records the reduced reproducer and the rejected broad rule. The clean
[v22 raster index](../renderer/generated/focused-primitive-raster-v22.json)
confirms 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded hashes unchanged from v21. The census still fails
exactness, with 941 unowned residual test IDs.

The prior [v20 clean census](../renderer/generated/four-profile-census-v20.json)
at `89a0a1f3` gained one exact comparison from v19:
`flexbox_multi-line-row-flex-fragmentation-027` at 1280×720@1.25.
No previously exact comparison regressed. Only that Open UI decoded image
changed; all 22,924 Chromium oracle identities and decoded hashes stayed
fixed. The [first-line break investigation](../renderer/nested-row-flex-first-line-break.md)
records the masked layout error, reduced reproducer, and neighboring guard.
The clean [v21 raster index](../renderer/generated/focused-primitive-raster-v21.json)
confirms 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded hashes unchanged from v20. The census still fails
exactness, with 943 unowned residual test IDs.

The prior [v19 clean census](../renderer/generated/four-profile-census-v19.json)
at `17952772` gained three exact comparisons from v18:
`out-of-flow-in-multicolumn-042`, `-043`, and `-045` at 1280×720@1.25.
No previously exact comparison regressed. Only those three Open UI decoded
images changed; all 22,924 Chromium oracle identities and decoded hashes
stayed fixed. The [relative continuation clip investigation](../renderer/multicol-relative-clip-translation.md)
records the cause, reduced reproducer, and authored-overflow guard. The clean
[v20 raster index](../renderer/generated/focused-primitive-raster-v20.json)
confirms both 40-profile matrices stayed exact with all 1,600 Open UI and
Chromium decoded hashes unchanged from the preceding clean v19 raster run. The
census had 944 unowned residual test IDs. The prior
[multicolumn investigation](../renderer/multicol-nested-positioned-continuation.md)
records the v18 repair. The prior
[fragment clip investigation](../renderer/fragment-decoration-clip-investigation.md)
records the v16 repair.
The [border-image seam investigation](../renderer/border-image-seam-investigation.md)
records a separate six-case fractional-scale residual and a rejected
offscreen-layer diagnostic; it changes no qualifying count.
The [repeat-space shader investigation](../renderer/background-repeat-space-shader-investigation.md)
records two rejected 92-comparison sampling diagnostics and a clean restored
runner; it also changes no qualifying count.

The clean [expanded v12 requalification](../renderer/generated/expanded-requalification-v12.json)
at `6e21bd9b` measured 22,015/23,724 exact comparisons, 1,709 different,
and zero errors. Only the same original `-046` image changed from v11, and it
became exact. All 23,724 Chromium oracle identities and decoded hashes stayed
fixed. All 200 native additions kept their prior four-profile statuses and
all 800 decoded Open UI images: 197 remain exact at all four profiles and
three remain demoted. The
[v13 diagnostic selection](../../tools/qualification/manifests/expanded-v13.json)
retains those 197 additions without changing the original manifest.

The prior clean [expanded v11 requalification](../renderer/generated/expanded-requalification-v11.json)
at `99fd4432` measured 22,014/23,724 exact comparisons, 1,710 different,
and zero errors. Only the same three original flex comparisons changed from
v10. All 23,724 Chromium oracle identities and decoded hashes stayed fixed.
All 200 native additions kept their prior four-profile statuses and all 800
decoded Open UI images: 197 remain exact at all four profiles and three remain
demoted. The [v12 diagnostic selection](../../tools/qualification/manifests/expanded-v12.json)
retains only those 197 additions; it does not alter the original manifest.

The prior [expanded v10 requalification](../renderer/generated/expanded-requalification-v10.json)
at `89a0a1f3` measured 22,011/23,724 exact comparisons, 1,713 different,
and zero errors. Only the same original `-027` comparison changed from the
previous full expanded run. All 23,724 Chromium oracle identities and decoded
hashes stayed fixed. All 200 native additions kept their prior four-profile
statuses and all 800 decoded Open UI images: 197 remain exact at all four
profiles and three remain demoted. The
[v11 diagnostic selection](../../tools/qualification/manifests/expanded-v11.json)
retains only those 197 additions; it does not alter the original manifest.

The prior clean [expanded v9 requalification](../renderer/generated/expanded-requalification-v9.json)
at `de8304fe` measured 22,010/23,724 exact comparisons, 1,714 different,
and zero errors. Against the prior v8 expanded run, four original
`out-of-flow-in-multicolumn-*` comparisons became exact: `-060` from the v18
census repair and `-042`, `-043`, `-045` from the v19 repair. No other Open UI
decoded image or status changed. All 23,724 Chromium oracle identities and
decoded hashes stayed fixed. All 200 native final-state additions retained
their prior four-profile statuses and all 800 decoded Open UI images: 197
remain exact at all four profiles and three remain demoted. The
[v10 diagnostic selection](../../tools/qualification/manifests/expanded-v10.json)
retains only the 197 exact additions; it does not alter the original manifest.
The prior [v8 ledger](../renderer/generated/expanded-requalification-v8.json)
and [v9 selection](../../tools/qualification/manifests/expanded-v9.json)
remain historical evidence.

At the earlier clean checkpoint `93b00601`, a
[four-profile additions recheck](../renderer/generated/expanded-additions-recheck-v1.json)
rendered all 200 native additions: 797/800 profile comparisons exact, three
different, and zero errors. The same three cases remain demoted. All 800
Open UI decoded hashes, Chromium oracle identities, and Chromium decoded
hashes match the prior clean v8 expanded run. This subset report is
diagnostic; it does not replace a complete expanded-manifest qualification.

The historical baseline records Chromium identity, viewport, device scale,
fonts, resources, and result hashes. A fresh pinned Chromium capture differs
from 203 archived Open UI images; the old screenshots are not expected output.
See the [audit](../renderer/frozen-oracle-audit.md). Chromium is the sole pixel
target but a test oracle only; supported builds and packages do not link or
load it.

## Implemented v0.2 surface

- Canonical generated typed style properties for Rust macros, engine metadata,
  and C tagged values.
- Thread-affine retained documents with generation-checked node handles,
  transactions, dirty generations, resources, hit testing, and immutable scene
  snapshots.
- Direct safe Rust framework with signals, effects, scopes, `Show`, keyed
  `For`, components, `view!`, `AppBuilder`, and `HeadlessApp`.
- Panic-contained C ABI with ownership/thread validation, structured errors,
  compound builders, events, controls, animation, owned accessibility-tree
  snapshots, and rendering.
- Core controls, routed pointer/keyboard/text/composition events, focus and
  modal containment, selection, Unicode editing, clipboard, undo/redo,
  scrolling, and pointer capture.
- Public native element activation and details disclosure: `Element::click`,
  `set_open`, and `is_open` use the same retained event and control path;
  closed details content leaves layout and the accessibility tree.
- Public native class-token changes and attached-document class lookup use
  the same retained tree from Rust callbacks.
- Public native kind lookup and `Element::kind` cover the deterministic
  test cases that find elements by tag, with attached-document order and
  generation-checked handles. Authored names such as `div` and `main` can
  share a native kind.
- Retained accessibility trees/actions and Linux AccessKit integration.
- Winit X11/Wayland runtime, backend-correct clipboard, IME/data events,
  softbuffer presentation, and multiple isolated windows.
- OpenGL presentation of the exact CPU Skia frame with automatic software
  fallback, diagnostics, resize recovery, and newest-scene scheduling.
- Typed transitions/keyframes, easing, manual clocks, lifecycle events,
  document/scroll/view timelines, smooth scrolling, snap, and reduced motion.
- Conformance, fuzz, sanitizer, Miri, soak, platform, performance, and
  reproducible package gates.

## Release blockers

- Direct Skia Ganesh raster builds behind explicit backend selection. On the
  clean Mesa llvmpipe [comparison](../renderer/generated/ganesh-raster-comparison-v1.json),
  it reached 408/640 focused and 624/960 primitive exact, below CPU Skia's
  640/640 and 960/960. It remains unpromoted; OpenGL presentation still uploads
  a CPU-rasterized frame.
- The [current hardening run](https://github.com/zhuowcui/open-ui/actions/runs/36365378115)
  still fails AddressSanitizer, LeakSanitizer, and fuzz on process-exit
  Fontconfig allocations. Its MSRV, Miri, Linux platform, and C UBSan jobs
  passed.
- The four-profile Chromium census fails exactness; 914 residual test IDs
  have no reviewed owner. Both 40-profile CPU raster matrices are exact.
- The C ABI covers the retained engine, headless renderer, owned full
  accessibility-tree snapshots, and the shared Rust Linux event loop;
  release-lab AT-SPI and packaged native C/C++ application qualification remain open.
- Retained per-node layers and compositor-owned animation curves are incomplete,
  so the strict blocked-UI 100-animation gate is not yet qualified.
- Full preserve-3d and backface layer semantics remain incomplete.
- Physical-GPU/context-loss and automated AT-SPI release-lab runs are pending.
- x86-64 and AArch64 clean-container packages must be built twice, compared,
  attested, signed, installed, and exercised.
- Crates.io publication requires final release approval and credentials.

See [release qualification](../v02/release.md),
[hardening](../v02/hardening.md), and the
[unsupported list](../v02/unsupported-features.md). Do not describe the WSL2
performance artifact as release qualification.

## Canonical local gates

```bash
cd bindings/rust
cargo test --workspace --locked
cd ../..

python3 tools/release/build_v02_linux.py --verify-source
python3 tools/accountability/audit.py
python3 tools/accountability/restore_frozen_openui_archive.py --check
python3 tools/accountability/audit_frozen_oracle.py --output /tmp/openui-historical-audit.json
python3 tools/ffi/verify_abi.py
```

Pixel qualification uses the pinned Chromium oracle through the complete
four-profile matrix described in the [renderer contract](../renderer/contract.md).
The archive check above only protects historical evidence.

For the workstation-only exact toolchain, pass
`--config .cargo/config.chromium.toml` to Cargo. Ordinary development and
release builds use the portable default configuration.

### Native atomic descendants: candidate evidence

The [shared paint and balancing candidate](../renderer/native-monolithic-column-balancing.md)
matches all 40 measured native states at five scales. All 65 preceding
constrained-box images and geometry records stay unchanged. An additional
80-pixel column constraint leaves three of eight scale-1 cases different.
All 8,491 workspace tests, 55 public Linux Rust scenarios, and the 109-export
C ABI gate pass. Complete renderer qualification for this candidate remains
required. At the preceding clean `547c7081`, the
[v47 focused/primitive index](../renderer/generated/focused-primitive-raster-v47.json)
is 640/640 and 960/960 exact with all 1,600 images and oracle identities
unchanged from `822e0462`. Its complete original and expanded runs are now
indexed above, with all images unchanged and the same remaining failures.

The atomic-column candidate's wider development checks are complete:
640/640 focused and 960/960 primitive exact, with 7,034/7,680 exact in the
four-profile column cohort. Its cohort images are identical to the paint-only
candidate. Four already-failing comparisons differ more than at `822e0462`;
no exact comparison regresses and no Chromium reference changes. These dirty
source diagnostics do not replace the complete clean renderer gates. The
[atomic-column index](../v02/generated/native-monolithic-column-balancing-v1.json)
records their counts, hashes and complete four-image delta.

The later [atomic-child deferral candidate](../renderer/native-atomic-column-deferral.md)
now matches 144 reduced native states against Chromium in both geometry and
pixels. The 65 earlier constrained guards retain their native bytes and
geometry. The full workspace passes 8,491 tests, all 55 public Rust scenarios
pass with Linux, and the 109-export C ABI passes six C consumers and the C++
header consumer. The new C guard rejects the preceding library. These remain
dirty development results. Its completed focused and primitive matrices are
640/640 and 960/960 exact; the column cohort is 7,034/7,680 exact, with 646
differences and zero errors. All 9,280 candidate images and Chromium oracle
identities are unchanged from the preceding painting and balancing source.
Complete clean renderer matrices and reviewed residual ownership remain
required before qualification.

The subsequent [vertical maximum-size correction](../renderer/native-vertical-max-block.md)
makes all ten reduced vertical comparisons exact at five scales. The full
reduced constrained-box sweep is 53/65 pixel-exact and 45/65 complete
geometry-exact. Its 55 horizontal guards and the 144 atomic-child states
retain their bytes and geometry. Public Rust and C consumers check both
vertical directions and last-column pointer interaction; the guards reject
the preceding source and library. All 8,491 workspace tests, 55 Linux Rust
scenarios and the 109-export ABI consumer gate pass. These are development
measurements; clean full matrices and release qualification remain required.

The clean vertical checkpoint `e9211183` passes the
[v48 focused/primitive gates](../renderer/generated/focused-primitive-raster-v48.json):
640/640 and 960/960 exact, with all 1,600 images and oracle identities unchanged
from `547c7081`. All six hosted jobs pass; MSRV, Miri, native sanitizer, C UB
sanitizer and fuzz jobs are skipped on the PR and remain required. Complete
original and expanded results are recorded in the
[v46 census](../renderer/generated/four-profile-census-v46.json) and
[v29 expanded index](../renderer/generated/expanded-requalification-v29.json),
including the four worsened fractional failures.

The separate [flow-root child-slice candidate](../renderer/native-flow-root-leaf-slices.md)
matches all five reduced flow-root states in geometry and pixels. The reduced
sweep is 55/65 pixel-exact and 50/65 geometry-exact, with all 60 other states
unchanged. A public Rust consuming application produces exact geometry and
pixels at all five scales and runs a native click callback through the same
retained document. Rust and C guards reject the preceding implementations.
The workspace, all 55 Linux Rust tests and the 109-export C ABI consumer
gate pass. Its uncommitted candidate matrices are 640/640 focused and
960/960 primitive exact; the wider column cohort is 7,034/7,680 exact,
with 646 differences and zero errors. All 9,280 images and Chromium oracle
identities remain unchanged from the atomic-child candidate. Complete
clean renderer matrices, residual closure and release qualification remain
required.

The separate [empty-fragment bounds correction](../renderer/native-empty-fragment-bounds.md)
returns the final all-empty rectangle, following Chromium's ordered union.
The public Rust and C guards reject the prior renderer and pass with the
correction, including native accessibility bounds. All 55 Linux application
scenarios and the 109-export ABI consumer gate pass. The reduced sweep is
55/65 geometry-exact and 55/65 pixel-exact; the five zero-height bounds
become correct, all 60 other geometry records and all 65 PNGs remain
unchanged. All 8,491 workspace tests pass, with 13 ignored in 147 suites;
formatting and read-only generators pass. Complete clean renderer verification
and final release qualification remain required.

The complete clean `e9211183` [v46 census](../renderer/generated/four-profile-census-v46.json)
is 21,291/22,924 exact, with 1,633 differences and zero errors. The
[v29 expanded requalification](../renderer/generated/expanded-requalification-v29.json)
is 22,094/23,728 exact, with 1,634 differences and zero errors; the same
200 of 201 additions pass all four profiles. All original rows agree between
the runs and every Chromium image and oracle identity stayed fixed. The
[full delta](../renderer/generated/native-vertical-max-block-full-delta-v1.json)
shows four original comparisons changed: `block-max-height-004` and its
reference worsen from 554 to 626 different pixels at 1.25 scale, and from
456 to 531 at 1.5 scale. All other images stayed fixed and no formerly exact
comparison regressed. Those fractional continuation/coverage failures need
review and repair; unchanged totals do not establish renderer stability.
The 900 residual IDs remain unreviewed, so this is diagnostic evidence.

The explicit [hosted hardening run](https://github.com/zhuowcui/open-ui/actions/runs/36812728952)
at `6e7901ca` finished. Miri handles, Rust 1.85, native C UBSan and application/
X11/Wayland conformance passed. Both native sanitizer jobs passed 25 FFI tests
before reporting 10,476 leaked Fontconfig bytes in 236 allocations at process
exit. Fuzzing stopped on a 2,606-byte Fontconfig leak in 59 allocations; the
remaining targets therefore remain unverified. The
[seven-job index](../v02/generated/hosted-hardening-6e7901ca-v1.json) records
all outcomes and raw log hashes. The leak gates remain failing.

The [column-flex continuation checkpoint](../renderer/native-column-flex-overflow.md#combined-native-checkpoint)
now combines item source-flow accounting with corrected empty bounds. The
public native Rust app is 45/55 geometry-exact, 43/55 pixel-exact, and 40/55
exact in both. The original reduced sweep is 60/65 exact in both, with all
PNG bytes unchanged from the prior column-flex candidate. All 144 atomic
guards retain exact geometry and pixels. The preceding 9,280 broader candidate
images and Chromium identities are unchanged. The combined workspace passes
8,491 tests (13 ignored), all 55 Linux application scenarios and the complete
109-export C/C++ consumer gate. Complete clean renderer matrices remain
required; row/reversed flex, following-block paint order and cloned decoration
remain measured native behavior to finish at that checkpoint.

The later [column paint and input correction](../renderer/native-column-paint-phases.md)
keeps a following ordinary block's background below overflowing atomic content
and orders input entries by the corresponding paint phase. The public Rust
application changes position and opacity through typed methods, checks the
element under the pointer, and observes one Rust click callback. Of 55 native
states, 35 match Chromium in geometry, pixels, and pointer targets together;
38 have exact pixels and 45 have matching pointer targets. The original
column-flex app gains five exact images, with every earlier geometry record
unchanged. All 144 atomic-child neighbors remain exact and unchanged.

The [candidate index](../v02/generated/native-column-paint-phases-v1.json)
records 8,491 workspace tests passing (13 ignored), all 55 Linux application
scenarios, the 109-export C/C++ consumer gate, read-only generators, and the
7/7 accountability audit. The new Rust and C overlap guards reject their
preceding implementations. The focused and primitive development matrices are
640/640 and 960/960 exact; the column/flex selection is 7,034/7,680 exact,
646 different, zero errors. All 9,280 broader images and Chromium oracle
identities remain unchanged, with no formerly exact regression. These
development-checkout matrices are diagnostic; clean complete verification is
still required. Row, grid, reversed flex, and following inline-block cases
remain explicitly owned and unfinished.
