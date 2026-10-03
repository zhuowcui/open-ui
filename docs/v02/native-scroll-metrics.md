# Native scroll dimensions and inset mutations

Open UI runs no JavaScript. Rust and C applications call native methods over
the same retained engine. Missing browser-style element behavior remains
framework work to implement and expose to the consuming application.

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
is based on the private SVG checkpoint `8d5a58a1`; it remains unapplied to the
umbrella renderer while complete pixel qualification is open.

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
be promoted. All patches remain unapplied to the umbrella renderer.

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
remain open. All patches remain unapplied; no release qualification is added.

The later [shared scroll-recording repair](../renderer/evidence/native-nested-scroll-v1/native-nested-scroll-v12.patch)
at clean `3f95e617` makes synthetic fragmentation clips retain their own
coordinates and keeps physical scroll snapping outside independently
rasterized backing content. The selected 152 original comparisons restore
both flex-overflow exact matches and three earlier fragmentation images.
The selection finishes 142 exact, ten different and zero errors, actual exit
1. Full original and expanded censuses are running; no complete result or
full no-regression claim is inferred.

This source passes the Linux workspace, all native consumer guards, and both
complete 40-profile raster matrices. Native dimensions remain 2,560/2,560
exact and pixels remain 1,775/2,560, with no exact gain or loss; 267
already-failing images change. All previous reference bytes remain fixed.
The [reversed native Rust app](../renderer/evidence/native-nested-scroll-v1/native-reversed-scroll-v1.json)
adds fractional relative requests and weak native callbacks to a compact
reproducer; its source is prepared but has not been compiled or run.

Applications still need a public Rust scroll-into-view operation. The existing
private accessibility method is not that public API and needs a shared
coordinate repair and independent native-app verification. This remains
framework work; it does not require JavaScript.
