# Native editing and rounded border follow-up

Open UI never executes JavaScript. Applications use public native Rust APIs and Rust callbacks over the retained Engine; C delegates to the same native behavior. Browser tests describe required behavior for those APIs. A test disposition does not waive a missing native operation.

## Completed border measurement

The private candidate `da33875c` removes radius-specific single-pixel alpha corrections and applies local-background clipping only when retained overflow creates a scroll container. The consuming Rust app changes element width from a Rust click callback. All 35 geometry states and callback/teardown assertions pass across five scales.

| Measurement | Exact images | Different images | Errors |
|---|---:|---:|---:|
| Recovered unchanged baseline | 4/35 | 31 | 0 |
| Private shared candidate | 14/35 | 21 | 0 |

Both background-attachment and visible-overflow neighbors become exact at all five scales: ten gains, no exact loss and no worsened comparison in this matrix. Hidden local-background coverage remains different. The candidate has 38 successful commands, 16 fresh linked framework artifacts, and unchanged final source identities. It remains private and unintegrated. Full focused, primitive, original, expanded and workspace/ABI qualification are still required.

The previous owner disappeared after 18 completed commands and eight border images. Its receipt remains incomplete and its whole exit is unknown. The separate recovery verifies those logs/images, renders the other 27 cases and checks unchanged Main/private source hashes. It does not rewrite the interrupted receipt or claim that run finished.

## Native editing evidence

A private input metadata implementation matches eleven Chromium scenarios and 69 callback rows in Rust, C and C++, twice each. Its first whole run failed a C guard's expected status. A later guard-only correction passes C and C++ twice; the original failed run and subsequent interrupted run are preserved separately. The framework binaries remain attributed to their actual build source.

Composition still differs in all four measured common-field scenarios. Additional repeated Chromium traces define required native behavior for composition callback mutation, cross-target focus, direct editing commands, readonly/disabled/detached controls and reactivation. They are reference evidence, not native API passes.

Range replacement needs a public Rust method, selection direction metadata and deferred native event delivery. Eight Unicode cases with representable scalar boundaries and twelve selection-task scenarios repeat byte-identically. Chromium coalesces selectionchange while retaining one select callback per changed selection; callbacks read the final retained state. Existing UTF-8/grapheme selection positions cannot represent every UTF-16 position in the browser reference. Those differences remain explicit; no fixed expected value or script execution substitutes for native behavior.

## Qualification remains open

Main renderer totals are unchanged by this private trial. All native APIs, full Chromium pixel equality, compositor, hardware and release qualification remain unfinished. Chromium references are immutable, comparison tolerance is zero, and old Open UI images are historical evidence.

The [evidence index](generated/native-editing-followup-v1.json) records source identities, actual command results, failed/interrupted receipts, reference traces and the completed border candidate.
