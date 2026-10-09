# Native text selection and element queries

Open UI never runs JavaScript. A consuming application changes its retained
document through Rust methods and handles interaction with Rust callbacks.
The C facade calls the same Rust document and event pipeline. Chromium is the
separate reference used to check behavior and pixels.

## Native operations

| Application operation | Rust API | C API |
|---|---|---|
| Read selection direction | `Element::selection_direction` | `oui_element_get_selection_direction_v1` |
| Set a directional text range | `Element::set_selection_range` | `oui_element_set_selection_range_v1` |
| Replace part of a control value | `Element::replace_control_range` | `oui_element_replace_control_range_v1` |
| Read an authored attribute | `Element::get_attribute` | `oui_element_get_attribute_v1` |
| Check attachment to the document | `Element::is_connected` | `oui_element_is_connected_v1` |
| Compare retained node identity | `Element::is_same_node` | `oui_element_is_same_node_v1` |
| Deliver pending native selection events | `Document::dispatch_pending_events` | `oui_document_dispatch_pending_events_v1` |
| Read editing intent and nullable text | `Event::input_info` | `oui_event_input_info_v1` |
| Read immutable event properties | `Event::bubbles`, `Event::cancelable` | `oui_event_properties_v1` |

`SelectionDirection` provides `None`, `Forward` and `Backward`. The declared
Linux/headless behavior normalizes `None` to `Forward`. Range positions use
UTF-8 byte offsets at Unicode scalar boundaries. A position inside a scalar
is rejected before mutation. The existing grapheme selection API retains its
existing contract. Browser UTF-16 positions that split a surrogate pair have
no equivalent native UTF-8 scalar position. Those reference cases remain
unqualified, with the indexing compatibility gap recorded explicitly.

`RangeSelectionMode` provides `Preserve`, `Select`, `Start` and `End`.
Programmatic replacement does not emit `input` or `change`. It may schedule
`selectionchange` and `select`. Setting an identical value preserves the
selection and direction; changing the value schedules `selectionchange`.
Readonly and disabled controls permit these programmatic changes. User editing
commands retain their separate editability checks.

## Callback delivery

Selection state changes immediately. Notifications arrive on a later native
task turn. Linux applications pump these tasks through the shared event loop.
Headless applications call `dispatch_pending_events` and use
`has_pending_events` to decide whether another turn is needed.

Delivery coalesces `selectionchange` separately for each target until that
target's notification is delivered. `select` uses a separate captured batch.
Callback mutations can schedule another turn. Recursive pumping does not
recursively deliver callbacks, and callbacks run after engine and listener
borrows are released. Detached live elements retain their notifications;
destroyed elements cannot receive them.

Disabling a focused control uses the shared Rust focus pipeline. Callbacks may
reenable the control or redirect focus. C observes the resulting attributes,
focus, related targets and events through the same retained state.

## C ownership

Attribute values are independently owned UTF-8 buffers. A present empty
attribute returns a buffer; an absent attribute returns null. Release a
returned buffer with `oui_buffer_destroy`. Its bytes survive element mutation
and document destruction. Node identity compares nodes, including independent
handles to the same node; it does not compare handle addresses.

Event property and input metadata queries require the event pointer supplied
to an active native callback on its owning thread. Nested callbacks may query
an outer active event. Borrowed event pointers must not be retained. Input
data returned in a snapshot is owned and can outlive the callback. Versioned
outputs preserve trailing storage and remain unchanged on error.

## Verification status

The public implementation at `6565488e` compiles and passes 196 targeted Rust
unit tests and thirteen integration guards. All six native Rust apps repeat
unchanged. Its C++ consumer check fails because two new test applications use
integer event arrays where the header requires `OuiEventType`; those arrays
are corrected in the next checkpoint. Combined verification remains pending.
The previous private implementation
matches 98 Rust scenarios and 136 C/C++ common-field scenarios, with repeated
runs unchanged. That private result does not qualify the new event property
and attribute queries or the expanded C Unicode and focus checks.

Fresh combined verification must compare 109 Rust scenarios and 109 scenarios
in each of C and C++ against immutable Chromium observations, exercise ownership
and callback guards, preserve existing exports and layouts, and complete the
workspace and renderer gates. Composition lifecycle behavior, other required
native APIs, exact full-corpus pixels and final release qualification remain
open. Existing explicit C composition dispatch also needs verification and
implementation through the shared Rust lifecycle. A script in a Chromium test
never waives a needed native operation.
