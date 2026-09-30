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

All application behavior runs through native APIs; Open UI never runs
JavaScript. The public `openui` crate exposes
retained `Document` and `Element` handles; the application may keep those
handles and call native methods from Rust callbacks. C applications use the
versioned C ABI over the same engine. There is no JavaScript execution,
`eval`, script binding, or embedded browser runtime in the application path.
Element APIs can be called directly from Rust; they do not require JavaScript.
Every browser-style element operation needed by a consuming native application
must be exposed as a public Rust method. Lookup, mutation, geometry, focus,
scrolling, controls, and event dispatch operate directly on the native engine:

| Application task | Public Rust API |
|---|---|
| Create, find, clone, move, detach, or destroy elements | `Element::create`, `Document::element_by_id`, `clone_subtree`, `append_child`, `insert_before`, `detach`, `remove` |
| Create, read, edit, attach, move, detach, or destroy text nodes | `Document::create_text_node`, `TextNode::data`, `set_data`, `detach`, `remove`, `Element::append_text_child`, `insert_text_before` |
| Find elements by native kind or class | `Document::elements_of_kind`, `elements_with_class`, `Element::kind` |
| Change class tokens | `Element::has_class`, `add_class`, `remove_class` |
| Read or change text, attributes, or typed style | `Element::text_content`, `set_text`, `set_attribute`, `set_property` and generated typed setters |
| Read resolved style | `Element::computed_style`, which returns an owned snapshot |
| Handle input or activate an element | `Element::on`, `on_capture`, `click`; Rust callbacks in `view!` |
| Focus, scroll, or inspect geometry | `focus`, `blur`, `scroll_to`, `scroll_by`, `bounding_rect`; `Document::hit_test` |
| Update form controls or details | `set_control_value`, `set_selection`, `set_checked`, `set_open` |
| Inject or cancel native IME input | `Document::dispatch_composition_start`, `dispatch_composition_update`, `dispatch_composition_end`, `dispatch_composition_cancel` |

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

## No JavaScript runtime

Open UI does not run application or test JavaScript and does not provide a
JavaScript engine, `eval`, script bindings, or a plan to add them. Chromium is
the separate test oracle. Offline qualification tooling may read Chromium's
WPT scripts as source data to identify a deterministic final visual state;
Open UI constructs that state with native Rust operations. The script is input
to offline qualification tooling only; no script is shipped to or executed by
Open UI. Needed element behaviors become native Rust APIs, not script bindings.

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
