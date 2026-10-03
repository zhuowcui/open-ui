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

The [nested scrolling evidence](../renderer/generated/native-nested-scroll-v1.json)
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
original and expanded suites are still running at this evidence checkpoint.
All 850 root scrolling controls remain exact. All 180 scroll-layer dimension
checks now match Chromium; their pixels remain 95/180 exact.

The C consumer still fails its new content-dimension assertion. Rust's public
constructor gives a `div` block layout, while C called the raw Engine factory
with an initial inline style. The [next source patch](../renderer/evidence/native-nested-scroll-v1/native-nested-scroll-v3.patch)
adds one shared native constructor used by both bindings. Its ten read-only
checks pass, but its build, workspace, consumer and pixel verification are
pending. No consumer assertion is changed to hide that discrepancy.
