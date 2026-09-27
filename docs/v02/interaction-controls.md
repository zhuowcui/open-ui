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

File, date, and color picker dialogs and media controls remain outside the v0.2
interactive contract. Their passive rendering roles are retained for exact
headless compatibility, but activating them does not invoke a native picker.
