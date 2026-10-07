# Native focus notifications — uncompiled draft

This private draft is unapplied and has no native execution or renderer
qualification. Open UI executes no JavaScript. Applications use public Rust
methods and callbacks; C uses the same retained Document and Engine.

The draft adds `Event::related_target`, `bubbles`, `cancelable`, and a public
immediate-stop query. Focus and blur capture without bubbling. Focusin and
focusout bubble, and all four notifications are noncancelable. Related targets
are weak, generation-checked handles; saved events cannot retain the document.
Target capture listeners precede target noncapture listeners. Focus callbacks
can redirect a transfer or detach its pending target.

C appends `FOCUS_IN`, `FOCUS_OUT`, an immediate-stop flag,
`oui_event_focus_info_v1`, and `oui_document_focused_element_v1`. The first
query requires the exact borrowed event pointer during an owning-thread native
focus callback. Nested callbacks preserve outer scopes. Metadata copies can
outlive the callback; each returned element handle must be destroyed. These
handles do not retain their document. Existing 114 exports and 31 layouts are
preserved; two exports and `OuiFocusEventInfoV1` are appended.

Explicit C focusin/focusout notifications also use the Rust listener route and
metadata scope; they do not change focus. New guards cover nested queries of
both the current and outer borrowed events, owning-thread restrictions, owned
handles after detach/node replacement/document destruction, output preservation
on errors, and untouched trailing bytes of a future caller's larger structure.
These guards are written but have not been executed.

The Rust, C and C++ consuming apps exercise all twelve scenarios in the
immutable offline Chromium observation set, including callback redirects and
immediate stopping. Rust adds weak-handle and cancellation guards. C/C++ source
checks and ABI generation checks pass; the first C++ enum-conversion error was
repaired. Rust compilation, every runtime observation, nested C scope/lifetime
guards, complete workspace/ABI consumers and renderer nonregression remain
required. No API, pixel or release pass is admitted by this draft.
