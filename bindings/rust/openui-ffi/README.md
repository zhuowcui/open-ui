# Open UI C ABI v0.2

`openui-ffi` is the native `staticlib`/`cdylib` surface over the same retained
Rust engine used by the safe `openui` crate. It has no dependency on the old
Blink application backend.

The checked-in public contract is generated from the canonical style and event
schemas:

- `include/openui.h`
- `include/openui_style_properties.h`
- `docs/v02/generated/openui-ffi-symbols.txt`
- `docs/v02/generated/openui-ffi-layout.json`
- `docs/v02/generated/openui-ffi-checksum.txt`

Regenerate with `python3 tools/ffi/generate_ffi.py`, or verify without writing
with `python3 tools/ffi/generate_ffi.py --check`. After building the crate,
`python3 tools/ffi/verify_abi.py` checks exact exports, compiles C and C++
consumers, and runs twelve headless C examples and six C++ consumers.

All opaque handles are generation-checked and thread-affine. Strings are
length-delimited UTF-8. Every status failure records a thread-local structured
error. Event callbacks are synchronous, may reenter the API on the owning
thread, and remain registered until their `OuiListener` is destroyed.
`oui_document_element_by_id` finds an attached element in document order and
returns an owned handle, or null on a successful miss. The caller destroys a
found handle with `oui_element_destroy`; detaching the node excludes it from
later lookups while preserving its existing handles for reattachment.
The owned `OuiAccessibilitySnapshot` API exposes node metadata, ordered
relations, focus, and changed/removed IDs without retaining engine borrows.
Snapshots remain readable after document destruction on their owning thread.

## Immutable raster configuration

`oui_document_create_with_raster_configuration_v1` and
`oui_app_create_with_raster_configuration_v1` copy the same immutable
`RasterConfiguration` used by public Rust `Document` and `EngineOptions`.
Initialize `OuiRasterConfigurationV1.struct_size` and `abi_version`, then
call `oui_raster_configuration_init_v1` with a named preset or provide the
typed fields directly. `oui_document_get_raster_configuration_v1` returns an
owned copy. The configuration needs no release operation and the input
storage may be reused immediately after construction. Element mutations,
callbacks and viewport changes keep the selected configuration.

The configuration covers the raster backend, pixel geometry, gamma, contrast
and separate authored, native and embedded text policies. Unknown enum values
and flag bits, out-of-range scalar values, short headers and incompatible ABI
versions fail without changing outputs. Larger caller structures keep their
trailing bytes. Existing constructors still use the existing Rust default;
all preceding 113 exports and struct layouts are unchanged.

App presentation preference remains separate from document raster selection.
Selecting Ganesh copies its policy but does not initialize a GPU context;
CPU document rendering rejects that selection. Ganesh release qualification
and complete C GPU operation remain open.

The [C consumer](../../../examples/c_v02/raster_configuration.c) and its
[C++ build](../../../examples/c_v02/raster_configuration.cc) exercise native
callbacks, input ownership, resize and retained frames at five scales. Their
optional font mode registers supplied font bytes and compares the same
64 logical text positions used by the public Rust app against Chromium.
This private candidate is prepared, not compiled or pixel-qualified. See the
[configuration contract](../../../docs/v02/native-c-raster-configuration.md).

## Native element reveal

`oui_element_scroll_into_view_v1` and
`oui_element_smooth_scroll_into_view_v1` use the same Engine reveal plan as
public Rust methods and accessibility. `OuiScrollAlignment` selects start,
center, end or nearest on each logical axis; `OuiScrollIntoViewContainer`
selects all or the nearest enclosing scrollport. These constants are passed
as `uint32_t` values. Smooth duration must be finite and non-negative.
Invalid values return `OUI_ERROR_INVALID_ARGUMENT` before any Engine mutation.

The [C consumer](../../../examples/c_v02/scroll_into_view.c) exercises native
callbacks, nested hidden/clip scrolling, instant and smooth endpoints, owned
geometry, invalid arguments and document teardown. Existing exports and
struct layouts remain intact. Scoped private C/C++ consumers pass; complete
API and own combined-source qualification remains open. See the
[native reveal contract](../../../docs/v02/native-scroll-metrics.md#native-scroll-into-view).

## Native style operations

`oui_style_value_color_f32_v1` constructs a native sRGB color from normalized,
unpremultiplied float channels for every color longhand, including text decoration
and emphasis. It preserves `Color::from_rgba_f32` precision through the shared
Rust engine. The returned property-bound compound is owned by the creating
thread; set the property, then release the compound. The element keeps its own
copy. Invalid channels or properties leave the output unchanged. Existing
`OuiColor` byte channels and all struct layouts are preserved. The
[`float_colors.c`](../../../examples/c_v02/float_colors.c) and C++ consumers
exercise all twelve properties, callback mutation, rendering and owned buffers
at five scales. The constructor and consumers are applied to the umbrella
branch. The [v16 evidence](../../../docs/renderer/generated/native-scroll-insets-v16.json)
records its own six-stage clean build, 8,528 workspace tests, five Rust runs,
eleven C and five C++ consumers. Callback mutations and owned snapshots and
frames survive teardown. All preceding 112 exports and layouts remain intact.
The C formatting correction recompiles and runs the same consumers against
the unchanged Rust library; all three hosted workflows pass, with five
hardening jobs skipped. Complete native API and renderer qualification remains open.
Existing calls and struct layouts remain compatible.

Native apps mutate retained style through public Rust setters or
`oui_element_set_property`. The shared engine now accepts author values for
35 additional primitive longhands, including independent border colors and
widths, alignment, filters, column dimensions, and optional scrollbar colors.
Relative lengths remain authored values until the engine resolves them.

`oui_style_value_parse` parses one native property value. It preserves
`currentcolor`, optional `auto` colors, safe/unsafe alignment modifiers, and
unsigned orphan/widow counts that exceed the scalar C integer range. These
values return an owned `OUI_STYLE_VALUE_COMPOUND` bound to the selected property.
Submit it to that property, then release it with `oui_style_compound_destroy`;
the element retains its own copy. Using the carrier for another property,
from another thread, or after release fails without mutating retained style.
Existing scalar encodings, exports, and struct layouts are unchanged.

The [Rust consumer](../openui/tests/native_primitive_styles.rs),
[C consumer](../../../examples/c_v02/primitive_styles.c), and
[C++ consumer](../../../examples/c_v02/primitive_styles.cc) exercise all 35
longhands at five scales, with callback mutations, owned snapshots, rendering,
and document teardown. This does not complete every style or element API.
Open UI never executes JavaScript; needed application behavior is implemented
in the shared engine and exposed through public native Rust methods.

Rust `Bitmap::pixels` and C `oui_document_render_rgba` return owned,
top-to-bottom RGBA8888 rows with premultiplied color channels. Shared CPU
readback now requests that format explicitly; platform-native Skia N32 bytes
previously swapped red and blue in these consumers and Linux presentation.
The PNG encoding path is separate. Full renderer and release-lab qualification
remain open.

## Native scroll dimensions

`oui_element_get_scroll_metrics_v1` exposes the same shared engine query as
Rust `Element::scroll_metrics`. Initialize `OuiScrollMetricsV1.struct_size`
and `abi_version`, then pass writable, nonoverlapping metrics and presence
outputs. The call resolves pending layout and copies client and content
dimensions in logical pixels. An element without a layout box returns
`OUI_OK`, a zero presence flag, and four zero dimensions. Errors leave both
outputs unchanged. Larger caller structures keep their trailing bytes.

The copy needs no release operation and remains valid after mutations or
document destruction. Call the query on the document's owning thread.
`oui_element_get_scroll_offset` also resolves pending layout, so a query after
content shrink or viewport resize returns the clamped retained offset.
`OUI_STYLE_PROPERTY_OVERFLOW_X` and `OVERFLOW_Y` accept the existing
`OuiOverflow` enum values independently, including values returned by
`oui_style_value_parse`.
The [C consumer](../../../examples/c_v02/scroll_metrics.c) and its C++ build
exercise these operations directly through the public native API at five
scales. Nested scrolling, scrollbar pointer/keyboard input, and accessibility
qualification remain separate work.

## Native SVG viewport

`oui_element_create(document, OUI_ELEMENT_SVG_FOREIGN_OBJECT, &element)` calls
the shared Rust foreignObject viewport constructor. Width and height describe
the fixed viewport, including border and padding, regardless of box sizing.
The native container uses ordinary element handles, styles, events and document
ownership; place it under a native `OUI_ELEMENT_SVG` parent for SVG content.
The new element-tag value is appended after the existing 0–38 values. All
110 exports, struct layouts and the ABI version remain unchanged; the generated
header checksum records the additive enum value.

The [C consumer](../../../examples/c_v02/svg_viewport.c) and its
[C++ build](../../../examples/c_v02/svg_viewport.cc) exercise viewport bounds,
border/padding and box-sizing mutation from a native click callback,
detach/reattach and document teardown at five scales. An ordinary native block
also changes from content-box to border-box sizing through the same callback;
the SVG viewport keeps its fixed dimensions. Both sizing literals use the
shared Rust style conversion and an owned C compound value. This implementation is
pending consuming-application verification and complete renderer qualification;
SVG coordinate/transform APIs and release-lab operation remain open. It does
not execute JavaScript.

## Native keyboard and text input

`oui_document_dispatch_key_input_v1` sends a logical key and separately
committed UTF-8 text through the same native Rust `Document` defaults as Linux
input. Initialize an existing `OuiEvent` with its size, ABI version, key-down
or key-up type, zero flags, key code, modifiers, and logical key name in `text`.
Pass committed text separately. Key-up, Control/Meta shortcuts, and platform
control characters do not insert that text. Enter in a textarea inserts one
newline through the cancelable default action.

`oui_document_dispatch_text_input_v1` injects committed text directly through
cancelable `beforeinput`; a successful edit emits one `input` event after the
value changes. Read-only and disabled controls reject user edits. Both calls
use the focused retained control, run synchronously on the document's owning
thread, and permit reentrant callbacks. They are available in both headless
and Linux builds. Application interaction uses native APIs; Open UI executes
no JavaScript.

## Native Linux windows

Build with `cargo build --locked -p openui-ffi --features linux`. The packaged
Linux SDK enables this feature. A default build keeps the headless ABI and
returns `OUI_ERROR_INVALID_STATE` from `oui_app_run`.

`oui_app_run(app, &config)` blocks in the same Linux event loop and Rust `App`
used by native Rust applications. It uses the app's existing retained document,
including C elements and listeners. Native controls, keyboard input, clipboard,
IME, and accessibility actions enter that shared Rust document path.
`oui_app_request_exit` ends the run from a callback on the main/UI thread.

Initialize `OuiAppRunConfig.struct_size`, `abi_version`, and zero reserved fields.
Its optional callback receives a versioned `OuiPlatformEvent` after document
handling and after engine and presentation borrows have ended. It can mutate
elements, render a snapshot, or request exit. Callback code and `user_data`
must remain valid until `oui_app_run` returns; event, text, and native path
bytes are borrowed only for the callback and must be copied to keep them.
Callbacks must not unwind. Full-width pointer IDs are available in the platform
event; the existing `OuiEvent.pointer_id` retains its 32-bit layout.

Call `oui_app_run` on the process's main/UI thread. A run attempt consumes that
app's native lifecycle; repeated attempts return `OUI_ERROR_INVALID_STATE`,
nested runs return `OUI_ERROR_REENTRANT`, and app destruction during a run is
rejected. Owned document handles remain valid after the run and app destruction.

The [C](../../../examples/c_v02/native/window.c) and
[C++](../../../examples/c_v02/native/window.cc) window consumers verify
callback mutation, a second presentation, and exit. With a Linux library and
an available display, run `python3 tools/ffi/verify_native_window.py --protocol
x11 --backend software` or `--protocol wayland --backend software`; X11 also
supports `--backend opengl`. `--report` writes source and binary identities
with both consumers' results. Hosted checks exercise X11 software/Mesa GL
and pure Wayland software; release-lab AT-SPI operation, physical GPUs, context
loss, and packaged application qualification remain separate gates.
