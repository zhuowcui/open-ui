# Native single-line input editing clip

Open UI never executes JavaScript. The consuming app creates elements, handles
Rust callbacks, changes typed properties, and reads owned bounds through public
Rust APIs. Needed element behavior must be implemented in the shared engine
and exposed to native apps. Pinned Chromium alone defines expected pixels.

## Reviewed cause and implementation

Chromium's anonymous single-line editor uses its own line box as a scrollport.
Its pinned [anonymous editor style](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/core/html/forms/text_control_inner_elements.cc)
(`TextControlInnerEditorElement::CustomStyleForLayoutObject`) clears
authored line height under specific host-height conditions, sets scrolling
overflow, hides scrollbars and creates a flow root. The native input painter
previously clipped text to a larger part of the host box.

Ten fresh Chromium captures, repeated across five profiles, keep every PNG
byte identical to the original reference. Read-only shadow-tree observations
measure the editor at x=31, y=26, width=78 and height=18 logical pixels, with
normal line height. At scale 1.25 its upper clip edge crosses a physical pixel;
Chromium paints that edge gray rather than solid black.

Clean private `e0f0e577` applies shared line-height handling and an antialiased
editor clip on the unchanged umbrella parent `2827a03d`. It subtracts right
padding from the editor width. It changes no font raster policy, public API,
test reference, or output pixels after rasterization. No font-name, font-size,
or test-ID rule is added. This remains a painter implementation; retained
anonymous layout, calculated-height neighbors, placeholder styles, native
editor scrolling, selection and other contexts still need qualification.

## Native application results

| Clean source | Exact images | Exact rectangles | Outcome |
|---|---:|---:|---|
| Umbrella `2827a03d` | 22/30 | 30/30 | Own-parent baseline |
| Editor clip `e0f0e577` | 28/30 | 30/30 | Six gains, no exact losses or more-wrong states |
| Text raster policy `a062ef3c` on the earlier glyph trial | 18/30 | 30/30 | Rejected: two exact losses and ten more-wrong states against its own parent |
| Editor clip `31def449` on the earlier glyph trial | 26/30 | 30/30 | Six gains against its own parent; still unqualified |

All thirty native images and geometry snapshots repeat byte for byte within
each source. The unchanged app executes fifteen Rust callbacks per run and
checks retained snapshots and teardown. Every corresponding Chromium PNG and
RGBA identity remains unchanged. Both remaining `e0f0e577` failures are at
scale 1.25: before x=39/y=54 and after x=40/y=55, each a 68-pixel black row
where Chromium is white. Text and paint, with native controls, own these open
glyph-mask failures. The accepted parent's ordinary and sticky large-text
states are already exact; the earlier glyph trial introduced those failures.

## Complete renderer checks

| Suite | Exact | Different | Errors | Actual exit |
|---|---:|---:|---:|---:|
| Focused | 640/640 | 0 | 0 | 0 |
| Primitive | 960/960 | 0 | 0 | 0 |
| Full | 21,337/22,924 | 1,587 | 0 | 1 |
| Expanded | 22,140/23,728 | 1,588 | 0 | 1 |

Both complete censuses retain their actual results above. The original gains
three exact comparisons, with no exact losses or worsened comparisons
against the accepted census.
Earlier diagnostics showed six worsened noninput rows in three
`block-size-with-min-or-max-content-1` variants at the desktop profiles.
Their build attribution was not trustworthy, as described below.

The fresh clean umbrella runner checks all twelve changed test IDs across four
profiles (48 comparisons). The patch changes 11 selected comparisons against its own freshly compiled umbrella parent, with 3 exact gains, 0 exact losses and no more-wrong comparison. The earlier six noninput differences do not establish regressions caused by the editor clip.
This selection is a diagnostic, not a new complete umbrella census.

Every Chromium PNG, RGBA and oracle identity agrees with the prior complete
censuses. Focused and primitive preserve all native bytes. Every current stage
runs with freshly compiled workspace libraries and its own measured source.
Pixel tolerance is zero.

## Build cache diagnosis

Cleaning only `pixel-compare` produced a new executable with the correct
embedded checkout identity, but did not bind its cached workspace libraries
to that checkout. An apparent clean umbrella baseline then produced the same
48 selected images as the private trial, including its input changes and six
noninput differences. The library dependency files used relative workspace
paths. Earlier source identity checks alone could not prove those libraries'
source.

After all local workspace packages are cleaned, the baseline freshly compiles
all nine linked crates from the umbrella checkout. Its 48 selected comparisons
match every accepted pixel and oracle invariant. This reproduces stale library
reuse; there is no contradictory Chromium image. Earlier mixed-library runs
are preserved as unqualified diagnostics, with their actual exits and bytes.
The current trial also freshly compiles all nine linked crates before rendering.

The build safeguard cleans local packages and verifies Cargo artifact
records for fresh compilation and the measured manifest and source paths.
It also requires an unchanged checkout and hashes the copied runner. Cached,
foreign or missing library records cannot qualify a new source identity.
Future matrix runs require its successful receipt and check its binary hash
and source identity; the receipt must stay unchanged through the whole matrix.
This is a build safeguard, not a new pixel pass or native API completion.

## Native interaction policy

The [versioned mutation audit](generated/javascript-mutation-audit-v4.json)
and manifest `expanded-v35.json` preserve all 201 candidate IDs and every AST
entry from the preceding audit. This revision changes policy wording only;
it admits no new pixel or API pass. Open UI never runs JavaScript, in any
version. Pixel exclusion or native fixture lowering never completes or waives
a needed public Rust API, its state changes, or its Rust callbacks. The older
audit, manifests and qualification evidence are unchanged.

## Failures and qualification limits

The first full run stops at its disk-space guard and remains incomplete. It
does not run expanded. Its entire process group is stopped before retry. Eight
unused historical binaries are moved to a larger filesystem only after process
and inode audits; their original paths remain symlinks and every hash is
verified. Live Codex history, reference bytes and active resources are untouched.
The retry completes both censuses using the same source-verified runner.

The editor source passes sixteen applicable read-only checks. Two additional
format commands fail because they name tests that exist only in a rejected
private glyph lineage; those errors are preserved and are not passes. The
workspace and C/ABI suite are not run on this source. Every pipeline retains
clean, unchanged source identities for its whole lifetime, including stage gaps.

The [versioned index](generated/native-input-editor-scrollport-v1.json) and
[completed archive](evidence/native-input-editor-scrollport-v1/completed-evidence.tar.gz)
preserve applications, lockfiles, reviewable patches, primary source bytes,
fresh shadow observations, repeated captures, difference regions and channel
deltas, actual exits, logs and complete matrix summaries. Every archive member
is independently hash verified. Existing archives remain unchanged.

All local owners are terminal. The implementation remains **private and
unapplied**, with no newly admitted release state. Accepted results remain
**21,334/22,924 original** and **22,137/23,728 expanded** exact. Full native
APIs, original pixel parity, residual ownership and release qualification
remain unfinished. Aggregate improvements do not establish a passing gate.
