# Native Rust raster options

Open UI applications call native Rust methods and handle events with Rust
callbacks. Open UI never executes JavaScript. Rendering options belong to the
same retained Engine used by those application APIs.

## Public API

| Task | Rust API |
|---|---|
| Create a document with explicit Engine options | `Document::with_options(viewport, options)` |
| Supply fonts and explicit options | `Document::with_font_collection_and_options(viewport, fonts, options)` |
| Read an owned copy of the immutable raster selection | `Document::raster_configuration()` |
| Configure a windowed app before construction | `App::builder().engine_options(options)` |
| Configure a headless app | `HeadlessApp::with_options(viewport, options)` |

```rust
use openui::{Document, EngineOptions, RasterConfiguration, ViewportMetrics};

let document = Document::with_options(
    ViewportMetrics::from_logical_size(800.0, 600.0, 1.25)?,
    EngineOptions {
        raster_configuration: RasterConfiguration::chromium_linux_lcd(),
    },
)?;
```

Options are copied into the shared Engine during construction. They stay
immutable across element mutations, document clones, and viewport changes.
Reading the configuration returns an owned copy. Existing constructors keep
their defaults.

`AppBuilder::backend` selects window presentation. `engine_options` selects
the document's raster configuration. OpenGL presentation currently uploads a
CPU frame. Selecting Ganesh explicitly does not make the software compositor
fall back to CPU; software bitmap and PNG methods return an error for that
selection. Direct Ganesh rendering remains unqualified.

## Executed verification

The [source-identified record](../renderer/generated/native-rust-options-v1.json)
at clean `27a47ba2` verifies four public API guards, 8,555 workspace tests with
zero failures and 13 ignored, fifteen read-only checks, and all seven hosted
hardening jobs with zero skips. The
[consuming Rust app](../../bindings/rust/openui/examples/native_raster_options.rs)
changes element state from a Rust click callback and verifies immutable
configuration, owned bounds, repeated rendering and document teardown.
All ten images and bounds agree exactly with pinned Chromium at five scales.
Twenty independent Chromium processes produce forty stable captures; the
native app also runs twice.

The code is integrated into umbrella source `2d338d6c` alongside the native C
keyword constructors. Its [clean combined verification](../renderer/generated/native-rust-options-v2.json)
passes all fifteen build stages, 8,558 workspace tests with zero failures and
13 ignored, and fifteen read-only checks. All four option guards execute
successfully. Rust/C/C++ callback consumers pass, and the ABI checker runs
thirteen C and seven C++ examples with 113 exports and 30 layouts preserved.
Each of the two native Rust consumers passes ten exact Chromium images and
bounds at five scales. Four native runs and forty independent Chromium
processes produce eighty stable captures.

The first combined captures fail before Chromium exposes its endpoint. A
fresh retry with a shorter native Linux temporary path succeeds; the failed
probes and a retry-owner preflight failure remain in the evidence. No source,
reference bytes or visual capture conditions change for that retry. The
startup diagnostic's error output does not prove a more specific cause.
Earlier source measurements retain their actual identities.

The [completed umbrella checks](../renderer/generated/native-rust-options-v3.json)
at clean `16187f4f` pass all fifteen read-only checks and all four hosted
workflows: thirteen jobs pass, five are skipped, and none fails. All seven
full hardening jobs pass. The hosted parity job executes all four public Rust
option guards and both native FFI keyword guards successfully. Every executed
job's log is retained. Skipped jobs do not count as release passes, and these
checks do not qualify the complete pixel matrices or every configuration
field's rendering effects.

## Remaining qualification

Passing these native API checks does not qualify every raster setting or the
complete renderer. Authored and embedded LCD phase behavior, default native
font pixels, all configuration fields' rendering effects, and complete
combined-source matrices remain open. See the
[field review](native-c-raster-configuration.md#consumer-corrections-and-remaining-field-behavior)
and [renderer contract](../renderer/contract.md).

The [private Fontations draft](../renderer/generated/native-font-choice-v1.json)
at `ef8880b0` prepares an explicit native Rust outline choice and its paint
data flow. Fifteen read-only checks pass, but it has not compiled or run a
consuming app or pixel matrix. It is unapplied and inherits the rejected glyph
candidate. The constructor is not available on the umbrella branch; it does
not change the supported API or qualify default native text.

Pinned Chromium defines expected pixels, with zero tolerance. Historical
Open UI screenshots preserve provenance. No new release state is admitted by
this API checkpoint.
