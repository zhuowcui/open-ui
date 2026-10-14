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

## Verified implementation

Clean implementation `7adb2130` passes all 18 local stages. The locked Linux
workspace run passes **8,569 tests**, with zero failures and 13 ignored tests.
All local Rust libraries are freshly compiled from that checkout; later cached
artifacts must match the hashes from the preceding verified build of the same
source. The copied C library is bound to those artifact records. The source
remains clean and unchanged through the entire pipeline, including the gaps
between commands.

The headless C ABI run passes **39/39 tests**. Five new guards cover all text
units and directions, selection extension, shared Rust/C history, callback
mutation, malformed headers, readonly/disabled controls, stale handles,
wrong-thread calls, outstanding borrows and panic containment. The initial
workspace run exits 101 because a new guard expects the wrong null-handle
status. That failure is preserved; the correction changes only that expectation.

All **114 exports**, every previous symbol and every prior ABI layout are
verified. **Fourteen C and eight C++ consumers** compile and run. The new editing
consumers repeat byte-identical output twice in each language, with six input
and two button callbacks per run. All 17 read-only checks also pass.

The six active hosted jobs pass. Five optional MSRV, Miri, fuzz and sanitizer
jobs are skipped, not qualified. All six C/C++ window consumers pass on X11
software, X11 Mesa OpenGL presentation and pure Wayland software. C observes
five input callbacks and C++ three per window run; selection-only calls add no
input callback. The hosted checkout is synthetic PR merge `4c5dc29c`, whose
Git tree is verified identical to `7adb2130`; the reports retain their actual
checkout identities, matching header and consumer-source hashes.

The [versioned evidence](generated/native-c-editing-commands-v1.json) and
[hash-verified archive](evidence/native-c-editing-commands-v1/completed-evidence.tar.gz)
preserve completed logs, Cargo records, consumers, ABI inputs, hosted window
reports, failed checks and corrected checks. C editing-command parity is
implemented and verified through the shared native Rust path.

These checks establish editing API behavior. Complete Chromium pixel matching,
editor scrolling, selection rendering, all native APIs and release
qualification remain open. Accepted renderer results stay **21,334/22,924**
original and **22,137/23,728** expanded exact. This work changes no renderer code,
reference image or pixel tolerance and admits no new pixel comparison.
