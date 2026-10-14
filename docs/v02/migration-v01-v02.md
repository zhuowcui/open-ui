# Migrating from Open UI v0.1 to v0.2

Version 0.2 is an intentional API break. It replaces the Blink-backed
application layer and stringly style mutation with one retained pure-Rust
engine. No permanent compatibility shim is provided.

The generated, exhaustive accountability table is
[`generated/migration-ledger.csv`](generated/migration-ledger.csv). It maps
every inventoried Rust and C symbol to a v0.2 replacement or removal.

## Rust applications

Application behavior remains native Rust. Use the public `Document` and
`Element` methods for element creation, updates, events, focus, scrolling, and
controls; retain element handles rather than relying on a browser DOM or a
JavaScript runtime. See the [native interaction API](supported-platforms.md#native-interaction-api).

Replace infallible, frame-count-oriented setup:

```rust,ignore
let mut app = App::new(800, 600);
app.render(view);
app.run_frames(1).render_to_png("output.png");
```

with the fallible native or headless lifecycle:

```rust,no_run
use openui::prelude::*;

# fn view() -> ViewNode { view! { <div style:display="block">"hello"</div> } }
# fn main() -> Result<(), Error> {
let app = App::builder()
    .title("Application")
    .size(LogicalSize::new(800.0, 600.0))
    .backend(BackendPreference::Auto)
    .build()?;
app.run(view)?;
# Ok(())
# }
```

For deterministic rendering:

```rust,no_run
use openui::prelude::*;
# fn view() -> ViewNode { view! { <div>"hello"</div> } }
# fn main() -> Result<(), Error> {
let viewport = ViewportMetrics::from_logical_size(800.0, 600.0, 1.0)?;
let mut app = HeadlessApp::new(viewport)?;
app.mount(view)?;
app.render_png_to(250.0, "frame.png")?;
# Ok(())
# }
```

Key changes:

| v0.1 | v0.2 |
|---|---|
| FFI-backed Rust `Document` | direct safe wrapper over `openui-engine` |
| `App::new` | `App::builder().build()?` or `HeadlessApp::new` |
| `run_frames` | owned `App::run` or deterministic `render_at` |
| `load_html` | typed `view!` or explicit typed node creation |
| generic `set_style(name, value)` | property-specific typed setters |
| injected stylesheet/CSS text | typed `Style`, transitions, and keyframes |
| resource-pack initialization | synchronous document-owned resources |
| implicit failure/panic | explicit `Result` |

Static `view!` style literals remain concise, but the macro parses and validates
them at compile time. Dynamic expressions must produce the exact property type.

```rust,compile_fail
# use openui::prelude::*;
let value = "0.5";
let _ = view! { <div style:opacity={value}></div> };
```

Use a number for opacity, a `Length` for dynamic width, a `Color` for dynamic
color, and the typed compound builders for transforms, borders, gradients,
transitions, and keyframes.

## C applications

All configuration and event structures start with `struct_size` and
`abi_version`. Initialize both fields and zero any unused tail fields.

```c
OuiDocumentConfig config = {
    .struct_size = sizeof(OuiDocumentConfig),
    .abi_version = OUI_ABI_VERSION,
    .viewport = {
        .logical_width = 800.0,
        .logical_height = 600.0,
        .physical_width = 800,
        .physical_height = 600,
        .device_scale_factor = 1.0,
        .authority = OUI_VIEWPORT_LOGICAL,
    },
};
OuiDocument* document = NULL;
OuiStatus status = oui_document_create(&config, &document);
```

Replace `oui_element_set_style(element, name, value)` with a generated property
identifier and matching tagged value:

```c
OuiStyleValue width = {
    .tag = OUI_STYLE_VALUE_LENGTH,
    .data.length = {.value = 320.0f, .unit = OUI_LENGTH_PX},
};
oui_element_set_property(element, OUI_STYLE_PROPERTY_WIDTH, &width);
```

Replace unbounded strings with `OuiUtf8 { data, length }`. Compound arrays are
copied into immutable handles by their builders; destroy each handle after the
engine has retained the value. Check every `OuiStatus`. On failure,
`oui_error_get_last` and `oui_error_message_copy` expose structured details for
the current thread.

Updates that make several mutations should use
`oui_document_begin_update`/`oui_document_end_update`. Handles belong to their
creating document and thread. Destroyed or removed nodes, cross-document
arguments, reentrant destruction, invalid enum values, and mismatched style
tags return errors rather than invoking undefined behavior.

## Removed without replacement

- HTML document loading and runtime CSS text parsing.
- Stylesheet injection and generic property-name mutation.
- Chromium resource-pack initialization and Blink application handles.
- JavaScript execution, URL fetching, and browser navigation.
- The old GN-produced default C/C++ application library.

If an application needs HTML/CSS ingestion, parse it outside Open UI and lower
only supported values into the typed tree. That adapter is application code,
not part of the v0.2 compatibility contract.
