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
framework APIs. Remaining generated C primitive property families still
require review and implementation. Native scrollbar keyboard/pointer input,
accessibility, nested scrolling, full renderer qualification, and release-lab
qualification also remain open.
