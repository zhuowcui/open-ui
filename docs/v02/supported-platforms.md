# Open UI v0.2 supported-platform contract

## Supported

Open UI v0.2 supports Linux x86_64 and aarch64 applications through native X11
and native Wayland, plus platform-independent deterministic headless rendering.
The Linux runtime provides OpenGL acceleration with automatic software fallback.
The safe Rust API and versioned C ABI are co-equal front ends to the same engine.

Core product behavior includes typed styles, reactive views, pointer and
keyboard input, focus, clipboard, IME, drag and drop, core form controls,
AccessKit/AT-SPI semantics, and typed transitions/keyframe/scroll animations.

Headless consumers do not enable or compile window-system dependencies. All
resource bytes used by deterministic rendering are supplied synchronously by
the application or an immutable resource registry; the engine performs no
network access.

## Native interaction API

Open UI is a native framework. Applications implement interactivity in Rust
by calling public methods on retained `Document` and `Element` handles and
registering Rust callbacks. C applications use the versioned C ABI over the
same engine. Open UI never runs JavaScript, in this or future versions.

Finding an element, changing its state, measuring it, focusing it, scrolling
it, and handling events are native framework operations. Browsers expose
these operations through JavaScript APIs; Open UI must expose every needed
operation as a fully implemented public Rust API that the consuming native
app can call directly. The shared Rust engine must provide its state changes,
geometry and events. A missing method is required framework work.

These operations require no JavaScript code or runtime. Open UI provides no
JavaScript glue, script bindings, or `eval`. Scripts used by offline Chromium
reference tools run in the separate Chromium process.

A missing public native method is unfinished API work. Verify each needed
operation from a consuming Rust app before claiming it is complete. This
requirement also applies when the corresponding Chromium test uses JavaScript
or is excluded from the pixel matrix. An internal Engine operation or a
test-only fixture does not complete the public application API.

Lookup, mutation, geometry, focus, scrolling, controls, and event dispatch
operate directly on the native engine:

| Application task | Public Rust API |
|---|---|
| Create, find, clone, move, detach, or destroy elements | `Element::create`, `Document::element_by_id`, `clone_subtree`, `append_child`, `insert_before`, `detach`, `remove` |
| Create, read, edit, attach, move, detach, or destroy text nodes | `Document::create_text_node`, `TextNode::data`, `set_data`, `detach`, `remove`, `Element::append_text_child`, `insert_text_before` |
| Find elements by native kind or class | `Document::elements_of_kind`, `elements_with_class`, `Element::kind` |
| Change class tokens | `Element::has_class`, `add_class`, `remove_class` |
| Read or change text, attributes, or typed style | `Element::text_content`, `set_text`, `set_attribute`, `set_property` and generated typed setters |
| Read resolved style | `Element::computed_style`, which returns an owned snapshot |
| Supply image bytes and display an image, background, or border image | `Document::register_image_resource`, `set_resource_provider`, `load_image_resource`; `Element::set_image_resource`, `set_background_layers`, `set_border_image` |
| Handle input, inspect its target, or activate an element | `Element::on`, `on_capture`, `click`; `Event::target`, `current_target`, `phase`; Rust callbacks in `view!` |
| Inject normalized keyboard or committed text input | `Document::dispatch_key_input`, `dispatch_key_event`, `dispatch_text_input` |
| Focus, scroll, or inspect geometry | `focus`, `blur`, `scroll_to`, `scroll_by`, `scroll_metrics`, `scroll_into_view`, `smooth_scroll_into_view`, `client_rects`, `bounding_rect`; `Document::hit_test` |
| Update form controls or details | `set_control_value`, `set_selection`, `set_checked`, `set_open` |
| Move or extend a text selection, delete by grapheme or word, select all, undo, or redo | `Element::edit_text(EditCommand)`, using the same engine and `input` callback path as keyboard editing |
| Inject or cancel native IME input | `Document::dispatch_composition_start`, `dispatch_composition_update`, `dispatch_composition_end`, `dispatch_composition_cancel` |
| Select immutable rendering options for the native document or app | `Document::with_options`, `with_font_collection_and_options`, `raster_configuration`; `AppBuilder::engine_options`; `HeadlessApp::with_options` |

These methods operate on the same retained document as rendering and native
input. Open UI does not promise browser-compatible names or the entire Web API
surface. Every element interaction needed by a consuming application must be
available through a public native Rust method backed by the shared engine.
For example, an app can look up a button, register a Rust click callback, and
change another element through `Document::element_by_id`, `Element::on`, and
`Element::set_text`. It does not inject or execute a script. An internal Engine
operation or test-only fixture is not sufficient application API coverage.

To close an API gap, implement the operation in the shared engine, expose it
through the public Rust API, and verify it from a consuming native Rust
application. Test coverage must exercise the retained state and resulting
events or rendering. A missing public method remains an implementation gap
until that native application path works.

The [native editing consumer](../../bindings/rust/openui/examples/native_edit_commands.rs)
invokes typed editing commands from a Rust button callback. Text changes dispatch
`input` after engine borrows are released; listeners can inspect the edited
control and change other elements. Selection-only commands dispatch no `input`.
Read-only controls permit selection commands, while disabled controls reject all
commands. Rust selection offsets are UTF-8 byte positions on grapheme boundaries.
C command parity and complete editor scrolling and pixels remain open.

The [native font-relative helper](../renderer/native-font-relative-lengths.md)
resolves `ch`, `ex` and `lh` from an owned computed style and the app's font
collection. Apps currently assign a pixel value and recompute after font changes.
The [private upright correction](../renderer/native-glyph-mask.md) matches all
1,440 measured rectangles, with 160 gains and no losses; it captures no native
pixels and remains unapplied. Its parent glyph correction passes focused and
primitive checks but regresses both complete censuses and is rejected.
Automatic typed declarations, complete contexts and C parity remain native
API work. None of these operations requires JavaScript.

The [native Rust raster options](native-rust-raster-options.md) expose the
Engine's immutable selection directly to consuming apps. Rust callback,
configuration, bounds and teardown checks pass, with ten exact Chromium
images at five scales on the tested source. Complete raster-field behavior
and full renderer qualification remain open.

The [native event consumer](../../bindings/rust/openui/examples/native_event_targets.rs)
uses a parent Rust callback to find and change the child that received a click.
`Event::target` returns that child; `Event::current_target` returns the element
whose listener is running. Both use generation-checked weak handles. Saved
events do not keep the document alive, and listener identity and phase clear
when callbacks return, including errors and panic unwinding. The
[completed checks](../renderer/generated/native-event-targets-v1.json) verify
the public app path, owned bounds, teardown and exact Chromium pixels at five
scales. This completes those event methods; other needed native APIs and the
full renderer gate remain open.

The [prepared native style integration](../renderer/generated/native-scroll-insets-v36.json)
combines authored-style inheritance, resolved-style snapshots and relative
values with the applied compositor cache repair. Its own local application
verification remains pending. Its named regression and nine inheritance tests
pass, but the harness incorrectly expects eight tests. The
[corrected queue](../renderer/generated/native-scroll-insets-v40.json)
checks the complete named inventory after clearing all 18 workspace packages.
Earlier `5cc75147` results include 50 exact native
relative-style images and eight exact C geometry observations, alongside 60
failing native static-position bounds and images. The new source is unapplied;
those earlier measurements do not establish current-source API completion.

The [native viewport application](../../bindings/rust/openui/examples/native_viewport_scroll.rs)
builds its document through typed Rust methods, scrolls it from a Rust click
callback, injects normalized wheel input, and reads owned geometry and
`ScrollMetrics`. The viewport uses shared layout dimensions to reserve
scrollbar gutters and clamp programmatic, wheel, and smooth scrolling. Pending
layout is resolved before reading offsets or dimensions; shrinking the content
or resizing the viewport also clamps the retained offset.
`Element::scroll_metrics` returns an owned snapshot of client and content
dimensions in logical pixels, or `None` when the element has no layout box.
The [C metrics query](native-scroll-metrics.md) now exposes the same owned
dimensions, with C and C++ consumers and versioned output validation. Native
inset length mutations also work through the shared engine and public Rust
callbacks. The [native style consumers](../../bindings/rust/openui-ffi/README.md#native-style-operations)
also exercise 35 primitive longhands through the public Rust, C, and C++
paths, including lossless alignment modifiers and optional colors. Their
callback, retained-state, raw-frame color, and owned-snapshot checks pass at
five scales. Nested scrolling ranges, native scrollbar input/accessibility,
remaining C property conversion, complete native API coverage, and complete
pixel qualification still require work. Those are native implementation gaps.

Native reveal now has public `Element::scroll_into_view` and
`smooth_scroll_into_view` methods, shared with accessibility and additive C
operations. The [native reveal evidence](native-scroll-metrics.md#native-scroll-into-view)
records 35 Engine tests, ten C/four C++ consumers and 30/30 reduced geometry
states on the private API source. Four of 20 endpoint images still differ.
Native scroll-margin/padding support and broader alignment, writing-mode and
containing-block qualification remain open. Own combined umbrella verification
is pending; the public methods do not complete every native element API.

The [scroll-layer Rust consumer](../renderer/evidence/native-viewport-scroll-v1/native_scroll_layer_opacity.rs)
changes background clipping, opacity, filters and scroll offsets from a Rust
click callback. Its [private renderer evidence](../renderer/generated/native-viewport-full-v17.json)
checks 180 states: all owned bounds, offsets, client dimensions, callback and
teardown checks pass. Scroll extents still include border geometry incorrectly,
and only 95 of the rendered states match Chromium exactly. These remain native
API and renderer gaps; the new states are not admitted release passes.

The [private SVG consumer](../renderer/evidence/native-svg-decoration-v1/native_svg_foreign_object_curved_v234.rs)
creates a viewport for native UI children through a proposed public Rust
constructor and mutates it from a Rust callback. Its
[clean evidence](../renderer/generated/native-svg-viewport-v11.json) checks
1,920 states across four border sides and five scales. Every owned bound,
callback and teardown check passes; shared solid-border painting makes
1,176 rendered states match Chromium exactly, while 744 still differ.
The rebased [C/C++ consumers](../renderer/evidence/native-svg-decoration-v1/native-c-v293/)
create the same viewport and change border, padding and box sizing from native
callbacks. Their bounds, detach/reattach and teardown checks pass at five scales.
The additive tag preserves the 110 exports and existing layouts. The constructor
and shared renderer source are now applied; own combined-source qualification
and native SVG coordinate/transform APIs remain open. These are native
implementation gaps, with no JavaScript execution or script bindings.

The [native flow-root application](../../bindings/rust/openui/examples/native_flow_root_geometry.rs)
constructs a document, queries owned fragments, finds a child under the pointer
and activates it through public Rust methods. A Rust click callback handles
that activation. It exports its geometry and PNG through the public framework;
its [five-scale evidence](../renderer/native-flow-root-leaf-slices.md) matches
Chromium without executing a script.

The [native column-flex app](../../bindings/rust/openui/examples/native_column_flex_geometry.rs)
constructs all states through typed Rust methods. Eight case variants match
Chromium in geometry and pixels at all five scales after the combined
[continuation and empty-bounds corrections](../renderer/native-column-flex-overflow.md#combined-native-checkpoint).
Its default state hits the overflowing child and runs a Rust click callback.
The later [column paint and input correction](../renderer/native-column-paint-phases.md)
makes the following ordinary block agree with Chromium in geometry, pixels,
and pointer targets at all five scales. A consuming Rust app changes its
position and opacity through public methods and activates the overflowing
child through a Rust callback. Row/reversed flex, grid continuations, and a
following inline-block remain needed native behavior.

`Element::client_rects` returns an owned list of border-box fragments in logical
viewport coordinates, including scroll offsets and transforms.
`bounding_rect` returns their combined bounds. These layout queries include
empty, hidden, clipped, and pointer-ineligible boxes; they do not borrow the
input hit-test list. Detached elements and `display: none` elements have no
layout rectangles. The [native geometry evidence](native-element-geometry.md)
records Rust and C consumer checks and the separate Chromium measurements.

Class tokens are native element metadata; styling changes use typed style
setters rather than a parsed CSS class rule.
Native kinds group some tag names, such as `div` and `main`; an application
that needs to distinguish them can retain its element handle or assign an ID.
Kind lookup covers attached authored elements and excludes text and generated
pseudo-elements.
`Element::detach` removes a subtree from presentation while keeping its
generation-checked handles, authored state, and Rust event listeners for later
reattachment. `Element::remove` destroys the subtree and invalidates those
handles. Detached subtrees remain owned by the document until reattached,
removed, or the document is dropped.
`TextNode::detach` does the same for a text node and transfers it to document
ownership so dropping the Rust handle does not destroy a node intended for
later reattachment.
The C ABI exposes the shared engine operation as `oui_element_detach` for both
element and text handles. It leaves the handle valid for `oui_element_append_child`
or `oui_element_insert_before`; `oui_element_remove` still destroys the node.
For example, the effects of looking up an element, activating it, focusing it,
scrolling it, and changing an input value are available through
`Document::element_by_id`, `Element::click`, `focus`, `scroll_to`, and
`set_control_value`. If an application needs another element operation and the
public Rust API cannot perform it, that is a native API gap to implement; it
does not require JavaScript.
`Element::focus` and `Element::blur` deliver native focus and blur callbacks
through the same document event path used by keyboard and accessibility input.
The engine's `FocusOrigin::Script` is a legacy name for programmatic native
focus; calling `Element::focus` does not run a script.
The C ABI's `oui_document_element_by_id` performs the same attached-element
lookup and returns an owned handle that the caller releases with
`oui_element_destroy`.

When a Chromium test uses a script to reach a visual state, we assess that
final state as a rendering case. Separately, we review each element operation
used to reach it: if a consuming application needs the behavior, the public
Rust API must provide it and exercise the same retained document and event
path. Excluding a behavioral test from the pixel matrix does not exclude the
needed native behavior from the product. Pixel equality from a test-only
fixture does not close an application API gap.
The presence of a script in a Chromium test is not, by itself, a reason to
exclude a deterministic visual state from pixel qualification. Historical
test data may call such a case `needs_javascript`; that label describes the
Chromium source file, not Open UI's implementation or a release waiver.
The current deterministic test scripts use ID, class, and tag lookups; the
native equivalents are `element_by_id`, `elements_with_class`, and
`elements_of_kind`. None requires a script engine.
The [native opacity consumer](../../bindings/rust/openui/examples/native_image_opacity.rs)
demonstrates a consuming application calling typed element setters from a
Rust click callback and querying the resulting owned bounds. Its
[renderer investigation](../renderer/native-image-opacity.md) records the
remaining pixel differences separately from that public native API path.

The [native PNG consumer](../../bindings/rust/openui/examples/native_png_sampling.rs)
supplies real image bytes through the public resource API and changes width,
height, and opacity from a Rust click callback. All 540 measured runs on the
current branch pass callback, owned-bounds, and document teardown checks. Its
[pixel investigation](../renderer/native-png-sampling.md) records the remaining
renderer failures; these states are not admitted release passes.

## No JavaScript runtime

Open UI does not run application or test JavaScript, in this or future
versions. It does not provide a JavaScript engine, `eval`, script bindings,
or a plan to add them. Chromium is
the separate test oracle. Offline qualification tooling may read Chromium's
WPT scripts as source data to identify a deterministic final visual state;
Open UI constructs that state with native Rust operations. The script is input
to offline qualification tooling only; no script is shipped to or executed by
Open UI. The separate capture tools may run JavaScript inside Chromium to
prepare its reference images. Application behavior always uses public native
Rust methods and Rust callbacks.

## Deferred

The following are not v0.2 defects or compatibility promises: macOS, Windows,
Android, iOS, Vulkan, Metal, Direct3D, navigation, browser-compatible DOM
names and full Web API coverage, URL fetching, HTML loading, runtime CSS
parsing, file/date/color picker dialogs, media playback, interactive embedded
documents, a visual inspector, and a general plugin ecosystem. Needed native
element operations remain part of the public Rust API contract above.

The frozen WPT inventory contains 1,912 test files with JavaScript in their
Chromium source and 30 nonvisual/crash-harness rows. Test tooling may read a
script to construct a fixed native Rust fixture for its final visual state;
the renderer then compares that state with Chromium at all four required
profiles. The original 5,731-case inventory
is immutable, but old Open UI screenshots are historical evidence, not
expected pixels.

## Build and release policy

- Rust MSRV: 1.85 until a release manifest explicitly raises it.
- Public dependencies use stable, lockfile-pinned releases; prereleases are
  rejected by the v0.2 contract verifier.
- Release builds do not require a Chromium checkout or resource pack.
- Every release carries generated API/reference metadata, native ABI layout
  metadata, an exported-symbol allowlist, checksums, licenses, an SBOM, and
  provenance.
- Chromium oracle captures and their input identities are immutable. Renderer
  changes qualify only against the pinned Chromium pixels; historical Open UI
  outputs are retained for provenance.
