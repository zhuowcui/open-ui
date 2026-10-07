# Native focus callbacks

Open UI never executes JavaScript. Applications focus and blur elements with
public native Rust methods and handle the resulting events with Rust callbacks.
The C facade uses that same retained Rust document.

```rust
input.on("focus", move |_| {
    // Update application state here with native Rust.
})?;
input.focus()?;
input.blur()?;
```

## Correction

The existing C `oui_element_focus` and `oui_element_blur` calls changed Engine
state directly and skipped native event delivery and composition cancellation.
The unchanged consuming C diagnostic reported zero callbacks after focus,
focus transfer and blur; native Tab then produced one callback.

The correction routes those existing calls through the public Rust operation's
Document path. Programmatic focus, keyboard traversal, pointer focus and
accessibility focus/blur share that transition. It cancels active composition
before clearing the old focus, delivers blur while no control is focused,
revalidates the requested target after callbacks, then delivers focus.

Callbacks run after engine and listener borrows are released. A blur callback
may choose another control, remove or disable the pending target, or focus and
then blur another control. A composition callback may also redirect focus.
Repeated focus on the current control and blur on another valid control are
no-ops. C listener user data must remain live until synchronous dispatch ends
and until its listener is destroyed. Calls stay on the owning thread, stale
handles fail, and the facade contains native callback panics.

No C export or struct layout changes. All 114 exports remain present.

## Verification

The [source-owned evidence](generated/native-focus-events-v1.json) records the
clean implementation commit, build artifacts, callback observations, checks
and the completed evidence archive. The first build's test type error is kept
as a failed attempt; it is not relabeled as a pass.

The clean locked Linux workspace passes **8,576 tests**, with zero failures
and 13 ignored tests. The headless C facade passes **41/41 tests**. All 18 local
stages and 17 read-only checks pass. All local Rust artifacts and the copied C
library are bound to the clean implementation checkout; the source stays
unchanged through the full pipeline. Fourteen C and eight C++ ABI consumers
compile and run; all 114 exports and existing layouts stay unchanged.

Six active hosted jobs pass; five optional hardening jobs are skipped, not
qualified. All six C/C++ window consumers pass on X11 software, X11 Mesa
presentation and pure Wayland software. The hosted reports retain synthetic
merge `bfec80f7`, whose Git tree is verified identical to `9349e5b3`.

Five Rust application tests cover six event sequences observed twice in pinned
Chromium 147.0.7727.50, composition cancellation and redirection, disabled
requests, keyboard traversal and accessibility. Two C facade guards cover
shared Rust/C state and callbacks, callback mutation, ownership, stale handles,
borrow errors and panic containment.

The unchanged C diagnostic now reports:

```text
after_c_focus=1
after_c_transfer=3
after_c_blur=4
after_native_tab=5
```

Both repeated runs agree. C and C++ window consumers also check two focus
callbacks and one blur callback, mutate retained state from those callbacks,
and continue their editing and application lifecycle checks.

Run the application and facade guards from `bindings/rust`:

```sh
cargo test --locked -p openui --test native_focus
cargo test --locked -p openui-ffi --lib focus_tests
```

## Later implementation

The [native focus follow-up](native-focus-behavior.md) implements negative
`tabindex` focus, modal callback delivery and non-bubbling propagation on
`442d7a64`. The v1 evidence above keeps its original source and scope.

## Work open at the v1 checkpoint

This evidence closes the demonstrated callback and composition gap. It does
not qualify all focus semantics or all element APIs. Focusing elements with a
negative `tabindex`, modal focus event delivery, focus event propagation and
other needed native behaviors require their own application checks; the
native Document/interaction layer owns that work.

No Chromium reference image or old archive image is changed. This checkpoint
runs no renderer census and admits no new pixel result. Full Chromium equality,
remaining native APIs, compositor behavior and release-lab gates remain open.
