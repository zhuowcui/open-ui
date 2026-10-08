# Native editing and rounded border follow-up

Open UI never executes JavaScript. Apps implement interaction through public native Rust methods and Rust callbacks over the retained Engine. C uses the same native behavior. Every needed browser-style element operation must have a working public Rust API, including its state changes and events. A test disposition does not waive missing native behavior.

## Completed native border check

Private candidate `6cc132dc` matches all **35/35 unchanged Chromium images** across five scales. A second native process for every case produces identical PNG and geometry bytes. The consuming Rust app changes element width through a Rust click callback; all callback, geometry and teardown assertions pass.

| Native test set | Exact images | Different images | Errors |
|---|---:|---:|---:|
| Recovered unchanged baseline | 4/35 | 31 | 0 |
| Earlier private clipping correction | 14/35 | 21 | 0 |
| Layer correction | 33/35 | 2 | 0 |
| Layer and fractional clip correction | 35/35 | 0 | 0 |

The shared painter now:

- Applies local-background clipping only when overflow creates a scroll container.
- Groups a rounded background and translucent border independently of the background fill's clip.
- Applies the scrollport's hard clip before filling a local background color.
- Keeps fractional layout edges until the physical clip transform.
- Removes radius-specific single-pixel alpha corrections.

All 73 commands and 16 freshly linked framework artifacts are verified, with unchanged final public/private source identities. There are 21 gains against the earlier 14/35 private trial and 31 against the unchanged 4/35 test set, with no exact loss or worsened comparison. The [source patch](evidence/native-rounded-border-v2/openui-native-rounded-border-final-source-v3093.patch.gz), consuming Rust app, immutable reference HTML/images, native images and command receipts are available in the [evidence index](generated/native-rounded-border-v2.json).

This implementation remains private and unintegrated. Complete focused, primitive, original, expanded and workspace/ABI checks are still required before integration. These 35 images do not qualify the complete renderer.

## Preserved unsuccessful runs

Changing the border drawing operation alone produced no exact gains and worsened twenty comparisons. That trial is rejected and reverted. An intermediate build failed after removing inner-border geometry still needed by inset effects; the geometry is restored. Both failures retain their actual command results and source audits.

The earlier native owner disappeared after 18 completed commands and eight border images. Its receipt remains incomplete and its whole exit is unknown. A separate recovery verifies those logs/images, renders the other 27 cases and checks unchanged source hashes. A later full-renderer attempt was deliberately interrupted before producing a matrix; its actual exit is 130. These runs are preserved separately and never counted as completed qualification.

## Native editing work remains

A private input metadata implementation matches eleven Chromium scenarios and 69 callback rows in Rust, C and C++, twice each. Its first whole run failed a C guard's expected status; a later guard correction passes C and C++ twice. Binaries remain attributed to their actual build source. [Earlier evidence](generated/native-editing-followup-v1.json).

Composition still differs in all four measured common-field scenarios. Repeated Chromium traces define required native behavior for callback mutation, cross-target focus, direct editing commands, readonly/disabled/detached controls and reactivation. Those are reference observations, not native API passes.

Range replacement needs a public Rust method, selection direction metadata and deferred native event delivery. Eight Unicode cases with representable scalar boundaries and twelve selection-task scenarios repeat byte-identically. A further 24 cross-control/reentry cases repeat with 98 callback rows. Selectionchange and select use distinct delivery phases; target-local coalescing lasts until notification delivery, callbacks observe current retained state, and detached live targets retain pending notifications. The earlier single-queue draft is insufficient. Shared native scheduling, public methods, Linux event-loop integration and C parity remain implementation work.

Existing UTF-8/grapheme positions cannot represent every UTF-16 position in the browser reference. Those differences remain explicit. Neither fixed expected values nor script execution substitutes for native behavior.

## Qualification remains open

The reviewed painter correction and native consuming app are now integrated
into the PR branch. The combined source still requires fresh workspace/ABI
and renderer qualification. Private `6cc132dc` completes all four matrices:

| Suite | Exact | Different | Errors | Actual exit |
|---|---:|---:|---:|---:|
| Focused | 640/640 | 0 | 0 | 0 |
| Primitive | 960/960 | 0 | 0 | 0 |
| Original | 21,338/22,924 | 1,586 | 0 | 1 |
| Expanded | 22,141/23,728 | 1,587 | 0 | 1 |

Both complete censuses gain eight exact comparisons with zero losses and zero
worsened rows against public `8e8318bf`. All 48,252 Chromium reference records
are unchanged. The completed audit verifies source bytes, eleven logs and 26
fresh compiled artifact hashes. The concurrent source reader preserves every
source byte and original hash order; the unchanged build script independently
embeds its identity using the official serial reader. The interrupted earlier
build remains recorded as actual exit 130, with no completed matrices.
[Full evidence](generated/native-rounded-layer-v3.json).

Twenty-four further pinned Chromium value/range scenarios produce 26 callback
rows, identically twice. Identical value assignment preserves selection and
direction. Changed value assignment emits selectionchange without select;
range replacement also emits select when selection or direction changes.
These are reference observations for required native APIs, not native passes.
 Full Chromium pixel equality, complete native APIs, compositor, hardware and release checks remain unfinished. Chromium references are immutable, comparison tolerance is zero, and old Open UI images preserve history.
