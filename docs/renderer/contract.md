# Renderer qualification contract

Open UI accepts native structure, state, typed styles, and immutable resources.
Its rendering target is pinned Chromium 147. Open UI never executes JavaScript,
in this or future versions. HTML parsing, navigation, networking, iframe
browsing contexts, storage, and media playback
are not part of the renderer contract.
Application interaction is implemented through the public native Rust API,
including retained `Document` and `Element` methods and Rust callbacks. A
test fixture that reaches a state through native Engine operations establishes
rendering evidence for that state. Each element behavior needed by a consuming
application also requires a public Rust operation over the same engine; fixture
lowering alone does not complete application API coverage.

The [clean private SVG evidence](generated/native-svg-viewport-v2.json)
implements a typed Rust foreignObject viewport constructor over the shared
Engine, verifies native mutation callbacks and teardown, and removes the
historical decoration alpha through shared layout and painting. Its later
curved double-border painter gains 112 exact native comparisons, with all
240 changed images improved and no exact loss. Across 1,920 controls covering
all four physical border sides, every owned bound and callback/teardown check
passes; 940 rendered states are exact and 980 remain failures. The earlier clean SVG
checkpoint passes both complete 40-profile gates with every comparison
invariant unchanged. Complete original and later raster reruns are running;
expanded and workspace qualification remain pending. These unapplied patches
do not establish a new accepted full-census result or completed SVG support.

The [native style/RGBA checkpoint](../v02/generated/native-primitive-styles-v1.json)
at clean `9e0f0145` verifies 35 primitive longhands through public Rust, C, and
C++ consumers. Its locked workspace passes 8,515 tests with 13 ignored, and
its complete focused/primitive matrices remain 640/640 and 960/960 exact.
Its [own complete original and expanded runs](generated/native-viewport-full-v17.json)
now finish with observed exits 1: 21,308/22,924 and 22,111/23,728 exact, zero
errors. Every comparison invariant remains unchanged from the accepted
viewport renderer. These complete failures keep the full pixel gate open. The separate [private sampling/corner candidate](generated/native-viewport-full-v14.json)
completes both censuses with zero errors and 21,299/22,924 original and
22,102/23,728 expanded exact. It loses 23 previously exact comparisons and
gains 14; every Chromium image and oracle identity is unchanged. A clean
configuration-only selected sweep reproduces all changed rows. Its image
sampling correction remains unresolved, and the candidate stays unapplied.
The subsequent [generated-tile investigation](generated-image-sampling.md)
identifies the generated background's backing format as the cause of 18 of
those exact regressions. Clean private `16fc959d` repairs all 18 in the
304-comparison selection and preserves all 850 exact native controls; five
exact regressions remain. This selected diagnostic does not establish a new
full-census result. The format correction and sampling configuration remain
unapplied to the umbrella renderer.

A later clean `b3c54ea8` composition trial restores the four scrolling losses
by following Chromium's `Src` draw for opaque quads. Its affected selection is
172/304 exact, zero errors, with all other 300 comparisons and all 850 native
controls unchanged. One SVG pixel regression remains. Background-color opacity
alone is insufficient layer metadata; conservative coverage/clipping/effect
proof and neighboring controls are required before adoption. The
[follow-up](generated-image-sampling.md#scroll-layer-composition-follow-up)
also rejects direct scroll-content replay, which worsens all four failures.
These results remain selected diagnostics, with no new full-census total.
Its [complete raster evidence](generated/native-viewport-full-v16.json) is
640/640 focused and 960/960 primitive exact, with observed exits 0 and all
1,600 comparison invariants unchanged. These gates do not qualify the full renderer.

The later clean `6368057f` [coverage-region trial](generated/native-viewport-full-v17.json)
uses actual raster clips, transformed physical coverage, region union and the
visible linear-sampling footprint. It retains every selected comparison from
`b3c54ea8` and all 850 existing native controls. Its complete focused and
primitive gates remain 640/640 and 960/960 exact. New consuming Rust apps
exercise 180 neighboring states through public methods and Rust callbacks:
95 pixels are exact; all owned bounds, offsets and client dimensions agree
with Chromium, while every scroll extent still fails. Sixteen controls stop
erasing the opaque canvas to transparency. The historical SVG `49/50`
decoration alpha remains unmodeled by the coverage proof and must be removed
through shared SVG layout/paint and native API work. These implementations
remain unapplied; their complete candidate census and other release gates
are still required. None of these new controls is admitted as a release pass.

The earlier complete clean `9b158cda` results are
[21,308/22,924 exact](generated/four-profile-census-v49.json) for the original
four-profile suite and [22,111/23,728 exact](generated/expanded-requalification-v32.json)
for the expanded suite, both with zero errors. The
[full delta](generated/native-image-full-delta-v1.json) verifies 30 improved
comparisons, 17 newly exact, no worsened comparison and no exact regression
from `056421db`; every Chromium image and oracle identity remains fixed.
All original rows agree between the suites. The same 200 of 201 additions
remain exact at all four profiles; the failing addition stays in the contract.
The clean [40-profile raster index](generated/focused-primitive-raster-v51.json)
remains 640/640 focused and 960/960 primitive exact with unchanged pixels.
The 1,616 original differences and 892 unreviewed residual IDs keep the
renderer gate open. The older checkpoint records below remain provenance.

The subsequent shared raster opacity correction is committed at umbrella
checkpoint `42cce619`. Its [completed clean evidence](generated/native-png-sampling-v3.json)
retains every original and expanded native and Chromium pixel, with the same
counts above and exact 640/640 focused and 960/960 primitive matrices. Those
matrices ran on clean `fccbcccb`; all code and build inputs match the umbrella
checkpoint, with only three documentation files differing. Fresh clean
umbrella builds reproduce all 880 native image controls. They gain 210 exact
comparisons with no exact regression; one already failing clipped-image edge
worsens and remains open. This is a renderer checkpoint, not a declaration
of release qualification.

The subsequent [neutral PNG gamma correction](generated/native-png-sampling-v4.json)
is committed at `63aeb672`. Complete clean `ebbe2b6f` matrices retain the same
original and expanded exact counts, with three original images changed and
no exact regression. The border-image comparison improves from 500 to 241
wrong pixels. Two already failing overflow-image comparisons keep 94 wrong
pixels each; each has nine newly differing green-channel cells while its
total absolute channel error falls from 211 to 195. Those residuals remain
failures under `openui-paint` ownership. All Chromium images and identities
remain fixed. The focused and primitive matrices remain 640/640 and 960/960
exact, with unchanged pixels. Fresh clean umbrella builds reproduce all 880
native images and owned bounds. Only documentation differs between the
qualified source and umbrella code/build inputs; the full matrices are
attributed to their actual clean source. No release gate is closed by these
counts.

Static media presentation may consume a generated first frame. Those pixels
are decoded ahead of rendering by the Chromium-matched codec revision, bound
to the source bytes and decoder by SHA-256 in
`media-first-frames-v1.json`, and transported as dimensioned sRGB RGBA8. This
does not add playback, timing, or an ambient host-codec dependency to the
renderer.

The generated v2 contract records four complete-suite profiles and the
40-profile focused viewport/scale cross-product. The 800×600@1 profile refers
to the existing 5,731-case test inventory by hash. The generator refuses to
rewrite or accept drift in that historical evidence. That baseline preserves
historical pass records from a comparator with a four-level channel tolerance
and a 15-pixel excluded right strip. Its 5,731 passes are archival
accountability data, not expected output bytes or a release gate. Exact
Chromium equality must be established by the separate zero-tolerance
qualification matrix. The [frozen oracle audit](frozen-oracle-audit.md)
records the discrepancy.
The Chromium source/API inventory remains pinned to `147.0.7727.24`; Linux
pixel qualification records and verifies the installed `147.0.7727.50`
Chrome-for-Testing raster oracle used by the frozen comparison harness.

Layout, input, hit testing, scrolling, selection, and accessibility use logical
CSS pixels. Paint commands are recorded in logical coordinates and replayed to
the profile's physical surface after applying device scale. Whole-frame
resampling is not a conforming presentation path.

`author-style-inventory.json` classifies every `ComputedStyle` field observed
by the layout and paint source trees. Engine bookkeeping has an explicit
`internal` classification; all remaining fields are author-facing typed
properties. An unclassified field is a qualification failure.

`javascript-disposition.json` preserves the immutable 393-case final-state
candidate inventory from historical WPT files that contain scripts. Offline
test tooling uses the Acorn copy in the pinned Chromium checkout to parse
those scripts into an ordered mutation IR in `javascript-mutation-audit-v3.json`.
Open UI does not execute that JavaScript. Native Rust test fixtures reproduce
only deterministic final visual states; a fixture is admitted only when its
Engine operations are exact against Chromium across all four profiles. A
script's behavioral or nonvisual outcome is outside **pixel** admission, but
an application-needed interaction remains a native API obligation. Every
rejected case carries an AST-derived reason; porter syntax is never a final
disposition. Product interactions must be available through the public
`openui` Rust API; test-only Engine lowering does not establish that coverage.
The [native adjoining-floats admission](adjoining-floats-native-admission.md)
versions one newly exact case into the expanded contract. The other 35
AST-lowered candidates remain pending. All 200 prior additions remain in the
new manifest, including one that currently fails a profile; admission never
turns their failing comparisons into passes.

At clean checkpoint `15f9f12d`, the
[v41 original census](generated/four-profile-census-v41.json) is
21,266/22,924 exact against cached Chromium captures, with 1,658 differences
and zero errors. The shared
[circular background one-axis clip repair](circular-background-one-axis-clip.md)
makes the F16 circular fill honor the same horizontal overflow scissor as the
direct rounded fill. One original comparison became exact, only that Open UI
image changed, and no exact comparison regressed. All 22,924 Chromium oracle
identities and decoded images stayed fixed. The 914 residual test IDs remain
unowned. The clean
[v42 focused/primitive index](generated/focused-primitive-raster-v42.json)
is 640/640 and 960/960 exact, with all 1,600 images unchanged from v41.
The complete [v24 expanded requalification](generated/expanded-requalification-v24.json)
is 22,069/23,728 exact, with 1,659 differences and zero errors. All original
results match the separate clean census, all 804 addition images stayed fixed,
and all 23,728 Chromium oracle identities and decoded images match v23.
The same 200 of 201 additions are exact at all four profiles. The
[v26 diagnostic selection](../../tools/qualification/manifests/expanded-v26.json)
lists those 200; the release contract retains all 201 and the remaining
fieldset legend failure.

The later complete clean run at `e51d88fd` is recorded in
[v42](generated/four-profile-census-v42.json): 21,278/22,924 exact, 1,646
different, and zero errors. Its full delta found three formerly exact
regressions in an extracted-spanner continuation, so this checkpoint is not
accepted as a renderer qualification. The first source correction restores the
spanner row's continuation ownership; its 116-comparison diagnostic retains
12 new exact comparisons with no exact regression. Its later complete clean
run at `0ad4b12f` is [v43](generated/four-profile-census-v43.json):
21,278/22,924 exact, 1,646 different, and zero errors, with 909 unowned residual
IDs. The [full delta](generated/native-geometry-full-delta-v1.json) recovers
three nested-column failures but introduces three ordinary-spanner failures at
1.25×. This checkpoint remains unqualified. Its
[expanded run](generated/expanded-requalification-v26.json) is
22,081/23,728 exact, with 1,647 differences and zero errors; all original rows
match the separate census, and 200 of the declared 201 additions remain exact
at all four profiles. Its
[focused/primitive matrices](generated/focused-primitive-raster-v45.json)
are exact with all 1,600 images unchanged from `e51d88fd`. The
[investigation](native-empty-block-fragment-slicing.md) records the failures.
Chromium inputs remain fixed.

The complete clean `822e0462` [v44 census](generated/four-profile-census-v44.json)
is 21,291/22,924 exact, with 1,633 differences, zero errors, and 900 unowned
residual IDs. Its [full delta](generated/native-spanner-boundary-full-delta-v1.json)
records 13 new exact comparisons and no exact regression from `0ad4b12f`.
Twenty-six Open UI images change, while all Chromium images and oracle
identities stay fixed. Two existing fieldset differences grow by 87 pixels
each. Its [expanded run](generated/expanded-requalification-v27.json) is
22,094/23,728 exact, with all original rows agreeing with the separate census
and the same 200 of the declared 201 additions exact at all four profiles.
The subsequent [native constrained-box repair](../v02/native-element-geometry.md#constrained-boxes-and-visible-child-overflow)
uses shared box-ownership data for application geometry and input. Its clean
`547c7081` [v45 census](generated/four-profile-census-v45.json) retains
21,291/22,924 exact, 1,633 differences, zero errors and 900 unowned residual IDs.
The [complete delta](generated/native-constrained-box-full-delta-v1.json)
records all original and expanded Open UI RGBA images, Chromium RGBA images
and oracle identities unchanged from `822e0462`. Every original row agrees
with the separate expanded run. The clean
[v28 expanded index](generated/expanded-requalification-v28.json) retains the
same 200 of 201 additions exact at all four profiles. Clean
[v47 focused/primitive matrices](generated/focused-primitive-raster-v47.json)
are 640/640 and 960/960 exact. No new renderer pass or full qualification
is claimed from unchanged images; the remaining failures stay open.

At clean checkpoint `822e0462`, the
[v46 focused/primitive index](generated/focused-primitive-raster-v46.json)
is 640/640 and 960/960 exact; all Open UI images, Chromium images, and oracle
identities are unchanged from `0ad4b12f`. The broader
[column/fragmentation diagnostic](generated/native-spanner-boundary-cohort-diagnostic-v1.json)
is 7,034/7,680 exact, with 646 differences and zero errors. Thirteen comparisons
become exact with no exact regression, but two existing fieldset differences
worsen by 87 pixels each. It has dirty diagnostic source and partial scope;
it does not qualify the renderer. Complete clean original and expanded runs
at `822e0462` are indexed above.

The later [scaled LCD font investigation](scaled-lcd-hinting-oracle-investigation.md)
found an older cached Chromium image that differs from six agreeing fresh
captures of `wpt/css2_floats/float-nowrap-3` at 1920×1080@1.5. Both variants have
the same recorded oracle identity. The
[read-only oracle audit](generated/chromium-font-oracle-audit-v1.json)
reports the contradiction and explicitly does not qualify a renderer. The
root cause remains unreviewed. Both images, both input records, and the exact
HTML reproducer are preserved; no cache entry, historical archive, or pinned
oracle was rewritten. The renderer experiment was rejected after 43 exact
regressions and restored. The existing clean census is evidence against its
cached inputs; final qualification also requires this identity discrepancy
to be reconciled.

At prior clean checkpoint `e14e3e64`, the
[v39 original census](generated/four-profile-census-v39.json) was
21,264/22,924 exact, with 1,660 differences and zero errors. The shared
[vertical-lr Ahem rotation-anchor repair](vertical-lr-ahem-rotation-anchor.md)
made nine original comparisons exact, changed only those nine Open UI images,
and reduced wrong pixels by 30,184. No exact comparison regressed, and all
22,924 Chromium oracle identities and decoded images stayed fixed. The 916
residual test IDs were unowned. The clean
[v40 focused/primitive index](generated/focused-primitive-raster-v40.json)
was 640/640 and 960/960 exact. The complete clean
[v22 expanded requalification](generated/expanded-requalification-v22.json)
was 22,066/23,728 exact, with 1,662 differences and zero errors. All original
decoded images and statuses match the separate clean census; the one changed
addition became exact at four profiles, leaving 199 of 201 additions exact.
Every Chromium oracle identity and decoded image matched the prior clean
expanded run. The
[v24 diagnostic selection](../../tools/qualification/manifests/expanded-v24.json)
listed 199 exact additions, while the release contract included all 201 and
their two failures at that checkpoint.

At the prior clean checkpoint `d39282e4`, the [v38 original census](generated/four-profile-census-v38.json)
is 21,255/22,924 exact, with 1,669 differences and zero errors. The shared
[analytic gradient-edge repair](fractional-multicol-seam-investigation.md)
made seven comparisons exact and reduced wrong pixels by 5,957 without an
exact regression. Exactly 29 Open UI images changed, and all 22,924
Chromium oracle identities and decoded images stayed fixed. The 923 residual
test IDs remain unowned. The clean
[v39 focused/primitive index](generated/focused-primitive-raster-v39.json)
is 640/640 and 960/960 exact; all 1,600 Open UI and Chromium images are
unchanged from v38. The clean
[201-addition guard](generated/expanded-additions-gradient-guard-v1.json)
is 801/804 exact with the same three failures and no decoded image changes.
The complete [v21 expanded requalification](generated/expanded-requalification-v21.json)
at clean checkpoint `e59ec07e` is 22,056/23,728 exact, with 1,672
differences and zero errors. Every original and added decoded image, status,
and Chromium oracle identity matches the separate clean census and addition
guard. The [v23 diagnostic selection](../../tools/qualification/manifests/expanded-v23.json)
retains 198 exact additions, while the release contract still includes all
201 and their three failures.

At clean checkpoint `dc451061`, the complete
[v20 expanded requalification](generated/expanded-requalification-v20.json)
is 22,049/23,728 exact, with 1,679 differences and zero errors. The same
198 of 201 additions are exact at all four profiles; the other three remain
failures. All original profile results match the clean v37 census, including
every pinned Chromium oracle identity and decoded image. The
[v22 diagnostic selection](../../tools/qualification/manifests/expanded-v22.json)
records the 198 exact additions; the release contract still includes all 201.

At the prior clean checkpoint `6b53a991`, the complete
[v19 expanded requalification](generated/expanded-requalification-v19.json)
is 22,046/23,728 exact, with 1,682 differences and zero errors. The same
198 of 201 additions are exact at all four profiles; three remain failures.
All original profile results and all added profile results match their clean
v36 census and selected-additions guard respectively, including every pinned
Chromium oracle identity and decoded image hash. The
[v21 diagnostic selection](../../tools/qualification/manifests/expanded-v21.json)
records the 198 exact additions; the release contract still includes all 201.

At clean checkpoint `dc451061`, the [v37 original census](generated/four-profile-census-v37.json)
is 21,248/22,924 exact, with 1,676 differences and zero errors. The
[adjacent row-flex continuation repair](adjacent-row-flex-fragmentation.md)
made three comparisons exact and reduced one other difference from 132 to
14 pixels. Only four Open UI images changed, no exact image regressed, and
all 22,924 Chromium oracle identities and decoded images stayed fixed. The
924 residual test IDs remain unowned. The clean
[v38 focused/primitive index](generated/focused-primitive-raster-v38.json)
is 640/640 and 960/960 exact; all 1,600 Open UI and Chromium images are
unchanged from the prior raster v37 index.

At the prior clean checkpoint `128edc38`, the [v36 original census](generated/four-profile-census-v36.json)
is 21,245/22,924 exact, with 1,679 differences and zero errors. The
[fractional Ahem strike repair](fractional-ahem-stripe-edges.md) changed 21
already failing Open UI images and reduced wrong pixels by 281. No exact case
regressed, and all 22,924 Chromium oracle identities and decoded images stayed
fixed. The 926 residual test IDs remain unowned.

At clean checkpoint `6676cf60`, the [v35 original census](generated/four-profile-census-v35.json)
is 21,245/22,924 exact, with 1,679 differences and zero errors. The
[full image-delta audit](generated/radial-ua-full-delta-v1.json) records nine
changed Open UI images, one new exact result, and no exact regression. All
22,924 Chromium oracle identities and decoded hashes stayed fixed; 926
residual test IDs remain unowned. The shared
[radial tile repair](radial-tile-edge-coverage.md) reduced two 1.25×
differences from 626 pixels to one each. The
[object fallback investigation](object-fallback-host-clip.md) records why the
`object` host clip was deferred after an intermediate exact-case regression.
The [fractional Ahem stripe investigation](fractional-ahem-stripe-edges.md)
records a shared text and border edge mismatch in multicolumn source and plain
reference pages; its residual cause and exact repair remain open.
The clean [v37 focused/primitive index](generated/focused-primitive-raster-v37.json)
at `e1bdccc4` is 640/640 and 960/960 exact with zero errors and no decoded
image changes from v36. The clean
[201-addition guard](generated/expanded-additions-object-deferred-v1.json)
is 801/804 exact with three differences and no changed Open UI or Chromium
image; the complete expanded-manifest rerun was open at that checkpoint and
has since completed in v20.

At clean checkpoint `d70c4696`, the [v33 original census](generated/four-profile-census-v33.json)
was 21,244/22,924 exact with 1,680 differences and zero errors. The
shared broken-image host content clip improved two already failing comparisons
without changing any other Open UI image. Every Chromium oracle identity and
decoded hash stayed fixed; 926 residual test IDs remain unowned.
The clean [v34 focused/primitive index](generated/focused-primitive-raster-v34.json)
was 640/640 and 960/960 exact with zero errors.

At clean checkpoint `09683348`, the [v32 original census](generated/four-profile-census-v32.json)
is 21,244/22,924 exact with 1,680 differences and zero errors. The
[fixed-point broken-image repair](broken-image-fractional-sampling.md) made
five 1.5× comparisons exact without regressing any previously exact image.
All 22,924 Chromium oracle identities and decoded hashes stayed fixed. The
complete [v18 expanded requalification](generated/expanded-requalification-v18.json)
is 22,045/23,728 exact, with 1,683 differences, zero errors, and the same
198 of 201 additions exact at all four profiles. The
[v33 focused/primitive index](generated/focused-primitive-raster-v33.json)
is 640/640 and 960/960 exact. The original renderer gate still has 926
unowned residual test IDs. The 35 [pending candidates](generated/pending-mutation-candidates-v7.json)
were requalified at clean checkpoint `8324c6b0`: 18/140 exact comparisons,
122 differences, zero errors, and none exact at all four profiles. All 140
comparison statuses, mismatch counts, and diff signatures match the prior
pending report.
At clean checkpoint `9534c9f6`, the [v31 original census](generated/four-profile-census-v31.json)
is 21,239/22,924 exact with 1,685 differences and zero errors. The complete
[v17 expanded requalification](generated/expanded-requalification-v17.json)
is 22,040/23,728 exact, with 1,688 differences and zero errors; 198 of 201
additions are exact at all four profiles. The
[v32 focused/primitive index](generated/focused-primitive-raster-v32.json)
is 640/640 and 960/960 exact. The 35 remaining
[pending candidates](generated/pending-mutation-candidates-v6.json)
are 18/140 exact comparisons, with none exact at all four profiles. All
23,724 prior expanded results retain their decoded Open UI and Chromium
hashes, statuses, and diff signatures. The original renderer gate still has
931 unowned residual test IDs.
At clean checkpoint `8950b426`, the 36 AST-lowered pending cases produced
22/144 exact comparisons, 122 differences, and zero errors. One case,
`adjoining-floats-dynamic`, is exact at all four required profiles and is
eligible for a future expanded-manifest admission; it is not counted as
admitted coverage yet. The other 35 remain open. The
[v2 pending-candidate evidence index](generated/pending-mutation-candidates-v2.json)
records every profile result and the ordered native mutation IR identity.
At clean checkpoint `5acc962a`, the 36 AST-lowered pending cases produced
21/144 exact comparisons, 123 differences, and zero errors. None was exact
at all four required profiles, so none was admitted. The
[pending-candidate evidence index](generated/pending-mutation-candidates-v1.json)
records every profile result and links each case to its ordered mutation IR.
At clean checkpoint `93b00601`, the
[200-addition diagnostic recheck](generated/expanded-additions-recheck-v1.json)
found 797/800 exact profile comparisons and zero errors. All 800 Open UI
decoded hashes and Chromium oracle identities and decoded hashes matched the
prior clean expanded requalification; the same three additions remain
demoted. This selected-ID report has `complete_contract_scope=false` and is
not a complete expanded-manifest qualification.
At clean checkpoint `db763999`, the [v2 selected-additions
recheck](generated/expanded-additions-recheck-v2.json) again found 797/800
exact, three different, and zero errors. Every one of its 800 Open UI
decoded hashes and Chromium oracle identities/decoded hashes matched the
preceding full expanded run. It uses the versioned
[`expanded-additions-v2.json`](../../tools/qualification/manifests/expanded-additions-v2.json)
ID list, and it remains a diagnostic selection with
`complete_contract_scope=false`.
At clean checkpoint `2f560e46`, the
[v3 selected-additions recheck](generated/expanded-additions-recheck-v3.json)
again found 797/800 exact, three different, and zero errors. All 800 Open UI
and Chromium decoded hashes and statuses matched v2. This selection does not
replace a complete expanded-manifest run at the new renderer commit.

Application and system font ownership, registration limits, and C handle
lifetime rules are documented in [font collections](font-collections.md).
Font selection and shaping precedence is documented in
[font selection and shaping](font-selection-and-shaping.md), and the shared
layout/paint flow is documented in [text layout and paint](text-layout-and-paint.md).

`tools/qualification/run_renderer_matrix.py` uses separate, contract-pinned
manifests for the complete 5,731-case census and the focused raster corpus.
The focused and primitive corpora are exact-equality gates across all 40
viewport/scale profiles. Residual investigations use `--suite residual` for
the five-scale 800×600 sweep plus the eight contract viewports at 1×, and use
`--suite residual-cross` when that sweep does not isolate the interaction.
The four-profile complete run is also qualifying only when it is
complete, decoded-RGBA exact, error-free, and produced from a clean source tree.

At clean checkpoint `8ca66ffd`, the CPU runner produced 640/640 exact focused
comparisons and 960/960 exact primitive comparisons. The
[v16 raster index](generated/focused-primitive-raster-v16.json) validates all
40 profiles in each suite with zero errors. All 1,600 Open UI and Chromium
decoded hashes match the preceding clean v15 raster run. The
[background-clip investigation](background-clip-hard-clip-investigation.md)
records the square inset-background paint repair.

At clean checkpoint `2dce665c`, the CPU runner produced 640/640 exact focused
comparisons and 960/960 exact primitive comparisons. The
[v15 raster index](generated/focused-primitive-raster-v15.json) validates all
40 profiles in each suite with zero errors. All 1,600 Open UI and Chromium
decoded hashes match the preceding clean v14 raster run. The
[flex negative-margin investigation](flex-negative-margin-investigation.md)
records the anonymous-line background repair and its fragmentainer guard.

At clean checkpoint `2ff236ce`, the CPU runner produced 640/640 exact focused
comparisons and 960/960 exact primitive comparisons. The
[v14 raster index](generated/focused-primitive-raster-v14.json) validates all
40 profiles in each suite with zero errors. All 1,600 Open UI decoded hashes,
Chromium oracle identities, and Chromium decoded hashes match the preceding
clean v13 raster run. The [flex negative-margin investigation](flex-negative-margin-investigation.md)
records the shared intrinsic-sizing repair and the fractional-scale
border/background differences that remained at that checkpoint.
At clean checkpoint `e942aebc`, the CPU runner produced 640/640 exact focused
comparisons and 960/960 exact primitive comparisons. The
[v13 raster index](generated/focused-primitive-raster-v13.json) validates all
40 profiles in each suite with zero errors. All 1,600 Open UI decoded hashes,
Chromium oracle identities, and Chromium decoded hashes match the preceding
clean v12 raster run. The
[column start clip investigation](column-start-clip-investigation.md) records
the shared fragmentainer repair and rejected broad diagnostic.
At clean checkpoint `dad5c9e8`, the CPU runner produced 640/640 exact focused
comparisons and 960/960 exact primitive comparisons. The
[v12 raster index](generated/focused-primitive-raster-v12.json) validates all
40 viewport/scale profiles in each suite with zero errors and zero unowned
residuals. Compared with the [v11 index](generated/focused-primitive-raster-v11.json),
the eight 1.25-scale rounded content-border comparisons became exact. Only
those eight Open UI decoded image hashes changed among the 1,600 comparisons;
all Chromium oracle identities and decoded hashes stayed unchanged. The
focused and primitive CPU gates are exact.
The v11 index records the earlier 952/960 primitive state. Compared with the
[v9 index](generated/focused-primitive-raster-v9.json), its preceding
rounded-border change made 32 comparisons exact.
The v9 index records eight fractional-scale shadow comparisons becoming exact
without changes to the other 1,592 Open UI decoded pixel hashes.
The v8 and v7 indices record four gradient profiles becoming exact after the
renderer selected tiles from the laid-out content footprint and clipped
overflowing paint at the physical viewport edge. The gradient now matches at
all 40 profiles. Earlier indices remain historical evidence.
The [rounded border coverage investigation](rounded-border-coverage-investigation.md)
records the clip-order repair and its earlier diagnostics.
An explicit Ganesh raster run on Mesa llvmpipe completed the same clean
40-profile suites. Its [backend comparison](generated/ganesh-raster-comparison-v1.json)
records 408/640 focused and 624/960 primitive exact, compared with CPU Skia's
640/640 and 900/960. All Chromium oracle hashes matched across backends;
Ganesh changed 508 CPU-exact comparisons to different and repaired none of the
60 CPU primitive residuals. The renderer code was unchanged between the CPU
and Ganesh source checkpoints. This diagnostic does not qualify Ganesh or
change the portable CPU selection.
The runner defaults to `bindings/rust/target/release/pixel_compare`. Rebuild
that executable from the clean checkpoint with the pinned toolchain and pass
its path explicitly using `--pixel-compare`; a recent debug build does not
refresh the release executable. Matrix reports record the executable SHA-256.
The optional historical byte replay uses its own explicitly selected
executable and never substitutes for Chromium qualification.
`expanded-v1.json` preserves the prior 200 AST-lowered admissions. The clean
complete expanded matrix at `1b652919` contains 21,966/23,724 exact
comparisons, 1,758 differences, and zero errors. Of the 200 additions, 197
are exact at all four profiles and three differ at one profile each. Their
statuses and all 800 Open UI and Chromium decoded hashes match the prior
expanded run. The [v2 requalification ledger](generated/expanded-requalification-v2.json)
records this result; `expanded-v3.json` retains only the 197 exact additions.
The later clean complete expanded matrix at `d73077e7` contains 21,976/23,724
exact comparisons, 1,748 differences, and zero errors. Its ten gains are in
the original `css_break` inventory. All 800 Open UI and Chromium decoded
hashes and statuses for the 200 additions match the prior expanded run, and
all Chromium oracle identities across the matrix remain fixed. The
[v3 requalification ledger](generated/expanded-requalification-v3.json)
records this result; `expanded-v4.json` again retains only the 197 exact
additions. The prior [v1 ledger](generated/expanded-requalification-v1.json),
`expanded-v2.json` manifest, and `expanded-v3.json` diagnostic selection
remain unchanged. The v4 selection is diagnostic until a new contract admits
it; neither the 36 originally pending cases nor the three demotions count as
current exact coverage.

An earlier clean expanded matrix at `9f983df9` contains 22,006/23,724 exact
comparisons, 1,718 differences, and zero errors. Its one new exact comparison
is in the original inventory. All 200 native additions retain their prior
four-profile statuses, and all 800 decoded Open UI and Chromium hashes match
the previous clean run. Every Chromium oracle identity remains fixed. The
[v8 requalification ledger](generated/expanded-requalification-v8.json)
records this result; [expanded-v9.json](../../tools/qualification/manifests/expanded-v9.json)
still retains only the 197 four-profile exact additions as a diagnostic
selection. The original manifest and 36 pending candidates remain unchanged.

The clean expanded matrix at `4b89fd05` contains 22,005/23,724 exact
comparisons, 1,719 differences, and zero errors. Its one new exact comparison
is in the original inventory. All 800 Open UI and Chromium decoded hashes
and statuses for the 200 additions match the prior expanded run, and every
Chromium oracle identity remains fixed. The
[v7 requalification ledger](generated/expanded-requalification-v7.json)
records this result; [expanded-v8.json](../../tools/qualification/manifests/expanded-v8.json)
again retains only the 197 exact additions as a diagnostic selection. The
original manifest and 36 pending candidates remain unchanged.

The prior clean expanded matrix at `8ca66ffd` contains 22,004/23,724 exact
comparisons, 1,720 differences, and zero errors. Its 22 gains are in the
original inventory. All 800 Open UI and Chromium decoded hashes and statuses
for the 200 additions match the prior expanded run, and every Chromium oracle
identity remains fixed. The
[v6 requalification ledger](generated/expanded-requalification-v6.json)
records this result; [expanded-v7.json](../../tools/qualification/manifests/expanded-v7.json)
again retains only the 197 exact additions as a diagnostic selection. The
original manifest and 36 pending candidates remain unchanged.

The clean expanded matrix at `2dce665c` contains 21,982/23,724 exact
comparisons, 1,742 differences, and zero errors. Its four gains are in the
original inventory. All 800 Open UI and Chromium decoded hashes and statuses
for the 200 additions match the prior expanded run, and every Chromium oracle
identity remains fixed. The
[v5 requalification ledger](generated/expanded-requalification-v5.json)
records this result; [expanded-v6.json](../../tools/qualification/manifests/expanded-v6.json)
again retains only the 197 exact additions as a diagnostic selection. The
original manifest and 36 pending candidates remain unchanged.

The earlier clean expanded matrix at `2ff236ce` contains 21,978/23,724 exact
comparisons, 1,746 differences, and zero errors. Its two gains are in the
original flex inventory. All 800 Open UI and Chromium decoded hashes and
statuses for the 200 additions match the prior expanded run, and every
Chromium oracle identity remains fixed. The
[v4 requalification ledger](generated/expanded-requalification-v4.json)
records this result; [expanded-v5.json](../../tools/qualification/manifests/expanded-v5.json)
again retains only the 197 exact additions as a diagnostic selection. The
original manifest and 36 pending candidates remain unchanged.

Non-exact runs emit a v2 residual ledger whose pixel bounds, connected regions,
channel deltas, scale behavior, reviewed root cause, owner, and minimized
reproducer are all explicit. Test names and fixture keywords never select an
owner. Unknown ownership fails the qualifying ledger; diagnostic reports retain
their pixel evidence and record the ownership error. A residual ledger never
becomes a tolerance, allowlist, or alternate baseline.

`tools/qualification/summarize_renderer_census.py` verifies the complete,
disjoint shard set against the pinned manifest, profile geometry, report hashes,
and common source/backend identities before emitting a versioned
`generated/four-profile-census-v*.json` evidence index. Its default mode fails
on any unowned residual. `--allow-unowned-diagnostics` explicitly emits a
nonqualifying snapshot for investigation; `--check` verifies that snapshot
without rewriting it. The current
[v30 diagnostic index](generated/four-profile-census-v30.json), with clean
`814a2005` source identity, contains 21,239 exact, 1,685 different, and zero
errors across the four required profiles; 931 residual test IDs remain
unowned. Compared with v29, exactly one Open UI image changed: the truncated
table row-group case became exact at 1.25×, with no exact regression. All
22,924 Chromium oracle identities and decoded hashes stayed fixed. The clean
[v31 raster index](generated/focused-primitive-raster-v31.json) is 640/640
focused and 960/960 primitive exact, with all 1,600 decoded images unchanged.
The complete clean
[v16 expanded requalification](generated/expanded-requalification-v16.json)
is 22,036/23,724 exact, with 197 of 200 prior additions exact and three
demoted; every addition image is unchanged. The
[v17 diagnostic selection](../../tools/qualification/manifests/expanded-v17.json)
retains those 197. The repaired case is exact in the
[40-profile sweep](generated/table-row-group-cross-v1.json). The
[table row-group clip investigation](truncated-table-row-group-clip.md)
records the source rule. The original release gate still fails.

The prior
[v29 diagnostic index](generated/four-profile-census-v29.json), with clean
`7cd8e574` source identity, contains 21,238 exact, 1,686 different, and zero
errors across the four required profiles; 932 residual test IDs remain
unowned. Compared with v28, exactly 12 Open UI images changed in repeated
table sections: 11 comparisons became exact, one residual shrank, and none
regressed. All 22,924 Chromium oracle identities and decoded hashes stayed
fixed. The clean [v30 raster index](generated/focused-primitive-raster-v30.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v15 expanded requalification](generated/expanded-requalification-v15.json)
is 22,035/23,724 exact, with 197 of 200 prior additions exact and three
demoted. The [v16 diagnostic selection](../../tools/qualification/manifests/expanded-v16.json)
retains those 197. The [repeated table body-slice investigation](repeated-table-body-slice.md)
records the shared fragment repair and guard. The original release gate
still fails.

The prior
[v28 diagnostic index](generated/four-profile-census-v28.json), with clean
`a0e3f4cd` source identity, contains 21,227 exact, 1,697 different, and zero
errors across the four required profiles; 937 residual test IDs remain
unowned. Compared with v27, exactly three Open UI images changed: two
margin-trim comparisons became exact and one iframe difference fell from 342
wrong pixels to six. No exact comparison regressed; all 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The clean
[v29 raster index](generated/focused-primitive-raster-v29.json) is 640/640
focused and 960/960 primitive exact. The complete clean
[v14 expanded requalification](generated/expanded-requalification-v14.json)
is 22,024/23,724 exact, with 197 of 200 prior additions exact and three
demoted. The [v15 diagnostic selection](../../tools/qualification/manifests/expanded-v15.json)
retains those 197. The [clipped replaced-background investigation](clipped-replaced-background-coverage.md)
records the shared paint fix and remaining inset-border corner samples. The
original release gate still fails.

The prior
[v27 diagnostic index](generated/four-profile-census-v27.json), with clean
`8950b426` source identity, contains 21,225 exact, 1,699 different, and zero
errors across the four required profiles; 939 residual test IDs remain
unowned. All residual statuses, bounds, pixel counts, and diff signatures
match v26. The clean [v28 raster index](generated/focused-primitive-raster-v28.json)
is 640/640 focused and 960/960 primitive exact. The clean
[v13 expanded requalification](generated/expanded-requalification-v13.json)
is 22,022/23,724 exact, 1,702 different, and zero errors. Its 200 prior
additions retain the same four-profile statuses: 197 exact, three demoted.
The [v14 diagnostic selection](../../tools/qualification/manifests/expanded-v14.json)
retains those 197 exact additions. The
[nested float investigation](nested-inline-float-propagation.md) records the
source fix and the newly eligible pending case; the original release gate
still fails.

The prior
[v26 diagnostic index](generated/four-profile-census-v26.json), with clean
`954648fb` source identity, contains 21,225 exact, 1,699 different, and zero
errors across the four required profiles; 939 residual test IDs remain
unowned. Compared with v25, exactly eight Open UI decoded images changed in
four resized one-axis bitmap test/reference pairs at fractional scales. Two
comparisons became exact, six moved closer, and none regressed. All 22,924
Chromium oracle identities and decoded hashes stayed fixed. The
[resized bitmap pattern investigation](resized-one-axis-bitmap-pattern.md)
records the source rule and remaining pixels. The clean
[v27 raster index](generated/focused-primitive-raster-v27.json) remains
640/640 focused and 960/960 primitive exact, with all 1,600 Open UI and
Chromium decoded images unchanged. The clean
[v5 selected-additions recheck](generated/expanded-additions-recheck-v5.json)
remains 797/800 exact, with all 800 Open UI and Chromium decoded images and
statuses unchanged.

The prior
[v25 diagnostic index](generated/four-profile-census-v25.json), with clean
`5c7aaa9c` source identity, contains 21,223 exact, 1,701 different, and zero
errors across the four required profiles; 939 residual test IDs remain
unowned. Compared with v24, exactly eight Open UI decoded images changed in
four one-axis repeated linear-gradient test/reference pairs at fractional
scales. Each moved closer to Chromium; no exact comparison regressed or became
exact. All 22,924 Chromium oracle identities and decoded hashes stayed fixed.
The [gradient picture-shader investigation](one-axis-gradient-picture-shader.md)
records the source rule and remaining PNG raster edges. The clean
[v26 raster index](generated/focused-primitive-raster-v26.json) remains
640/640 focused and 960/960 primitive exact, with all 1,600 Open UI and
Chromium decoded images unchanged. The clean
[v4 selected-additions recheck](generated/expanded-additions-recheck-v4.json)
remains 797/800 exact, with all 800 Open UI and Chromium decoded images and
statuses unchanged.

The earlier
[v24 diagnostic index](generated/four-profile-census-v24.json), with clean
`2f560e46` source identity, contains 21,223 exact, 1,701 different, and zero
errors across the four required profiles; 939 residual test IDs remain
unowned. Compared with v23, 16 Open UI decoded images changed across four
mixed-border fixtures, four comparisons became exact, and none regressed.
All 22,924 Chromium oracle identities and decoded hashes stayed fixed. The
[mixed-border paint-order investigation](mixed-border-paint-order.md) records
the source rule and remaining pixels. The clean
[v25 raster index](generated/focused-primitive-raster-v25.json) remains
640/640 focused and 960/960 primitive exact, with all 1,600 decoded images
unchanged. The clean
[v3 selected-additions recheck](generated/expanded-additions-recheck-v3.json)
remains 797/800 exact, with all 800 decoded images and statuses unchanged.

The prior
[v23 diagnostic index](generated/four-profile-census-v23.json), with clean
`d0592ccd` source identity, contains 21,219 exact, 1,705 different, and zero
errors across the four required profiles; 939 residual test IDs remain
unowned. Compared with v22, only `out-of-flow-in-multicolumn-047` at
1280×720@1.25 changed its Open UI decoded image, becoming exact from 125
differing pixels. No prior exact comparison regressed. All 22,924 Chromium
oracle identities and decoded hashes stayed fixed. The
[consumed nested columns investigation](multicol-consumed-nested-columns.md)
records the source rules and reduced evidence. The clean
[v24 raster index](generated/focused-primitive-raster-v24.json) remains
640/640 focused and 960/960 primitive exact, with all 1,600 Open UI and
Chromium decoded hashes unchanged from v23.

The prior [v22 diagnostic index](generated/four-profile-census-v22.json), with clean
`6e21bd9b` source identity, contains 21,218 exact, 1,706 different, and zero
errored comparisons across the four required profiles; 940 residual test IDs
remain unowned. Compared with v21, only
`out-of-flow-in-multicolumn-046` at 1280×720@1.25 changed its Open UI decoded
image, becoming exact. No prior exact comparison regressed. All 22,924
Chromium oracle identities and decoded hashes stayed fixed. The
[zero-height continuation investigation](multicol-zero-height-positioned-decoration.md)
records the source rule and reduced evidence. The clean
[v23 raster index](generated/focused-primitive-raster-v23.json) remains 640/640
focused and 960/960 primitive exact, with all 1,600 Open UI and Chromium
decoded hashes unchanged from v22. The complete clean
[v12 expanded requalification](generated/expanded-requalification-v12.json)
measured 22,015/23,724 exact, 1,709 different, and zero errors. Only that
original column comparison changed; all 23,724 Chromium oracle identities
and decoded hashes stayed fixed. Of the 200 native additions, 197 remain
exact at all four profiles and three remain demoted. The
[v13 diagnostic selection](../../tools/qualification/manifests/expanded-v13.json)
does not change `complete-5731.json` or admit a failing case.

The prior [v21 diagnostic index](generated/four-profile-census-v21.json), with clean
`99fd4432` source identity, contains 21,217 exact, 1,707 different, and zero
errored comparisons across the four required profiles; 941 residual test IDs
remain unowned. Compared with v20, only
`flexbox_multi-line-row-flex-fragmentation-029` at 1280×720@1.25 and `-030`
at 1280×720@1.25 and 1920×1080@1.5 changed their Open UI decoded images,
each becoming exact. No prior exact comparison regressed. All 22,924 Chromium
oracle identities and decoded hashes stayed fixed. The
[final-decoration investigation](flex-final-continuation-decoration.md)
records the source-extent repair and rejected broad diagnostic. The clean
[v22 raster index](generated/focused-primitive-raster-v22.json) remains 640/640
focused and 960/960 primitive exact, with all 1,600 Open UI and Chromium
decoded hashes unchanged from v21. The complete clean
[v11 expanded requalification](generated/expanded-requalification-v11.json)
measured 22,014/23,724 exact, 1,710 different, and zero errors. Only those
three original flex comparisons changed; all 23,724 Chromium oracle identities
and decoded hashes stayed fixed. Of the 200 native additions, 197 remain exact
at all four profiles and three remain demoted. The
[v12 diagnostic selection](../../tools/qualification/manifests/expanded-v12.json)
does not change `complete-5731.json` or admit a failing case.

The prior [v20 diagnostic index](generated/four-profile-census-v20.json), with clean
`89a0a1f3` source identity, contains 21,214 exact, 1,710 different, and zero
errored comparisons across the four required profiles; 943 residual test IDs
remain unowned. Compared with v19, only
`flexbox_multi-line-row-flex-fragmentation-027` at 1280×720@1.25 changed its
Open UI decoded image, becoming exact from 125 differing pixels. No prior
exact comparison regressed. All 22,924 Chromium oracle identities and decoded
hashes stayed unchanged. The [first-line break investigation](nested-row-flex-first-line-break.md)
records the shared layout repair and reduced evidence. The clean
[v21 raster index](generated/focused-primitive-raster-v21.json) remains 640/640
focused and 960/960 primitive exact. All 1,600 decoded Open UI images and
Chromium oracle hashes match the preceding clean raster run. The subsequent
complete clean [v10 expanded requalification](generated/expanded-requalification-v10.json)
measured 22,011/23,724 exact, 1,713 different, and zero errors. Only the
same original flex comparison changed from the previous full expanded run;
all 23,724 Chromium oracle identities and decoded hashes stayed fixed. The
200 native additions retained 797/800 exact profile comparisons; 197 remain
exact at all four profiles, and the same three are demoted. The
[v11 diagnostic selection](../../tools/qualification/manifests/expanded-v11.json)
does not change `complete-5731.json` or admit a failing case.

The prior [v19 diagnostic index](generated/four-profile-census-v19.json), with clean
`17952772` source identity, contains 21,213 exact, 1,711 different, and zero
errored comparisons across the four required profiles; 944 residual test IDs
remain unowned. Compared with v18, only `out-of-flow-in-multicolumn-042`,
`-043`, and `-045` at 1280×720@1.25 changed their Open UI decoded images,
each becoming exact from 125 differing pixels. No prior exact comparison
regressed. All 22,924 Chromium oracle identities and decoded hashes stayed
unchanged. The [relative continuation clip investigation](multicol-relative-clip-translation.md)
records the shared repair and reduced evidence. The clean
[v20 raster index](generated/focused-primitive-raster-v20.json) at `ef7214b3`
remains 640/640 focused and 960/960 primitive exact. All 1,600 decoded
Open UI images and Chromium oracle hashes match the preceding clean raster
run. The subsequent complete clean
[v9 expanded requalification](generated/expanded-requalification-v9.json)
at `de8304fe` measured 22,010/23,724 exact, 1,714 different, and zero
errors. Compared with the prior v8 expanded run, four original column
comparisons became exact and no other Open UI decoded image changed. All
23,724 Chromium oracle identities and decoded hashes stayed fixed. The 200
native final-state additions retained 797/800 exact profile comparisons;
197 remain exact at all four profiles, and the same three are demoted. The
[v10 diagnostic selection](../../tools/qualification/manifests/expanded-v10.json)
does not change `complete-5731.json` or admit a failing case. The prior
[v18 diagnostic index](generated/four-profile-census-v18.json), with clean
`8269ea09` source identity, contains 21,210 exact, 1,714 different, and zero
errored comparisons across the four required profiles; 947 residual test IDs
remain unowned. Eight complete disjoint debug-runner shards share the same
source, runner, backend, and oracle identities. Compared with v17, only
`out-of-flow-in-multicolumn-060` at 1280×720@1.25 changed its Open UI decoded
image and became exact. No prior exact comparison regressed. All 22,924
Chromium oracle identities and decoded hashes stayed unchanged. The
[v19 raster index](generated/focused-primitive-raster-v19.json) confirms
640/640 focused and 960/960 primitive comparisons exact across their 40
profiles. All 1,600 decoded Open UI images and Chromium oracle identities
and hashes match the preceding clean v18 raster run. The
[multicolumn investigation](multicol-nested-positioned-continuation.md)
records the shared continuation paint-order repair.
The prior [v17 diagnostic index](generated/four-profile-census-v17.json), with clean
`9f983df9` source identity, contains 21,209 exact, 1,715 different, and zero
errored comparisons across the four required profiles; 948 residual test IDs
remain unowned. Eight complete disjoint shards share the same source, runner,
backend, and oracle identities. Compared with v16, two Open UI images
changed: one became exact, one remains different, and none regressed. All
22,924 Chromium oracle identities and decoded hashes stayed unchanged. The
clean [v18 raster index](generated/focused-primitive-raster-v18.json) stayed
640/640 focused and 960/960 primitive exact, with all 1,600 decoded Open UI
and Chromium images unchanged. The
[multicolumn investigation](multicol-nested-positioned-continuation.md)
records the cause, reduced reproducer, and then-remaining seam. The prior
[v16 diagnostic index](generated/four-profile-census-v16.json), with clean
`4b89fd05` source identity, contains 21,208 exact, 1,716 different, and zero
errored comparisons across the four required profiles; 949 residual test IDs
remain unowned. Compared with v15, five Open UI images changed: one became
exact, four remaining differences shrank, and none regressed. All 22,924
Chromium oracle identities and decoded hashes stayed unchanged. The prior
[v15 diagnostic index](generated/four-profile-census-v15.json), with clean
`8ca66ffd` source identity, contains 21,207 exact, 1,717 different, and zero
errored comparisons across the four required profiles; 949 residual test IDs
remain unowned. Eight complete disjoint shards share the same source, runner,
backend, and oracle identities. Compared with v14, 28 Open UI images changed:
22 comparisons became exact, six remaining differences shrank, and none
regressed. All 22,924 Chromium oracle identities and decoded hashes stayed
unchanged. The prior
[v14 diagnostic index](generated/four-profile-census-v14.json), with clean
`2dce665c` source identity, contains 21,185 exact, 1,739 different, and zero
errored comparisons across the four required profiles; 965 residual test IDs
remain unowned. Eight complete disjoint shards share the same source, runner,
backend, and oracle identities. Compared with v13, five Open UI images
changed: four comparisons became exact, one remaining difference shrank, and
none regressed. All 22,924 Chromium oracle identities and decoded hashes
stayed unchanged. The prior
[v13 diagnostic index](generated/four-profile-census-v13.json), with clean
`2ff236ce` source identity, contains 21,181 exact, 1,743 different, and zero
errored comparisons across the four required profiles; 968 residual test IDs
remain unowned. Eight complete disjoint shards share the same source, runner,
backend, and oracle identities. Compared with v12, only the four Open UI images
for `negative-margins-001` changed: two comparisons became exact, two retain
fractional border/background differences, and none regressed. All 22,924
Chromium oracle identities and decoded hashes stayed unchanged. The prior
[v12 diagnostic index](generated/four-profile-census-v12.json), with clean
`e942aebc` source identity, contains 21,179 exact, 1,745 different, and zero
errored comparisons across the four required profiles; 968 residual test IDs
remain unowned. Compared with the
[v11 index](generated/four-profile-census-v11.json), 24 Open UI decoded images
changed across ten multicolumn test IDs: ten comparisons became exact, 14
remain different, and none regressed from exact. All 22,924 Chromium oracle
identities and decoded hashes stayed unchanged. The v11 index records the
earlier 21,169 exact and 1,755 different comparisons. The intermediate
[v9 index](generated/four-profile-census-v9.json) records a paint checkpoint
with five exact-to-different regressions; v10 repaired them. Earlier indices
remain historical evidence. None is a qualification result while residuals
remain.
The earlier v3 repair was inline text reaching a later block's border:
that later decoration must paint in the block phase before the earlier text
ink. The change applies by fragment geometry, while preserving atomic flex,
grid, mask, and paint-containment groups outside the established text path.

Results can be resumed through a content-addressed cache; decoded evidence PNGs
are retained by content hash. Chromium captures live in a separate immutable,
write-once oracle cache whose identity contains only browser-side inputs: the
Chromium binary and build, capture harness, fixture, fonts, resources, feature
flags, viewport, and device scale. OpenUI source, binary, and backend identities
are deliberately excluded, so a renderer revision cannot silently recapture a
different expected glyph strike. A conflicting producer for an existing oracle
identity fails closed.

Each OpenUI result-cache identity includes the Git commit and source-tree hash,
qualification-harness hash, renderer binary, the immutable Chromium oracle
identity, logical and physical viewport, device scale, immutable raster and
GPU/driver identity, generated fixture, fonts, and resources. Shards are
deterministic; a final unsharded run can reuse completed shards while rejecting
stale or cross-profile entries. Authoritative runs reject dirty worktrees.
Explicitly allowed dirty runs carry source and harness hashes and remain
nonqualifying. A partial `--profile`, `--test-id`, or sharded run is marked
incomplete and cannot be represented as full contract evidence. Use `--plan`
to inspect scope without producing or mutating qualification evidence.

CPU Skia remains the portable and qualification backend. `ganesh-gl` is an
explicit feature and raster configuration backed by offscreen EGL/Mesa; it is
never selected from the environment. Reports and cache keys record its GL
renderer, version, EGL/driver identity, color type, sample count, and surface
properties. It may replace CPU qualification only after repeated primitive,
focused, and full-census runs are byte-identical and improve the global census.

The font matrix uses licensed TTF, OTF, WOFF, WOFF2, TTC, and deterministically
generated OTC fixtures from the pinned WPT checkout. The generator verifies
source and output hashes, and Rust/C tests require identical container, face
index, byte length, and SHA-256 metadata. These fixtures remain document-owned;
they do not expand qualifying dependence on ambient system fonts.

The later complete clean `e9211183` [v46 census](generated/four-profile-census-v46.json)
and [v29 expanded requalification](generated/expanded-requalification-v29.json)
retain the preceding exact/different totals, with zero errors. The
[full delta](generated/native-vertical-max-block-full-delta-v1.json) records
four worsened existing failures at fractional scales in `block-max-height-004`
and its reference. Every Chromium image and oracle identity stays fixed,
and no formerly exact comparison regresses. Those failures remain open;
this evidence does not qualify the renderer.
