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
consumers, and runs seven headless C examples and two C++ consumers.

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
The [C consumer](../../../examples/c_v02/scroll_metrics.c) and its C++ build
exercise these operations directly through the public native API at five
scales. Nested scrolling, scrollbar pointer/keyboard input, and accessibility
qualification remain separate work.

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
