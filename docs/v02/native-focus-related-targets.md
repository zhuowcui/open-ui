# Native focus notifications

The implementation is integrated into the umbrella PR. Clean combined source
`3ee3f9d3` passes native execution, compatibility and hosted hardening checks.
Both static raster matrices are exact; the complete original and expanded
pixel gates still fail. Open UI executes no JavaScript. Applications use public Rust methods and callbacks; C uses the
same retained Document and Engine.

The implementation adds `Event::related_target`, `bubbles`, `cancelable`, and a public
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
All eight C focus guards execute successfully on the measured source.

The Rust, C and C++ consuming apps exercise all twelve scenarios in the
immutable offline Chromium observation set, including callback redirects and
immediate stopping. Rust adds weak-handle and cancellation guards. C/C++ source
checks and ABI generation checks pass; the first C++ enum-conversion error is
preserved. Clean combined `3ee3f9d3` matches all 338 callback rows twice in
each language and passes twelve Rust focus guards, eight C guards and the
existing pointer boundary guard. All 8,595 workspace tests and 47 headless C
tests pass, with fifteen C and nine C++ consumers and all twenty verification
stages. All seven hosted hardening jobs pass on that same source. See the
[source-owned evidence](../renderer/generated/native-controls-focus-v2.json).
The earlier private source and failed attempts retain their separate identities.

These results qualify the measured scenarios and preserve every prior C layout
and export. They do not establish all focus contexts or complete element APIs.
Hidden/inert elements, additional controls, autofocus and nested focus scopes
remain open. Focused 640/640 and primitive 960/960 are exact. Original
21,330/22,924 and expanded 22,133/23,728 are exact, with zero errors and
actual exits 1. Every static comparison remains unchanged from the preceding
current-protocol baseline. Complete Chromium equality and reference capture
reconciliation remain open. No new pixel or release pass is admitted.
