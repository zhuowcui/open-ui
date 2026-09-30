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
consumers, and runs the five C examples.

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
