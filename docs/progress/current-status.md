# Open UI current status

Open UI is in v0.2 release-candidate closure for the pure-Rust Linux/headless
product. Waves W0 through W10 are locally committed. W11 source packaging and
documentation are implemented, but final external and hardware qualifications
remain open.

## Verified repository state

| Evidence | Result |
|---|---:|
| Historical frozen SP20 pass records | 5,731, using a tolerant comparator |
| Optional historical byte replay | 5,549 unchanged, 182 changed, 0 errors; not a gate |
| Four-profile census against cached Chromium captures | 21,266/22,924 exact, 1,658 different, 0 errors |
| Chromium oracle consistency audit | One older cached capture differs from six fresh captures under the same recorded identity; reconciliation open |
| Focused / primitive 40-profile matrices | 640/640 / 960/960 exact |
| Expanded native final-state additions | 200/201 exact at all four profiles in the latest clean run; one still fails |
| Pending native final-state candidates | 0/35 exact at all four profiles after the latest clean recheck |
| Full inventory | 7,673 |
| Explicitly unported | 1,942 |
| Accountability audit | 7/7 |
| Application conformance scenarios | 43 across 10 domains |
| Frozen / current C exports | 84 / 106 |
| C examples / C++ consumers | 6 / 2, including native C/C++ window consumers |
| Workspace tests | pass |
| Python closure and qualification tests | 236 pass |
| Owned objects after 10,000 mutation soak | no growth/leak |
| Unchanged-frame lifecycle | zero layout, paint, and raster work |

The [latest complete clean census](../renderer/generated/four-profile-census-v41.json)
at `15f9f12d` is 21,266/22,924 exact, 1,658 different, and zero errors
against the cached Chromium captures.
The [circular background one-axis clip repair](../renderer/circular-background-one-axis-clip.md)
makes the F16 circular fill apply the same horizontal overflow scissor as the
direct rounded fill. One original comparison became exact; only that Open UI
image changed, and no exact comparison regressed. All 22,924 Chromium oracle
identities and decoded images stayed fixed. The 914 residual test IDs remain
unowned. The clean
[v42 focused/primitive index](../renderer/generated/focused-primitive-raster-v42.json)
is 640/640 and 960/960 exact, with all 1,600 images unchanged from v41.
The complete [v24 expanded requalification](../renderer/generated/expanded-requalification-v24.json)
is 22,069/23,728 exact, 1,659 different, and zero errors. Every original
result matches the separate census; all 804 addition images stayed fixed,
leaving 200 of 201 additions exact at all four profiles. All 23,728 Chromium
oracle identities and decoded images stayed fixed. The
[v26 diagnostic selection](../../tools/qualification/manifests/expanded-v26.json)
lists those 200; the release contract retains all 201, including the fieldset
legend failure. The other 35 AST-lowered candidates remain pending.
The [fractional Ahem legend investigation](../renderer/fieldset-legend-ahem-raster-investigation.md)
records that case's 14 extra glyph rows and three rejected shared-raster
experiments; no renderer fix or qualification is claimed from that work.

The [scaled LCD font investigation](../renderer/scaled-lcd-hinting-oracle-investigation.md)
rejected a shared no-hinting change: among 904 real-font comparisons, 28 became
exact but 43 formerly exact comparisons regressed. The renderer was restored.
Of 12 fresh Chromium captures, 11 matched their cached images and one differed;
five additional fresh captures of that case all matched the first fresh one.
The [oracle audit](../renderer/generated/chromium-font-oracle-audit-v1.json)
records two decoded images under one identical recorded input identity. The
root cause is unreviewed, both variants remain preserved, and final renderer
qualification requires reconciliation. This diagnostic does not update the
clean census counts or qualify any renderer change.

At the prior clean checkpoint `e14e3e64`, the
[v39 census](../renderer/generated/four-profile-census-v39.json)
was 21,264/22,924 exact, 1,660 different, and zero errors.
The [vertical-lr Ahem rotation-anchor repair](../renderer/vertical-lr-ahem-rotation-anchor.md)
made nine original comparisons exact, changed only those nine Open UI images,
and reduced wrong pixels by 30,184. No formerly exact comparison regressed;
all 22,924 Chromium oracle identities and decoded images stayed fixed. The
916 remaining residual test IDs were unowned. The clean
[v40 focused/primitive index](../renderer/generated/focused-primitive-raster-v40.json)
was 640/640 and 960/960 exact. The complete
[v22 expanded requalification](../renderer/generated/expanded-requalification-v22.json)
was 22,066/23,728 exact, 1,662 different, and zero errors. The same nine
original comparisons plus one admitted native final-state comparison became
exact; 199 of 201 additions passed all four profiles. The
[v24 diagnostic selection](../../tools/qualification/manifests/expanded-v24.json)
listed those 199, while the release contract kept all 201 and their two
failures at that checkpoint. The other 35 AST-lowered candidates remained pending.

At the prior clean checkpoint `d39282e4`, the
[v38 census](../renderer/generated/four-profile-census-v38.json)
was 21,255/22,924 exact, 1,669 different, and zero errors.
The [analytic gradient-edge repair](../renderer/fractional-multicol-seam-investigation.md)
changed 29 Open UI images, made seven comparisons exact, and reduced the
remaining wrong-pixel count by 5,957. No formerly exact comparison regressed;
all 22,924 Chromium oracle identities and decoded images stayed fixed. The
923 remaining residual test IDs are unowned. The clean
[v39 focused/primitive index](../renderer/generated/focused-primitive-raster-v39.json)
is 640/640 and 960/960 exact, with all 1,600 Open UI and Chromium images
unchanged from the prior raster v38 index. The clean
[201-addition guard](../renderer/generated/expanded-additions-gradient-guard-v1.json)
is 801/804 exact, with the same three failures and all 804 Open UI and
Chromium images unchanged from the prior complete expanded run. The complete
[v21 expanded requalification](../renderer/generated/expanded-requalification-v21.json)
at clean checkpoint `e59ec07e` is 22,056/23,728 exact, 1,672 different,
and zero errors. All 22,924 original and 804 added images match their clean
v38 census and selected-additions guard respectively, including all Chromium
oracle identities and decoded image hashes. The
[v23 diagnostic selection](../../tools/qualification/manifests/expanded-v23.json)
lists the 198 four-profile-exact additions; the release contract retains all
201, including the same three failures.

At the prior clean checkpoint `dc451061`, the
[v37 census](../renderer/generated/four-profile-census-v37.json) was
21,248/22,924 exact, 1,676 different, and zero errors.
The [adjacent row-flex continuation repair](../renderer/adjacent-row-flex-fragmentation.md)
made three fractional-scale comparisons exact and reduced one other difference
from 132 to 14 pixels. Only four Open UI images changed across 22,924
comparisons; no formerly exact case regressed, and all Chromium oracle
identities and decoded images stayed fixed. The 924 remaining residual test
IDs are unowned. The clean
[v38 focused/primitive index](../renderer/generated/focused-primitive-raster-v38.json)
is 640/640 and 960/960 exact, with all 1,600 Open UI and Chromium images
unchanged from the prior raster v37 index. The complete clean
[v20 expanded requalification](../renderer/generated/expanded-requalification-v20.json)
is 22,049/23,728 exact, 1,679 different, and zero errors; 198 of 201
additions remain exact at all four profiles, with the same three failures.

At the prior clean checkpoint `128edc38`, the
[v36 census](../renderer/generated/four-profile-census-v36.json) was
21,245/22,924 exact, 1,679 different, and zero errors.
The fractional Ahem hinting repair changed 21 already failing Open UI images,
reduced their combined wrong-pixel count by 281, and introduced no exact
regression. All 22,924 Chromium oracle identities and decoded images stayed
fixed. Its [remaining edge mismatch](../renderer/fractional-ahem-stripe-edges.md)
still needs an exact source-level repair.
At the prior clean checkpoint `6676cf60`,
the [v35 image-delta audit](../renderer/generated/radial-ua-full-delta-v1.json)
found nine changed Open UI images, one newly exact comparison, and no
previously exact regression. All 22,924 Chromium oracle identities and decoded
hashes stayed fixed. The [radial tile repair](../renderer/radial-tile-edge-coverage.md)
reduced two already failing 1.25× cases from 626 wrong pixels to one each.
The [object fallback investigation](../renderer/object-fallback-host-clip.md)
records an exact-case regression in the intermediate v34 census and its scoped
repair; all six original `object` fixtures are exact at four profiles again.
The clean [v37 raster index](../renderer/generated/focused-primitive-raster-v37.json)
at `e1bdccc4` is 640/640 focused and 960/960 primitive exact with zero errors.
All 1,600 Open UI and 1,600 Chromium decoded images are unchanged from the
prior raster index.
The [201-addition clean guard](../renderer/generated/expanded-additions-object-deferred-v1.json)
at `6676cf60` retained 801 exact and three different profile comparisons;
none of its Open UI or Chromium decoded images changed from the prior guard.
This selected guard is not a complete expanded-manifest run.
The [new expanded admission](../renderer/adjoining-floats-native-admission.md)
retains all 200 prior additions and adds one case exact at all four required
profiles. The complete clean expanded run at `6b53a991` is
[22,046/23,728 exact](../renderer/generated/expanded-requalification-v19.json),
with 1,682 differences, zero errors, and 198 of 201 additions exact across
four profiles. The [v21 diagnostic selection](../../tools/qualification/manifests/expanded-v21.json)
lists those 198; the release contract still includes all 201. The same three
earlier additions remain failures. The remaining 35
[pending candidates](../renderer/generated/pending-mutation-candidates-v7.json)
produced 18/140 exact comparisons, 122 differences, and zero errors at clean
checkpoint `8324c6b0`; none is exact at all four profiles. All 140 comparison
statuses, mismatched-pixel counts, and diff signatures match the prior pending
report. At that prior checkpoint, the original release gate had 926 unowned
residual test IDs.
Open UI runs no JavaScript; the newly admitted interaction has a public native
Rust API path. A script in a Chromium test is a source-data fact, not an
application feature requirement or an excuse to omit a needed native Rust API.
The public `Element::detach` and `TextNode::detach` operations now keep authored
nodes available for reattachment while removing them from the presented
document. A new conformance scenario checks element lookup, focus,
reattachment, listener delivery, and eventual destruction; the text-node
scenario now checks detachment and reattachment too. The conformance suite is
43/43.
The append-only C `oui_element_detach` export uses the same engine operation
for element and text handles. `oui_document_element_by_id` now provides owned
handles for native C element lookup. ABI verification reports 106 current
symbols; the 84 frozen symbols and all C struct layouts remain intact.
`oui_app_run` and `oui_app_request_exit` now run native C applications through
Rust `App` and the same retained `Document`. Versioned platform callbacks can
mutate elements and request exit after engine and presentation borrows end.
Native C/C++ consumers exercise X11 software/OpenGL and pure Wayland software
windows. Shared Rust keyboard handling now respects cancelled keydown events,
separates committed text from logical key names, and ignores text input on
noneditable controls. Physical release-lab and packaged application
qualification, including AT-SPI operation, remains open.
The locked Rust workspace suite passed with CI's 4 MiB libtest worker-thread
stack. The default 2 MiB worker stack still aborts in an existing fragmented
multicol integration test; its isolated rerun passes at 4 MiB.

A [fractional overflow-clip investigation](../renderer/fractional-rectangular-overflow-clip.md)
previously found a coincident button/child edge that differed by 125 pixels at
1.25×. A
dirty hard-clip experiment made that case exact but regressed 33 previously
exact comparisons in a 3,392-comparison diagnostic. A narrower contained-child
experiment still regressed four exact comparisons in 1,696 fractional-scale
comparisons. Both edits were reverted. The later scoped
[button repair](../renderer/button-content-clip-coverage.md) is the clean
qualification evidence above.

A [broken-image sampling investigation](../renderer/broken-image-fractional-sampling.md)
isolated five identical 1.5× differences to Chromium's fallback-image icon.
The pinned Skia fixed-point coordinate sequence made all five exact in the
clean census. No previously exact comparison regressed and every Chromium
oracle hash stayed fixed. Nineteen already failing comparisons changed size;
two scrolling/alt-text overflow cases worsened and remain open.

The [prior complete clean census](../renderer/generated/four-profile-census-v30.json)
at `814a2005` is 21,239/22,924 exact, 1,685 different, and zero errors.
The [table row-group clip repair](../renderer/truncated-table-row-group-clip.md)
made the remaining repeated-section comparison exact at 1.25×. Across all
22,924 comparisons, only that Open UI image changed; no exact image regressed
and every Chromium oracle identity and decoded hash stayed fixed. The clean
[v31 raster index](../renderer/generated/focused-primitive-raster-v31.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 decoded
images unchanged. The complete clean
[v16 expanded requalification](../renderer/generated/expanded-requalification-v16.json)
is 22,036/23,724 exact; all 800 addition images remain unchanged, with 197
of 200 additions exact across four profiles and three demoted. The
[v17 diagnostic selection](../../tools/qualification/manifests/expanded-v17.json)
retains those 197. The clean
[v5 pending-candidate index](../renderer/generated/pending-mutation-candidates-v5.json)
is unchanged at 22/144 exact, with one of 36 cases exact at all four profiles.
The repaired table case is exact in the
[40-profile sweep](../renderer/generated/table-row-group-cross-v1.json). The
release renderer gate still fails with 931 unowned residual test IDs.

The [prior complete clean census](../renderer/generated/four-profile-census-v29.json)
at `7cd8e574` is 21,238/22,924 exact, 1,686 different, and zero errors.
The shared [repeated table body-slice fix](../renderer/repeated-table-body-slice.md)
made 11 comparisons exact and reduced one other difference, with no
regression. Exactly 12 Open UI images changed and all 22,924 Chromium oracle
hashes stayed fixed. The [v30 raster index](../renderer/generated/focused-primitive-raster-v30.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v15 expanded requalification](../renderer/generated/expanded-requalification-v15.json)
is 22,035/23,724 exact, with all 200 additions unchanged: 197 exact, three
demoted. The [v16 diagnostic selection](../../tools/qualification/manifests/expanded-v16.json)
retains those 197. The clean
[v4 pending-candidate index](../renderer/generated/pending-mutation-candidates-v4.json)
again records one of 36 cases exact at all four profiles. The release renderer
gate still fails with 932 unowned residual test IDs.

The [prior complete clean census](../renderer/generated/four-profile-census-v28.json)
at `a0e3f4cd` is 21,227/22,924 exact, 1,697 different, and zero errors.
The shared [clipped replaced-background coverage fix](../renderer/clipped-replaced-background-coverage.md)
made two margin-trim comparisons exact and reduced a third iframe difference
from 342 pixels to six. Only those three Open UI images changed, all 22,924
Chromium oracle hashes stayed fixed, and no exact comparison regressed. The
[v29 raster index](../renderer/generated/focused-primitive-raster-v29.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v14 expanded requalification](../renderer/generated/expanded-requalification-v14.json)
is 22,024/23,724 exact; all 200 additions have the same pixels and statuses,
with 197 exact and three demoted. The
[v15 diagnostic selection](../../tools/qualification/manifests/expanded-v15.json)
retains those 197. The clean
[v3 pending-candidate index](../renderer/generated/pending-mutation-candidates-v3.json)
again records one of 36 cases exact at all four profiles. The release renderer
gate still fails with 937 unowned residual test IDs.

The [prior complete clean census](../renderer/generated/four-profile-census-v27.json)
at `8950b426` remains 21,225/22,924 exact, 1,699 different, and zero errors.
All 939 residual test IDs and their pixel diff signatures match the prior
clean census. The [v28 raster index](../renderer/generated/focused-primitive-raster-v28.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v13 expanded requalification](../renderer/generated/expanded-requalification-v13.json)
is 22,022/23,724 exact, 1,702 different, and zero errors; the same 197 of
200 additions are exact at all four profiles and the same three are demoted.
The [v14 diagnostic selection](../../tools/qualification/manifests/expanded-v14.json)
retains those 197. The clean
[v2 pending-candidate index](../renderer/generated/pending-mutation-candidates-v2.json)
records 22/144 exact comparisons. `adjoining-floats-dynamic` is now exact at
all four profiles and eligible for a future expanded-manifest admission; the
other 35 cases remain open. The original renderer release gate still fails.
The [nested float investigation](../renderer/nested-inline-float-propagation.md)
records the shared layout fix and the separate sticky-position residual.

The [prior complete clean census](../renderer/generated/four-profile-census-v26.json)
at `954648fb` changed exactly eight Open UI images in four resized one-axis
bitmap test/reference pairs at fractional scales. Two comparisons became
exact, six moved closer, and none regressed. All 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[shared bitmap-pattern investigation](../renderer/resized-one-axis-bitmap-pattern.md)
records the source rule and remaining one-to-three-pixel differences. The
clean [v27 raster index](../renderer/generated/focused-primitive-raster-v27.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded images unchanged. The clean
[v5 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v5.json)
remains 797/800 exact, with all 800 Open UI and Chromium decoded images
unchanged. The release census still fails exactness with 1,699 differences
across 939 unowned residual test IDs.

The [prior complete clean census](../renderer/generated/four-profile-census-v25.json)
at `5c7aaa9c` changed exactly eight Open UI images in four one-axis repeated
linear-gradient test/reference pairs. Their wrong-pixel counts fell sharply
at fractional scales, but none became exact because the separate PNG edge
still differs. No exact comparison regressed, and all 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[gradient picture-shader investigation](../renderer/one-axis-gradient-picture-shader.md)
records the source rule and remaining raster edge. The clean
[v26 raster index](../renderer/generated/focused-primitive-raster-v26.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded images unchanged. The clean
[v4 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v4.json)
remains 797/800 exact, with all 800 Open UI and Chromium decoded images
unchanged. The release census still fails exactness with 1,701 differences
across 939 unowned residual test IDs.

An [earlier complete clean census](../renderer/generated/four-profile-census-v24.json)
at `2f560e46` gained four exact comparisons from v23 by painting mixed border
styles in Chromium's alpha, style, and side order. Exactly 16 Open UI images
changed across four border fixtures; none regressed, and all 22,924 Chromium
oracle identities and decoded hashes stayed fixed. The
[border paint-order investigation](../renderer/mixed-border-paint-order.md)
records the compact case, source rule, rejected clip-only diagnostic, and
four-profile pixel counts. The clean
[v25 raster index](../renderer/generated/focused-primitive-raster-v25.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded hashes unchanged from v24. The clean
[v3 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v3.json)
remains 797/800 exact with three differences and zero errors; all 800 decoded
images and statuses are unchanged. The complete census still fails exactness
with 1,701 differences across 939 unowned residual test IDs.

An [earlier complete clean census](../renderer/generated/four-profile-census-v23.json)
at `d0592ccd` gained one exact comparison from v22:
`out-of-flow-in-multicolumn-047` at 1280×720@1.25. Its parent decoration
now ends at its used block size, and the outer visual continuation no longer
replays inner columns whose in-flow source was already consumed. Only that
Open UI decoded image changed; no previously exact comparison regressed.
All 22,924 Chromium oracle identities and decoded hashes stayed fixed. The
[consumed nested columns investigation](../renderer/multicol-consumed-nested-columns.md)
includes reduced reproductions of both masked errors. The clean
[v24 raster index](../renderer/generated/focused-primitive-raster-v24.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium hashes unchanged from v23. The census still fails exactness,
with 1,705 differences across 939 unowned residual test IDs.
The clean [v2 selected-additions
recheck](../renderer/generated/expanded-additions-recheck-v2.json) at
`db763999` measured 797/800 exact, three different, and zero errors. All
800 Open UI and Chromium hashes matched the prior full expanded run; this
selection does not replace complete expanded-manifest qualification.

The prior [v22 clean census](../renderer/generated/four-profile-census-v22.json)
at `6e21bd9b` gained one exact comparison from v21:
`out-of-flow-in-multicolumn-046` at 1280×720@1.25. Its zero-height
positioned parent no longer paints a background through the visual
continuation needed by its absolute child. Only that Open UI decoded image
changed; no previously exact comparison regressed. All 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[zero-height continuation investigation](../renderer/multicol-zero-height-positioned-decoration.md)
includes a reduced reproducer. The clean
[v23 raster index](../renderer/generated/focused-primitive-raster-v23.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium hashes unchanged from v22. The census still fails exactness,
with 1,706 differences across 940 unowned residual test IDs.

The prior [v21 clean census](../renderer/generated/four-profile-census-v21.json)
at `99fd4432` gained three exact comparisons from v20:
`flexbox_multi-line-row-flex-fragmentation-029` at 1280×720@1.25 and
`-030` at 1280×720@1.25 and 1920×1080@1.5. Only those three Open UI decoded
images changed; no previously exact comparison regressed. All 22,924 Chromium
oracle identities and decoded hashes stayed fixed. The
[final-decoration investigation](../renderer/flex-final-continuation-decoration.md)
records the reduced reproducer and the rejected broad rule. The clean
[v22 raster index](../renderer/generated/focused-primitive-raster-v22.json)
confirms 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded hashes unchanged from v21. The census still fails
exactness, with 941 unowned residual test IDs.

The prior [v20 clean census](../renderer/generated/four-profile-census-v20.json)
at `89a0a1f3` gained one exact comparison from v19:
`flexbox_multi-line-row-flex-fragmentation-027` at 1280×720@1.25.
No previously exact comparison regressed. Only that Open UI decoded image
changed; all 22,924 Chromium oracle identities and decoded hashes stayed
fixed. The [first-line break investigation](../renderer/nested-row-flex-first-line-break.md)
records the masked layout error, reduced reproducer, and neighboring guard.
The clean [v21 raster index](../renderer/generated/focused-primitive-raster-v21.json)
confirms 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded hashes unchanged from v20. The census still fails
exactness, with 943 unowned residual test IDs.

The prior [v19 clean census](../renderer/generated/four-profile-census-v19.json)
at `17952772` gained three exact comparisons from v18:
`out-of-flow-in-multicolumn-042`, `-043`, and `-045` at 1280×720@1.25.
No previously exact comparison regressed. Only those three Open UI decoded
images changed; all 22,924 Chromium oracle identities and decoded hashes
stayed fixed. The [relative continuation clip investigation](../renderer/multicol-relative-clip-translation.md)
records the cause, reduced reproducer, and authored-overflow guard. The clean
[v20 raster index](../renderer/generated/focused-primitive-raster-v20.json)
confirms both 40-profile matrices stayed exact with all 1,600 Open UI and
Chromium decoded hashes unchanged from the preceding clean v19 raster run. The
census had 944 unowned residual test IDs. The prior
[multicolumn investigation](../renderer/multicol-nested-positioned-continuation.md)
records the v18 repair. The prior
[fragment clip investigation](../renderer/fragment-decoration-clip-investigation.md)
records the v16 repair.
The [border-image seam investigation](../renderer/border-image-seam-investigation.md)
records a separate six-case fractional-scale residual and a rejected
offscreen-layer diagnostic; it changes no qualifying count.
The [repeat-space shader investigation](../renderer/background-repeat-space-shader-investigation.md)
records two rejected 92-comparison sampling diagnostics and a clean restored
runner; it also changes no qualifying count.

The clean [expanded v12 requalification](../renderer/generated/expanded-requalification-v12.json)
at `6e21bd9b` measured 22,015/23,724 exact comparisons, 1,709 different,
and zero errors. Only the same original `-046` image changed from v11, and it
became exact. All 23,724 Chromium oracle identities and decoded hashes stayed
fixed. All 200 native additions kept their prior four-profile statuses and
all 800 decoded Open UI images: 197 remain exact at all four profiles and
three remain demoted. The
[v13 diagnostic selection](../../tools/qualification/manifests/expanded-v13.json)
retains those 197 additions without changing the original manifest.

The prior clean [expanded v11 requalification](../renderer/generated/expanded-requalification-v11.json)
at `99fd4432` measured 22,014/23,724 exact comparisons, 1,710 different,
and zero errors. Only the same three original flex comparisons changed from
v10. All 23,724 Chromium oracle identities and decoded hashes stayed fixed.
All 200 native additions kept their prior four-profile statuses and all 800
decoded Open UI images: 197 remain exact at all four profiles and three remain
demoted. The [v12 diagnostic selection](../../tools/qualification/manifests/expanded-v12.json)
retains only those 197 additions; it does not alter the original manifest.

The prior [expanded v10 requalification](../renderer/generated/expanded-requalification-v10.json)
at `89a0a1f3` measured 22,011/23,724 exact comparisons, 1,713 different,
and zero errors. Only the same original `-027` comparison changed from the
previous full expanded run. All 23,724 Chromium oracle identities and decoded
hashes stayed fixed. All 200 native additions kept their prior four-profile
statuses and all 800 decoded Open UI images: 197 remain exact at all four
profiles and three remain demoted. The
[v11 diagnostic selection](../../tools/qualification/manifests/expanded-v11.json)
retains only those 197 additions; it does not alter the original manifest.

The prior clean [expanded v9 requalification](../renderer/generated/expanded-requalification-v9.json)
at `de8304fe` measured 22,010/23,724 exact comparisons, 1,714 different,
and zero errors. Against the prior v8 expanded run, four original
`out-of-flow-in-multicolumn-*` comparisons became exact: `-060` from the v18
census repair and `-042`, `-043`, `-045` from the v19 repair. No other Open UI
decoded image or status changed. All 23,724 Chromium oracle identities and
decoded hashes stayed fixed. All 200 native final-state additions retained
their prior four-profile statuses and all 800 decoded Open UI images: 197
remain exact at all four profiles and three remain demoted. The
[v10 diagnostic selection](../../tools/qualification/manifests/expanded-v10.json)
retains only the 197 exact additions; it does not alter the original manifest.
The prior [v8 ledger](../renderer/generated/expanded-requalification-v8.json)
and [v9 selection](../../tools/qualification/manifests/expanded-v9.json)
remain historical evidence.

At the earlier clean checkpoint `93b00601`, a
[four-profile additions recheck](../renderer/generated/expanded-additions-recheck-v1.json)
rendered all 200 native additions: 797/800 profile comparisons exact, three
different, and zero errors. The same three cases remain demoted. All 800
Open UI decoded hashes, Chromium oracle identities, and Chromium decoded
hashes match the prior clean v8 expanded run. This subset report is
diagnostic; it does not replace a complete expanded-manifest qualification.

The historical baseline records Chromium identity, viewport, device scale,
fonts, resources, and result hashes. A fresh pinned Chromium capture differs
from 203 archived Open UI images; the old screenshots are not expected output.
See the [audit](../renderer/frozen-oracle-audit.md). Chromium is the sole pixel
target but a test oracle only; supported builds and packages do not link or
load it.

## Implemented v0.2 surface

- Canonical generated typed style properties for Rust macros, engine metadata,
  and C tagged values.
- Thread-affine retained documents with generation-checked node handles,
  transactions, dirty generations, resources, hit testing, and immutable scene
  snapshots.
- Direct safe Rust framework with signals, effects, scopes, `Show`, keyed
  `For`, components, `view!`, `AppBuilder`, and `HeadlessApp`.
- Panic-contained C ABI with ownership/thread validation, structured errors,
  compound builders, events, controls, animation, owned accessibility-tree
  snapshots, and rendering.
- Core controls, routed pointer/keyboard/text/composition events, focus and
  modal containment, selection, Unicode editing, clipboard, undo/redo,
  scrolling, and pointer capture.
- Public native element activation and details disclosure: `Element::click`,
  `set_open`, and `is_open` use the same retained event and control path;
  closed details content leaves layout and the accessibility tree.
- Public native class-token changes and attached-document class lookup use
  the same retained tree from Rust callbacks.
- Public native kind lookup and `Element::kind` cover the deterministic
  test cases that find elements by tag, with attached-document order and
  generation-checked handles. Authored names such as `div` and `main` can
  share a native kind.
- Retained accessibility trees/actions and Linux AccessKit integration.
- Winit X11/Wayland runtime, backend-correct clipboard, IME/data events,
  softbuffer presentation, and multiple isolated windows.
- OpenGL presentation of the exact CPU Skia frame with automatic software
  fallback, diagnostics, resize recovery, and newest-scene scheduling.
- Typed transitions/keyframes, easing, manual clocks, lifecycle events,
  document/scroll/view timelines, smooth scrolling, snap, and reduced motion.
- Conformance, fuzz, sanitizer, Miri, soak, platform, performance, and
  reproducible package gates.

## Release blockers

- Direct Skia Ganesh raster builds behind explicit backend selection. On the
  clean Mesa llvmpipe [comparison](../renderer/generated/ganesh-raster-comparison-v1.json),
  it reached 408/640 focused and 624/960 primitive exact, below CPU Skia's
  640/640 and 960/960. It remains unpromoted; OpenGL presentation still uploads
  a CPU-rasterized frame.
- The [current hardening run](https://github.com/zhuowcui/open-ui/actions/runs/36365378115)
  still fails AddressSanitizer, LeakSanitizer, and fuzz on process-exit
  Fontconfig allocations. Its MSRV, Miri, Linux platform, and C UBSan jobs
  passed.
- The four-profile Chromium census fails exactness; 914 residual test IDs
  have no reviewed owner. Both 40-profile CPU raster matrices are exact.
- The C ABI covers the retained engine, headless renderer, owned full
  accessibility-tree snapshots, and the shared Rust Linux event loop;
  release-lab AT-SPI and packaged native C/C++ application qualification remain open.
- Retained per-node layers and compositor-owned animation curves are incomplete,
  so the strict blocked-UI 100-animation gate is not yet qualified.
- Full preserve-3d and backface layer semantics remain incomplete.
- Physical-GPU/context-loss and automated AT-SPI release-lab runs are pending.
- x86-64 and AArch64 clean-container packages must be built twice, compared,
  attested, signed, installed, and exercised.
- Crates.io publication requires final release approval and credentials.

See [release qualification](../v02/release.md),
[hardening](../v02/hardening.md), and the
[unsupported list](../v02/unsupported-features.md). Do not describe the WSL2
performance artifact as release qualification.

## Canonical local gates

```bash
cd bindings/rust
cargo test --workspace --locked
cd ../..

python3 tools/release/build_v02_linux.py --verify-source
python3 tools/accountability/audit.py
python3 tools/accountability/restore_frozen_openui_archive.py --check
python3 tools/accountability/audit_frozen_oracle.py --output /tmp/openui-historical-audit.json
python3 tools/ffi/verify_abi.py
```

Pixel qualification uses the pinned Chromium oracle through the complete
four-profile matrix described in the [renderer contract](../renderer/contract.md).
The archive check above only protects historical evidence.

For the workstation-only exact toolchain, pass
`--config .cargo/config.chromium.toml` to Cargo. Ordinary development and
release builds use the portable default configuration.
