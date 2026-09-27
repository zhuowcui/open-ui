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
The owned `OuiAccessibilitySnapshot` API exposes node metadata, ordered
relations, focus, and changed/removed IDs without retaining engine borrows.
Snapshots remain readable after document destruction on their owning thread.
