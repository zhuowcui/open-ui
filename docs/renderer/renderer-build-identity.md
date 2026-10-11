# Renderer executable source checks

Pixel qualification must identify both the source and the executable that
actually produced the images. Recording the current Git tree beside an older
executable can attribute results to a renderer change that was never built.

The offline `pixel_compare` Cargo build now records its Git commit, worktree
status, source-tree hash, qualification-harness hash and Skia build arguments
inside the executable. `pixel_compare build-source-identity` returns that
record. The matrix runner checks it against the current source before it reads
cached renderer results or starts a comparison. An older executable without
this record, or one built from different source, is rejected. Rebuild the
`pixel-compare` package after changing source; building only a native example
does not update the comparison executable.

The matrix runner also checks the source and executable after the run. A
change during the run prevents qualification, even if every image is exact.
Dirty-source runs remain diagnostic evidence. This check does not relax the
zero-tolerance Chromium gate or replace the clean-source release requirement.

The [verification index](generated/renderer-build-identity-v1.json) records
the compiled executable accepting its actual build source and rejecting both
a changed source identity and an older executable. A single-case matrix
integration passes with unchanged source and binary checks and remains
explicitly nonqualifying. The 241 CI Python tests, three packaging tests and
eight read-only generators pass.

This build check belongs to the offline comparison tool. It requires Python 3
and Git; consuming Rust applications do not depend on that tool. Application
interaction uses public native Rust methods and callbacks. Open UI executes
no JavaScript.

## Chromium capture shutdown

Clean umbrella `5779f376` fixes an observed capture-cleanup race. The old
harness could write the expected PNG, then report a failure while removing
Chromium's temporary profile: a browser child was still writing its `Default`
directory. Moving parent shutdown earlier did not fully resolve it; that
prototype's failure remains in the evidence.

The harness now starts each browser in its own session, closes its CDP client,
stops the browser and its process group, and waits for live Linux profile
writers to exit before removing the profile. Viewport checks, stable screenshot
checks and zero pixel tolerance remain intact. Browser capture flags, fixture
bytes, fonts and reference images are unchanged.

Five regression tests cover successful capture, CDP failure, invalid viewport,
stalled shutdown and an orphaned profile writer. All 47 related Python tests
pass. Six independent prototype captures and two clean production captures
reproduce the preceding reference PNG bytes with no cleanup errors. The
[source-owned report](generated/native-control-intrinsics-v1.json) preserves
the failed attempts, source identities, completed logs and image hashes.

Changing the harness creates a new capture identity. The complete fresh
original census preserves every native image, but ten Chromium captures have
different bytes from the preceding protocol. The full audit fails; the earlier
small selections did not establish full-census reference stability. Both
reference sets remain immutable. The [controls and focus evidence](generated/native-controls-focus-v1.json)
records every changed row and the actual terminal failure. A possible font
strike association is diagnostic evidence, not a reviewed runtime cause.
Original results are 21,330/22,924 exact and expanded results 22,133/23,728,
zero errors; both pixel gates remain red. This correction admits no new
renderer comparison or release result.

## Reproduced reference identity conflict

Clean umbrella `3ee3f9d3` reproduces the discrepancy with the unmodified
production capture path. Three independent fresh captures each of
`float-nowrap-3` at 375×667@2 and `float-nowrap-4-ref2` at 1280×720@1.25
match their preceding Chromium reference and differ from their current cached
reference. The exact recorded oracle identities are equal, including browser
and harness hashes, every font/resource byte, fixtures, profile dimensions,
backend and feature flags. The immutable cached PNGs and entries stay unchanged.
The [source-owned evidence](generated/native-controls-focus-v2.json) includes
all six observations and the original driver/capture failures.

This establishes that the recorded inputs do not yet account for the observed
capture difference. Font strike hypotheses and the separate startup-scale
experiment do not establish its runtime cause. No reference is replaced,
no tolerance is added and no result is admitted as a release pass. Pinned
Chromium alone defines correct pixels.

The same clean combined source completes focused 640/640 and primitive
960/960 exact matrices, original 21,330/22,924 and expanded 22,133/23,728
exact, zero errors. Both full gates exit 1. All nine comparison invariants
across 48,252 rows remain unchanged from the preceding current-protocol
baseline. Native and hosted checks pass for that measured source, while full
renderer qualification and reference reconciliation remain open.

## Discovered attribution error

The development runs under
`out/renderer-evidence/native-opacity-clip-v1/wider-v8` copied the earlier
unbounded-opacity executable, with SHA-256
`7f839af5f6d0d1c88fe8df087d75917afbe4a8a6c1d5f0fa01254a903d71fce3`.
Only the native examples had been rebuilt after the additional Skia build flag
and clip-origin changes. The old executable has no clip-origin symbol; the
fresh comparison executable does. Those wider results cannot verify the
newer implementation. The unfinished original census was stopped; its partial
outputs and the completed diagnostic runs are preserved. Separate native
consumer comparisons used their rebuilt example executables.

Replacement development matrices use the newly built executable under
`fresh-runner-v10`. Focused and primitive matrices are 640/640 and 960/960
exact, with all decoded images and Chromium identities unchanged from the
clean checkpoint. The [complete original census](generated/native-opacity-full-regression-v1.json)
is 21,297/22,924 exact, with 1,627 differences and zero errors. It regresses
23 formerly exact comparisons and makes 12 comparisons exact. The prototype
remains unapplied while those shared paint differences are investigated.
All 22,924 Chromium images and oracle identities remain unchanged. These
development measurements remain nonqualifying. The published clean
`9b158cda` census remains the latest accepted renderer measurement. Chromium
reference bytes and historical Open UI archives remain unchanged.

## Repeated production text variants

Clean `8e8318bf` repeats the same two production cases eight times each. Both
produce two decoded pixel variants under unchanged recorded browser, harness,
fixture, font/resource and profile inputs. The immutable current and preceding
references remain preserved; fresh runs can reproduce either. A separate
250 ms wait after the device-metrics override still produces both variants and
is not applied to the capture harness.

The [source-owned combined evidence](generated/native-input-combined-v1.json)
records all sixteen production captures, the separate diagnostic outcomes and
the font-input audit. The cache identity does not individually record sixty
participating font configurations or inherited font-related environment inputs.
This is an input-accounting gap, not proof those settings changed or caused the
variants. The runtime cause remains open. These observations never change
Open UI's target: pinned Chromium alone defines expected pixels.

## Chromium process and startup diagnostics

The [process diagnostic evidence](generated/chromium-startup-variants-v1.json)
records eight processes and 64 screenshots. Every process produces identical
pixels across its eight screenshot reads, while the mobile case produces two
variants across fresh processes. The desktop case has one variant in this
sample; neither result proves global production determinism.

Sixteen diagnostic launches with an extra startup device-scale argument each
produce a stable third variant, matching neither immutable production reference.
These changed-command diagnostics retain separate identities and never replace
reference images. They do not prove the runtime cause. Pinned Chromium remains
the only pixel target; no reference or capture-harness change is applied.
