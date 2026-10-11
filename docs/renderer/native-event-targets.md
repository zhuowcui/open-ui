# Native Rust event targets

Open UI applications handle events in Rust. A parent callback can inspect the
child that received an event and call its native element methods directly:

```rust,ignore
parent.on("click", |event| {
    if let Some(child) = event.target() {
        child.set_background_color(Color::RED).unwrap();
    }
}).unwrap();
```

`Event::target()` returns the element receiving the event, including a pointer
capture target. `Event::current_target()` returns the element whose listener
is running. The handles are weak and generation checked. Saving an event does
not keep its document alive, and removing its target makes `target()` return
`None`. Listener identity and phase clear after callbacks return, including
errors and panic unwinding. Callbacks use the shared retained document after
engine borrows have been released.

The [consuming Rust app](../../bindings/rust/openui/examples/native_event_targets.rs)
checks capture, target and bubble callbacks; mutation through the event target;
owned bounds; and teardown. It renders the before and after states at five
scales through the public application API. Open UI executes no JavaScript.
Separate offline Chromium tools construct the reference state.

## Completed evidence

The [versioned evidence](generated/native-event-targets-v1.json) records clean
source `1c8540e9f8c6eeb8fce10a76cb9a32c2f42f01fd` and 153 preserved artifacts:

- The named baseline actually fails its event phase assertion; all five fixed
  guards pass, including pointer capture, stale handles, errors and panics.
- After clearing all 18 workspace packages, 8,536 tests pass, zero fail and
  13 are ignored. All 113 C exports and existing layouts remain intact;
  11 C and five C++ consumers pass.
- All ten images and bounds match Chromium exactly at five scales. Five
  repeated native pairs and 40 consecutive captures from 20 independent
  Chromium processes agree. The audit rechecks their bytes and exact RGBA.
- Focused and primitive suites pass at 640/640 and 960/960 exact. Original
  and expanded suites finish at 21,334/22,924 and 22,137/23,728 exact, zero
  errors and actual exits 1. All 48,252 comparison invariants remain unchanged.
- All seven [own-source hardening jobs](https://github.com/zhuowcui/open-ui/actions/runs/37277487481)
  pass, with zero skips.

The methods are now implemented on the umbrella branch. These checks complete
their native application path. They admit no new rendering case and do not
complete full Chromium parity, other needed native APIs or release
qualification. Old Open UI screenshots remain historical evidence; pinned
Chromium is the sole pixel target.
