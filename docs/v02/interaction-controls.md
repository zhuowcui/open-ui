# Interaction and core controls

Open UI v0.2 routes all headless, Rust-framework, C-ABI, and Linux input through
the retained engine. Hit testing walks painted fragments back-to-front and
accounts for affine transforms, scroll offsets, overflow and rounded clips,
stacking order, visibility, and typed `pointer-events` eligibility.

Events use capture, target, and bubble phases. Default actions run only after
dispatch and may be canceled. Pointer state supplies enter/leave synthesis,
active state, pointer capture, and click activation. Focus state tracks pointer,
keyboard, native API, and accessibility origins. It supports tab order,
focus-visible, modal containment, and restoration when a modal closes.

The engine owns control state for buttons, single-line and multiline text
inputs, checkboxes, named radio groups, selects and options, ranges,
details/summary, and scroll containers. Editing operates on UTF-8 byte ranges
at Unicode grapheme or word boundaries and includes selection, clipboard
commands, undo/redo, password masking, placeholders, and IME composition
ranges. Headless event injection is the reference behavior used by platform
adapters.

Applications receive and handle these events through native Rust methods and
callbacks; Open UI runs no JavaScript. The public `Document` methods
`dispatch_composition_start`, `dispatch_composition_update`,
`dispatch_composition_end`, and `dispatch_composition_cancel` operate on the
same retained text control as the Linux input adapter. Start an edit before
updating its preview. An empty preview clears the visible preedit text while
keeping the edit active, including when Winit clears preedit immediately before
commit. The final committed text replaces the preview, even when the two
strings differ. The committed edit is one undo step.

Cancellation restores the original value and selection and preserves existing
undo/redo history. Passing an empty string to `dispatch_composition_end` also
cancels. A native `beforeinput` callback may cancel the final commit. Successful
commit delivers `compositionend` followed by one `input` event after updating
the retained value; cancellation delivers an empty `compositionend` without
`input`. Callbacks run without engine borrows and may inspect or mutate the
document. An edit stays with its original text control: changing element focus
or losing native window focus cancels it, and later updates cannot insert text
into a newly focused element. Noneditable or disabled controls ignore text
composition. IME disable also clears the preview and requests presentation.

Native keyboard and text input use `Document::dispatch_key_event` and
`Document::dispatch_text_input`. A consuming Rust app can also call
`Document::dispatch_key_input` in either a headless or Linux build. It accepts
the logical key name separately from committed text and uses the same native
default actions as the Linux adapter. Key-up and Control/Meta shortcuts do not
insert committed text, and platform control characters are filtered.
Enter in a textarea replaces the current
selection with a newline after cancelable `keydown` and `beforeinput`
callbacks. Enter and Space on text controls do not synthesize a click.
Committed characters use the same text input path: `beforeinput` runs before
the edit, and exactly one `input` notification follows a successful edit.
The Linux adapter uses this path and filters platform control characters so
Enter does not insert a second newline.

The C ABI exposes this same path through
`oui_document_dispatch_key_input_v1` and
`oui_document_dispatch_text_input_v1`. The key call consumes a versioned
`OuiEvent` descriptor for key-down or key-up, with zero flags and the logical
key in `text`; committed text is a separate UTF-8 argument. Both calls are
synchronous on the document's owning thread. Native callbacks can cancel
`keydown` or `beforeinput` and inspect or mutate the retained control without
an engine borrow. The additions preserve all existing event layouts and
exports. They do not execute scripts.

Applications make an input or textarea read-only with
`Element::set_attribute("readonly", "")` and restore editing with
`Element::remove_attribute("readonly")`. Read-only controls allow selection
and explicit application `set_control_value` calls. User typing, deletion,
undo/redo, IME edits, and accessibility value changes cannot modify them.
Accessibility snapshots report read-only state and advertise selection
actions without advertising value-editing actions.

The current public application suite has 55 scenarios. The textarea scenario
checks UTF-8 selection replacement, one-step undo, callback cancellation,
focus changes during callbacks, and absence of synthesized clicks. The new
read-only scenario checks both text control kinds through the public Rust
keyboard, text, IME, and accessibility APIs, then reenables editing. The
Linux-enabled framework and engine check passes 162 tests with eight ignored;
it includes Enter with a platform carriage-return payload. The subsequent
framework, engine, C and Linux-platform check passes 191 tests with eight
ignored. The full locked workspace passes 8,489 tests with 13 ignored, and all
six C consumers and the C++ header consumer pass with the same 107 exports.
The C accessibility consumer verifies read-only state, rejected value changes,
allowed selection and owned snapshots for both text control kinds. The
[diagnostic index](generated/native-text-input-diagnostic-v1.json) records the
test logs and the preceding source's genuine textarea, read-only and C failures.
These native interaction checks do not qualify physical IME or AT-SPI operation.

The subsequent public normalized-input check passes 153 headless Rust/C tests
and 193 Linux Rust/C/platform tests, each with eight ignored. The full locked
workspace passes 8,491 tests with 13 ignored. All six headless C consumers and
the C++ header consumer pass with 109 current exports; existing structs and
the preceding 107 exports are preserved. The C window consumer invokes native
Enter and text input from a presentation callback and reads the updated value
inside its two input callbacks. The
[normalized-input diagnostic](generated/native-keyboard-input-diagnostic-v1.json)
records the evidence. These checks do not qualify the remaining renderer,
physical input, accessibility, or hardware gates.

The preceding seven native IME regressions are public Rust application tests in
[`v02_conformance.rs`](../../bindings/rust/openui/tests/v02_conformance.rs).
At that checkpoint, the suite was 50/50. The locked workspace
passes 8,480 tests with 13 ignored. The Linux-enabled Rust/C/platform checks
pass 185 tests with eight ignored, including Winit event normalization
and C composition dispatch. The ABI generator reports no drift; the 106 C
exports and frozen layouts are unchanged. These checks establish the native
input behavior above. At clean checkpoint `04394c86`, the
[static raster guards](../renderer/generated/focused-primitive-raster-v43.json)
are 640/640 focused and 960/960 primitive exact. The
[image comparison](../renderer/generated/native-ime-raster-delta-v1.json)
confirms that all 1,600 Open UI images, Chromium images, and oracle identities
are unchanged from the prior guards. This does not update the original or
expanded full-census counts. Operating a physical IME and AT-SPI service on the
release lab, and final Chromium rendering qualification, remain open.

File, date, and color picker dialogs and media controls remain outside the v0.2
interactive contract. Their passive rendering roles are retained for exact
headless compatibility, but activating them does not invoke a native picker.
