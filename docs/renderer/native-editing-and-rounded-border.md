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

These are the earlier private measurements. The shared implementation and consuming Rust app are now integrated and verified on public checkpoint `5b05e1b5`, as recorded below. The 35 images alone do not qualify the complete renderer.

## Preserved unsuccessful runs

Changing the border drawing operation alone produced no exact gains and worsened twenty comparisons. That trial is rejected and reverted. An intermediate build failed after removing inner-border geometry still needed by inset effects; the geometry is restored. Both failures retain their actual command results and source audits.

The earlier native owner disappeared after 18 completed commands and eight border images. Its receipt remains incomplete and its whole exit is unknown. A separate recovery verifies those logs/images, renders the other 27 cases and checks unchanged source hashes. A later full-renderer attempt was deliberately interrupted before producing a matrix; its actual exit is 130. These runs are preserved separately and never counted as completed qualification.

## Native editing work remains

A private input metadata implementation matches eleven Chromium scenarios and 69 callback rows in Rust, C and C++, twice each. Its C change/blur/focusout event properties were recorded as constants; actual property queries remain required before those fields establish C API parity. Its first whole run failed a C guard's expected status; a later guard correction passes C and C++ twice. Binaries remain attributed to their actual build source. [Earlier evidence](generated/native-editing-followup-v1.json).

Composition still differs in all four measured common-field scenarios. Repeated Chromium traces define required native behavior for callback mutation, cross-target focus, direct editing commands, readonly/disabled/detached controls and reactivation. Those are reference observations, not native API passes.

Range replacement, selection direction and deferred event delivery are implemented in the measured private checkpoint; fresh combined public verification remains required. Eight Unicode cases with representable scalar boundaries and twelve selection-task scenarios repeat byte-identically. A further 24 cross-control/reentry cases repeat with 98 callback rows. Selectionchange and select use distinct delivery phases; target-local coalescing lasts until notification delivery, callbacks observe current retained state, and detached live targets retain pending notifications. The earlier single-queue draft is insufficient. The measured private implementation provides shared native scheduling, public methods and Linux event-loop integration. New C event property and attribute queries, Unicode cases and focus neighbors are integrated; complete fresh public verification remains required.

Existing UTF-8/grapheme positions cannot represent every UTF-16 position in the browser reference. Those differences remain explicit. Neither fixed expected values nor script execution substitutes for native behavior.

## Completed public verification; release gates remain open

The reviewed painter correction and native Rust app are integrated and
verified on clean PR checkpoint `5b05e1b5`. All four matrices finish:

| Suite | Exact | Different | Errors | Actual exit |
|---|---:|---:|---:|---:|
| Focused | 640/640 | 0 | 0 | 0 |
| Primitive | 960/960 | 0 | 0 | 0 |
| Original | 21,338/22,924 | 1,586 | 0 | 1 |
| Expanded | 22,141/23,728 | 1,587 | 0 | 1 |

Both complete censuses gain eight exact comparisons with zero losses and zero
worsened rows against public `8e8318bf`. All Chromium reference fields remain
unchanged across the 48,252 rows in the four matrices. The completed audit
verifies every source byte, thirteen logs and 53 compiled artifact paths,
including fourteen freshly built framework packages. The unchanged build
script independently embeds its source identity using the official serial
reader. Native captures remain 35/35 exact twice, with all seventy actual
commands, click callbacks, geometry and teardown checks passing.

The same source passes **8,597 workspace tests**, 48 headless C tests, sixteen
C and ten C++ consumers and twenty verification stages, preserving all 116
exports and 32 layouts. Six ordinary CI jobs and seven explicit hardening
jobs pass. Five optional skips in ordinary CI remain recorded. Hosted virtual
X11 software/OpenGL and pure Wayland software window checks pass; physical
hardware and release lab qualification remain open. The earlier serde_json
example compilation failure and earlier artifact audit failures remain
preserved with their actual status.
[Completed public evidence](generated/native-rounded-layer-v4.json).
[Earlier private evidence](generated/native-rounded-layer-v3.json).

## Private native selection verification

Clean private `5449f22d` implements range replacement, selection direction,
coalesced selectionchange, separate select batches, reentrant task delivery,
node identity/connectivity and focused-disable behavior. A fresh build passes
all 25 commands and 209 unit/integration tests. Its whole result remains a
failure because its original C comparator omitted Chromium focus fields.
The corrected comparator reruns the immutable original binaries twice:
**98/98 Rust scenarios** and **136/136 C/C++ common-field scenarios** match,
including actual focus on both sides. Source and executable bytes stay unchanged.
The earlier failed compilation and guard assertions retain actual failure
results. [Evidence](generated/native-selection-followup-v1.json).

That private selection matrix does not observe C event properties or include
the eight Unicode and twenty-two disabled-focus neighbor cases in C. The new
C event property and owned attribute queries and those extended consumers are
integrated. The first public Rust build passes 196 unit tests and thirteen
integration guards, with six native apps repeating unchanged. Its C++ consumer
check fails on integer event arrays; the corrected enum arrays require a fresh
run. The combined public API port requires fresh Rust,
C, C++, workspace/ABI, hosted and renderer verification. Composition and
complete native API qualification remain open.

The current expanded contract contains 201 admitted native final-state cases;
35 additional AST-lowered cases await exact qualification. This inventory does
not execute scripts in Open UI or waive missing public Rust APIs. Full Chromium
pixel equality, complete native APIs, compositor, hardware and release checks
remain unfinished. Comparison tolerance is zero, Chromium references are
immutable, and old Open UI images preserve history.

## Completed public native selection and queries

The native APIs are verified on clean public `fcabea38`: all 109 Rust scenarios
and 109 scenarios in each of C and C++ match Chromium, with repeated runs
unchanged. C observes actual event properties, attributes, focus and node
identity. All 8,607 workspace tests, 50 headless C tests, 22 C and 16 C++
consumers and twenty local stages pass. The 125 exports and 34 layouts preserve
previous callers. Six ordinary CI and seven explicit hardening jobs pass.
The renderer reports retain exact focused and primitive suites and failing
full pixel gates, with fixed references and no regression.

The pending public verification and C property-query requirements above are
superseded for this measured selection/query/metadata scope by the
[completed public evidence](generated/native-selection-public-v1.json).
Older private scopes remain historical measurements. Composition and complete
native API parity stay open. A fieldset probe has ten focus differences in
sixteen repeated checks; effective state, events, structural mutation and
appearance remain unqualified. These need shared native Rust implementation.
Open UI never runs JavaScript.
