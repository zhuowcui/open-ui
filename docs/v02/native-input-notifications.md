# Native input notifications

Open UI executes no JavaScript. Consuming applications use public Rust methods
and callbacks, with C using the same retained Document and Engine.

`input` and `change` report an edit or activation that has already happened.
They are noncancelable: attempting to prevent their default does not mark the
notification canceled or undo its state change. `beforeinput` remains
cancelable and may reject the pending edit. Saved Rust event clones retain
immutable cancellation policy even if their public type label is changed.
The C facade discards cancellation attempts between callbacks and before
explicit input/change dispatch. Existing exports and layouts are unchanged.

The [source-owned evidence](../renderer/generated/native-input-notifications-v1.json)
records two repeated pinned Chromium runs and native Rust/C/C++ consuming
applications. The baseline reproduces the mismatch. Corrected private
`cd6f9d4b` matches all five Rust observations twice, including bubbling and
cancellation metadata, final value and checkbox state. C/C++ match the common
type, target, cancellation and final-state fields twice each. The saved-event
Rust guard and completed native/explicit C notification guard pass.

Later private `0b249f3b` adds the same C/C++ sources for review and passes
8,597 locked Linux workspace tests, zero failures and 13 ignored; all 48
headless C tests, 16 C and 10 C++ consumers, ABI verification, and all twenty
verification stages pass. The current ABI retains 116 exports and 32 layouts.
The first C guard compilation and both disk-guard interruptions remain recorded.
Successful workspace results and the owned library from the interrupted run
retain their original receipt; the final continuation completes the remaining
checks with temporary files in memory-backed storage and unchanged guards.

The implementation is integrated at `c09eb639`. The preceding measurements
retain their actual private source identities. [Fresh combined verification](../renderer/generated/native-input-combined-v1.json)
now measures clean `8e8318bf`: Rust/C/C++ each match all five input observations
twice, and all 338 focus observations in twelve scenarios twice. Both input
guards and the focus/pointer guards pass. All 8,597 workspace tests, 48 headless
C tests, 16 C and 10 C++ consumers and twenty verification stages pass, with
116 exports and 32 layouts unchanged. Six ordinary CI and all seven manually
enabled hardening jobs pass on that source; optional skips remain separate.

Focused 640/640 and primitive 960/960 are exact. Original 21,330/22,924 and
expanded 22,133/23,728 remain exact, zero errors; both full pixel gates fail.
All nine comparison fields are unchanged across 48,252 rows. Remaining element
APIs, complete Chromium equality and release qualification stay open.
