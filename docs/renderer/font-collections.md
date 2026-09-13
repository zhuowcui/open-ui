# Font collections

Every Open UI document owns an explicit `FontCollection`. Production
collections search application faces first, installed system faces second,
and locale-aware system fallback last. Vendored parity fonts are available
only through `FontCollection::deterministic_test()`; creating a production
document never installs or overrides a family globally.

Rust applications register immutable `Arc<[u8]>` data with a validated
`FontFaceDescriptor`. Descriptors include the family, collection face index,
style/weight/stretch ranges, Unicode ranges, OpenType defaults, size
adjustment, and metric overrides. A stable `FontFaceHandle` supports queries
and removal. Registry changes advance the collection generation, clear its
bounded font-instance cache, and invalidate intrinsic layout through the
engine. Submitted scenes retain immutable face bytes independently of later
registry removal.

TTF, OTF, TTC/OTC, WOFF, and WOFF2 signatures are accepted. Parsing is done
by the pinned Skia/FreeType build with WOFF2 enabled; malformed data and bad
collection indices are reported as errors. Per-face input is limited to 64
MiB, a collection to 256 MiB and 512 faces, and the resolved-instance cache to
256 entries.

The C ABI exposes the same lifecycle through
`oui_document_register_font()`, `oui_font_face_get_info()`, the three
length-delimited metadata copy functions, `oui_font_face_unregister()`, and
`oui_font_face_destroy()`. Call a copy function with a null destination and
zero capacity to query its required length. Font bytes and descriptor arrays
are copied during registration, so callers retain no borrowed storage after
the function returns.
