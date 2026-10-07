# Native C text-editing commands

Open UI never executes JavaScript. Applications call native methods and supply
native callbacks. Rust `Element::edit_text(EditCommand)` and C
`oui_element_edit_text_v1` operate on the same retained Rust document, Engine,
editing history and `input` event path used by native keyboard editing.

## Public contract

`OuiEditCommandV1` starts with `struct_size` and `abi_version`. Its 32-byte
layout has 4-byte alignment. The command is copied during the call and never
retained. Existing ABI layouts and symbols are unchanged; the new export is
additive and the generated checksum records the current surface.

| Command | Fields | Behavior |
|---|---|---|
| `OUI_EDIT_MOVE` | direction, unit, extend_selection | Move or extend selection |
| `OUI_EDIT_DELETE` | direction, unit | Delete the selection, or delete to the requested boundary |
| `OUI_EDIT_SELECT_ALL` | All remaining fields zero | Select all text |
| `OUI_EDIT_UNDO` | All remaining fields zero | Restore the preceding value from shared editing history |
| `OUI_EDIT_REDO` | All remaining fields zero | Restore the undone value |

Directions are backward and forward; units are grapheme, word, line and
document. Only move permits `extend_selection = 1`. Reserved and unused fields
must be zero. Unknown values, short headers and wrong ABI versions are rejected
before mutation or callbacks. Larger version-compatible structs are accepted.

Calls require the document's owning thread and a live input or textarea.
Readonly controls permit selection commands; disabled controls reject every
command. Selection offsets are UTF-8 byte positions on grapheme boundaries.
Value changes synchronously emit `input` after engine borrows are released,
so C listeners can read the value and mutate other elements. Selection-only
commands emit no `input`. The operation does not synthesize keyboard events.
Listener and user-data ownership follows the existing C callback contract.

The [C consumer](../../examples/c_v02/edit_commands.c) invokes deletion from a
native button callback, observes the resulting input callback, and checks
Unicode, history, selection, readonly/disabled errors and teardown. The
[C++ consumer](../../examples/c_v02/edit_commands.cc) exercises the same public
ABI. Both native window consumers also call editing commands from the Linux
presentation callback.

## Qualification status

This implementation checkpoint awaits the clean-source Rust, C/C++, ABI and
Linux window checks. Passing those checks will establish editing API behavior,
not complete Chromium pixel matching, editor scrolling, IME behavior, all
native APIs or release qualification. Pinned Chromium remains the sole
expected-pixel target. No renderer code or reference image changes here.
