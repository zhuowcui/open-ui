# Native focus behavior follow-up

Applications call native Rust methods and Rust callbacks. Open UI never
executes JavaScript. The C facade operates on the same retained document.

## Implemented behavior

- Programmatic focus accepts a valid negative `tabindex`. Sequential traversal
  excludes negative entries, but starts from the focused element's position
  in document order. Positive Tab order stays intact.
- Focus and blur dispatch ancestor capture and target listeners, without
  bubbling. Calling `prevent_default` does not cancel these notifications.
- Changing modal containment selects the next focus target without moving
  focus under an engine borrow. The Document cancels composition and delivers
  blur/focus callbacks through its shared transition, and restores the prior
  valid focus when modal containment ends. Leaving inactive modal mode is a
  no-op. A callback that changes modal containment cancels the old pending
  focus request.
- Explicit C traversal, modal changes, pointer focus and focus/blur requests
  share those native Rust operations. Traversal returns the final focused
  control after callbacks, rather than an earlier proposed target. C exports,
  struct layouts and the ABI checksum are unchanged.

The public Rust entry points are `Element::focus`, `blur`,
`Document::advance_focus`, `set_modal_root`, and accessibility/input operations.
The C entry points include `oui_document_advance_focus`,
`oui_document_set_modal_root`, `oui_document_dispatch_pointer_event`, and
`oui_document_dispatch_event`, alongside existing element focus and blur.

## Evidence

The [versioned report](generated/native-focus-events-v2.json) binds the clean
implementation, fresh local Cargo artifacts, consuming applications, tests,
ABI checks and completed logs. It also preserves the initial compile failure:
a Linux test depended on an import that production code no longer used.
The correction imports that enum in the test itself. A subsequent guard
clicked the center of an empty control rectangle and correctly hit the
root instead of the input. Its failed result and source-owned diagnostic are
also kept. The corrected pointer guard supplies visible authored bounds and
checks the actual hit target before checking focus.

Pinned Chromium 147.0.7727.50 is measured twice for each of three reference
probes. They establish programmatic negative focus, forward and backward
traversal from a negative entry, capture/target notification order and a
basic modal enter/leave focus sequence. All repeated observations agree.
Reference scripts execute in separate offline Chromium tooling; Open UI
executes none of them. No reference image is changed.

Native traversal wraps within its document. One positive-order Chromium
probe leaves the document for browser UI at an endpoint. That observation
is preserved and is not counted as a matching native result.

Clean implementation `442d7a64` passes **8,583 locked workspace tests**,
with zero failures and 13 ignored tests, and **44/44 headless C facade tests**.
All 18 local stages and 17 read-only checks pass. Local artifacts and the copied
C library are bound to that clean checkout, which stays unchanged through the
whole pipeline. Fourteen C and eight C++ consumers compile and run; all 114
exports and prior layouts remain intact.

Six active hosted jobs pass; five optional hardening jobs are skipped, not
qualified. All six window consumers pass on X11 software, X11 Mesa presentation
and pure Wayland software. Their actual synthetic merge
`b012ba99` has the identical Git tree to `442d7a64`.

Four new Rust application guards and three C facade guards verify these
behaviors, composition cancellation, callback mutation and modal reentry.
The previous five Rust and two C focus guards remain. Native C/C++ windows
exercise negative focus, explicit traversal and modal restoration while
retaining their editing and lifecycle checks; each observes four focus and
three blur callbacks.

## Remaining qualification

These checks do not establish every focus behavior or the complete element
API. Focus-in/out events, related-target metadata, default modal autofocus
selection, additional control kinds and nested focus scopes require review
and consuming application evidence. The native Document/interaction and C
facade own these remaining behaviors.

Unstyled low-level input elements report zero bounds in the native diagnostic.
Default control geometry and pointer usability remain an owned native-layout
investigation; no Chromium pixel pass is claimed for those controls.

This checkpoint runs no pixel census and admits no new pixel result. Complete
Chromium rendering equality, remaining native APIs, compositor behavior,
hardware, performance and release qualification remain open.
