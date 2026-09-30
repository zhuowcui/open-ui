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

The seven native IME regressions are public Rust application tests in
[`v02_conformance.rs`](../../bindings/rust/openui/tests/v02_conformance.rs).
Together with the existing scenarios, that suite is 50/50. The locked workspace
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
