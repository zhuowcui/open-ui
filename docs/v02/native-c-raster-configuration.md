# Prepared native C raster configuration

Open UI runs no JavaScript. Rust applications use public methods and Rust
callbacks over the retained engine. The C ABI provides equivalent operations
over that same engine; it does not have a separate renderer.

## Prepared implementation

Clean private source `3395cefa` follows the unqualified native font candidate
`0601cd30`. The [versioned evidence](../renderer/generated/native-scroll-insets-v31.json)
preserves its [reviewable patch](../renderer/evidence/native-c-raster-v1/native-c-raster-v1.patch),
source, generated header and verification programs. Ten read-only repository
checks pass; Rust and C formatting checks pass. No local compilation or native
runtime verification is claimed.

This private candidate adds four versioned functions and two owned value
structures. It has not yet been compiled or exercised by consuming
applications. The umbrella branch still has 113 exports. This candidate has
117, preserving every preceding export and all 30 existing layouts.

| Operation | C API |
|---|---|
| Initialize an explicit raster preset | `oui_raster_configuration_init_v1` |
| Create a retained document with an immutable policy | `oui_document_create_with_raster_configuration_v1` |
| Create an app with that retained document | `oui_app_create_with_raster_configuration_v1` |
| Read the selected policy as an owned value | `oui_document_get_raster_configuration_v1` |

`OuiRasterConfigurationV1` is 72 bytes, aligned to four bytes. Its first two
fields are `struct_size` and `abi_version`. It contains the backend, pixel
geometry, gamma and contrast, plus three 16-byte text policies for authored,
native and embedded text. The API accepts the same scalar ranges and enum
values as the Rust configuration. Text flags select subpixel positioning and
autohint; LCD phase uses signed 1/64 physical pixel units.

Initialize a writable output's size and ABI fields before calling init or
query. Creation copies all configuration fields into the shared Rust engine;
the caller can immediately reuse its input. Query returns a value with no
pointers, release operation or surviving engine borrow. Later tree mutation,
native callbacks, viewport resize and owned scenes preserve the selection.
Larger caller structures keep their trailing bytes. Short prefixes and
invalid fields fail before borrowing a complete caller structure, and leave
outputs unchanged. Callers must supply storage matching their declared size.

Document and app handles retain their existing ownership and thread rules.
Callbacks may query and mutate on the owner thread after engine borrows have
been released. Existing constructors retain their existing Rust default.
Application presentation preference and document raster selection are
separate choices. Selecting Ganesh does not initialize a GPU context, and
CPU document rendering rejects it; C GPU operation still needs implementation
and qualification.

## Required verification

The prepared [C consumer](../renderer/evidence/native-c-raster-v1/raster_configuration.c)
and [C++ build](../renderer/evidence/native-c-raster-v1/raster_configuration.cc)
each cover five CPU presets at five scales, with
click callbacks, reused configuration storage, viewport resize and owned RGBA
frames that remain readable after document teardown. Optional font mode loads
real font bytes through the public C API and creates the same 64 logical text
positions, before and after a callback, as the public Rust consumer.

The planned font matrix covers four families, five sizes, five scales and
both explicit FreeType and Fontations policies. Two native processes per
language cover 1,200 Rust/C/C++ images and 76,800 logical phase states. These
are new diagnostics, not original WPT passes. C and C++ must be compared
directly with the unchanged pinned Chromium references. Equality with Rust
alone is not a Chromium pass. Repeated native processes must be deterministic,
and every admitted image must be exact with zero tolerance.

Rust boundary tests exercise short allocations, unaligned complete storage,
untouched tails and failure outputs, thread ownership, reentrancy, document
lifetime and the shared native engine. The short-prefix test is also added to
the existing Miri hardening job with strict provenance enabled.

Compilation, consuming-application execution, Miri, exact font pixels,
complete original and expanded censuses, all configuration fields' rendering
effects, default native rendering, and release-lab operation remain open.
Preset selection alone does not establish Chromium equality. No new release
state is admitted by this prepared implementation.

The queued pipeline waits for all commands in the earlier cache, fieldset,
font and clean-cache pipelines to finish before it builds or captures images.
Its thirteen build stages include the five Rust boundary tests, the Linux
workspace, native examples and all twelve C/six C++ ABI consumers. The new
Miri prefix test and the complete focused, primitive, original and expanded
pixel suites remain pending. The accepted renderer remains 21,334/22,924
exact, with 1,590 differences and zero errors; the full pixel gate fails.
