# Native scroll dimensions and inset mutations

Open UI runs no JavaScript. Rust and C applications call native methods over
the same retained engine. Missing browser-style element behavior remains
framework work to implement and expose to the consuming application.

The [latest private trial](../renderer/generated/native-scroll-insets-v13.json)
retains collapsed end margins in shared layout data, separately from an empty
child's position. Its consuming Rust app reads dimensions and bounds, scrolls
and mutates margins through public methods. Ten read-only checks pass; the
clean build and native probes are queued after the locked image sweeps.
The 765 fresh Chromium states are references, not Open UI passes. All 45 prior
reference states remain unchanged. The 20 earlier native failures and 14 white
pixel failures remain open until the new source is tested.

The framework never executes JavaScript, in any version. Needed element
operations must be callable through public Rust methods and Rust callbacks.
Separate reference tools may inspect or run scripts inside Chromium; those
tools are not part of the application framework.

Rust applications already use `Element::scroll_metrics`. C applications now
use `oui_element_get_scroll_metrics_v1` to read the same client and content
dimensions in logical pixels. Initialize the versioned output header; the
query resolves pending layout and copies the result into caller storage.
There is no handle or buffer to release. Copies survive later mutations and
document destruction. Detached elements and `display: none` elements return
no metrics and four zero dimensions. Errors leave both outputs unchanged.

The existing C offset query also resolves pending layout. Content shrink and
viewport resize therefore clamp retained offsets before returning them.
Independent horizontal and vertical overflow values now pass through the
native C enum conversion and the shared Rust overflow value grammar.

The same consumer checks exposed incomplete inset length handling. Native
`left`, `top`, `right`, and `bottom` mutations now resolve through shared
retained style state. A consuming Rust app sets lengths from a Rust click
callback, reads owned bounds, changes to percentage right/bottom positioning,
and resizes its viewport. No internal fixture or script supplies those changes.


## Scroll clip ownership investigation

The [composition review](../renderer/generated/native-scroll-insets-v10.json)
preserves a native two-child scroll box across seven origins and five scales.
The analytic-clip candidate gains seven exact images at 1.25 scale and loses
four at 1.5; every geometry state remains exact. Four diagnostic Chromium
captures preserve the original reference bytes while logging the compositor's
property trees. At 1.25 the scroll node is not composited and its overflow clip
stays in paint. At 1.5 the scroll node is composited and a clip in its parent
paint-offset space belongs to the compositor.

Chromium's exact release-tag source confirms that its text policy stops
strongly preferring LCD text at DPR 1.5, allowing ordinary user-scrollable
contents to be composited. CPU replay must preserve this clip ownership without
implicitly selecting a GPU backend. The private `b6a62a9b` correction completes
seven fresh build stages, with 8,538 workspace tests and ten read-only checks
passing. All 935 existing native states and 70 new Rust/C states are exact.
The focused matrix passes 640/640, but the primitive matrix fails at 952/960:
eight gradient images lose exactness at 1.5 scale. Its 44-case selection is
108/176 exact and retains one earlier exact loss in multicolumn scrolling.
Complete censuses were not started after the primitive failure. Opaque
scrolling paint chunks and direct compositing reasons can
also composite below the threshold and remain unfinished ownership work.
No test ID, output-pixel correction or changed reference is used.

The [next evidence](../renderer/generated/native-scroll-insets-v11.json) records
two shared causes. Native scroll dimensions omit a child's trailing margin:
eight of sixteen public C queries return a height of 300 instead of Chromium's
320. Separately, five fresh captures of a non-overflowing `auto` box reproduce
the immutable Chromium images and show no element scroll transform at any
scale. That box's clip remains in paint; the high-DPI policy cannot be selected
from authored overflow alone.

Clean `45ddeee3` retains child margins in scroll extents and shares reachable
scroll geometry between paint translation and clip ownership. Its public Rust
consumer reads owned metrics for one or two children, and a native mutation
regression checks margin growth, shrinkage and offset clamping. The
[completed guards](../renderer/generated/native-scroll-insets-v12.json) pass all
seven fresh build stages, 8,539 workspace tests and ten read-only checks.
Both raster matrices are exact: 640/640 focused and 960/960 primitive, with all
1,600 comparison invariants unchanged. All 935 previous native states and 70
new two-child Rust/C states are exact. All 105 public dimension queries match
Chromium; the green images are 70/70 exact, while 14 of 35 opaque white images
still differ. Complete original and expanded censuses are running. The earlier
compile failure at `2cc6b8d5` is retained. The
[source patch](../renderer/evidence/native-scroll-clip-range-v1/native-scroll-clip-range-v1.patch)
remains unapplied. Collapsed margins, low-DPI opaque paint chunks and complete
property-space ownership still need implementation and native checks.

A [public C mutation reproducer](../renderer/evidence/native-scroll-clip-range-v1/native-scroll-collapsed-margins.c)
now checks leaf, nested and empty-child margins at five scales. It uses the
same Rust Engine, preserves owned metric snapshots through mutation and
document teardown, and renders no images. Its 45 dimension/bounds states are
25 exact, gaining ten without a whole-state exact loss. Twenty states still
fail: nested collapsed margins return scrollHeight=100 instead of 120; empty
children return 160 instead of 130, with ten newly wrong height fields. Their
initial bounds are also ten pixels too low on both old and new sources.
The scroll extent and normal-flow position need retained collapsed-margin
data. Both actual probe exits remain 1; this candidate is unqualified.


## Private scroll margin and padding qualification

The [private candidate](../renderer/generated/native-scroll-insets-v1.json)
at clean `45ddd7a4` adds 18 physical and logical scroll-margin/padding setters
for consuming Rust and C apps. Earlier property IDs, 112 exports and 30
existing layouts are preserved. Ordered aliases and relative units survive
writing-mode, font and viewport changes; invalid values leave the document
unchanged. Instant, smooth and accessibility reveal use the shared plan.
The [reviewable patch](../renderer/evidence/native-scroll-insets-v1/native-scroll-insets-v1.patch)
is unapplied to the umbrella implementation.

Its complete Linux-enabled workspace passes 8,532 tests, zero failures and
13 ignored, with source unchanged. Ten C and four C++ headless consumers run,
including 70 C inset cases. All ten read-only checks pass. Focused and primitive
40-profile matrices remain 640/640 and 960/960 exact, with every comparison
invariant unchanged from `d174ea0b`.

The earlier consuming Rust run checks 170 cases at five scales against two
identical fresh Chromium captures per case. It records 510/510 matching geometry
states and 452/510 exact images, with 58 edge differences at 1.25 scale.
Its complete original and expanded censuses now finish with observed exits 1:
21,334/22,924 and 22,137/23,728 exact, zero errors. Every comparison invariant,
including all Chromium bytes and identities, agrees with the umbrella renderer.

The [screen-origin correction](../renderer/generated/native-scroll-insets-v3.json)
at clean `dac78e25` recovers all 58 differences: the same 510 initial, instant
and smooth states now match Chromium in geometry and pixels. Every earlier
exact image, owned geometry state and callback count stays unchanged. The
shared painter includes the containing screen origin and realized ancestor
translations when snapping scroll movement. It changes neither logical scroll
offsets nor reference bytes and uses no test-ID or output-pixel correction.
All 8,532 workspace tests and ten read-only checks pass. Focused and primitive
matrices remain 640/640 and 960/960 exact, with every comparison invariant fixed.

The smaller C matrix checks 175 states at five scales and seven integer/fractional
container positions. All geometry agrees; 165/175 images are exact, gaining 22
with no exact losses. Its ten remaining static nested-clip differences are
unchanged under paint ownership. Pinned Chromium combines compatible rectangular
clips before rasterization; native replay still repeats their edge coverage.
Its now-complete original and expanded censuses are 21,340/22,924 and
22,143/23,728 exact, zero errors, with actual exits 1. Eight comparisons become
exact but two lose exactness: an unscrolled scrollport's trailing coverage
changes after the new screen snap. Both losses have paint ownership. The
correction remains unapplied and cannot be promoted with those regressions.

The [previous evidence](../renderer/generated/native-scroll-insets-v9.json) records
two separate follow-ups. Clean `cf59ea29` skips a redundant rectangular mask
when a compatible child clip contains the existing parent clip. The tighter
vector clip remains active; effects, rounded clips, transforms and scroll
boundaries retain their own property spaces. All 175 C geometry/image states
are now exact, with ten gains and no losses. All 510 Rust states and all 1,600
focused/primitive comparison invariants stay unchanged and exact. Its workspace
passes 8,532 tests and ten read-only checks. Its complete censuses finish at
21,340/22,924 original and 22,143/23,728 expanded exact, zero errors, observed
exits 1. Every comparison invariant agrees with `dac78e25`; no original or
expanded result changes. All Chromium bytes/identities and 804 addition results
stay fixed. The two screen-snap regressions remain open.
The [source patch](../renderer/evidence/native-clip-containment-v1/native-clip-containment-v1.patch)
is available for review.

Clean `6d6768a8` makes reveal follow the retained fragment-box containers used
by bounds and hit testing. Public native transforms now establish a containing
block even when their matrix is identity; removing a transform removes that
containing block and requests layout. Instant and smooth Rust callbacks and
instant C callbacks reproduce six flow/absolute/fixed cases at five scales:
90/90 Rust and 60/60 C geometry/image states are exact. C gains 25 geometry and
15 pixel matches without losing an exact image; all 510 existing Rust states
remain byte-identical. Its workspace passes 8,535 tests and ten read-only
checks. Both raster gates pass with every comparison invariant unchanged. The
[source and Rust consumer patch](../renderer/evidence/native-reveal-containers-v1/native-reveal-containers-v1.patch)
and [C consumer](../renderer/evidence/native-reveal-containers-v1/native-reveal-containers.c)
are reviewable.

A wider viewport test first exposed five Engine failures on `6d6768a8`:
revealing an offscreen fixed target incorrectly moved the document and target.
Clean `2cc950e0` now completes all six fresh build steps, with 8,537 workspace
tests passing, zero failures and 13 ignored. All 20 C and 30 Rust new states
match Chromium; all 660 prior Rust/C states retain every image byte, geometry
state and callback count. The C viewport check gains five geometry and five
pixel matches with no exact losses. Ten read-only checks and both raster
gates pass with all 1,600 comparison invariants unchanged. The
[source patch](../renderer/evidence/native-reveal-root-v1/native-reveal-root-v1.patch)
and [C reproducer](../renderer/evidence/native-reveal-root-v1/native-reveal-root.c)
remain reviewable and unapplied.

A fresh already-scrolled check is only 10/20 exact on that source. With root
scrollTop=120, a viewport-fixed control authored at y=180 wrongly paints and
reports bounds at y=60; Chromium keeps it at y=180. The absolute neighbor is
exact. Both fresh Chromium repeats agree; all native processes, callbacks and
owned teardowns pass. All ten failures retain connected regions, channel
deltas and five-scale behavior, with Engine/paint ownership. The following
combined prototype addresses that broader transform ownership; no repair is
inferred before its build and pixel checks.

The [scroll-edge prototype](../renderer/evidence/native-scroll-clip-v1/native-scroll-clip-v1.patch)
at clean `c5769f2d` retains partial scrollport-edge coverage for `overflow: scroll`
at fractional scale. Its five fresh build stages pass, with 8,532 workspace
tests, zero failures and 13 ignored. Ten read-only checks, all 685 native
geometry/image states and both raster gates pass unchanged. The reduced
144-comparison selection repairs the two earlier exact losses but introduces
three new exact losses against `dac78e25` at 1.5 scale. It finishes 85/144
exact, 59 different, zero errors, actual exit 1. Applying an analytic mask
through direct scroll replay does not fully reproduce Chromium's transform
and clip conversion. The three failures have paint ownership and prevent
promotion; this selection cannot establish a complete census result.

A separate [small native C consumer](../renderer/evidence/native-scroll-clip-v1/native-scroll-edge.c)
reduces the edge investigation to two scroll boxes, seven integer/fractional
positions and five scales. Its two captured native builds give 140/140 exact
geometry states and 90/140 exact images against identical fresh Chromium
repeats. All native operations and owned teardowns pass. These remain
diagnostic failures under paint ownership; no release states are admitted.

The [combined viewport-fixed prototype](../renderer/evidence/native-viewport-fixed-v1/native-viewport-fixed-v1.patch)
at clean `4dd50621` shares fixed-containing-block ownership between geometry
and paint. It cancels document scrolling for viewport-fixed bounds, hit testing
and painted content, while authored transform/containment and descendant
scrollports retain their own transforms. A consuming Rust app now prepares
visible fixed and absolute targets on an already-scrolled page; the
[C consumer](../renderer/evidence/native-viewport-fixed-v1/native-viewport-fixed.c)
uses the same native Engine. This source also combines the two private clip
changes. Ten read-only checks pass after adding two generated inventory
consumer paths; the earlier failed check remains evidence. All six fresh build
stages now pass, with 8,538 workspace tests, zero failures and 13 ignored.
All 935 native Rust/C geometry/image states are exact. All 885 previous states
retain their geometry, callbacks and image bytes; the already-scrolled C check
recovers ten geometry and ten image matches without loss. Rust and C agree on
all shared states. Both raster gates pass with all 1,600 comparison invariants
unchanged. Both complete censuses now finish with actual exits 1:
21,341/22,924 original and 22,144/23,728 expanded exact, zero errors. There are
881 residual original IDs. Six comparisons lose exactness against `cf59ea29`
and five against the applied umbrella; all Chromium inputs and 804 addition
results remain unchanged. Those regressions prevent promotion.

These private patches remain unapplied and unqualified; no new cases are
admitted as release passes. General clip intersections, broader transformed,
fragmented and fixed-viewport traversal, other reveal options and complete
needed public native API coverage remain required. Open UI runs no JavaScript;
consuming applications use native Rust operations and callbacks.

The [v14 follow-up](../renderer/generated/native-scroll-insets-v14.json)
finishes the preceding `45ddeee3` image censuses at 21,350/22,924 original and
22,153/23,728 expanded exact, with zero errors and actual exits 1. Chromium
inputs and all 804 addition results are unchanged; one multicolumn regression
against the applied renderer remains. Corrected margin source `2240ee9c`
passes all eight build stages, 8,540 workspace tests and 45/45 public C states.
The wider public Rust consumer is 555/765 exact; 210 vertical empty-block
states still fail. Native processes and owned snapshots pass, but the failed
API gate prevents image sweeps. Public native behavior remains unfinished.

The [v15 investigation](../renderer/generated/native-scroll-insets-v15.json)
confirms an Engine gap behind the vertical cases: unstyled native descendants
do not inherit their parent's writing mode. A consuming Rust app checks its
owned computed-style snapshots and fails for both vertical modes, while its
horizontal neighbor passes. The Engine regression test also fails. Private
`944068e1` repairs propagation on attachment, mutation, detach, moves and clones,
while preserving explicit values and fully resolved anonymous/pseudo snapshots.
It passes ten read-only checks, all eight clean build stages, 8,542 workspace
tests and all 765 Rust / 45 C native states. All 210 prior vertical failures are
repaired with no exact loss. Owned snapshots and unchanged Chromium measurements
are checked. Its pixel guards are running. Writing-mode repair does not
complete other inherited properties or qualify pixels.

## Verification

The [clean evidence](generated/native-c-scroll-metrics-v1.json) records source
`dfae5ae9`, unchanged before and after each completed process:

- 28 native API tests pass, including short and wrong-version headers,
  larger output storage, wrong-thread calls, reentrant borrows, stale handles,
  detached elements, and owned snapshot lifetime.
- Seven C examples and two C++ consumers compile, link, and run successfully.
  Both scroll consumers check five scales: 1, 1.25, 1.5, 2, and 3.
- The public Rust inset consumer passes its callback, resize, bounds, and
  document teardown checks at those five scales.
- All previous 109 exports, including the frozen 84, remain present. All 29
  previous recorded layouts and ABI version `0x00020000` remain unchanged.
  The additive metrics structure is 40 bytes, aligned to 8 bytes.
- Eight generators, archive integrity, and the 7/7 repository accountability
  audit pass without changing source.

The evidence retains two earlier failed C consumer attempts and the native
fixes they required. It also hashes 25 existing Chromium geometry captures
for the base scroll states. The resize and asymmetric-axis checks are native
behavior guards; they do not establish new pixel qualification.

The [C source](../../examples/c_v02/scroll_metrics.c), its
[C++ build](../../examples/c_v02/scroll_metrics.cc), and the
[Rust consumer](../../bindings/rust/openui/tests/native_insets.rs) use public
framework APIs. The later [native style checkpoint](generated/native-primitive-styles-v1.json)
verifies 35 primitive longhands through public Rust, C, and C++ consumers.
Remaining C property conversion and native API coverage still require review
and implementation. Native scrollbar keyboard/pointer input,
accessibility, nested scrolling, full renderer qualification, and release-lab
qualification also remain open.

## Nested scrolling candidate

The [nested scrolling evidence](../renderer/generated/native-nested-scroll-v2.json)
records additional shared Engine and layout work. Its
[source patch](../renderer/evidence/native-nested-scroll-v1/native-nested-scroll-v2.patch)
is based on the private SVG checkpoint `8d5a58a1`. The later reviewed stack
and native reveal operations are now applied to the umbrella; own combined
source qualification is pending, and historical evidence remains immutable.

The candidate derives native client and content dimensions from the final
physical layout. Client dimensions exclude borders. Reachable content includes
in-flow padding, child overflow, 2D transforms, clips, writing direction and flex
reversal. Native scroll calls use these same ranges, including negative offsets
in reversed directions. Layout updates clamp retained offsets after content
shrink and resize. Element scroll offsets round to logical pixels as the pinned
Chromium does; the root viewport retains its existing precision.

A consuming Rust application constructs the elements through public methods
and changes them through Rust click callbacks. It checks extreme and relative
scroll calls, smooth scrolling with a manual clock, content shrink, resize,
owned dimensions and document teardown. The C and C++ scroll consumers call
the same retained Engine. The shared native value parser now accepts scrollbar
width and gutter values and named colors; Rust also exposes `Color::from_named`.
These are native APIs. No JavaScript is executed by Open UI.

Detached geometry and dimension queries return empty results without updating
unrelated attached layout. A query on an attached parent remains a layout
barrier and clamps its scrolling range when its content has been removed.
Repeated Chromium observations at five scales verify both paths. The existing
Rust geometry test keeps its original expectations and adds the attached-parent
guard at those five scales.

The first broad run and its rounding repair retain all reference bytes. The
repair makes 270 more states exact in geometry and 179 more images exact, with
no loss of an exact image. Its remaining 240 geometry failures belong to six
cases where nested classic scrollbar gutters are still missing from layout.
Fractional-scale paint coverage and scroll backing phase also remain open.
Each differing image retains bounds, connected regions and channel deltas.
These native checks do not establish the complete renderer or release gate;
the linked evidence identifies each measured source and completed process.

Clean source `6e255ca3` passes 8,516 Linux-enabled workspace tests, with zero
failures and 13 ignored, and all ten read-only checks. Its own focused and
primitive matrices finish at 640/640 and 960/960 exact; every comparison
invariant remains unchanged from the prior SVG checkpoint. The complete
original and expanded suites finish with observed exits 1: 21,321/22,924 and
22,124/23,728 exact, zero errors. All Chromium images and identities are
unchanged. Eight comparisons worsen, including four exact losses, in two
sticky tests. Anonymous nowrap line wrappers omit child overflow from the
new range calculation, causing valid scrolling to clamp to zero.
All 850 root scrolling controls remain exact. All 180 scroll-layer dimension
checks now match Chromium; their pixels remain 95/180 exact.

The earlier C consumer fails its new content-dimension assertion. Rust's public
constructor gives a `div` block layout, while C called the raw Engine factory
with an initial inline style. The [next source patch](../renderer/evidence/native-nested-scroll-v1/native-nested-scroll-v3.patch)
adds one shared native constructor used by both bindings. Later clean
`02c0296e` passes nine C and four C++ consumer processes, including nested
scrolling at five scales. The existing pixel equality assertion now compares
public Rust `Document`/`Element` construction with C application construction;
the raw Engine retains initial CSS values for complete resolved-style callers.
Earlier failed workspace attempts and their causes are preserved.

### Element paint movement

The next [paint candidate](../renderer/evidence/native-nested-scroll-v1/native-nested-scroll-v4.patch)
addresses unsnapped scroll movement in ordinary elements. Chromium snaps a
scroll transform to physical pixels even when the element has no separate
compositor layer, through its [property tree](https://chromium.googlesource.com/chromium/src/+/refs/tags/147.0.7727.50/cc/trees/property_tree.cc#893).
Open UI already does this for the root viewport. The
candidate applies the same physical rule to direct element paint and its
software backing, while keeping API offsets and bounds logical. At scale
1.25, a logical offset of 50 moves paint by 63 physical pixels.

The [source review](../renderer/generated/native-nested-scroll-v2.json) pins
the exact Chromium `147.0.7727.50` tag files. Relevant snapping, promotion,
layout-box and overflow files match the local `.24` checkout byte for byte;
an unrelated border-shape call differs in the paint-property builder.
An offset-only snapping trial gains six native matches but loses four in RTL
scrolling. Chromium snaps `ScrollPosition = ScrollOrigin + ScrollOffset`, and
adds the origin to contents paint coordinates. The later source includes
that reversed origin, including at API offset zero. Its native sweep gains
12 exact images with zero loss versus `6e255ca3`. Transformed, SVG and animated
ancestry still require complete screen-space transform ownership.

### Verified native sticky interaction

The [current source patch](../renderer/evidence/native-nested-scroll-v1/native-nested-scroll-v7.patch)
also propagates child overflow through anonymous line fragments. It restores
all eight affected original comparisons in a 128-comparison sticky diagnostic.
That diagnostic is 124/128 exact; four unchanged fractional paint failures
remain. It does not establish a complete census result.

Native scroll-to/by and smooth-scroll samples previously retained sticky
positions from the old layout. The shared Engine now refreshes the sticky
dependencies owned by the scrolled container. Ordinary scrolling retains
compositor invalidation, and unchanged offsets keep the zero-work path.

The reduced consuming Rust application uses public methods and Rust click
callbacks for extreme, relative and smooth scrolling, content shrink and
resize. All [50 geometry and pixel states](../renderer/evidence/native-nested-scroll-v1/native-sticky-oracle-v1.json)
match two fresh Chromium sequences at five scales. Owned snapshots and weak
callback teardown pass. The [reference HTML](../renderer/evidence/native-nested-scroll-v1/native-sticky-oracle-v1.html)
is external oracle data; Open UI runs only the native Rust application.

Clean `02c0296e` passes 8,516 Linux-enabled workspace tests, zero failures,
13 ignored, ten read-only checks, nine C/four C++ consumer processes, and its
640/640 focused plus 960/960 primitive matrices. All 1,600 raster comparison
invariants are unchanged. All 850 root controls remain exact. All 180 layer
dimensions agree; layer pixels remain 95/180, with no exact image lost.

The [current evidence](../renderer/generated/native-nested-scroll-v7.json)
records 1,775/2,560 native scroll images and 2,320/2,560 geometry states exact.
The 240 missing scrollbar-layout states and 545 additional paint failures
remain owned work. The complete original run finishes at 21,332/22,924 exact
and expanded at 22,135/23,728, zero errors, with actual exits 1. Every Chromium
image and identity remains unchanged; original rows agree between suites and
all 804 additions stay unchanged. The earlier eight sticky regressions are
exact in the complete census. Nine original comparisons gain exactness, but
two flex-overflow matches are lost at scale 1.25 and three fragmentation rows
worsen. A sixth image changes with the same mismatch count. All six paint
investigations have an owner; layout dumps are unchanged. This source cannot
be promoted. Its failed source remains preserved; the later reviewed stack
is applied with its own umbrella qualification pending.

The next [block scrollbar candidate](../renderer/evidence/native-nested-scroll-v1/native-nested-scroll-v10.patch)
retains physical scrollbar space separately from authored border and padding.
Block child sizing, native ranges, clipping, hit testing and shared theme paint
consume that geometry. Its first compile fails because a flex-child constraint
initializer omits the new optional input. The failure and zero executed tests
are preserved. The next source also fails before running tests because two
existing painter test fragment literals omit the optional field. Clean
`b5a2044f` repairs both without changing assertions. Its Linux workspace has
8,516 passed, zero failed and 13 ignored; ten read-only checks, all clean
builds, nine C and four C++ consumer processes pass.

All 2,560 native states now match Chromium in metrics and owned geometry,
repairing the 240 layout failures. Pixels remain 1,775/2,560 exact, with no
exact image lost. The remaining 785 native image differences are paint work.
All 850 root controls and 50 reduced public Rust sticky states stay exact;
the complete focused and primitive matrices remain 640/640 and 960/960 exact.
The partial sticky gate remains 124/128, with four prior differences, and
fails. No complete original or expanded census is inferred for this source.
Flex, grid, table and replaced sizing, native scrollbar pointer/keyboard and
accessibility operation, and the earlier six complete-census paint reviews
remain open. The later stack is now applied; no release qualification is added.

The later [shared scroll-recording repair](../renderer/evidence/native-nested-scroll-v1/native-nested-scroll-v12.patch)
at clean `3f95e617` makes synthetic fragmentation clips retain their own
coordinates and keeps physical scroll snapping outside independently
rasterized backing content. The selected 152 original comparisons restore
both flex-overflow exact matches and three earlier fragmentation images.
The selection finishes 142 exact, ten different and zero errors, actual exit
1. Both complete runs finish with actual exit 1: 21,319/22,924 original and
22,122/23,728 expanded exact, zero errors. Against SVG, 15 original exact
matches are lost and 20 comparisons worsen. The unchanged Chromium inputs and
804 addition rows are audited; 200/201 additions meet all four profiles.

This source passes the Linux workspace, all native consumer guards, and both
complete 40-profile raster matrices. Native dimensions remain 2,560/2,560
exact and pixels remain 1,775/2,560, with no exact gain or loss; 267
already-failing images change. All previous reference bytes remain fixed.
The [reversed native Rust app](../renderer/evidence/native-nested-scroll-v1/native-reversed-scroll-v1.json)
adds fractional relative requests and weak native callbacks to a compact
reproducer; its source is prepared but has not been compiled or run.

The later `83d45e0c` repairs shared scrollbar capture precedence and clip-margin
propagation through overflow clips and paint containment. All 15 lost exact
comparisons recover in the 172-row selection: 157 exact, 15 different, zero
errors. Four previously failing comparisons still worsen against SVG; a full
census is required. The shared code is applied in this umbrella checkpoint.

## Native scroll-into-view

`Element::scroll_into_view(ScrollIntoViewOptions)` and
`Element::smooth_scroll_into_view(ScrollIntoViewOptions, duration_ms)` call a
shared Engine plan over owned element and enclosing scrollport geometry.
Accessibility reveal uses that same plan. Applications construct and mutate
the document in Rust; Open UI executes no JavaScript.

`ScrollIntoViewOptions` selects block and inline alignment (`Start`, `Center`,
`End`, `Nearest`) and enclosing containers (`All`, `Nearest`). Defaults are
block-start, inline-nearest and all containers. Pending layout is resolved
before planning; hidden overflow is scrollable, while visible and clip
containers are skipped. Detached or unboxed targets are no-ops. Foreign and
stale handles return errors. Smooth reveal uses the existing native clock,
returns owned animation IDs, and settles immediately for zero duration or
reduced motion. Duration must be finite and non-negative.

The append-only C functions `oui_element_scroll_into_view_v1` and
`oui_element_smooth_scroll_into_view_v1` use the same Engine operation.
Alignment/container values and duration are validated before borrowing the
Engine. Existing symbols and struct layouts are preserved; the ABI remains
`0x00020000`, with 112 exports. C callback and user-data lifetime/thread rules
remain those of the retained document API.

The [Rust consumer](../../bindings/rust/openui/examples/native_scroll_into_view.rs)
and [C consumer](../../examples/c_v02/scroll_into_view.c) create nested hidden
and clip containers, reveal through native callbacks, reset and smooth-scroll,
inspect owned geometry, and check teardown. Private clean `65147ab2` passes
35 Engine tests and all ten C/four C++ consumer processes. Its 30 geometry
states agree with two independent fresh Chromium captures at five scales;
16/20 endpoint images are exact. Four scale-1.25 endpoints each differ in
52 pixels and remain paint work. Broader alignment, oversized targets, writing
modes, root/containing-block behavior, native scroll-margin/padding properties,
and complete API qualification remain open. The code is now applied; own
clean umbrella consumer, workspace and pixel results are pending. No private
result is relabeled as an umbrella or release pass.
