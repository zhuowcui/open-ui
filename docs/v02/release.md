# Open UI v0.2 release qualification

The v0.2 source contract and artifact pipeline are checked in. This document
distinguishes verified repository evidence from release-lab work that cannot be
claimed by source code alone.

| Area | Current evidence | State |
|---|---|---|
| Historical Open UI archive | Archive and records are byte-pinned; optional [replay](../renderer/generated/frozen-replay-v1.json) found 5,549/5,731 unchanged, 182 changed | provenance pass; replay diagnostic |
| Chromium pixel target | Pinned Chromium 147 is the sole expected output for the declared renderer tests | see matrix below |
| Chromium oracle consistency | [Audit](../renderer/generated/chromium-font-oracle-audit-v1.json) found one older cached image that differs from six fresh captures under the same recorded identity; both variants are preserved | reconciliation open |
| Four-profile renderer matrix | [Complete clean umbrella census](../renderer/generated/native-nested-scroll-v9.json) at `d174ea0b`: 21,334/22,924 exact, 1,590 different, zero errors; actual exit 1. All Chromium inputs stay fixed; 882 residual original IDs need closure | fail |
| Focused and primitive raster | 640/640 focused exact; 960/960 primitive exact on clean float-API umbrella source `580539c2`; all 1,600 comparison invariants retained from `d174ea0b` | pass |
| Direct Ganesh raster | Clean Mesa llvmpipe [comparison](../renderer/generated/ganesh-raster-comparison-v1.json): 408/640 focused and 624/960 primitive exact; CPU remains the qualification backend | unpromoted |
| Expanded native final-state fixtures | [Complete clean umbrella run](../renderer/generated/native-nested-scroll-v9.json) at `d174ea0b` is 22,137/23,728 exact, 1,591 different, zero errors; actual exit 1. Original rows agree with the separate census; all 804 additions stay unchanged, with 200/201 exact at all four profiles. The other 35 AST-lowered cases remain [pending](../renderer/generated/pending-mutation-candidates-v7.json). Open UI runs no JavaScript | open |
| Accountability | 7/7 over 7,673 rows | pass |
| Rust workspace and docs | clean float-API checkpoint `580539c2`, Linux C feature enabled: 8,528 passed, zero failed, 13 ignored; source unchanged | pass |
| Rust 1.85 MSRV | [Own-source manual hardening](https://github.com/zhuowcui/open-ui/actions/runs/37131163576): locked headless and Linux checks passed at `d174ea0b` | pass |
| Rust/C application contract | 58 scenarios; 113 current exports with all preceding symbols and layouts preserved. Own clean `580539c2` build and eleven C/five C++ headless consumers pass; public Rust float-color consumers pass at five scales | scoped verification pass; remaining API review and lab qualification open |
| Native element interaction | Public Rust `Document`, `Element`, and `TextNode` APIs cover ID/class/native-kind lookup, class-token updates, retained detach/reattach, mutation, callbacks, activation, focus, scrolling, and controls; browser-style operations needed by applications must be exposed through native APIs | core implemented; remaining API coverage review open |
| C-owned X11/Wayland application loop | `oui_app_run` and `oui_app_request_exit` use Rust `App` and the same retained document; versioned platform callbacks and [clean native C/C++ window runs](native-c-lifecycle-evidence.md) cover X11 software/GL and Wayland software | implemented; release-lab qualification open |
| C platform accessibility | owned full-tree snapshots, node metadata/relations/focus, and changed/removed IDs export from the shared engine; automated AT-SPI operation in a C window remains unqualified | open |
| Generated sources | style, ABI, migration, closure generators are read-only clean | pass |
| No-work frame | zero layout, paint, and raster on unchanged snapshots | pass |
| Mutation ownership | 10,000-iteration soak, no owned-object leak | pass |
| Local performance smoke | 0.108 ms p95, 308 UI-thread animation fps, 1.389% RSS growth | non-qualifying pass |
| X11/Wayland software and Mesa GL | [Own-source manual hardening](https://github.com/zhuowcui/open-ui/actions/runs/37131163576) passed native C/C++ windows and Rust smoke paths at `d174ea0b`; physical release-lab tests remain open | provisional pass |
| Miri C handle ownership | [Own-source manual hardening](https://github.com/zhuowcui/open-ui/actions/runs/37131163576): opaque-handle ownership test passed under pinned Miri at `d174ea0b` | pass |
| ASan/LSan/fuzz | Font-cache fix applied at `a41fdeb9`; [new snapshot](../renderer/generated/native-font-cache-lifetime-v4.json) preserves all previous local checks and complete original/expanded pixel invariants. Own workspace: 8,526 passed, zero failed, 13 ignored; ten read-only checks pass. All seven hosted manual hardening jobs pass, including both sanitizers, Miri, C UBSan, MSRV, Linux windows and all five fuzz targets | pass for this checkpoint |
| Native C UBSan | [Own-source manual hardening](https://github.com/zhuowcui/open-ui/actions/runs/37131163576): ABI consumers passed at `d174ea0b` | pass |
| x86-64/AArch64 SDK, deb, rpm | deterministic source pipeline and tag matrix | pending tag build |
| Clean Ubuntu/Fedora install | release workflow consumer jobs | pending tag build |
| Physical GPU/context loss | release-lab profile | open |
| 100 compositor animations during 250 ms UI stall | retained animation layers required | open |
| AT-SPI inspect/operate | release-lab accessibility session | open |
| Two signed reproducible builds | tag workflow, keyless signatures/attestations | open |
| crates.io publication | credentials and final release approval | open |

The current umbrella applies the reviewed shared SVG and scrolling repairs and
native Rust/C/accessibility reveal operations. Its own clean workspace and
consumers pass. Its complete original and expanded pixel runs fail, at
21,334/22,924 and 22,137/23,728 exact, zero errors. All original rows agree;
all 804 additions stay unchanged, with 200/201 exact at every required profile.
Against SVG, nine comparisons become exact and none lose exactness; four
already failing comparisons worsen. All seven manual hardening jobs complete:
four pass and three fail. See the
[versioned implementation evidence](../renderer/generated/native-nested-scroll-v9.json).

The font-cache fix is applied in `a41fdeb9`; its native source, tools, headers
and examples match the completely measured private candidate. All pixel
comparison invariants stay unchanged, and its own workspace and ten read-only
checks pass. The earlier failed hardening run above remains historical.
The new [manual run](https://github.com/zhuowcui/open-ui/actions/runs/37149510887)
passes all seven jobs: both sanitizers, Miri, C UBSan, MSRV, Linux windows and
all five fuzz targets. The full renderer, needed native API, hardware and
publication gates remain open.

The [latest private scroll evidence](../renderer/generated/native-scroll-insets-v9.json)
records complete `dac78e25` runs at 21,340/22,924 original and 22,143/23,728
expanded exact, zero errors, with actual exits 1. Eight become exact and two lose
exactness; the screen correction remains unapplied pending repair of both
paint-owned regressions. Chromium inputs and all 804 addition results are fixed.

Separate clean `cf59ea29` repairs all ten nested rectangular-clip differences:
175/175 C and 510/510 Rust geometry/image states are exact, without losing prior
exactness. All 1,600 focused/primitive comparison invariants stay unchanged and
exact. Its workspace passes 8,532 tests and ten read-only checks; complete
original and expanded censuses now finish at 21,340/22,924 and 22,143/23,728
exact, zero errors, actual exits 1. Every comparison invariant remains unchanged
from `dac78e25`; both screen exact losses remain open.

Separate clean `6d6768a8` fixes native reveal container traversal and containing
blocks created or removed through public transforms. All 90 Rust and 60 C
geometry/image states match Chromium; the 510 existing Rust states retain all
pixels, geometry and callbacks. Its workspace passes 8,535 tests and ten
read-only checks. Its raster gates pass with all 1,600 comparison invariants
unchanged. The viewport cutoff at `2cc950e0` now passes all six fresh build
stages, 8,537 workspace tests, ten read-only checks and all 20 C/30 Rust new
states. It preserves all 660 previous states and recovers five C matches with
zero loss; both raster gates pass with all 1,600 comparison invariants unchanged.
An already-scrolled viewport
check remains 10/20 exact: fixed controls wrongly move with document scroll
in bounds and pixels. Its ten failures have Engine/paint ownership. The
combined `4dd50621` patch now passes six fresh build stages, 8,538 workspace
tests, ten read-only checks, all 935 native Rust/C geometry/image states and
both raster gates. All 885 previous states remain unchanged, and the C check
recovers all ten fixed-control failures without loss. Complete censuses finish
at 21,341/22,924 original and 22,144/23,728 expanded exact, zero errors, actual
exits 1. Six comparisons lose exactness against `cf59ea29` and five against the
applied umbrella; all Chromium inputs and 804 addition results stay unchanged.
Its included `c5769f2d` scroll-edge change
repairs the two old exact losses in a reduced 144-comparison selection, but
introduces three exact losses against `dac78e25` at 1.5 scale. That selection
finishes 85/144 exact, 59 different, zero errors, actual exit 1. It blocks
promotion and cannot establish a complete census result.
Patches remain unapplied and unqualified;
no new release states are admitted. Full renderer and native API gates remain
open.

The [next scroll evidence](../renderer/generated/native-scroll-insets-v11.json)
preserves the `b6a62a9b` primitive failure: 952/960 exact, eight gradient
regressions at 1.5 scale, actual exit 1. Its 935 existing and 70 new native
states and 640 focused comparisons are exact. The reduced 176-comparison run
retains one earlier multicolumn exact loss and cannot qualify a complete census.
Pinned Chromium inspection confirms a missing native trailing-margin extent
and a non-overflowing `auto` box whose clip stays in paint. A clean shared Rust
correction now passes seven fresh build stages, 8,539 workspace tests, all 935
earlier native states and both exact raster matrices, restoring the eight
gradient failures. The [latest native guards](../renderer/generated/native-scroll-insets-v12.json)
still fail 14 opaque white images and 20 of 45 collapsed-margin dimension/bounds
states, including ten newly wrong scroll-height fields. Both full image
censuses are running; the candidate remains unapplied and unqualified. No
reference bytes, tolerance or required gate changed.

The [private collapsed-margin trial](../renderer/generated/native-scroll-insets-v13.json)
at `af89e377` retains shared layout data and adds a consuming public Rust app.
Ten read-only checks pass; its clean build and native probes are queued after
the locked image sweeps. Its 765 fresh Chromium metric states are references,
not native API passes. No prior workspace, pixel or hardening pass is inherited.

The [v14 evidence](../renderer/generated/native-scroll-insets-v14.json)
completes `45ddeee3` at 21,350/22,924 original and 22,153/23,728 expanded
exact, zero errors, actual exits 1. One multicolumn exact loss against the
applied renderer remains. All Chromium inputs and 804 addition results stay
unchanged. Corrected margin source `2240ee9c` passes 8,540 workspace tests and
45/45 C states, but only 555/765 wider Rust states; the 210 vertical empty-block
failures stop its pixel guards. Both preceding compiler exits 101 are retained.
The reduced opaque-scroll review still fails 29/40 pixels under paint ownership.
A separate owned float-color C API preserves every existing export and struct
layout and adds one symbol; its own private runtime verification is recorded
below. The renderer patches remain unapplied and unqualified, with no new
release admission.

The [v15 evidence](../renderer/generated/native-scroll-insets-v15.json)
authenticates the private float-color API's completed six-stage build, 8,542
workspace tests and public Rust/C/C++ consumers. All original float-color
reference inputs and all 32 earlier native control images remain unchanged.
The eight native process failures are resolved; all 40 geometry states agree,
but only 11 images are exact. The 29 paint failures remain open.

A clean native Rust check proves that attached descendants retain the wrong
writing mode. Private `944068e1` repairs shared Engine inheritance across parent
and tree mutations. Its own eight-stage build, 8,542 workspace tests and ten
read-only checks pass; all 765 Rust / 45 C native states are exact, with 210
gains and no exact losses. All Chromium measurements remain unchanged. Its
pixel guards now preserve all 935 earlier native states and 1,600 raster
comparisons exactly, with 70/70 two-child states and 105/105 dimension queries
agreeing. Fourteen opaque white images still fail with paint ownership and
actual metrics-probe exit 1. Its full censuses were not started. Other
inherited-property behavior remains separate work. Documentation source
`e345cf69` passes all three hosted workflows,
with six successful and five skipped jobs. None of these private changes is
promoted or admitted as a release pass by that checkpoint.

The standalone float-color API and public consumers are now applied on the
umbrella branch. The [v16 evidence](../renderer/generated/native-scroll-insets-v16.json)
records clean `580539c2`: six build stages, 8,528 workspace tests, five Rust
runs, eleven C/five C++ consumers, and unchanged exact focused/primitive
matrices. Its first hosted format failure is retained. Whitespace-only
`aeed821c` passes C/C++ formatting, rebuilt consumers against the unchanged
Rust library, ten read-only checks, and all three hosted workflows: six jobs
succeed and five hardening jobs are skipped. No new Rust build, full census,
manual hardening or release qualification is claimed for that correction.

The [native inheritance and style trials](../renderer/generated/native-scroll-insets-v20.json)
remain separate from the accepted renderer. The earlier `f328ed62` complete
censuses fail at 21,312/22,924 original and 22,115/23,728 expanded exact,
losing 22 exact matches across twelve static-position tests. Its `3f1d296f`
follow-up passes 8,535 workspace tests, eight native C geometry states,
50 native Rust images and both complete 40-profile raster matrices; that
checkpoint has no own complete census.

The clean private `5cc75147` follow-up identifies and repairs the regression:
resolved computed fixture values had been sent through the normal inheritance
path, replacing descendant indentation zero with the parent's 20px. Two
independent Chromium queries verify the original reset behavior. The shared
Engine now preserves computed snapshots while native app declarations still
inherit. All 48 selected comparison invariants return to the applied baseline,
restoring all 22 lost exact matches. The other 26 selected failures remain.
Chromium images, source fixtures, fonts and resources stay unchanged.

Five new guards also cover snapshot replacement, inherited animations,
unchanged work and percentage line-height transports. The public typed
line-height setter already passed; the generic Renderer payload is repaired.
The fresh eight-stage build passes 8,540 workspace tests, zero failures and
13 ignored, eleven C/five C++ consumers, ten read-only checks and Rust
formatting. All 113 exports and ABI layouts are preserved. Its own native
relative guards retain all eight exact C geometry states and 50 exact Rust
images against repeated Chromium captures.

The new public Rust static-position example passes all 30 callback and owned
snapshot runs. Its 60 bounds and 60 pixel comparisons all fail the exact gate.
Absolute auto widths omit inherited indentation; subpixel text advances and
glyph painting also require review. Layout, text and paint ownership is
recorded, with minimized inputs, repeated Chromium captures, bounds, connected
regions and channel deltas. Failed capture probes remain separate evidence.
Its own raster matrices and original/expanded censuses are queued or running as recorded in the versioned snapshot. Incomplete runs establish no qualification pass.
This candidate is unapplied and unqualified; no new release state is admitted.
Open UI runs no JavaScript in any version. Needed interaction uses public
native Rust methods and Rust callbacks over the shared Engine.

The checked-in performance artifact is a WSL2 smoke result and explicitly has
`qualification: false`. It must not be relabeled as reference-machine evidence.

Chromium test files may contain scripts, but Open UI runs no JavaScript. Test
tooling may translate a script's deterministic visual result into native Rust
operations for pixel qualification. A test excluded from the pixel matrix for
behavioral or nonvisual reasons is not a waiver for application interaction:
needed element behavior must be available through the public native Rust API
over the same retained document and event path.
Each needed operation must be callable by the consuming native Rust app and
verified through that public API. An internal Engine operation or a test-only
fixture does not close a native application API gap.

The [native style checkpoint](generated/native-primitive-styles-v1.json) at
`9e0f0145` verifies 35 primitive longhands through public Rust, C, and C++
consumers at five scales. The same retained engine preserves optional colors,
`currentcolor`, alignment modifiers, and full unsigned counts through C
transport. Shared raw-frame readback now returns the documented premultiplied
RGBA order. All previous 110 exports and 30 layouts are unchanged. The full
workspace and both 40-profile pixel suites pass, with eight generators,
archive integrity, and accountability also passing. Six own-checkpoint hosted
jobs pass; five skipped hardening jobs remain open. Its own complete original
and expanded censuses now reproduce every accepted comparison invariant, with
21,308/22,924 and 22,111/23,728 exact and zero errors. These full pixel failures,
complete needed native API coverage, and release-lab gates remain open.

The [nested scrolling evidence](../renderer/generated/native-nested-scroll-v7.json)
records complete `6e255ca3` results: 21,321/22,924 original exact and
22,124/23,728 expanded exact, zero errors, observed exits 1. Eight comparisons
worsen, four losing exactness, because anonymous line fragments omit child
overflow and valid scrolling is clamped away. Chromium inputs are unchanged.
Later clean `02c0296e` repairs shared propagation, native sticky invalidation,
Rust/C construction and reversed scroll origins. It passes 8,516 Linux-enabled
workspace tests, ten read-only checks, nine C/four C++ consumer processes and
both complete 40-profile raster matrices. All 50 reduced public Rust app
states match Chromium, including manual-clock smooth scrolling from native
callbacks. All eight affected original comparisons are exact in the partial
sticky diagnostic; that diagnostic still has four prior paint failures.
Native metrics match in 2,320/2,560 states and pixels in 1,775/2,560, gaining
12 exact images with zero loss versus `6e255ca3`. All 850 root controls remain
exact and all 180 layer dimensions agree; layer pixels remain 95/180.
The 240 scrollbar-layout failures and 545 additional native paint failures
remain open. Its complete original and expanded runs finish at
21,332/22,924 and 22,135/23,728 exact, zero errors, with actual exits 1.
Chromium inputs and all addition rows remain unchanged. Nine original exact
matches are gained and two flex-overflow matches are lost at scale 1.25;
three fragmentation rows also worsen. Six paint investigations remain open,
so this source cannot be promoted. Earlier failed trials remain preserved and all
the failed source remains preserved. The later clean block scrollbar source
`b5a2044f` repairs all 240 geometry failures, matching Chromium in all 2,560
native states. Pixels remain 1,775/2,560 exact, with no exact image lost;
785 paint failures remain. Its Linux workspace passes 8,516 tests with zero
failed and 13 ignored. Ten read-only checks, all clean builds, nine C and four
C++ consumers pass. All 850 root controls, 50 reduced Rust sticky states and
both complete 40-profile raster matrices stay exact. The partial sticky gate
still fails at 124/128. Flex/grid/table/replaced scrollbar layout, native
scrollbar operation and the earlier six complete-census paint reviews remain
open. No complete original or expanded result is inferred for this source.
These checks add no release qualification.
Native interaction uses Rust APIs and callbacks; no JavaScript runtime or
glue is part of the work.

Clean `3f95e617` repairs shared scroll-transform ownership. Its partial
152-comparison gate restores both flex exact losses and three earlier
fragmentation images: 142 exact, ten different, zero errors, actual exit 1.
The Linux workspace, ten read-only checks, clean builds, C/C++ consumers and
both complete 40-profile raster matrices pass. Native geometry is
2,560/2,560 exact and pixels remain 1,775/2,560; 267 already-failing images
change, with no exact gain or loss: six improve, 42 worsen and 219 keep the
same mismatch count. Both complete censuses finish with actual exits 1:
21,319/22,924 original and 22,122/23,728 expanded exact, zero errors. Against
SVG, 15 exact matches are lost and 20 comparisons worsen; this source fails
qualification. Every Chromium input and all addition rows remain unchanged.

The later `83d45e0c` fixes shared capture scrollbar precedence and clip-margin
propagation, restoring all 15 exact losses in the affected selection:
157/172 exact, 15 different, zero errors, actual exit 1. Four previously failing
rows still worsen against SVG. Its layout tests pass; its stale generated
style inventory is regenerated when applying the umbrella code. No full
census is inferred for this private correction.

The reviewed source is now applied with public native instant/smooth reveal,
shared accessibility behavior and two append-only C operations. Private
`65147ab2` passes 35 Engine tests and ten C/four C++ consumers; all 30 reduced
geometry states agree with Chromium, while 16/20 endpoint images are exact.
Native scroll-margin/padding, full option coverage and four scale-1.25 pixel
failures remain open. Own clean umbrella verification and replacement of the
two live legacy contour calibration functions remain required. No release
qualification is inferred.

The [scroll coverage trial](../renderer/generated/native-viewport-full-v17.json)
preserves all four scrolling repairs, all 850 existing native controls and
both complete 40-profile raster gates. New consuming Rust apps have 95/180
exact pixels, with all bounds, offsets and client dimensions matching Chromium
but every scroll extent failing. Sixteen transparent-canvas erasures are
repaired. The historical SVG decoration alpha is still unmodeled by the
coverage proof; shared SVG layout/paint and native API work must remove it.
Those earlier trial patches remain preserved. The later reviewed source is
now applied, with its own combined-source census pending.
These new controls add no admitted release passes.

The [native PNG application](../renderer/native-png-sampling.md) verifies
resource registration, typed image styles, mutation through Rust callbacks,
owned bounds, and teardown in 540 runs on the current branch. After the shared
raster opacity and neutral gamma corrections, 183 of those images match
Chromium exactly.
The [completed opacity evidence](../renderer/generated/native-png-sampling-v3.json)
records unchanged full original and expanded matrices on clean source with
the same code and build inputs as umbrella checkpoint `42cce619`, plus fresh
clean umbrella consumer runs. The image-opacity and clipping consumers gain
109 and 27 exact results respectively. No exact result is lost; one existing
failed-image edge worsens and remains open. These cases add no admitted release
passes. The neutral gamma implementation is now committed at `63aeb672`;
its [completed standalone evidence](../renderer/generated/native-png-sampling-v4.json)
makes 35 additional native PNG images exact against the opacity checkpoint.
Fresh clean umbrella binaries reproduce all 880 native images from clean
`ebbe2b6f`, whose complete original and expanded matrices are 21,308/22,924
and 22,111/23,728 exact respectively, with observed exit 1. All Chromium
references remain unchanged.
Two already failing images each acquire nine differing green-channel cells
while total channel error decreases; those residuals remain failures. The
original pixel gate, normal published-consumer builds and residual ownership
remain open; these controls add no admitted release passes.

The [native viewport implementation](../renderer/generated/native-viewport-scroll-v4.json)
now exposes `Element::scroll_metrics`
through the public Rust API and uses shared viewport layout for paired gutters,
programmatic/wheel/smooth scroll limits, pending-layout queries, and clamping
after content shrink or viewport resize. A
[consuming native Rust application](../../bindings/rust/openui/examples/native_viewport_scroll.rs)
verifies all 850 offsets, owned bounds, client/content dimensions, callback
invocations and teardowns against Chromium. Clean qualification source
`8f45444e` is 196/250 original and 488/600 direction-guard images exact.
All 510 integer-scale comparisons remain exact; 166 fractional-scale image
failures remain unqualified. Physical scroll snapping, thumb enclosure without
interior-edge antialiasing, and whole-track recordings for non-scrollable
viewports remove 32,877 differing pixels and make three more images exact
against the preceding recording checkpoint. No exact image is lost or
mismatch count increased. Four traced Chromium captures reproduce immutable
oracle bytes and confirm the distinct shared drawing paths.

The full locked workspace passes 8,498 tests with 13 ignored at clean
`8f45444e`. Recorded controls survive live style changes and document drop.
Fresh complete raster gates pass 640/640 focused and 960/960 primitive exact,
with all 1,600 native/Chromium images and oracle identities unchanged. Fresh
clean umbrella binaries reproduce all 850 native controls. The pre-repair full
census reruns at `8f45444e` have finished with observed exit 1 and zero errors:
21,292/22,924 original and 22,095/23,728 expanded exact. All original rows agree
between those complete runs and with the earlier `574864d0` run in every native
image, Chromium image and difference signature. Hash/count validation passes;
these failed results do not qualify the clipping repair's unfinished runs.
The earlier complete viewport runs at `574864d0` have finished with observed
exit 1: 21,292/22,924 original and 22,095/23,728 expanded exact, zero errors.
All original rows agree between those runs, and 200/201 additions still pass
all four profiles. Their 16 formerly exact regressions are reproduced by the
preceding runtime and owned by `openui-paint`; Chromium inputs are unchanged.
The [completed report index](../renderer/generated/native-viewport-full-v1.json)
preserves this failure. The [clean shared clipping repair](../renderer/generated/native-viewport-full-v2.json)
at `1366b72f` restores
all 16 in the 96-comparison affected selection, while retaining all 850 native
control images, geometry and difference signatures. It is now implemented
on the umbrella, whose renderer/build/harness/resource inputs match the clean
qualification source; documentation differs. The repair passes 8,498 locked
workspace tests, 13 ignored, and complete 640/640 focused and 960/960 primitive
exact matrices, with observed exits 0 and all 1,600 native/Chromium images and
oracle identities unchanged. Its
[complete original and expanded runs](../renderer/generated/native-viewport-full-v7.json)
now have observed exits 1: 21,308/22,924 and 22,111/23,728 exact, zero errors.
All 16 earlier exact results are restored, with no new exact loss or worsened
pixel count. All Chromium images and oracle identities remain unchanged,
original rows agree across both runs, and all 804 addition comparisons remain
unchanged with 200/201 additions exact at every profile. Clean source and
binary identities stay fixed. The full pixel gate still fails. Outer viewport
clipping moves to final surface/tile assembly only when no scrollbar gutter is
reserved. Generated
contracts, archive integrity, formatting, release-source verification and
7/7 repository metadata accountability pass. The index preserves the first
audit failure on absent ignored historical PNGs and the successful existing
clean-checkout audit mode; the strict pixel gate remains unchanged.

Fresh clean umbrella binaries at implementation checkpoint `28cde831`
[reproduce all 850 native images](../renderer/generated/native-viewport-full-v3.json),
geometry, callbacks, teardowns and difference signatures from the clean repair
source. The six renderer crates are cleaned before rebuilding. Source remains
clean and unchanged through build and both native runs, with observed exit 0;
fresh umbrella release-source verification passes too. The complete repair
censuses above remain failures, and the 166 native fractional pixel failures
still fail.

The [per-axis scrollbar candidate](../renderer/generated/native-viewport-full-v4.json)
at clean `cc056cee` is included in the combined scrollbar and client clip
correction now implemented in the umbrella. Eight fresh Chromium traces reproduce
immutable oracle PNG bytes and show a forced bar with no range as an ordinary
picture while the opposite axis uses a composited scrollbar. The candidate
improves eight already failing images by 1,898 differing pixels, with no exact
loss, no new exact image and all 850 geometry/callback/teardown records
unchanged. The 166 fractional failures remain failures. Its
[completed focused and primitive checks](../renderer/generated/native-viewport-full-v5.json)
pass 640/640 and 960/960 exact, with all 1,600 native/Chromium images and
difference signatures unchanged. The combined workspace and complete census
results are recorded below.

A subsequent clean client-scissor candidate at `079208f8` follows Chromium's
physical enclosing clip and improves four already failing images, removing 640
differing pixels without a worsened color-channel cell. No exact image is lost
or gained. All 850 geometry, oracle, callback and teardown checks remain
unchanged. These corrections are now implemented in the umbrella. Its
[clean workspace verification](../renderer/generated/native-viewport-full-v6.json)
passes 8,498 tests with zero failures and 13 ignored, with tracked test output
restored and source unchanged. Its
[complete focused and primitive checks](../renderer/generated/native-viewport-full-v7.json)
pass 640/640 and 960/960 exact with observed exits 0; all 1,600 native/Chromium
images, oracle identities and differences remain unchanged. The
[complete original and expanded censuses](../renderer/generated/native-viewport-full-v9.json)
on clean `079208f8` have finished with observed exits 1: 21,308/22,924 and
22,111/23,728 exact, zero errors. Every original and addition image, oracle
identity and difference is unchanged from the accepted clipping repair.
All 8,282 tracked code, build, test, resource and workflow files match the
umbrella implementation. The [fresh clean umbrella build](../renderer/generated/native-viewport-full-v10.json)
at `a5547e7e` reproduces all 850 native images, geometry, oracle records,
callbacks, teardowns and differences, with observed exits 0 and clean source
unchanged through build and execution.
The complete pre-repair
expanded run also retains all 804 addition images unchanged from `574864d0`
and the same 200/201 additions exact at every required profile.

The evidence also preserves the rejected unsnapped-scroll hypothesis and
pinned primary sources for opaque-layer boundary clearing and background
selection. A private immutable content-layer prototype at clean `cae25afc`
reaches 706/850 native images exact, with 22 newly exact, zero exact losses and
all geometry/oracle/callback/teardown records unchanged. All 510 integer-scale
images remain exact. Six already failing direction cases worsen and 144
fractional failures remain, so the prototype is unapplied and unqualified.
`openui-paint` owns the unresolved layer raster/composition differences. Its
8,498 locked workspace tests pass, zero failures and 13 ignored, with source
clean and unchanged. Its
[complete focused and primitive matrices](../renderer/generated/native-viewport-full-v8.json)
pass 640/640 and 960/960 exact with observed exits 0 and all 1,600 native and
Chromium images, oracle identities and differences unchanged. Complete
censuses and general layer clip/effect/transform metadata remain required;
the six composition regressions prevented acceptance of that checkpoint.
The earlier `80dd2353` attempt lost three exact images and remains rejected
evidence. The [solid-tile investigation](../renderer/generated/native-viewport-full-v9.json)
confirms the white boundary tiles through pinned Chromium's grey tile debug
borders. Disabling debugging restores all 12 normal captures byte-for-byte,
including six existing oracles. The private implementation retains owned
paint operations and derives tile colors before raster. An intermediate
recording-time clip query lost improvements in 33 already failing comparisons;
clean `d64d3f7f` selects visible tiles before recording the fractional band.
It retains 706/850 exact, with 22 newly exact and no exact loss, worse mismatch
count or worse color-channel cell against `079208f8`. All geometry, callbacks,
teardowns and 510 integer-scale images remain exact. Its 8,498 workspace tests
pass, zero failures and 13 ignored. Twelve viewport guards pass geometry and
callbacks; four images are exact and eight still fail. The prototype remains
unapplied, with 144 fractional failures. Its
[complete focused and primitive matrices](../renderer/generated/native-viewport-full-v10.json)
pass 640/640 and 960/960 exact with observed exits 0, clean source unchanged,
and all 1,600 native/Chromium images, oracle identities and differences
unchanged. Its later completed original and expanded censuses retain
21,308/22,924 and 22,111/23,728 exact, zero errors, with all images and
differences unchanged. Those complete failures do not qualify a release.

The [private sampling and corner investigation](../renderer/generated/native-viewport-full-v11.json)
finds that pinned Chromium enables `SK_ENABLE_LEGACY_SHADERCONTEXT`, which the
current comparison configuration omits. Clean `8d103fb1` adds only that setting,
keeps the existing Skia pin, and reaches 823/850 exact native viewport images
without an exact loss or worse color-channel cell. Its complete focused and
primitive matrices pass 640/640 and 960/960 exact with unchanged pixels and oracle
identities. Clean `64fa1a7c` adds shared exterior corner coverage and reaches
850/850 exact native images plus 12/12 exact wider-viewport guards. Geometry,
Rust callbacks, teardown and all reference bytes remain unchanged. Both
prototypes remain unapplied. The corner candidate's
[completed workspace](../renderer/generated/native-viewport-full-v12.json)
passes 8,498 tests, zero failures and 13 ignored, with clean source unchanged
after tracked output restoration. Full original and expanded qualification
remain required. Its [completed replacement raster matrices](../renderer/generated/native-viewport-full-v13.json)
pass 640/640 focused and 960/960 primitive exact with unchanged pixels and oracle
identities. Its [completed original and expanded censuses](../renderer/generated/native-viewport-full-v14.json)
have observed exits 1: 21,299/22,924 and 22,102/23,728 exact, zero errors.
The 144 changed comparisons include 23 exact losses, 14 exact gains and
46 worsened pixel counts. All Chromium images and oracle identities remain
fixed. A clean configuration-only sweep reproduces every changed comparison
on 304 selected rows; its partial scope is diagnostic, not qualification.
The sampler/build-configuration correction remains unresolved and the
candidate is rejected for promotion. All 804 additions are unchanged;
200/201 remain exact at all four profiles. The older solid-tile candidate's complete censuses have observed
exits 1 and retain 21,308/22,924 original and 22,111/23,728 expanded exact, zero
errors, with every image and difference signature unchanged against the
preceding accepted renderer. Those complete failures do not qualify a release.
The first corner raster attempts were stopped with observed exits 143 after an
overlapping workspace test was discovered; their partial outputs do not qualify.

The [generated-image sampling investigation](../renderer/generated-image-sampling.md)
identifies and repairs 18 generated-background exact regressions on clean
private `16fc959d`. Its affected selection is 168/304 exact, zero errors;
all 850 exact native controls remain unchanged. Five exact regressions still
prevent promotion. Its focused and primitive matrices are 640/640 and
960/960 exact, with all 1,600 comparison invariants unchanged. A strict
image-rectangle scroll replay changes none of
those results. The trials remain unapplied; the selected scope cannot prove
full renderer qualification or close a release gate.

A clean opaque-layer composition trial at `b3c54ea8` restores all four
scrolling losses in the same selection, reaching 172/304 exact with zero
errors. All other 300 comparisons and all 850 exact native controls remain
unchanged. One SVG pixel regression remains. Conservative opacity metadata,
transparent/clipped/effected neighbors and full
original/expanded qualification remain required; the trial stays unapplied.
Its [complete raster checks](../renderer/generated/native-viewport-full-v16.json)
finish with observed exits 0: 640/640 focused and 960/960 primitive exact,
with all 1,600 comparison invariants unchanged.
The [follow-up review](../renderer/generated-image-sampling.md#scroll-layer-composition-follow-up)
rejects direct CPU scroll-content replay, which worsens all four failures.
Hosted workflows at `fdac09eb` have finished with six successful jobs and
five skipped hardening jobs. Skips do not qualify those hardening gates.

All six ordinary hosted checks at `427f7df4` pass; five skipped hardening jobs
are unverified. No final-release or later hosted result is claimed.
The generated consumer inventory is corrected to exactly 203 public fields,
including the existing typed thumb-color property; no unclassified field is
allowed. All eight generator checks, archive integrity and accountability pass. Runs at
`574864d0` exclude the later recording and transform changes. The
[older recording index](../renderer/generated/native-viewport-scroll-v3.json)
and rejected proposals remain immutable evidence. Native scrollbar operation
and accessibility, nested scrolling, propagation, remaining C property
conversion and pixel/public API gaps still require implementation and verification. No
JavaScript runtime or glue is part of that work.

The [renderer source check](../renderer/renderer-build-identity.md) verifies
the source recorded inside the comparison executable before accepting a run,
then checks the source and binary again after it. Its
[verification](../renderer/generated/renderer-build-identity-v1.json) preserves
the rejected attribution of older executable results to newer opacity source.
Replacement development raster suites are exact. The
[complete original replacement census](../renderer/generated/native-opacity-full-regression-v1.json)
is 21,297/22,924 exact, with 1,627 differences, zero errors and 23 formerly
exact regressions. That prototype remains unapplied; its Chromium references
are unchanged. These measurements do not replace the clean release matrix
above.

The [source-less image prototype](../renderer/native-source-less-images.md)
is 1,673/1,800 exact across 360 script-free native states and five scales,
with all owned bounds exact. It remains unapplied and does not qualify the
complete renderer or admit new release cases.

The [current clean SVG evidence](../renderer/generated/native-svg-viewport-v11.json)
records two complete original censuses at 21,325/22,924 exact, zero errors,
terminal exits 1. They gain 17 exact comparisons and lose none; 13 already
failing comparisons worsen. The solid-border source passes both full
40-profile raster suites. Clean `8d5a58a1` rebases those renderer changes over
the current native API/ABI and adds C viewport creation through the shared
Rust constructor. Nine headless C and four C++ consumers pass, including
native sizing callbacks and teardown; existing exports and layouts remain
unchanged. Its Linux-enabled workspace passes 8,516 tests, zero failures and
13 ignored, on unchanged clean source. Its own focused and primitive matrices
finish at 640/640 and 960/960 exact, with all comparison invariants unchanged
and observed exits 0. Its own complete original census finishes at
21,325/22,924 exact, 1,599 different and zero errors, with observed exit 1.
All 22,924 comparison invariants agree with the double-border source. Its
complete expanded run also finishes with observed exit 1: 22,128/23,728
exact, 1,600 different and zero errors. Every original row agrees with its
separate census, and all 804 addition invariants remain unchanged from the
accepted renderer, with 200/201 cases exact at all four profiles. Both
complete pixel gates fail. Its Rust controls preserve all
1,920 SVG images and 850 existing
scrolling images, while 744 SVG comparisons and all 180 scroll-layer extents
still fail. Complete rebased pixel qualification, SVG transforms and nested
scrolling remain open. The source patch stays unapplied and the accepted
renderer totals in the release table are unchanged. Earlier disk failures
and partial outputs remain preserved.

The earlier [native SVG viewport investigation](../renderer/native-svg-viewport.md)
uses a proposed public Rust element constructor, with 134/480 exact pixel
comparisons and 480/480 exact owned bounds across 96 script-free controls.
Its workspace passes 8,492 tests, but the patch remains unapplied: 346 native
comparisons still differ and other required renderer qualification remains
open. The former `49/50` SVG opacity adjustment is replaced in that prototype
by viewport sizing and shared border drawing; no reference pixels change.
Its complete original diagnostic now reaches 21,328/22,924 exact, with zero
errors and no formerly exact regression against C9. Thirteen already failing
image comparisons worsen and source is a development checkout. The complete
expanded diagnostic is 22,131/23,728 exact, with zero errors and unchanged
original results and addition images; 200/201 additions remain exact. Both
runs have observed terminal exit 1. This does not replace the clean renderer
gate in the table above or establish clean-source release qualification.

The [native column paint and input checkpoint](../renderer/native-column-paint-phases.md)
verifies public Rust position/opacity mutations, pointer queries and a Rust
click callback, with C consumers over the same engine. Its 55 native states
include 35 exact in geometry, pixels and pointer targets together. The
[candidate index](generated/native-column-paint-phases-v1.json) preserves
the completed 9,280-comparison development matrices and negative consumer
guards. These dirty-source diagnostic results do not qualify the release;
clean complete renderer matrices and the remaining native behavior are required.

The [native image fallback correction](../renderer/native-image-fallback.md)
passes all 120 isolated image/bounds comparisons through public Rust APIs.
Its complete development column/flex selection is 7,039/7,680 exact, with
zero errors, five newly exact comparisons and no formerly exact regression.
Focused and primitive matrices retain 640/640 and 960/960 exact results.
Native column geometry, source-less frames and opacity remain open; clean
complete original and expanded matrices are required at the new checkpoint.

The [native opacity investigation](../renderer/native-image-opacity.md)
verifies public Rust setters, owned image bounds and a Rust click callback.
Its shared renderer prototype matches 320 reduced opacity comparisons, but
the additional paint cases remain 90/140 exact and include one worsened
clipped-image comparison. The prototype remains unapplied and unqualified;
the clean renderer totals above remain authoritative.

The [scaled LCD font investigation](../renderer/scaled-lcd-hinting-oracle-investigation.md)
rejected a renderer change that regressed 43 formerly exact comparisons. It
also found one older cached Chromium image that differs from six agreeing
fresh captures. The capture identity and fixture bytes are the same; the
reviewed cause remains unknown. The clean census below remains evidence
against those preserved cached captures, and final qualification requires
oracle reconciliation. No cached reference was replaced or result promoted.

The earlier complete clean `9b158cda` [v49 census](../renderer/generated/four-profile-census-v49.json)
is 21,308/22,924 exact, 1,616 different and zero errors. Its
[full delta](../renderer/generated/native-image-full-delta-v1.json)
verifies 30 improved comparisons and 17 newly exact, with no worsened comparison
or exact regression from `056421db`. Every Chromium image and oracle identity
remains unchanged. All original rows agree with the complete
[v32 expanded run](../renderer/generated/expanded-requalification-v32.json),
which is 22,111/23,728 exact and retains 200 of 201 exact additions. The first
execution was interrupted by an environment reset; its incomplete evidence
remains preserved. Separate replacement runs have complete terminal reports
that pass source, count and hash validation. Process handles expired before
exit codes were observed, so no exit code is inferred. The source remained
clean and unchanged from build through final audit. The 892 residual IDs lack
reviewed ownership and the required pixel gate remains failing. The clean
[v51 raster matrices](../renderer/generated/focused-primitive-raster-v51.json)
pass 640/640 focused and 960/960 primitive comparisons, with every image and
oracle identity unchanged. The preceding `497e322d`
[delta index](../renderer/generated/native-column-flex-full-delta-v1.json)
preserves its original command's exit-143 observation and complete report.
The
[hosted hardening](https://github.com/zhuowcui/open-ui/actions/runs/36840619251)
has finished with four passing and three failing jobs. Both native sanitizers
pass 25 FFI tests before reporting 10,476 Fontconfig bytes in 236 allocations
at exit; fuzzing stops at `tree_mutations` with 2,606 bytes in 59 allocations.
The three later fuzz targets remain unverified. The
[seven-job index](generated/hosted-hardening-9b158cda-v1.json) preserves the
complete failed-job logs and each outcome. Font-manager lifetime/root cause
review remains open. Incomplete renderer results do not qualify that source.

The preceding clean `e9211183` [v46 census](../renderer/generated/four-profile-census-v46.json)
has the same exact/different totals. Its
[full delta](../renderer/generated/native-vertical-max-block-full-delta-v1.json)
records four changed original images: `block-max-height-004` and its reference
worsen by 72 pixels each at 1.25 scale and 75 each at 1.5 scale. Every Chromium
image and oracle identity stays fixed, and no formerly exact comparison
regresses. The [v29 expanded run](../renderer/generated/expanded-requalification-v29.json)
retains the same 200 of 201 exact additions. These failures remain open.

The preceding complete clean census at `547c7081` is
[21,291/22,924 exact](../renderer/generated/four-profile-census-v45.json),
with 1,633 differences, zero errors and 900 unowned residual IDs. The
[complete delta](../renderer/generated/native-constrained-box-full-delta-v1.json)
records all 22,924 Open UI RGBA images and Chromium oracle identities
unchanged from `822e0462`, with no exact regressions. The complete expanded
run is [22,094/23,728 exact](../renderer/generated/expanded-requalification-v28.json),
with 1,634 differences and zero errors; the same 200 of 201 additions are
exact at every required profile. Every original row agrees with the separate
census, and all expanded images and oracle identities stay unchanged.
The later [atomic-child deferral](../renderer/native-atomic-column-deferral.md)
and vertical maximum-size corrections retain their separate development
evidence and still need complete clean matrices.

The preceding complete clean census at `822e0462` is
[21,291/22,924 exact](../renderer/generated/four-profile-census-v44.json), with
1,633 differences and zero errors. Its
[complete delta](../renderer/generated/native-spanner-boundary-full-delta-v1.json)
records 13 new exact comparisons, 26 changed Open UI images, no exact
regressions, and fixed Chromium images and identities. The 900 residual test
IDs remain unowned. Two existing fieldset differences increase by 87 pixels
each. The complete expanded run is
[22,094/23,728 exact](../renderer/generated/expanded-requalification-v27.json),
with the same 200 of 201 additions exact at every required profile.
The subsequent [native constrained-box geometry work](native-element-geometry.md#constrained-boxes-and-visible-child-overflow)
has the complete clean renderer runs indexed above; the original remaining
pixel failures and wider native API qualification still need closure.

The preceding clean census at `0ad4b12f` is
[21,278/22,924 exact](../renderer/generated/four-profile-census-v43.json),
with 1,646 differences, zero errors, and 909 unowned residual test IDs. The
[complete delta](../renderer/generated/native-geometry-full-delta-v1.json)
recovers the preceding checkpoint's three nested-column failures but introduces
three ordinary-spanner failures at 1.25×. No Chromium image or oracle identity
changes. This checkpoint is not accepted as renderer qualification. The full
expanded run is [22,081/23,728 exact](../renderer/generated/expanded-requalification-v26.json),
with 1,647 differences and zero errors. All original rows agree with the
separate census, and 200/201 additions remain exact at all four profiles.
The release manifest retains all 201. The focused and primitive matrices are
[640/640 and 960/960 exact](../renderer/generated/focused-primitive-raster-v45.json),
with all 1,600 images unchanged from the preceding checkpoint.

The preceding clean census at `15f9f12d` is
[21,266/22,924 exact](../renderer/generated/four-profile-census-v41.json),
with 1,658 differences, zero errors, and 914 unowned residual test IDs. The
[circular background one-axis clip repair](../renderer/circular-background-one-axis-clip.md)
makes the F16 circular fill use the same horizontal overflow scissor as the
direct fill. One original comparison became exact; only that Open UI image
changed, and no exact comparison regressed. All 22,924 Chromium oracle
identities and decoded images stayed fixed. The clean
[v42 focused/primitive index](../renderer/generated/focused-primitive-raster-v42.json)
is 640/640 and 960/960 exact, with all 1,600 images unchanged from v41.
The complete [v24 expanded requalification](../renderer/generated/expanded-requalification-v24.json)
is 22,069/23,728 exact, with 1,659 differences and zero errors. Every original
result matches the separate clean census, and all 804 addition images stayed
fixed. All 23,728 Chromium oracle identities and decoded images match v23.
The same 200 of 201 additions are exact at all four profiles. The
[v26 diagnostic selection](../../tools/qualification/manifests/expanded-v26.json)
lists those 200; the release contract retains all 201 and the fieldset legend
failure.

At the prior clean checkpoint `e14e3e64`, the census was
[21,264/22,924 exact](../renderer/generated/four-profile-census-v39.json),
with 1,660 differences, zero errors, and 916 unowned residual test IDs. The
[vertical-lr Ahem rotation-anchor repair](../renderer/vertical-lr-ahem-rotation-anchor.md)
made nine original comparisons exact, changed only those nine Open UI images,
and reduced wrong pixels by 30,184. No exact comparison regressed; all 22,924
Chromium oracle identities and decoded images stayed fixed. The clean
[v40 focused/primitive index](../renderer/generated/focused-primitive-raster-v40.json)
was 640/640 and 960/960 exact. The complete clean
[v22 expanded requalification](../renderer/generated/expanded-requalification-v22.json)
was 22,066/23,728 exact, with 1,662 differences and zero errors. One previously
failing native final-state comparison became exact, so 199 of 201 additions
were exact at all four profiles. The
[v24 diagnostic selection](../../tools/qualification/manifests/expanded-v24.json)
listed 199 exact additions; the release contract included all 201 and their
two failures at that checkpoint.

At the prior clean checkpoint `d39282e4`, the census was
[21,255/22,924 exact](../renderer/generated/four-profile-census-v38.json),
with 1,669 differences, zero errors, and 923 unowned residual test IDs. The
[analytic gradient-edge repair](../renderer/fractional-multicol-seam-investigation.md)
removed a redundant hard clip around a square gradient fill. Compared with
v37, seven comparisons became exact, no exact comparison regressed, and the
wrong-pixel count fell by 5,957. Exactly 29 Open UI images changed; all
22,924 Chromium oracle identities and decoded images stayed fixed. The clean
[v39 focused/primitive index](../renderer/generated/focused-primitive-raster-v39.json)
is 640/640 and 960/960 exact, with no decoded image changes from v38. The
clean [201-addition guard](../renderer/generated/expanded-additions-gradient-guard-v1.json)
is 801/804 exact with the same three failures and no Open UI or Chromium
image changes. That selected guard is not a complete expanded-manifest run;
the complete clean [v21 expanded requalification](../renderer/generated/expanded-requalification-v21.json)
at `e59ec07e` is 22,056/23,728 exact, 1,672 different, and zero errors.
Every original and added decoded image, status, and Chromium oracle identity
matches the separate clean census and addition guard. The
[v23 diagnostic selection](../../tools/qualification/manifests/expanded-v23.json)
lists 198 exact additions; all 201 remain in the release contract.

At the prior clean checkpoint `dc451061`, the census was
[21,248/22,924 exact](../renderer/generated/four-profile-census-v37.json),
with 1,676 differences, zero errors, and 924 unowned residual test IDs. The
[adjacent row-flex repair](../renderer/adjacent-row-flex-fragmentation.md)
made three comparisons exact and improved one other failing comparison by 118
pixels. No formerly exact image regressed, and all Chromium oracle identities
and decoded images stayed fixed. The clean
[v38 focused/primitive index](../renderer/generated/focused-primitive-raster-v38.json)
is 640/640 and 960/960 exact. The complete clean
[v20 expanded requalification](../renderer/generated/expanded-requalification-v20.json)
is 22,049/23,728 exact, 1,679 different, and zero errors; the same 198 of
201 additions are exact at all four profiles.

The [fractional multicolumn seam investigation](../renderer/fractional-multicol-seam-investigation.md)
records two remaining one-column pixel signatures against Chromium. Its
coverage explanation is still a hypothesis; those cases remain failing and
unowned for release qualification.

At the prior clean checkpoint `128edc38`, the v36 census was
[21,245/22,924 exact](../renderer/generated/four-profile-census-v36.json),
with 1,679 differences, zero errors, and 926 unowned residual test IDs. The
[fractional Ahem strike repair](../renderer/fractional-ahem-stripe-edges.md)
changed 21 already failing images and reduced wrong pixels by 281 without an
exact regression. All 22,924 Chromium oracle identities and decoded hashes
remained fixed. At the prior clean checkpoint `6676cf60`, the
[v35 delta audit](../renderer/generated/radial-ua-full-delta-v1.json) recorded
nine changed Open UI images, one new exact comparison, no exact regression,
and unchanged Chromium oracle identities and decoded images. The
[radial repair](../renderer/radial-tile-edge-coverage.md) left one wrong pixel
in each of two formerly 626-pixel differences. The
[object fallback repair](../renderer/object-fallback-host-clip.md) restored an
intermediate exact-case regression. The clean
[201-addition guard](../renderer/generated/expanded-additions-object-deferred-v1.json)
retained 801 exact and three different comparisons with no changed Open UI or
Chromium image; this selected guard is not a full expanded-manifest run. The
[new native final-state admission](../renderer/adjoining-floats-native-admission.md)
adds one four-profile-exact case while retaining all 200 earlier additions.
The complete clean expanded run at `6b53a991` is
[22,046/23,728 exact](../renderer/generated/expanded-requalification-v19.json),
with 1,682 differences and zero errors. Exactly 198 of 201 additions are
exact at all four profiles; the same three earlier additions remain failures.
All 22,924 original profile results match the v36 census, and all 804 added
profile results match the clean selected-additions guard, including their
Chromium oracle identities and decoded image hashes. The new full result has
one more exact original comparison than the older v18 expanded report.
The [v21 diagnostic selection](../../tools/qualification/manifests/expanded-v21.json)
lists 198 exact additions while the release contract retains all 201.
The [fixed-point broken-image repair](../renderer/broken-image-fractional-sampling.md)
made five original comparisons exact with no exact regression and no Chromium
oracle change. The clean
[v37 focused/primitive index](../renderer/generated/focused-primitive-raster-v37.json)
is 640/640 and 960/960 exact. The other 35
[AST-lowered candidates](../renderer/generated/pending-mutation-candidates-v7.json)
produced 18/140 exact profile comparisons, 122 differences, and zero errors at
clean checkpoint `8324c6b0`. None is exact at all four profiles. All 140
statuses, mismatch counts, and diff signatures match the prior report. The
release renderer gate remains open.

The prior clean census at `814a2005` is
[21,239/22,924 exact](../renderer/generated/four-profile-census-v30.json),
with 1,685 differences, zero errors, and 931 unowned residual test IDs. The
[table row-group clip investigation](../renderer/truncated-table-row-group-clip.md)
records the shared paint repair: the remaining repeated-section comparison
became exact at 1.25×, and the same case is exact in its
[40-profile sweep](../renderer/generated/table-row-group-cross-v1.json).
Exactly one Open UI image changed across the full census; no exact comparison
regressed, and all Chromium oracle identities and decoded hashes stayed
fixed. The clean
[v31 raster index](../renderer/generated/focused-primitive-raster-v31.json)
is 640/640 focused and 960/960 primitive exact, with all 1,600 decoded images
unchanged. The complete clean
[v16 expanded requalification](../renderer/generated/expanded-requalification-v16.json)
is 22,036/23,724 exact. All 800 addition images are unchanged, with 197 of
200 additions still exact and three demoted. The
[v17 diagnostic selection](../../tools/qualification/manifests/expanded-v17.json)
retains those 197. The clean
[v5 pending-candidate index](../renderer/generated/pending-mutation-candidates-v5.json)
again finds `adjoining-floats-dynamic` exact at all four profiles; it remains
outside admitted release coverage pending a new contract manifest.

The prior clean census at `7cd8e574` is
[21,238/22,924 exact](../renderer/generated/four-profile-census-v29.json),
with 1,686 differences, zero errors, and 932 unowned residual test IDs. The
[repeated table body-slice investigation](../renderer/repeated-table-body-slice.md)
records the shared layout-to-paint repair: 11 comparisons became exact and
one other difference shrank. Exactly 12 Open UI images changed, no exact
comparison regressed, and all Chromium oracle identities and decoded hashes
stayed fixed. The clean
[v30 raster index](../renderer/generated/focused-primitive-raster-v30.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v15 expanded requalification](../renderer/generated/expanded-requalification-v15.json)
is 22,035/23,724 exact, with 197 of 200 additions still exact and three
demoted. The [v16 diagnostic selection](../../tools/qualification/manifests/expanded-v16.json)
retains those 197. The clean
[v4 pending-candidate index](../renderer/generated/pending-mutation-candidates-v4.json)
again finds `adjoining-floats-dynamic` exact at all four profiles; it remains
outside admitted release coverage pending a new contract manifest.

The prior clean census at `a0e3f4cd` is
[21,227/22,924 exact](../renderer/generated/four-profile-census-v28.json),
with 1,697 differences, zero errors, and 937 unowned residual test IDs. The
[clipped replaced-background investigation](../renderer/clipped-replaced-background-coverage.md)
records the shared coverage rule: two margin-trim comparisons became exact,
and a third iframe comparison fell from 342 wrong pixels to six. Exactly three
Open UI images changed; every Chromium oracle identity and decoded hash stayed
fixed, and no exact comparison regressed. The clean
[v29 raster index](../renderer/generated/focused-primitive-raster-v29.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v14 expanded requalification](../renderer/generated/expanded-requalification-v14.json)
is 22,024/23,724 exact, with 197 of 200 additions still exact and three
demoted. The [v15 diagnostic selection](../../tools/qualification/manifests/expanded-v15.json)
retains those 197. The clean
[v3 pending-candidate index](../renderer/generated/pending-mutation-candidates-v3.json)
again finds `adjoining-floats-dynamic` exact at all four profiles; it remains
outside admitted release coverage pending a new contract manifest.

The prior clean census at `8950b426` is
[21,225/22,924 exact](../renderer/generated/four-profile-census-v27.json),
with 1,699 differences, zero errors, and 939 unowned residual test IDs.
Every residual diff signature matches the prior clean census, so the nested
float fix did not change the original comparison inventory. The
[v28 raster index](../renderer/generated/focused-primitive-raster-v28.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v13 expanded requalification](../renderer/generated/expanded-requalification-v13.json)
is 22,022/23,724 exact, with the same three failing additions. The
[v14 diagnostic selection](../../tools/qualification/manifests/expanded-v14.json)
retains only the 197 exact additions. The clean
[v2 pending-candidate index](../renderer/generated/pending-mutation-candidates-v2.json)
records one newly eligible case, `adjoining-floats-dynamic`, at four of four
profiles. The other 35 pending cases remain open; the newly eligible case is
not yet counted as admitted release coverage.

The prior clean census at `954648fb` changed eight original Open UI images
in the resized one-axis bitmap family at fractional scales. The PNG background
now uses one shared image shader across its repeated destination, matching
Chromium's pattern path. Two 1280×720@1.25 comparisons became exact, six
others moved closer, and no comparison regressed. All 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[v26 census](../renderer/generated/four-profile-census-v26.json) is now
21,225 exact, 1,699 different, and zero errors, with 939 unowned residual
test IDs. The [bitmap pattern investigation](../renderer/resized-one-axis-bitmap-pattern.md)
records the source path and remaining one-to-three-pixel differences. The
clean [v27 raster index](../renderer/generated/focused-primitive-raster-v27.json)
is 640/640 focused and 960/960 primitive exact; all 1,600 Open UI and
Chromium decoded hashes stayed fixed. The clean
[v5 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v5.json)
is 797/800 exact with three differences and zero errors; all 800 Open UI and
Chromium decoded hashes stayed fixed. It does not replace a complete expanded
manifest run.

The prior clean census at `5c7aaa9c` changed eight original Open UI images
in the one-axis repeated linear-gradient family. Their gradient interiors now
match Chromium at the two fractional scales; each complete image still has
93–98 differing pixels on a separate PNG image edge. No exact comparison
regressed or became exact, and all 22,924 Chromium oracle identities and
decoded hashes stayed fixed. The [v25 census](../renderer/generated/four-profile-census-v25.json)
therefore remains 21,223 exact, 1,701 different, and zero errors, with 939
unowned residual test IDs. The [gradient investigation](../renderer/one-axis-gradient-picture-shader.md)
records the pinned Chromium source path, the eight image-level improvements,
and the remaining PNG edge. The clean [v26 raster index](../renderer/generated/focused-primitive-raster-v26.json)
is 640/640 focused and 960/960 primitive exact; all 1,600 Open UI and
Chromium decoded hashes stayed fixed. The clean [v4 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v4.json)
is 797/800 exact with three differences and zero errors; all 800 Open UI and
Chromium decoded hashes stayed fixed. It does not replace a complete expanded
manifest run.

The prior clean census at `2f560e46` repaired four mixed-border comparisons:
one at 800×600@1 and three at 375×667@2. Open UI changed exactly 16 images
in four border fixtures, every changed image moved closer to Chromium, and
no previously exact comparison regressed. All 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[border paint-order investigation](../renderer/mixed-border-paint-order.md)
records the source rule and remaining fractional-scale pixels. The remaining
1,701 differences across 939 unowned residual test IDs keep the renderer
gate red. The clean
[v25 raster index](../renderer/generated/focused-primitive-raster-v25.json)
is 640/640 focused and 960/960 primitive exact, with all 1,600 decoded
images unchanged. The clean
[v3 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v3.json)
is 797/800 exact with three differences and zero errors; all 800 decoded
images and statuses stayed fixed. It does not replace full expanded-manifest
qualification.

The prior clean census at `d0592ccd` repaired the 1280×720@1.25
`out-of-flow-in-multicolumn-047` comparison. The inner multicolumn source
had already been consumed, but its column boxes were replayed inside an
outer visual continuation. The same continuation also painted too much
parent background. Both now follow the source interval, while the direct
absolute child still paints in the later outer column. Only that Open UI
decoded image changed, and it became exact against Chromium. No previously
exact comparison regressed; all 22,924 Chromium oracle identities and
decoded hashes stayed fixed. The
[nested-column investigation](../renderer/multicol-consumed-nested-columns.md)
records both reduced diagnostic variants. The remaining 1,705 differences
and 939 unowned residual test IDs keep the renderer gate red. The clean
[v24 raster index](../renderer/generated/focused-primitive-raster-v24.json)
is 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI and
Chromium hashes unchanged from v23.
The clean [v2 selected-additions
recheck](../renderer/generated/expanded-additions-recheck-v2.json) at
`db763999` found 797/800 exact, three different, and zero errors, with all
800 Open UI and Chromium hashes unchanged from the prior full expanded run.
It is a diagnostic selection, not a complete expanded-manifest result.

The prior clean census at `6e21bd9b` repaired the 1280×720@1.25
`out-of-flow-in-multicolumn-046` comparison. A zero-height positioned parent
retains the visual continuation for its absolute child but paints no parent
background in that extra area. Only that Open UI decoded image changed, and
it became exact against Chromium. No previously exact comparison regressed;
all 22,924 Chromium oracle identities and decoded hashes stayed fixed. The
[zero-height continuation investigation](../renderer/multicol-zero-height-positioned-decoration.md)
records the reduced evidence and source rule. The remaining 1,706 differences
and 940 unowned residual test IDs keep the renderer gate red. The clean
[v23 raster index](../renderer/generated/focused-primitive-raster-v23.json)
is 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI and
Chromium decoded hashes unchanged from v22.

The complete clean expanded run at `6e21bd9b` measured 22,015/23,724 exact,
1,709 different, and zero errors. Only the same original `-046` comparison
changed; all 23,724 Chromium oracle identities and decoded hashes stayed
fixed. The [v12 ledger](../renderer/generated/expanded-requalification-v12.json)
retains 197 of the 200 native additions at all four profiles and demotes the
same three; the [v13 diagnostic selection](../../tools/qualification/manifests/expanded-v13.json)
records that set without changing the original manifest.

The prior clean census at `99fd4432` repaired three fractional-scale row
flex comparisons: `-029` at 1280×720@1.25 and `-030` at 1280×720@1.25 and
1920×1080@1.5. Only those three Open UI decoded images changed, and each
became exact against Chromium. No previously exact comparison regressed; all
22,924 Chromium oracle identities and decoded hashes stayed fixed. The
[final-decoration investigation](../renderer/flex-final-continuation-decoration.md)
records the reduced case and rejected broad rule. The remaining 1,707
differences and 941 unowned residual test IDs keep the renderer gate red. The
clean [v22 raster index](../renderer/generated/focused-primitive-raster-v22.json)
is 640/640 focused and 960/960 primitive exact. All 1,600 Open UI decoded
images and Chromium oracle hashes match the preceding clean raster run.

The complete clean expanded run at `99fd4432` measured 22,014/23,724 exact,
1,710 different, and zero errors. Only the same three original flex
comparisons changed; all 23,724 Chromium oracle identities and decoded hashes
stayed fixed. The [v11 ledger](../renderer/generated/expanded-requalification-v11.json)
retains 197 of the 200 native additions at all four profiles and demotes the
same three; the [v12 diagnostic selection](../../tools/qualification/manifests/expanded-v12.json)
records that set without changing the original manifest.

The prior clean census at `89a0a1f3` repaired one fractional-scale nested
row flex comparison. Exactly one Open UI decoded image changed and became
pixel exact against Chromium. No previously exact comparison regressed; all
22,924 Chromium oracle identities and decoded hashes stayed fixed. The
[first-line break investigation](../renderer/nested-row-flex-first-line-break.md)
records the reduced case and guard. The remaining 1,710 differences and 943
unowned residual test IDs keep the renderer gate red. The clean
[v21 raster index](../renderer/generated/focused-primitive-raster-v21.json)
is 640/640 focused and 960/960 primitive exact. All 1,600 Open UI decoded
images and Chromium oracle hashes match the preceding clean raster run.

The prior complete clean expanded run at `89a0a1f3` measured 22,011/23,724 exact,
1,713 different, and zero errors. Only that original flex comparison changed;
all 23,724 Chromium oracle identities and decoded hashes stayed fixed. The
[v10 ledger](../renderer/generated/expanded-requalification-v10.json) retains
197 of the 200 native additions at all four profiles and demotes the same
three; the [v11 diagnostic selection](../../tools/qualification/manifests/expanded-v11.json)
records that set without changing the original manifest.

The prior clean census at `17952772` repaired three fractional-scale
`out-of-flow-in-multicolumn-*` comparisons. Exactly three Open UI decoded
images changed, and each became pixel exact against Chromium. No previously
exact comparison regressed; all 22,924 Chromium oracle identities and decoded
hashes stayed fixed. The [relative continuation clip investigation](../renderer/multicol-relative-clip-translation.md)
records the cause and neighboring guards. At that checkpoint, 1,711
differences and 944 unowned residual test IDs kept the renderer gate red. The clean
[v20 raster index](../renderer/generated/focused-primitive-raster-v20.json)
at `ef7214b3` is 640/640 focused and 960/960 primitive exact. All 1,600
decoded Open UI images and Chromium oracle hashes match the preceding clean
raster run.

The complete clean expanded run at `de8304fe` measured 22,010/23,724 exact,
1,714 different, and zero errors. Four original column comparisons became
exact since the prior v8 expanded run; no other Open UI decoded image or
status changed. All 23,724 Chromium oracle identities and decoded hashes
stayed fixed. The [v9 ledger](../renderer/generated/expanded-requalification-v9.json)
retains 197 of the 200 native additions at all four profiles and demotes the
same three; the [v10 diagnostic selection](../../tools/qualification/manifests/expanded-v10.json)
records that set without changing the original manifest.

The complete clean census at `9f983df9` measured the nested positioned
continuation repair across all four profiles. One comparison became exact,
one neighboring image changed but remains different, and none regressed. All
22,924 Chromium oracle identities and decoded image hashes stayed fixed. The
remaining 1,715 differences and 948 unreviewed residual IDs keep the renderer
gate red. The [multicolumn investigation](../renderer/multicol-nested-positioned-continuation.md)
records the reduced reproducer and remaining seam. The focused and primitive
matrices stayed 640/640 and 960/960 exact. All 1,600 Open UI and Chromium
decoded hashes stayed fixed from the prior clean raster run.

The clean expanded run at the same checkpoint measured 22,006/23,724 exact,
1,718 different, and zero errors. All 200 native final-state additions kept
their prior four-profile statuses and all 800 Open UI/Chromium decoded hashes:
197 remain exact and three remain demoted. The
[v8 ledger](../renderer/generated/expanded-requalification-v8.json) and
[v9 diagnostic selection](../../tools/qualification/manifests/expanded-v9.json)
record this without changing the original manifest or Chromium oracle.

The earlier clean expanded run at `d73077e7` gained ten exact
`css_break` comparisons and made no other Open UI image changes. The 200
native final-state additions kept their prior statuses and decoded hashes;
197 remain exact at all four required profiles. The
[v3 requalification ledger](../renderer/generated/expanded-requalification-v3.json)
and [v4 diagnostic selection](../../tools/qualification/manifests/expanded-v4.json)
record this without changing the original manifest or Chromium oracle.

The [manual hardening run](https://github.com/zhuowcui/open-ui/actions/runs/36235389665)
on checkpoint `e555c442` records every job: X11/Mesa and pure-Wayland
conformance passed; native C UBSan passed; Rust 1.85 MSRV failed on dependency
metadata; Miri failed when the Engine test called Skia C FFI; AddressSanitizer,
LeakSanitizer, and fuzz failed on process-exit Fontconfig allocations. The
ordinary [PR CI run](https://github.com/zhuowcui/open-ui/actions/runs/36235359772)
passed Rust parity, Python accountability, platform conformance, and both
format checks. Its frozen replay was still running when that status was
written; this replay is now diagnostic. Later check results must be recorded
before qualification.

The next [hardening validation](https://github.com/zhuowcui/open-ui/actions/runs/36236727149)
on checkpoint `68db3c74` passed Rust 1.85 headless/Linux checks, the Miri C
handle test, and native C UBSan. ASan and LSan still report Fontconfig
allocations after all 17 FFI tests pass; fuzz stops on the same allocation
path. Its Linux platform smoke job was still running when this status was
written.

The later [hardening run](https://github.com/zhuowcui/open-ui/actions/runs/36313866551)
on checkpoint `73502f9d` passed MSRV, Miri, X11/Mesa and pure-Wayland
platform smoke, and C UBSan. All 18 FFI tests passed under both sanitizers,
then LeakSanitizer and AddressSanitizer reported 10,476 bytes in 236
Fontconfig allocations at process exit. Fuzz stopped on a 2,606-byte
Fontconfig allocation report.

The [manual hardening run](https://github.com/zhuowcui/open-ui/actions/runs/36812728952)
on the current `ceebef52` evidence checkpoint passed X11/Mesa and pure-Wayland
platform smoke, Miri handles, and C UBSan. AddressSanitizer and LeakSanitizer
again failed after all 18 ABI tests passed: each reported 10,476 bytes in 236
Fontconfig allocations at process exit. Fuzz failed on 2,606 bytes in 59
Fontconfig allocations. The MSRV check passed. These failures keep hardening
unqualified.

The optional frozen replay checks the archived Open UI bytes; it does not prove
those bytes equal Chromium. The historical pixel comparator accepted
per-channel differences up to 4 and excluded the rightmost 15 pixels. At least
186 of its 5,731 reported passes have a nonzero channel delta in the compared
area. The
archived result totals and generated kickoff baseline remain immutable
historical records, not zero-tolerance qualification evidence or pixel gates.
See the [frozen oracle audit](../renderer/frozen-oracle-audit.md).

The archived image for
`wpt/css_backgrounds/background-image-gradient-interpolation-repaint-ref`
differs from the pinned Chromium pixels at 800×600@1. That old Open UI image
is not a pixel target. The [historical evidence](../renderer/generated/frozen-oracle-audit-v1.json)
preserves the inputs and measured difference. Its old blocked status records
the retired two-gate policy; it is not the current release decision.

The final-head [PR CI run](https://github.com/zhuowcui/open-ui/actions/runs/36237911865)
passed its other jobs but failed the obsolete frozen replay and strict
historical-exactness steps. Those steps have been replaced by archive
integrity and metadata checks. The four-profile Chromium matrix remains a
separate, failing release gate; green PR CI alone does not qualify pixels.

## Release decision

Do not publish or tag v0.2 final while any required row is open. A release
candidate may be packaged for qualification. Final publication requires a
clean Git tree, two identical artifact runs for both architectures, all hosted
and lab gates, signed checksums/provenance, and recorded crates.io/native SDK
installation evidence.

Known technical gaps are maintained in
[`unsupported-features.md`](unsupported-features.md) and the implementation
area documents for compositor, animations, accessibility, and hardening.
