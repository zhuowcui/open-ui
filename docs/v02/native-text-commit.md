# Native text edit commits

Open UI never executes JavaScript, in any version. Consuming apps call public
Rust methods and callbacks over the shared retained Engine. C uses the same
Document and editing path. Pinned Chromium supplies reference behavior and
expected pixels; old Open UI images preserve history.

The [source-owned evidence](../renderer/generated/native-text-commit-v1.json)
records the unchanged native baseline: five of six scenarios differ from
Chromium, producing 23 callback rows instead of 29. User text edits do not emit
the required change notification on blur or Enter, and keyboard deletion lacks
beforeinput. A programmatic value change alone correctly emits no change.

Private clean `f73435f3` adds shared pending user-edit and committed-value state.
Blur commits before blur/focusout notifications; single-line Enter can commit
through cancelable beforeinput. Delete, undo and redo run through beforeinput.
The commit resets its state before invoking callbacks, preventing a reentrant
callback from committing the same edit twice. Programmatic changes alone do
not become user edits, and canceling IME preview does not commit preview text.

Both real Rust consumer runs match all six Chromium scenarios and all 29 full
callback rows. Fifteen text-commit, focus and input-notification guards pass.
C and C++ each compile and link, then match all six scenarios and 29 common
callback rows twice; their compared fields are type, value, active element and
final state. Every linked local artifact is freshly built from the measured
private source, and final public/private/application source identities stay
unchanged. This does not qualify all C metadata or all native APIs.

The first C measurement is rejected because cached local artifacts lack prior
source proof. No consumers ran in that attempt. A new measurement cleans and
freshly builds all local packages; it retains the rejected receipt and does
not weaken the proof requirement.

## Remaining native editing metadata

A separate Chromium reference completes two byte-identical runs of seven
editing scenarios and 42 callback rows. It reports insertion, backward and
forward deletion, line breaks, undo and redo as distinct input intents, with
nullable data. Source inspection finds no corresponding typed intent and
nullable data snapshot in the public Rust Event API or the legacy C event
layout. The native metadata behavior has not been measured. The first oracle
probe fails due to its evaluation expression; its receipt remains preserved,
and the corrected probe has a separate identity.

The correction above remains private and unapplied. Broader reentry checks,
complete workspace/ABI and own-source renderer checks remain required before
integration. Needed public Rust APIs and additive, versioned C metadata remain
implementation work. Complete Chromium equality, compositor and release
qualification are still open.
