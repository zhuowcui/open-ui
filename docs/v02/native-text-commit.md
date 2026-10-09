# Native text edit commits

Open UI never executes JavaScript, in any version. Consuming apps call public
Rust methods and callbacks over the shared retained Engine. C uses the same
Document and editing path. Pinned Chromium supplies reference behavior and
expected pixels; old Open UI images preserve history.

The [source-owned evidence](../renderer/generated/native-text-commit-v1.json)
records the historical native baseline: five of six scenarios differ from
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

## Public editing metadata now verified

The separate seven-scenario Chromium reference remains immutable. Its two
identical runs report 42 callback rows for insertion, deletion, line breaks,
undo and redo, with distinct input intents and nullable data. The original
source inspection described an API gap at that earlier checkpoint.

Public `fcabea38` now exposes `Event::input_info` and the versioned C
`oui_event_input_info_v1`. The completed public scope matches 109 scenarios in
Rust and 109 in each of C and C++, including actual editing metadata, event
properties, focus, attributes and identity. Workspace, ABI, focused and
primitive checks pass on that clean source; the full pixel gates still fail.
[Completed public evidence](../renderer/generated/native-selection-public-v1.json).

The private text-commit trial above remains historical evidence and does not
qualify its six scenarios on a later source. Its rejected build and initial
oracle-expression failure stay preserved. Composition, complete native APIs,
Chromium pixel equality, compositor and release qualification remain open.
