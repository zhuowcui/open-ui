# Open UI

Open UI is a typed, reactive desktop and headless UI framework built around one
pure-Rust renderer. Version 0.2 targets Linux on x86-64 and AArch64, with X11,
Wayland, OpenGL presentation, automatic software fallback, and a co-equal C ABI.

```text
safe Rust API ─┐
               ├─> openui-engine ─> style/layout/text ─> immutable scene
typed C ABI ───┘                                           │
Linux events ─────> interaction/accessibility ─────────────┼─> OpenGL window
headless clock ────────────────────────────────────────────└─> exact Skia raster
```

The supported application path has no Blink/Chromium runtime, resource pack,
HTML loader, CSS text parser, JavaScript engine, or network stack. Chromium 147
is the pinned rendering reference used to prove renderer compatibility.
Open UI never executes JavaScript, in this or future versions. Applications
handle interaction through public native Rust APIs, signals, and Rust event
callbacks. The consuming app calls `openui::Document` and `openui::Element`
methods to create, find, change, measure, focus, and scroll elements and operate
controls. When an app needs an operation that a browser exposes through
JavaScript, Open UI must provide the equivalent as a public native Rust method.
The implementation must include its state changes and events. Interacting
with elements never requires JavaScript in Open UI. A missing method is
unfinished framework work, even when an internal test fixture can already
produce the same visual state. See the
[native interaction contract](docs/v02/supported-platforms.md#native-interaction-api).

## Verified status

The [real Fontations factory prototype](docs/renderer/native-fontations-factory.md)
builds from the unchanged Skia pin and passes standalone Ahem and DejaVu Sans
probes. It supplies a real Fontations typeface and native advances. Consuming
app pixels, element bounds and full renderer qualification remain open.
Documentation/evidence checkpoint `9f0d2211` passes all three PR workflows:
six jobs pass, five skip and none fail. These are separate from the prototype.

The [private Linux font policy follow-up](docs/renderer/native-linux-font-policy.md)
matches all 30 repeated Chromium font-metric observations. Its first consuming
Rust app sweep improves from 62/200 to 162/200 exact images, with 100 gains and
no exact losses; 38 images and 1,280 bounds still differ. A revised explicit
automatic policy preserves fixed caller hinting and passes 345 text tests.
Its complete repeated image sweep retains all 200 PNGs byte-identically.
These sources are unapplied, inherit an older rejected descriptor correction,
and still need complete typeface, C and renderer qualification. Umbrella
`0c39998c` passes all hosted workflows and all seven full hardening jobs.

The [native variable-font correction](docs/renderer/native-variable-font-instance.md)
keeps the font instance selected by a consuming Rust callback when drawing
outlines. The old callback changes no pixels at five scales; the corrected
callback matches the font's independent reference glyph at all five. Both
sources pass fifteen read-only checks. The correction passes 342 text tests,
8,559 workspace tests, all eight build stages and C/C++ ABI consumers.
Focused 640/640 and primitive 960/960 Chromium checks pass. Both complete
censuses finish at 21,334/22,924 original and 22,137/23,728 expanded exact,
with zero errors and actual exits 1. All 48,252 rows preserve their nine
comparison invariants: no changed rows, gains, losses or changed Chromium
inputs. Umbrella `e0e9382d` passes all hosted workflows and all seven full
hardening jobs; its new native guard actually runs and passes. The native
callback's glyph comparison is not a Chromium pixel result. Full parity
remains unfinished. Open UI executes no JavaScript.

The [native text-width follow-up](docs/renderer/native-intrinsic-snap.md)
finds that shared sizing discards small shaped fractions which Chromium keeps.
A narrow correction reproduces the old failure and passes its consuming Rust
callback guard at all five scales. Thirteen source checks and the neighboring
native guards pass. Its clean build passes 8,552 workspace tests and all
thirteen build stages. Chromium geometry is now 38,400/38,400 exact, while
images remain 0/600 exact. All 400 actual C/C++ images match Rust. Focused
640/640 and primitive 960/960 image suites pass. Its complete censuses finish
at 21,313/22,924 original and 22,116/23,728 expanded exact, zero errors.
Each loses 21 exact comparisons and gains none. The correction is rejected
and remains unapplied.

The [native glyph audit](docs/renderer/native-glyph-descriptor.md) finds that
Chromium uses channel-specific coverage in 184/200 existing images, while
Open UI uses grayscale coverage in all 200. A general physical strike
correction removes the old 10px phase and origin overrides. Fourteen read-only
checks pass. Fresh integrated-source trial `c68d946c` passes fifteen source
checks, the descriptor guard, all 343 text tests, 8,560 workspace tests and
eight build stages. Its real Rust app remains 0/400 Chromium images exact;
23,040/25,600 geometry states match. Default native images stay unchanged.
Explicit LCD output changes 28 images, with none becoming exact and 21
worsening. Focused 640/640 and primitive 960/960 pass. Both complete censuses
finish at 21,305/22,924 original and 22,108/23,728 expanded exact, zero errors.
Each loses 29 exact comparisons and gains none. All Chromium inputs remain
unchanged. The correction is rejected and remains unapplied. The native references select
Fontations, while the original real-font references select FreeType; the
current native Rust configuration lacks an explicit Fontations choice.
The private `ef8880b0` outline option now passes its clean build and 8,560
workspace tests. Its real Rust callback app matches 62/200 unchanged Chromium
images: 40/40 at scale 1 and 22/160 at larger scales. Default and FreeType
remain 0/200, with all 400 prior native images unchanged. The
[completed evidence](docs/renderer/generated/native-font-choice-v2.json)
also records repeated Chromium baseline measurements and its Linux scale
policy. The option still uses FreeType for matching, shaping and metrics,
inherits the rejected descriptor change, and has no new complete census.
It remains private and unqualified.

The [shared native font-inheritance correction](docs/renderer/native-text-style-inheritance.md)
combines Rust/C text replacement with authored style propagation. Its new
public Rust callback regression fails before the fix and passes at all five
scales after it. Nine named style guards, thirteen source checks and 8,551
workspace tests pass; all thirteen clean build stages and C/C++ smokes pass.
All 400 actual C/C++ images match Rust. Chromium matches 34,560/38,400 geometry
states, but 0/600 images; some text widths remain 1/64 pixel short. All seven
own-source hosted hardening jobs pass. Focused 640/640 and primitive 960/960
suites pass. Both full matrices finish at 21,334/22,924 original and
22,137/23,728 expanded exact, with zero errors. All 48,252 comparison rows
preserve their nine recorded invariants. The text/style correction is now
integrated into the umbrella branch; its clean integration build also passes
all thirteen stages and 8,551 workspace tests. Full Chromium parity remains open.

The [native text replacement review](docs/renderer/native-text-content.md)
finds a C API bug: its setter stores text on a container, while layout reads
Text children. All 800 C/C++ font images are blank. A shared Rust/C Engine
correction and consuming applications are prepared; thirteen source checks
pass. Its first hosted tests expose an inconsistent viewport in the new guard.
A fresh test setup corrects that input and passes all seven hosted hardening
jobs. The old C setter fails the named regression; the shared correction passes
it, the 10,000-update storage guard and all 58 native conformance scenarios.
The clean build passes 8,538 workspace tests and ABI consumers, then the C
smoke stops because the harness omitted the library's required filename.
A fresh retry installs and verifies `libopenui.so.0` and passes the complete
build, then is interrupted after 306 app images. Its geometry still fails
because text children do not inherit authored styles. The shared font
correction above completes a fresh app matrix. Shared text replacement and
style inheritance are now integrated; the strict Chromium gates remain open.

The [authored glyph precision investigation](docs/renderer/native-author-glyph-precision.md)
finds a source-supported explanation for three Ahem images: rounding shaped
advances before Skia selects the LCD phase moves the second glyph. A private
correction preserves those advances. Eleven source checks pass. A fresh run
reproduces the named baseline failure, passes the fixed guard and all 342 text
tests. Its original queued owner is interrupted before execution and preserved.
Application pixels and full matrices have not run. No new exact result is claimed.

The [table source-retention follow-up](docs/renderer/native-table-progress.md#canonical-source-retention-follow-up)
keeps an immutable full table subtree for ancestor continuations. A geometry-only
diagnostic finds a 60-pixel cropped row over a 100-pixel source body. Eleven
source checks and seven own-source hosted jobs pass on the private correction;
the local retry stops at the disk guard before tests. Native geometry, teardown
and pixel verification still require a fresh run. It remains unapplied.

The [latest native review](docs/renderer/generated/native-review-v3.json)
rejects the inline-image candidate: its native app passes, but one expanded
case loses pixel equality at all four profiles. The
[repeated-table candidate](docs/renderer/native-table-progress.md) fails its
geometry guard: both baseline and proposed correction produce four fragments
where Chromium produces 41. Neither change is applied. The raster retry passes
its regression guards and all 17 clean build stages. Its native application
passes 828 of 840 contracts; twelve still fail. Its font consumer is 73/1,200
images and 25,600/76,800 geometry states exact. The static consumer is exact
only with explicit Fontations (60/60); default and FreeType each match 0/60.
The selection is 648/880 exact, with zero errors. Focused 640/640 and primitive
960/960 gates pass. The [complete trial audit](docs/renderer/generated/native-review-v4.json)
finishes at 21,251/22,924 original and 22,050/23,728 expanded exact, zero errors.
It loses 83 original and 87 expanded exact comparisons, gains none, and keeps
every Chromium input unchanged. This raster change is rejected for application.

The [fresh image-fallback candidate](docs/renderer/native-inline-fallback.md)
keeps fallback children in normal flow and adds native Rust/C image clearing.
Thirteen read-only checks pass; two Chromium runs agree on all 120 ordered
geometry queries. Its local harness stops on a nonexistent restore branch
before native guards or pixels execute. The candidate is unapplied; a fresh
verified harness is required and no renderer gain is claimed.

The [native keyword constructors](docs/renderer/native-keyword-values.md) and
[public Rust raster options](docs/v02/native-rust-raster-options.md) are
integrated at `2d338d6c`. Apps can configure the shared Engine through
`Document`, `AppBuilder`, and `HeadlessApp` and read its immutable selection.
The clean combined build passes all fifteen stages, 8,558 workspace tests,
fifteen read-only checks, and Rust/C/C++ consumers. All twenty native Rust
images and bounds match Chromium at five scales, with repeat captures stable.
Failed capture attempts remain preserved. The keyword source's complete
matrices retain 640/640 focused, 960/960 primitive, 21,334/22,924 original and
22,137/23,728 expanded exact, with no changed comparison invariants. Complete
combined-source matrices and all raster settings' behavior remain unqualified;
no release state is admitted.
The [completed umbrella checks](docs/renderer/generated/native-rust-options-v3.json)
at clean `16187f4f` pass fifteen read-only checks and all four hosted workflows:
thirteen jobs pass, five are skipped and none fails, including all seven full
hardening jobs. All six native Rust/FFI guards execute and pass in hosted parity.
These checks do not qualify the complete Chromium pixel gates.

The current v0.2 release candidate has:

- a historical archive of 5,731 Open UI renders, with 5,549 byte-identical
  on replay and 182 changed; these old screenshots are not pixel targets;
- a [complete clean umbrella census](docs/renderer/generated/native-scroll-insets-v42.json)
  at `2e443f49` with 21,334 of 22,924 comparisons exact, 1,590 different, and
  zero render errors; all Chromium images and identities remain unchanged,
  and the full pixel gate still fails;
- clean 40-profile raster matrices with 640/640 focused and 960/960 primitive
  comparisons exact after clearing all 18 workspace packages and rebuilding
  `2e443f49`; [v41](docs/renderer/generated/native-scroll-insets-v41.json)
  audits all 1,600 unchanged comparison invariants;
- 201 native final-state cases in the expanded contract, including one newly
  added case; 200 of 201 meet the four-profile gate in the earlier complete
  viewport run, and one remains a failure in the
  [expanded evidence](docs/renderer/generated/native-viewport-full-v9.json).
  The other 35 AST-lowered cases remain pending;
- a 7/7 repository accountability audit over all 7,673 inventoried tests;
- 58 application scenarios covering retained updates, controls, editing,
  accessibility, resources, scrolling, animation, bidi, and multi-document use;
- generation-checked Rust and C handles, deterministic manual clocks, immutable
  scenes, X11/Wayland operation, software presentation, and OpenGL upload;
- 84 frozen retained-engine/headless C exports, with 113 current exports and
  checked layouts and an ABI checksum; the [native scroll and inset consumers](docs/v02/native-scroll-metrics.md)
  and [native style consumers](bindings/rust/openui-ffi/README.md#native-style-operations)
  pass through public Rust, C and C++ APIs; the clean umbrella checkpoint
  passes 8,528 workspace tests with 13 ignored at `580539c2`; its own
  640/640 focused and 960/960 primitive pixel suites remain exact, with all
  1,600 comparison invariants unchanged;
- sanitizer, Miri, fuzz, leak, latency, idle-work, and package gates defined
  in CI; several remain open or failing.

Chromium is the sole pixel target. The archived Open UI bytes disagree with
Chromium for some fixtures, which is why replaying old screenshots cannot be a
release gate. A [font oracle audit](docs/renderer/scaled-lcd-hinting-oracle-investigation.md)
also found one older cached Chromium capture that differs from six fresh
captures under the same recorded identity; that evidence needs reconciliation.
The latest clean umbrella census has 1,590 differences. Against the SVG
checkpoint, nine comparisons become exact and none lose exactness, while four
already failing comparisons worsen. Every residual still requires review.
A fresh clean build passes 8,531 workspace tests, with zero failures and 13
ignored, plus the native ABI consumers. Its focused and primitive matrices
pass. Its [complete original and expanded runs](docs/renderer/generated/native-scroll-insets-v42.json)
finish at 21,334/22,924 and 22,137/23,728 exact, zero errors, actual exits 1.
All 46,652 comparison invariants match the preceding renderer. The
older workspace count containing a test absent from its declared source
remains unqualified. The private
[opaque image background candidate](docs/renderer/native-image-background-culling.md)
uses public Rust callbacks and shared paint logic. Ten read-only checks pass;
its native execution and pixel verification are queued. No improvement to the
full census is claimed for that candidate. A separate image-edge candidate
at `2eacae2c` keeps sampled colors intact and lets Skia apply geometric coverage
while blending. Its ten read-only checks pass; Engine assertions, 128 fieldset
images, 720 public Rust callback images and all four matrices are queued.
It is unapplied and unqualified.
The separate [font engine trial](docs/renderer/native-font-engines.md) loses
493 exact comparisons. A shared routing discrepancy selects a different
authored text path from the explicit FreeType reference. The named physical
outline guard now fails on its baseline and passes on the correction. After
correcting two position types in the native Rust example, its next build stops
on a missing geometry-member diagnostic before tests execute. That member is
present in clean source; a complete workspace rebuild is needed to check
artifact reuse. Compilation and pixel verification remain pending.
The [native C raster configuration](docs/v02/native-c-raster-configuration.md)
is also prepared on a private checkpoint. It copies the same immutable Rust
options into the shared engine and adds C/C++ callback consumers. Ten read-only
checks pass and ABI metadata preserves all 113 existing exports and 30 layouts.
Its first boundary-test build also stops on geometry-member diagnostics;
consuming-application execution, Miri and exact pixels remain pending.
the umbrella branch still has 113 exports and admits no new release case.
The private [raster-field correction](docs/renderer/native-raster-configuration-fields.md)
now carries requested Fontations settings and applies LCD phase during paint.
It removes two 10px-only adjustments and adds native application checks. Ten
read-only checks pass; compilation and exact Chromium verification are queued.
Default native raster and remaining font-policy overrides still require work.
The [recording cache repair](docs/renderer/generated/native-scroll-insets-v35.json)
is applied. Its earlier run passes the CPU/Ganesh cache guards and all seven
hosted hardening jobs and reports 8,532 workspace tests. The
[source-attribution audit](docs/renderer/generated/native-scroll-insets-v39.json)
finds one executed text test absent from that declared source; the earlier
workspace result needs full clean revalidation. The applied source reports
8,531 tests, and both complete pixel matrices retain all 46,652 comparison
invariants. The four-profile pixel gates still fail. A separate clean queue
cleans all 18 workspace packages at every source switch before rebuilding
and rerunning all matrices. All seven own umbrella
[manual hardening jobs](https://github.com/zhuowcui/open-ui/actions/runs/37244559627)
pass at `287e176a`, with zero skips; the full Chromium pixel gate still fails.
The [prepared native style/cache integration](docs/renderer/generated/native-scroll-insets-v36.json)
at private `0ccc37da` passes ten read-only checks. Its named regression check
and all nine inheritance tests pass, but the local harness incorrectly
expected eight. The [corrected queue](docs/renderer/generated/native-scroll-insets-v40.json)
checks the full named inventory after clearing all workspace packages;
native consumers and all pixel matrices remain pending. All seven own-source
hosted hardening jobs pass, with zero skips. Prior style results are attributed to
`5cc75147`, including the 60 failing native static-position states. This new
integration remains unapplied and unqualified.
The [intrinsic constraints and whitespace investigation](docs/renderer/native-intrinsic-constraints.md)
records the completed fieldset trial's 64 original exact losses and two shared
layout causes. The [completed review](docs/renderer/generated/native-review-v1.json)
of private `a6d386e4` records 8,550 passing workspace tests, zero failures and
13 ignored. Focused and primitive pixels pass, but the original census is
21,299/22,924 exact and expanded is 22,098/23,728 exact, zero errors, exits 1.
It loses 35 previously exact original comparisons and four addition comparisons,
and gains none. All Chromium inputs stay fixed. The consuming-app capture gate
is incomplete because a Chromium reference pair differs. This source remains
unapplied; accepted renderer totals and release admission stay unchanged.

The [latest native review](docs/renderer/generated/native-review-v2.json)
records five terminal stops: a raster harness branch-restoration error, an
inline image geometry failure, a worsened rounded-border pixel assertion,
and two disk-guard stops. All original evidence remains preserved. The
[isolated inline replaced correction](docs/renderer/native-inline-replaced.md)
at private `c92e2d08` reproduces the named baseline failure and passes two fixed
guards and thirteen read-only checks. Both pinned Chromium query runs agree
on all 48 box observations across four profiles. Its
[clean native verification](docs/renderer/generated/native-inline-replaced-v1.json)
passes 8,538 tests, twelve C and six C++ consumers, and all 60 consuming Rust
images and bounds against 240 stable Chromium captures. All four complete
pixel matrices are running; the correction remains unapplied.
Accepted renderer counts and release admission remain unchanged.

The earlier review records native constructor candidate `893ea292`. Rust apps
already have typed setters for these operations; the shared C constructor was
missing column-fill, fragmentation, border-style and table-role values. The
candidate fills those paths with owned, property-bound values and includes
consuming Rust, C and C++ examples. All 13 read-only checks pass, including
generator consistency and C/C++ syntax. Its baseline clean is stopped by the
disk guard before named tests execute. Clean builds, native callbacks and exact
Chromium comparisons require a fresh complete run.
This is native API work; Open UI executes no JavaScript. The candidate remains
unapplied and unqualified.
The [v40 raster evidence](docs/renderer/generated/native-scroll-insets-v40.json)
records all native guards and five C boundary tests passing at `3d4eea11`.
Its workspace stops on an example's unavailable transitive-crate import,
before pixel verification. Private `fb284c54` exposes all five raster-setting
types through the public Rust API and makes the example use those exports.
Renderer bodies and the C ABI stay unchanged. Ten read-only checks pass;
the clean native and pixel queue remains pending. All seven parent-source
hosted checks pass, with zero skips, in [v39](docs/renderer/generated/native-scroll-insets-v39.json);
the revised source needs its own hosted checks.
The same evidence preserves 4,500 repeatable Chromium geometry
observations without generating images; native verification remains required.
Earlier renderer measurements below remain attributed to their named sources. The
[private sampling candidate](docs/renderer/generated/native-viewport-full-v14.json)
loses 23 exact Chromium comparisons and gains 14 in its complete census;
it remains unapplied. A subsequent
[generated-tile format trial](docs/renderer/generated-image-sampling.md)
repairs 18 of those regressions in the affected selection, preserving all
850 exact native controls. A later opaque-layer composition trial restores
four more exact comparisons in that selection, leaving one SVG pixel
regression. A subsequent [coverage-region trial](docs/renderer/generated/native-viewport-full-v17.json)
preserves those four matches and all 850 existing native controls while fixing
transparent-canvas erasure in neighboring Rust consumers. The SVG decoration
alpha, native scroll extents, remaining control pixels, and complete candidate
qualification remain open before promotion.
The [clean private SVG work](docs/renderer/generated/native-svg-viewport-v11.json)
adds native viewport creation and corrects shared curved-border painting.
Two complete original censuses finish at 21,325/22,924 exact, zero errors:
17 comparisons become exact, none lose exactness, and 13 already failing
comparisons worsen. The solid-border source also passes both complete
40-profile raster suites. These private results do not replace the accepted
renderer's census. The work is now rebased over the current native APIs.
Its Rust consumers preserve every SVG and scrolling image; 1,176/1,920 SVG
states and all 850 existing scrolling controls match Chromium exactly.
All SVG owned bounds, callbacks and teardown checks pass. C/C++ consumers
verify the shared viewport constructor and native sizing callbacks, preserving
110 exports and existing layouts. Its Linux-enabled workspace passes 8,516
tests with zero failures and 13 ignored. Its fresh 40-profile matrices pass
640/640 focused and 960/960 primitive comparisons, with unchanged results.
Its own complete original census also finishes at 21,325/22,924 exact,
1,599 different and zero errors, with all comparison invariants unchanged
from the double-border source. Its complete expanded run is 22,128/23,728
exact, 1,600 different and zero errors. All original rows agree between the
two suites, and all 804 additions stay unchanged: 200/201 cases meet all four
profiles. Both complete pixel gates still fail.
Complete rebased qualification, the 744
remaining SVG pixel failures, transforms and scrolling ranges remain open.
The [reviewable source patch](docs/renderer/evidence/native-svg-decoration-v1/native-svg-rebase-api-v293.patch)
is applied in this umbrella checkpoint together with the reviewed scrolling
repair and native reveal API. Its complete clean umbrella pixel gate still fails.
Earlier disk failures are preserved.

The [nested scrolling work](docs/v02/native-scroll-metrics.md#nested-scrolling-candidate)
implements shared native dimensions, ranges, offset rounding and detached
queries. Its earlier complete `6e255ca3` run is 21,321/22,924 original exact
and 22,124/23,728 expanded exact, zero errors. Eight comparisons worsen and
four lose exactness because anonymous lines omit child overflow. The next
clean `02c0296e` repairs that shared propagation and native sticky invalidation.
It restores all eight comparisons in the partial sticky sweep, matches all
50 reduced Rust application states, and gains 12 exact native scroll images
with zero loss versus `6e255ca3`: 1,775/2,560 pixels and 2,320/2,560 dimensions.
Its shared Rust/C constructor passes nine C and four C++ consumer processes;
8,516 Linux-enabled workspace tests and both 40-profile raster gates pass.
Its complete original census is 21,332/22,924 exact and expanded is
22,135/23,728 exact, zero errors, with actual exits 1. All Chromium inputs stay
unchanged. It gains nine original exact matches but loses two flex-overflow
matches at scale 1.25; three fragmentation comparisons also worsen. Six paint
reviews remain open, so this source cannot be promoted. A later clean block
scrollbar candidate at `b5a2044f` repairs all 240 missing geometry states:
2,560/2,560 match Chromium. Pixels remain 1,775/2,560 exact, with no exact
image lost. Its workspace, C/C++ consumers, 50 reduced Rust states and both
40-profile raster matrices pass. Its partial sticky gate still fails in four
states, and no complete census is inferred for that source.
[Versioned evidence](docs/renderer/generated/native-nested-scroll-v7.json)
and unapplied patches preserve earlier failures. These private results do not
replace the accepted renderer's census.

The complete `3f95e617` runs finish at 21,319/22,924 original and
22,122/23,728 expanded exact, zero errors, with actual exits 1. They expose
15 lost exact comparisons against the SVG checkpoint. The shared scrollbar
capture precedence and clip-margin correction at `83d45e0c` restores all 15
in the affected selection: 157/172 exact, 15 different, zero errors. Four
already-failing comparisons still worsen against SVG; no full result is
inferred for this correction.

This umbrella checkpoint applies the reviewed SVG and scroll repairs and adds
public Rust `scroll_into_view` and `smooth_scroll_into_view`, backed by the same
Engine operation as accessibility and two additive C functions. The private
API source passes 35 Engine tests and ten C/four C++ consumers; all 30 reduced
native geometry states match Chromium, while 16/20 endpoint images are exact.
Scroll-margin/padding support, broader alignment coverage, four scale-1.25
pixel failures and two legacy contour calibration paths remain open.
[Versioned evidence](docs/renderer/generated/native-nested-scroll-v9.json)
preserves every earlier failure. Own clean umbrella runs complete at
21,334/22,924 original and 22,137/23,728 expanded exact, zero errors, with
observed exits 1. All original rows agree, all 804 additions stay unchanged,
and 200/201 additions meet all four profiles. The workspace, ten C/four C++
headless consumers, 249 Python tests and ten read-only checks pass.

The font-cache lifetime fix is applied and pushed at `a41fdeb9`. Its own
Linux-enabled workspace passes 8,526 tests, zero failures and 13 ignored;
all ten read-only checks pass, with source unchanged. The
[latest lifetime evidence](docs/renderer/generated/native-font-cache-lifetime-v4.json)
records complete clean private original and expanded pixel sweeps. Every
comparison invariant agrees with `d174ea0b`: the fix changes no rendered
pixels and leaves the full pixel failures open. Its earlier local sanitizer,
fuzz and ABI checks and failed attempts remain preserved.

All three pull-request workflows at this checkpoint pass. The separate
[manual hardening run](https://github.com/zhuowcui/open-ui/actions/runs/37149510887)
passes all seven jobs: address/leak sanitizers, Miri, C UBSan, Linux windows,
MSRV and all five fuzz targets. Skipped pull-request jobs remain open results.
The [latest private scroll evidence](docs/renderer/generated/native-scroll-insets-v9.json)
records complete `dac78e25` runs at 21,340/22,924 original and 22,143/23,728
expanded exact, zero errors, with actual exits 1. Eight comparisons become exact
but two lose exactness; the screen correction remains unapplied. The two losses
are owned by painting and must be repaired before promotion.

A separate clean `cf59ea29` combines compatible nested rectangular clips before
rasterization. Its C matrix now matches all 175 geometry and pixel states,
repairing the remaining ten clip differences with no exact losses. All 510 Rust
states and all 1,600 focused/primitive comparisons stay unchanged and exact.
Its workspace passes 8,532 tests, zero failures and 13 ignored; ten read-only
checks pass. Complete original and expanded runs finish at 21,340/22,924 and
22,143/23,728 exact, zero errors, actual exits 1. Every comparison invariant
agrees with `dac78e25`, including both known exact losses.

The separate `6d6768a8` fixes native reveal traversal through layout containers
and containing-block ownership when public transforms are added or removed.
All 90 Rust and 60 C geometry and pixel states match Chromium; the C test gains
25 geometry matches and 15 pixel matches without losing an exact image.
All 510 existing Rust states stay byte-identical. Its workspace passes 8,535
tests, zero failures and 13 ignored, and ten read-only checks. Both raster gates
pass with all 1,600 comparison invariants unchanged. The viewport cutoff at
`2cc950e0` recovers five C geometry/image matches, preserving all 660 previous
states and both raster gates. A wider check then finds ten failures for
viewport-fixed controls on an already-scrolled page.
The combined `4dd50621` repair passes all six fresh build stages, 8,538 workspace
tests, ten read-only checks, all 935 native Rust/C geometry/image states and
both raster gates. It preserves all 885 earlier states and recovers all ten
fixed-control failures. Its complete censuses finish at 21,341/22,924 original
and 22,144/23,728 expanded exact, zero errors, actual exits 1. Six comparisons
lose exactness against `cf59ea29` and five against the applied umbrella; all
Chromium inputs and 804 addition results remain unchanged.
The included scroll-edge change at `c5769f2d` repairs the two earlier exact
losses in a 144-comparison selection, but introduces three new exact losses
against `dac78e25` at 1.5 scale. That blocks promotion; reduced results do not
establish a complete census result.
Reviewable source patches are retained; these private changes
remain unapplied and unqualified, and no new release passes are admitted.

The [new scroll investigation](docs/renderer/generated/native-scroll-insets-v11.json)
records a missing native trailing-margin extent and a clip policy applied to
an `auto` box without a scroll transform. The high-DPI candidate passes its
native consumers and focused matrix, but fails eight primitive comparisons.
A clean Rust follow-up passes 8,539 workspace tests, all 935 earlier native
states and both exact raster matrices; the eight gradient regressions are
repaired. All 105 new dimension queries match, but 14 opaque white images and
20 wider collapsed-margin API states still fail. Complete image censuses are
running. The [latest guards](docs/renderer/generated/native-scroll-insets-v12.json)
retain every failure; the patch remains unapplied and unqualified.
The [collapsed-margin follow-up](docs/renderer/generated/native-scroll-insets-v13.json)
retains shared layout data and adds a consuming Rust app. Its ten read-only
checks pass; its clean build and native probes are queued behind the immutable
image sweeps. All 765 fresh Chromium metric states are reference observations,
not native passes. No rendering or API qualification is claimed for this source.

The [v14 follow-up evidence](docs/renderer/generated/native-scroll-insets-v14.json)
completes the private `45ddeee3` censuses: 21,350/22,924 original and
22,153/23,728 expanded exact, zero errors, actual exits 1. One multicolumn
regression against the applied renderer remains; all Chromium inputs and 804
addition results stay unchanged. Corrected margin source `2240ee9c` passes
8,540 workspace tests and all 45 C mutation states, but its wider Rust probe is
555/765 exact, with 210 vertical empty-block failures. Its pixel guards did not
start. A separate owned float-color C API and public Rust/C/C++ consumers are
implemented in a private checkpoint; their own runtime build and native
consumers now pass, as recorded in the
[v15 evidence](docs/renderer/generated/native-scroll-insets-v15.json).
The opaque-scroll review owns 29 of 40 neighboring pixel failures. None of
these private changes is promoted or admitted as a release pass.

The float-color candidate passes 8,542 workspace tests and all public Rust,
eleven C and five C++ consumers. It resolves the eight native process failures
using the original float-color inputs; all 40 geometry states agree, but only
11 images are exact. All 32 earlier native control images stay unchanged.
A public Rust check confirms missing writing-mode inheritance on native
attachment. The shared Engine repair passes all eight clean build stages,
8,542 workspace tests, 765/765 Rust states and 45/45 C states. All 210 prior
vertical failures are repaired without exact losses or changed Chromium inputs.
Its pixel guards now pass all 935 earlier native states, 70 two-child states
and both raster matrices unchanged. All 105 dimension queries agree, but
14 opaque white images still differ. Other inherited properties and pixel
qualification remain separate work; this candidate remains unapplied.

The standalone float-color constructor and public Rust/C/C++ consumers are
now applied to the umbrella branch. The existing Rust API supplies float
colors directly; C uses the new owned `oui_style_value_color_f32_v1` operation
over the same Engine. The [v16 evidence](docs/renderer/generated/native-scroll-insets-v16.json)
records six fresh umbrella build stages, 8,528 workspace tests, five Rust runs,
eleven C and five C++ consumers, and both exact raster matrices. The first
hosted checkpoint failed C formatting; `aeed821c` corrects only C whitespace,
passes the rebuilt C/C++ consumers and all three hosted workflows. Five
skipped hardening jobs remain unverified on this source. The margin and
writing-mode repairs remain separate, unapplied candidates; no new full
census or release pass is claimed.

The [native inheritance and style trials](docs/renderer/generated/native-scroll-insets-v21.json)
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
Its own complete matrices now finish at 640/640 focused, 960/960 primitive,
21,334/22,924 original and 22,137/23,728 expanded exact. All comparison
invariants restore the applied baseline, including all 804 additions. Both
complete census commands exit 1; the full pixel gate still fails.
This candidate is unapplied and unqualified; no new release state is admitted.
Open UI runs no JavaScript in any version. Needed interaction uses public
native Rust methods and Rust callbacks over the shared Engine.

The [native event API checks](docs/renderer/generated/native-event-targets-v1.json)
are complete at clean `1c8540e9`. Rust callbacks can call `Event::target()` and
`current_target()` to inspect and change elements. Five guards, 8,536 workspace
tests, native ABI consumers and seven hosted hardening jobs pass. The public
Rust app matches Chromium in all ten images and bounds at five scales.
Both complete renderer matrices preserve every comparison result: original
21,334/22,924 and expanded 22,137/23,728 exact, zero errors, exits 1. The
focused and primitive gates pass. Full pixel parity remains unfinished.

The [intrinsic sizing follow-up](docs/renderer/generated/native-scroll-insets-v28.json) reviews the
missing width behavior against 120 repeated Chromium advance observations and
2,000 neighboring measurements at five scales. Chromium retains positive
shaped-width remainders on its 1/64px layout grid and includes first-line
indentation. The private source preserves both, uses shaped intrinsic text,
and adds a consuming Rust app for 200 sizing cases. It also exposes immutable
raster choice through native `Document`, `AppBuilder` and `HeadlessApp`
constructors; the previous document API always used the default.

The initial new Engine guard used the wrong text setup and measured an empty
box on both sources. Its failed build is preserved, with 208 tests passed and
one failure before the workspace stopped; the two raster API guards passed.
The corrected guard attaches actual native text children. It now proves the
old 80px width fails the 80.015625px Chromium result, and the fixed source
passes at all five scales, including inherited indentation and a reset. Ten
read-only checks pass. The complete build then stopped after 5,763 passing
tests and one real regression: an inline canvas lost its natural width. The
new intrinsic sequence exposed that replaced content was collected as an
ordinary inline wrapper. The shared fix collects it as an atomic box. All 26
existing canvas, image and positioning guards now pass, and the native width
guard still passes. That source completed all nine build stages: 8,543 workspace tests passed,
none failed and 13 were ignored; eleven C and five C++ consumers passed with
113 exports. The combined 200-case native sizing app then failed in float
layout after reaching about 43 GiB RSS. Individual cases all finished, and a
bounded group probe isolated duplicate inherited float exclusions. The shared
fix propagates only newly added floats. All 61 block tests pass, and the full
native app now finishes at five scales with peak RSS below 85 MiB. It matches
700/1,000 Chromium size measurements; 300 still differ. Correct JSON number
formatting removes six diagnostic false differences without changing layout
or adding tolerance. The subsequent shared inline-block, preserved-newline and wrapping fixes now
match all 1,000 native sizing measurements exactly. That clean source passes
all nine build stages, 8,544 workspace tests, ten read-only checks and the same
eleven C/five C++ consumers with 113 exports. The static-position app matches
all 120 bounds measurements at both raster settings, with repeatable output,
but none of its 120 pixel comparisons are exact. The remaining differences
are in glyph coverage. Explicit Chromium LCD settings still selected the
portable author outline path; a shared policy fix is prepared and has ten
passing read-only checks. Its pixel effect and full build are unverified.
The sizing source now completes all 640 focused and 960 primitive comparisons
exactly. All 1,600 native and Chromium image, oracle, status and difference
invariants remain unchanged from the earlier clean renderer. Its complete
original census now finishes at 21,264/22,924 exact, 1,660 different and zero
errors, with actual exit 1. Against `5cc75147`, it loses 70 exact comparisons
and gains none; 115 comparisons change across 33 test IDs. The expanded run
finishes at 22,063/23,728 exact, 1,665 different and zero errors, also exit 1.
All original rows agree with the separate census. It loses four more exact
comparisons in the broken-image multicol addition, leaving 199/201 additions
exact at all four profiles. All Chromium inputs remain unchanged. The changed
causes still need minimized reproducers and review; this candidate and its
descendants remain unapplied. The 1,000 native geometry matches and exact
focused suites do not override those full-census failures.

Source review also found a compositor cache bug: scene generations restart for
each document, but both compositors checked only that number before reusing a
frame. A different document can receive the previous document's pixels.
A shared fix now requires the immutable recording's identity and keeps it owned
while its frame is cached. It includes native document, viewport, resource
lifetime and explicit Ganesh guards. Ten read-only checks pass at `107e2e36`;
its local baseline regression, fixed tests, builds and pixels have not run.
Its own hosted hardening run passes all seven jobs with none skipped, including
native Linux consumers, MSRV, Miri, sanitizers and fuzz smoke. Those jobs do not
execute the new direct cache and Ganesh unit guards. Its local pipeline waits
for the LCD pipeline to terminate before Cargo; the intrinsic pipeline is now
terminal. The inherited sizing regressions also prevent promotion.
The candidate remains unapplied and unqualified. Full Chromium pixel parity
and retained compositor animation remain open. No runtime
promotion or new pixel admission is claimed. Prior snapshots and all
reference bytes remain unchanged.

This repository is not yet
declaring the final v0.2 release. Physical-GPU
and reference-machine qualification, automated AT-SPI operation, direct Skia
GPU qualification, retained per-node layers, compositor-owned animation
curves, release-lab C/C++ application qualification, and signed publication
still remain. The C ABI now runs native Linux windows through the Rust `App`
and retained `Document`. See
[current status](docs/progress/current-status.md)
and [release qualification](docs/v02/release.md).

## Rust quick start

Rust 1.85 or newer, C/C++ build tools, and the host C runtime development files
are required. A Chromium checkout is not.

```toml
[dependencies]
openui = { version = "0.2.0", features = ["linux"] }
```

```rust,no_run
use openui::prelude::*;

fn main() -> Result<(), Error> {
    let count = create_signal(0_i32);
    let app = App::builder()
        .title("Open UI")
        .size(LogicalSize::new(800.0, 600.0))
        .backend(BackendPreference::Auto)
        .build()?;

    app.run(move || view! {
        <button
            style:display={Display::Flex}
            style:padding="8px 16px"
            on:click={move |_| count.update(|value| *value += 1)}
        >
            {count.get()}
        </button>
    })
}
```

From this checkout:

```bash
cd bindings/rust
cargo run --locked --package hello                 # deterministic hello.png
cargo run --locked --package hello --features linux # native Linux window
cargo run --locked --package framework-test -- --headless /tmp/openui-framework-test.png
cargo run --locked --package framework-test --features linux -- --window
```

Use `OUI_BACKEND=software` or `OUI_BACKEND=opengl` to force a window backend.
Headless applications use `HeadlessApp::render_at(time)` for repeatable frames.
The [framework test app](bindings/rust/examples/framework-test/README.md) checks
a reactive click and writes the resulting PNG in headless mode.
If the linker reports missing `Scrt1.o` or `crti.o`, the host C runtime
development files are absent. On the Chromium-equipped maintainer machine,
the checked-in `.cargo/config.chromium.toml` supplies a pinned sysroot; add
`--config .cargo/config.chromium.toml` immediately after `cargo` in the
commands above.

## Native SDK

The v0.2 header uses length-delimited UTF-8, versioned configuration structs,
tagged style values, checked ownership, and structured thread-local errors.

```bash
python3 tools/release/build_v02_linux.py \
  --target x86_64-unknown-linux-gnu --format sdk --format deb
```

The release driver emits headers, static/shared libraries, pkg-config and CMake
metadata, C and Rust examples, detached debug symbols, licenses, an SPDX SBOM,
checksums, and SLSA-style provenance. RPM production runs on Fedora through the
release workflow. See [packaging instructions](docs/v02/packaging.md).

## Repository map

| Path | Purpose |
|---|---|
| `bindings/rust/openui` | Safe application framework and reactive runtime |
| `bindings/rust/openui-engine` | Retained document, interaction, animation, resources, accessibility |
| `bindings/rust/openui-compositor` | Immutable scenes and raster scheduling |
| `bindings/rust/openui-platform` | Feature-gated Linux event loop and presentation |
| `bindings/rust/openui-ffi` | Validated static/shared C ABI |
| `bindings/rust/openui-{style,layout,text,paint}` | Exact rendering pipeline |
| `include/` | Generated v0.2 C headers |
| `examples/c_v02/` | C examples matching the Rust examples |
| `tools/accountability/` | Chromium comparison inventory and historical evidence audit |
| `tools/release/` | Contract generation and reproducible packaging |
| `docs/v02/` | Supported architecture and release contract |

Historical GN/Blink and SP2 experiments remain in Git for provenance, but are
excluded from the workspace and v0.2 packages. They are not supported engines.

## Documentation

- [Development](docs/DEVELOPMENT.md)
- [Architecture](docs/architecture/rendering-pipeline-overview.md)
- [CI and release gates](docs/CI.md)
- [Rust/C migration guide](docs/v02/migration-v01-v02.md)
- [Unsupported features](docs/v02/unsupported-features.md)
- [Typed style reference](docs/v02/generated/style-properties.md)
- [C ABI guide](bindings/rust/openui-ffi/README.md)

Open UI is licensed under Apache-2.0. Bundled font and dependency notices are
preserved with the relevant sources and release artifacts.
