# Open UI v0.2 release qualification

The v0.2 source contract and artifact pipeline are checked in. This document
distinguishes verified repository evidence from release-lab work that cannot be
claimed by source code alone.

| Area | Current evidence | State |
|---|---|---|
| Historical Open UI archive | Archive and records are byte-pinned; optional [replay](../renderer/generated/frozen-replay-v1.json) found 5,549/5,731 unchanged, 182 changed | provenance pass; replay diagnostic |
| Chromium pixel target | Pinned Chromium 147 is the sole expected output for the declared renderer tests | see matrix below |
| Chromium oracle consistency | [Audit](../renderer/generated/chromium-font-oracle-audit-v1.json) found one older cached image that differs from six fresh captures under the same recorded identity; both variants are preserved | reconciliation open |
| Four-profile renderer matrix | 21,278/22,924 exact, 1,646 different, zero errors against cached Chromium captures in the [latest clean census](../renderer/generated/four-profile-census-v43.json); three exact regressions against the preceding checkpoint | fail |
| Focused and primitive raster | 640/640 focused exact; 960/960 primitive exact in the [latest raster index](../renderer/generated/focused-primitive-raster-v45.json) | pass |
| Direct Ganesh raster | Clean Mesa llvmpipe [comparison](../renderer/generated/ganesh-raster-comparison-v1.json): 408/640 focused and 624/960 primitive exact; CPU remains the qualification backend | unpromoted |
| Expanded native final-state fixtures | Latest clean full expanded run retains 200 of 201 exact additions and [demotes one](../renderer/generated/expanded-requalification-v26.json); 22,081/23,728 total comparisons exact, 1,647 different, zero errors. The other 35 AST-lowered cases remain [pending](../renderer/generated/pending-mutation-candidates-v7.json). Open UI runs no JavaScript | open |
| Accountability | 7/7 over 7,673 rows | pass |
| Rust workspace and docs | full locked workspace suite | pass |
| Rust 1.85 MSRV | [current hardening](https://github.com/zhuowcui/open-ui/actions/runs/36365378115): locked headless and Linux checks passed | pass |
| Rust/C application contract | 55 scenarios, 107 current exports, six headless C examples and a C++ header consumer, plus native C/C++ window consumers; previous symbols and layouts preserved | remaining API review and lab qualification open |
| Native element interaction | Public Rust `Document`, `Element`, and `TextNode` APIs cover ID/class/native-kind lookup, class-token updates, retained detach/reattach, mutation, callbacks, activation, focus, scrolling, and controls; browser-style operations needed by applications must be exposed through native APIs | core implemented; remaining API coverage review open |
| C-owned X11/Wayland application loop | `oui_app_run` and `oui_app_request_exit` use Rust `App` and the same retained document; versioned platform callbacks and [clean native C/C++ window runs](native-c-lifecycle-evidence.md) cover X11 software/GL and Wayland software | implemented; release-lab qualification open |
| C platform accessibility | owned full-tree snapshots, node metadata/relations/focus, and changed/removed IDs export from the shared engine; automated AT-SPI operation in a C window remains unqualified | open |
| Generated sources | style, ABI, migration, closure generators are read-only clean | pass |
| No-work frame | zero layout, paint, and raster on unchanged snapshots | pass |
| Mutation ownership | 10,000-iteration soak, no owned-object leak | pass |
| Local performance smoke | 0.108 ms p95, 308 UI-thread animation fps, 1.389% RSS growth | non-qualifying pass |
| X11/Wayland software and Mesa GL | [current hardening](https://github.com/zhuowcui/open-ui/actions/runs/36365378115) passed both smoke paths; physical release-lab tests remain open | provisional pass |
| Miri C handle ownership | [current hardening](https://github.com/zhuowcui/open-ui/actions/runs/36365378115): opaque-handle ownership test passed under pinned Miri | pass |
| ASan/LSan/fuzz | [current hardening](https://github.com/zhuowcui/open-ui/actions/runs/36365378115): all 18 FFI tests passed, then both sanitizers reported 10,476 bytes through Fontconfig at process exit; fuzz stopped on a 2,606-byte Fontconfig allocation report | fail |
| Native C UBSan | [current hardening](https://github.com/zhuowcui/open-ui/actions/runs/36365378115): ABI consumers passed | pass |
| x86-64/AArch64 SDK, deb, rpm | deterministic source pipeline and tag matrix | pending tag build |
| Clean Ubuntu/Fedora install | release workflow consumer jobs | pending tag build |
| Physical GPU/context loss | release-lab profile | open |
| 100 compositor animations during 250 ms UI stall | retained animation layers required | open |
| AT-SPI inspect/operate | release-lab accessibility session | open |
| Two signed reproducible builds | tag workflow, keyless signatures/attestations | open |
| crates.io publication | credentials and final release approval | open |

The checked-in performance artifact is a WSL2 smoke result and explicitly has
`qualification: false`. It must not be relabeled as reference-machine evidence.

Chromium test files may contain scripts, but Open UI runs no JavaScript. Test
tooling may translate a script's deterministic visual result into native Rust
operations for pixel qualification. A test excluded from the pixel matrix for
behavioral or nonvisual reasons is not a waiver for application interaction:
needed element behavior must be available through the public native Rust API
over the same retained document and event path.
Each needed operation must be callable by the consuming native Rust app and
verified through that public API. An internal Engine operation or a test-only
fixture does not close a native application API gap.

The [scaled LCD font investigation](../renderer/scaled-lcd-hinting-oracle-investigation.md)
rejected a renderer change that regressed 43 formerly exact comparisons. It
also found one older cached Chromium image that differs from six agreeing
fresh captures. The capture identity and fixture bytes are the same; the
reviewed cause remains unknown. The clean census below remains evidence
against those preserved cached captures, and final qualification requires
oracle reconciliation. No cached reference was replaced or result promoted.

The latest clean census at `0ad4b12f` is
[21,278/22,924 exact](../renderer/generated/four-profile-census-v43.json),
with 1,646 differences, zero errors, and 909 unowned residual test IDs. The
[complete delta](../renderer/generated/native-geometry-full-delta-v1.json)
recovers the preceding checkpoint's three nested-column failures but introduces
three ordinary-spanner failures at 1.25×. No Chromium image or oracle identity
changes. This checkpoint is not accepted as renderer qualification. The full
expanded run is [22,081/23,728 exact](../renderer/generated/expanded-requalification-v26.json),
with 1,647 differences and zero errors. All original rows agree with the
separate census, and 200/201 additions remain exact at all four profiles.
The release manifest retains all 201. The focused and primitive matrices are
[640/640 and 960/960 exact](../renderer/generated/focused-primitive-raster-v45.json),
with all 1,600 images unchanged from the preceding checkpoint.

The preceding clean census at `15f9f12d` is
[21,266/22,924 exact](../renderer/generated/four-profile-census-v41.json),
with 1,658 differences, zero errors, and 914 unowned residual test IDs. The
[circular background one-axis clip repair](../renderer/circular-background-one-axis-clip.md)
makes the F16 circular fill use the same horizontal overflow scissor as the
direct fill. One original comparison became exact; only that Open UI image
changed, and no exact comparison regressed. All 22,924 Chromium oracle
identities and decoded images stayed fixed. The clean
[v42 focused/primitive index](../renderer/generated/focused-primitive-raster-v42.json)
is 640/640 and 960/960 exact, with all 1,600 images unchanged from v41.
The complete [v24 expanded requalification](../renderer/generated/expanded-requalification-v24.json)
is 22,069/23,728 exact, with 1,659 differences and zero errors. Every original
result matches the separate clean census, and all 804 addition images stayed
fixed. All 23,728 Chromium oracle identities and decoded images match v23.
The same 200 of 201 additions are exact at all four profiles. The
[v26 diagnostic selection](../../tools/qualification/manifests/expanded-v26.json)
lists those 200; the release contract retains all 201 and the fieldset legend
failure.

At the prior clean checkpoint `e14e3e64`, the census was
[21,264/22,924 exact](../renderer/generated/four-profile-census-v39.json),
with 1,660 differences, zero errors, and 916 unowned residual test IDs. The
[vertical-lr Ahem rotation-anchor repair](../renderer/vertical-lr-ahem-rotation-anchor.md)
made nine original comparisons exact, changed only those nine Open UI images,
and reduced wrong pixels by 30,184. No exact comparison regressed; all 22,924
Chromium oracle identities and decoded images stayed fixed. The clean
[v40 focused/primitive index](../renderer/generated/focused-primitive-raster-v40.json)
was 640/640 and 960/960 exact. The complete clean
[v22 expanded requalification](../renderer/generated/expanded-requalification-v22.json)
was 22,066/23,728 exact, with 1,662 differences and zero errors. One previously
failing native final-state comparison became exact, so 199 of 201 additions
were exact at all four profiles. The
[v24 diagnostic selection](../../tools/qualification/manifests/expanded-v24.json)
listed 199 exact additions; the release contract included all 201 and their
two failures at that checkpoint.

At the prior clean checkpoint `d39282e4`, the census was
[21,255/22,924 exact](../renderer/generated/four-profile-census-v38.json),
with 1,669 differences, zero errors, and 923 unowned residual test IDs. The
[analytic gradient-edge repair](../renderer/fractional-multicol-seam-investigation.md)
removed a redundant hard clip around a square gradient fill. Compared with
v37, seven comparisons became exact, no exact comparison regressed, and the
wrong-pixel count fell by 5,957. Exactly 29 Open UI images changed; all
22,924 Chromium oracle identities and decoded images stayed fixed. The clean
[v39 focused/primitive index](../renderer/generated/focused-primitive-raster-v39.json)
is 640/640 and 960/960 exact, with no decoded image changes from v38. The
clean [201-addition guard](../renderer/generated/expanded-additions-gradient-guard-v1.json)
is 801/804 exact with the same three failures and no Open UI or Chromium
image changes. That selected guard is not a complete expanded-manifest run;
the complete clean [v21 expanded requalification](../renderer/generated/expanded-requalification-v21.json)
at `e59ec07e` is 22,056/23,728 exact, 1,672 different, and zero errors.
Every original and added decoded image, status, and Chromium oracle identity
matches the separate clean census and addition guard. The
[v23 diagnostic selection](../../tools/qualification/manifests/expanded-v23.json)
lists 198 exact additions; all 201 remain in the release contract.

At the prior clean checkpoint `dc451061`, the census was
[21,248/22,924 exact](../renderer/generated/four-profile-census-v37.json),
with 1,676 differences, zero errors, and 924 unowned residual test IDs. The
[adjacent row-flex repair](../renderer/adjacent-row-flex-fragmentation.md)
made three comparisons exact and improved one other failing comparison by 118
pixels. No formerly exact image regressed, and all Chromium oracle identities
and decoded images stayed fixed. The clean
[v38 focused/primitive index](../renderer/generated/focused-primitive-raster-v38.json)
is 640/640 and 960/960 exact. The complete clean
[v20 expanded requalification](../renderer/generated/expanded-requalification-v20.json)
is 22,049/23,728 exact, 1,679 different, and zero errors; the same 198 of
201 additions are exact at all four profiles.

The [fractional multicolumn seam investigation](../renderer/fractional-multicol-seam-investigation.md)
records two remaining one-column pixel signatures against Chromium. Its
coverage explanation is still a hypothesis; those cases remain failing and
unowned for release qualification.

At the prior clean checkpoint `128edc38`, the v36 census was
[21,245/22,924 exact](../renderer/generated/four-profile-census-v36.json),
with 1,679 differences, zero errors, and 926 unowned residual test IDs. The
[fractional Ahem strike repair](../renderer/fractional-ahem-stripe-edges.md)
changed 21 already failing images and reduced wrong pixels by 281 without an
exact regression. All 22,924 Chromium oracle identities and decoded hashes
remained fixed. At the prior clean checkpoint `6676cf60`, the
[v35 delta audit](../renderer/generated/radial-ua-full-delta-v1.json) recorded
nine changed Open UI images, one new exact comparison, no exact regression,
and unchanged Chromium oracle identities and decoded images. The
[radial repair](../renderer/radial-tile-edge-coverage.md) left one wrong pixel
in each of two formerly 626-pixel differences. The
[object fallback repair](../renderer/object-fallback-host-clip.md) restored an
intermediate exact-case regression. The clean
[201-addition guard](../renderer/generated/expanded-additions-object-deferred-v1.json)
retained 801 exact and three different comparisons with no changed Open UI or
Chromium image; this selected guard is not a full expanded-manifest run. The
[new native final-state admission](../renderer/adjoining-floats-native-admission.md)
adds one four-profile-exact case while retaining all 200 earlier additions.
The complete clean expanded run at `6b53a991` is
[22,046/23,728 exact](../renderer/generated/expanded-requalification-v19.json),
with 1,682 differences and zero errors. Exactly 198 of 201 additions are
exact at all four profiles; the same three earlier additions remain failures.
All 22,924 original profile results match the v36 census, and all 804 added
profile results match the clean selected-additions guard, including their
Chromium oracle identities and decoded image hashes. The new full result has
one more exact original comparison than the older v18 expanded report.
The [v21 diagnostic selection](../../tools/qualification/manifests/expanded-v21.json)
lists 198 exact additions while the release contract retains all 201.
The [fixed-point broken-image repair](../renderer/broken-image-fractional-sampling.md)
made five original comparisons exact with no exact regression and no Chromium
oracle change. The clean
[v37 focused/primitive index](../renderer/generated/focused-primitive-raster-v37.json)
is 640/640 and 960/960 exact. The other 35
[AST-lowered candidates](../renderer/generated/pending-mutation-candidates-v7.json)
produced 18/140 exact profile comparisons, 122 differences, and zero errors at
clean checkpoint `8324c6b0`. None is exact at all four profiles. All 140
statuses, mismatch counts, and diff signatures match the prior report. The
release renderer gate remains open.

The prior clean census at `814a2005` is
[21,239/22,924 exact](../renderer/generated/four-profile-census-v30.json),
with 1,685 differences, zero errors, and 931 unowned residual test IDs. The
[table row-group clip investigation](../renderer/truncated-table-row-group-clip.md)
records the shared paint repair: the remaining repeated-section comparison
became exact at 1.25×, and the same case is exact in its
[40-profile sweep](../renderer/generated/table-row-group-cross-v1.json).
Exactly one Open UI image changed across the full census; no exact comparison
regressed, and all Chromium oracle identities and decoded hashes stayed
fixed. The clean
[v31 raster index](../renderer/generated/focused-primitive-raster-v31.json)
is 640/640 focused and 960/960 primitive exact, with all 1,600 decoded images
unchanged. The complete clean
[v16 expanded requalification](../renderer/generated/expanded-requalification-v16.json)
is 22,036/23,724 exact. All 800 addition images are unchanged, with 197 of
200 additions still exact and three demoted. The
[v17 diagnostic selection](../../tools/qualification/manifests/expanded-v17.json)
retains those 197. The clean
[v5 pending-candidate index](../renderer/generated/pending-mutation-candidates-v5.json)
again finds `adjoining-floats-dynamic` exact at all four profiles; it remains
outside admitted release coverage pending a new contract manifest.

The prior clean census at `7cd8e574` is
[21,238/22,924 exact](../renderer/generated/four-profile-census-v29.json),
with 1,686 differences, zero errors, and 932 unowned residual test IDs. The
[repeated table body-slice investigation](../renderer/repeated-table-body-slice.md)
records the shared layout-to-paint repair: 11 comparisons became exact and
one other difference shrank. Exactly 12 Open UI images changed, no exact
comparison regressed, and all Chromium oracle identities and decoded hashes
stayed fixed. The clean
[v30 raster index](../renderer/generated/focused-primitive-raster-v30.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v15 expanded requalification](../renderer/generated/expanded-requalification-v15.json)
is 22,035/23,724 exact, with 197 of 200 additions still exact and three
demoted. The [v16 diagnostic selection](../../tools/qualification/manifests/expanded-v16.json)
retains those 197. The clean
[v4 pending-candidate index](../renderer/generated/pending-mutation-candidates-v4.json)
again finds `adjoining-floats-dynamic` exact at all four profiles; it remains
outside admitted release coverage pending a new contract manifest.

The prior clean census at `a0e3f4cd` is
[21,227/22,924 exact](../renderer/generated/four-profile-census-v28.json),
with 1,697 differences, zero errors, and 937 unowned residual test IDs. The
[clipped replaced-background investigation](../renderer/clipped-replaced-background-coverage.md)
records the shared coverage rule: two margin-trim comparisons became exact,
and a third iframe comparison fell from 342 wrong pixels to six. Exactly three
Open UI images changed; every Chromium oracle identity and decoded hash stayed
fixed, and no exact comparison regressed. The clean
[v29 raster index](../renderer/generated/focused-primitive-raster-v29.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v14 expanded requalification](../renderer/generated/expanded-requalification-v14.json)
is 22,024/23,724 exact, with 197 of 200 additions still exact and three
demoted. The [v15 diagnostic selection](../../tools/qualification/manifests/expanded-v15.json)
retains those 197. The clean
[v3 pending-candidate index](../renderer/generated/pending-mutation-candidates-v3.json)
again finds `adjoining-floats-dynamic` exact at all four profiles; it remains
outside admitted release coverage pending a new contract manifest.

The prior clean census at `8950b426` is
[21,225/22,924 exact](../renderer/generated/four-profile-census-v27.json),
with 1,699 differences, zero errors, and 939 unowned residual test IDs.
Every residual diff signature matches the prior clean census, so the nested
float fix did not change the original comparison inventory. The
[v28 raster index](../renderer/generated/focused-primitive-raster-v28.json)
is 640/640 focused and 960/960 primitive exact. The complete clean
[v13 expanded requalification](../renderer/generated/expanded-requalification-v13.json)
is 22,022/23,724 exact, with the same three failing additions. The
[v14 diagnostic selection](../../tools/qualification/manifests/expanded-v14.json)
retains only the 197 exact additions. The clean
[v2 pending-candidate index](../renderer/generated/pending-mutation-candidates-v2.json)
records one newly eligible case, `adjoining-floats-dynamic`, at four of four
profiles. The other 35 pending cases remain open; the newly eligible case is
not yet counted as admitted release coverage.

The prior clean census at `954648fb` changed eight original Open UI images
in the resized one-axis bitmap family at fractional scales. The PNG background
now uses one shared image shader across its repeated destination, matching
Chromium's pattern path. Two 1280×720@1.25 comparisons became exact, six
others moved closer, and no comparison regressed. All 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[v26 census](../renderer/generated/four-profile-census-v26.json) is now
21,225 exact, 1,699 different, and zero errors, with 939 unowned residual
test IDs. The [bitmap pattern investigation](../renderer/resized-one-axis-bitmap-pattern.md)
records the source path and remaining one-to-three-pixel differences. The
clean [v27 raster index](../renderer/generated/focused-primitive-raster-v27.json)
is 640/640 focused and 960/960 primitive exact; all 1,600 Open UI and
Chromium decoded hashes stayed fixed. The clean
[v5 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v5.json)
is 797/800 exact with three differences and zero errors; all 800 Open UI and
Chromium decoded hashes stayed fixed. It does not replace a complete expanded
manifest run.

The prior clean census at `5c7aaa9c` changed eight original Open UI images
in the one-axis repeated linear-gradient family. Their gradient interiors now
match Chromium at the two fractional scales; each complete image still has
93–98 differing pixels on a separate PNG image edge. No exact comparison
regressed or became exact, and all 22,924 Chromium oracle identities and
decoded hashes stayed fixed. The [v25 census](../renderer/generated/four-profile-census-v25.json)
therefore remains 21,223 exact, 1,701 different, and zero errors, with 939
unowned residual test IDs. The [gradient investigation](../renderer/one-axis-gradient-picture-shader.md)
records the pinned Chromium source path, the eight image-level improvements,
and the remaining PNG edge. The clean [v26 raster index](../renderer/generated/focused-primitive-raster-v26.json)
is 640/640 focused and 960/960 primitive exact; all 1,600 Open UI and
Chromium decoded hashes stayed fixed. The clean [v4 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v4.json)
is 797/800 exact with three differences and zero errors; all 800 Open UI and
Chromium decoded hashes stayed fixed. It does not replace a complete expanded
manifest run.

The prior clean census at `2f560e46` repaired four mixed-border comparisons:
one at 800×600@1 and three at 375×667@2. Open UI changed exactly 16 images
in four border fixtures, every changed image moved closer to Chromium, and
no previously exact comparison regressed. All 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[border paint-order investigation](../renderer/mixed-border-paint-order.md)
records the source rule and remaining fractional-scale pixels. The remaining
1,701 differences across 939 unowned residual test IDs keep the renderer
gate red. The clean
[v25 raster index](../renderer/generated/focused-primitive-raster-v25.json)
is 640/640 focused and 960/960 primitive exact, with all 1,600 decoded
images unchanged. The clean
[v3 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v3.json)
is 797/800 exact with three differences and zero errors; all 800 decoded
images and statuses stayed fixed. It does not replace full expanded-manifest
qualification.

The prior clean census at `d0592ccd` repaired the 1280×720@1.25
`out-of-flow-in-multicolumn-047` comparison. The inner multicolumn source
had already been consumed, but its column boxes were replayed inside an
outer visual continuation. The same continuation also painted too much
parent background. Both now follow the source interval, while the direct
absolute child still paints in the later outer column. Only that Open UI
decoded image changed, and it became exact against Chromium. No previously
exact comparison regressed; all 22,924 Chromium oracle identities and
decoded hashes stayed fixed. The
[nested-column investigation](../renderer/multicol-consumed-nested-columns.md)
records both reduced diagnostic variants. The remaining 1,705 differences
and 939 unowned residual test IDs keep the renderer gate red. The clean
[v24 raster index](../renderer/generated/focused-primitive-raster-v24.json)
is 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI and
Chromium hashes unchanged from v23.
The clean [v2 selected-additions
recheck](../renderer/generated/expanded-additions-recheck-v2.json) at
`db763999` found 797/800 exact, three different, and zero errors, with all
800 Open UI and Chromium hashes unchanged from the prior full expanded run.
It is a diagnostic selection, not a complete expanded-manifest result.

The prior clean census at `6e21bd9b` repaired the 1280×720@1.25
`out-of-flow-in-multicolumn-046` comparison. A zero-height positioned parent
retains the visual continuation for its absolute child but paints no parent
background in that extra area. Only that Open UI decoded image changed, and
it became exact against Chromium. No previously exact comparison regressed;
all 22,924 Chromium oracle identities and decoded hashes stayed fixed. The
[zero-height continuation investigation](../renderer/multicol-zero-height-positioned-decoration.md)
records the reduced evidence and source rule. The remaining 1,706 differences
and 940 unowned residual test IDs keep the renderer gate red. The clean
[v23 raster index](../renderer/generated/focused-primitive-raster-v23.json)
is 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI and
Chromium decoded hashes unchanged from v22.

The complete clean expanded run at `6e21bd9b` measured 22,015/23,724 exact,
1,709 different, and zero errors. Only the same original `-046` comparison
changed; all 23,724 Chromium oracle identities and decoded hashes stayed
fixed. The [v12 ledger](../renderer/generated/expanded-requalification-v12.json)
retains 197 of the 200 native additions at all four profiles and demotes the
same three; the [v13 diagnostic selection](../../tools/qualification/manifests/expanded-v13.json)
records that set without changing the original manifest.

The prior clean census at `99fd4432` repaired three fractional-scale row
flex comparisons: `-029` at 1280×720@1.25 and `-030` at 1280×720@1.25 and
1920×1080@1.5. Only those three Open UI decoded images changed, and each
became exact against Chromium. No previously exact comparison regressed; all
22,924 Chromium oracle identities and decoded hashes stayed fixed. The
[final-decoration investigation](../renderer/flex-final-continuation-decoration.md)
records the reduced case and rejected broad rule. The remaining 1,707
differences and 941 unowned residual test IDs keep the renderer gate red. The
clean [v22 raster index](../renderer/generated/focused-primitive-raster-v22.json)
is 640/640 focused and 960/960 primitive exact. All 1,600 Open UI decoded
images and Chromium oracle hashes match the preceding clean raster run.

The complete clean expanded run at `99fd4432` measured 22,014/23,724 exact,
1,710 different, and zero errors. Only the same three original flex
comparisons changed; all 23,724 Chromium oracle identities and decoded hashes
stayed fixed. The [v11 ledger](../renderer/generated/expanded-requalification-v11.json)
retains 197 of the 200 native additions at all four profiles and demotes the
same three; the [v12 diagnostic selection](../../tools/qualification/manifests/expanded-v12.json)
records that set without changing the original manifest.

The prior clean census at `89a0a1f3` repaired one fractional-scale nested
row flex comparison. Exactly one Open UI decoded image changed and became
pixel exact against Chromium. No previously exact comparison regressed; all
22,924 Chromium oracle identities and decoded hashes stayed fixed. The
[first-line break investigation](../renderer/nested-row-flex-first-line-break.md)
records the reduced case and guard. The remaining 1,710 differences and 943
unowned residual test IDs keep the renderer gate red. The clean
[v21 raster index](../renderer/generated/focused-primitive-raster-v21.json)
is 640/640 focused and 960/960 primitive exact. All 1,600 Open UI decoded
images and Chromium oracle hashes match the preceding clean raster run.

The prior complete clean expanded run at `89a0a1f3` measured 22,011/23,724 exact,
1,713 different, and zero errors. Only that original flex comparison changed;
all 23,724 Chromium oracle identities and decoded hashes stayed fixed. The
[v10 ledger](../renderer/generated/expanded-requalification-v10.json) retains
197 of the 200 native additions at all four profiles and demotes the same
three; the [v11 diagnostic selection](../../tools/qualification/manifests/expanded-v11.json)
records that set without changing the original manifest.

The prior clean census at `17952772` repaired three fractional-scale
`out-of-flow-in-multicolumn-*` comparisons. Exactly three Open UI decoded
images changed, and each became pixel exact against Chromium. No previously
exact comparison regressed; all 22,924 Chromium oracle identities and decoded
hashes stayed fixed. The [relative continuation clip investigation](../renderer/multicol-relative-clip-translation.md)
records the cause and neighboring guards. At that checkpoint, 1,711
differences and 944 unowned residual test IDs kept the renderer gate red. The clean
[v20 raster index](../renderer/generated/focused-primitive-raster-v20.json)
at `ef7214b3` is 640/640 focused and 960/960 primitive exact. All 1,600
decoded Open UI images and Chromium oracle hashes match the preceding clean
raster run.

The complete clean expanded run at `de8304fe` measured 22,010/23,724 exact,
1,714 different, and zero errors. Four original column comparisons became
exact since the prior v8 expanded run; no other Open UI decoded image or
status changed. All 23,724 Chromium oracle identities and decoded hashes
stayed fixed. The [v9 ledger](../renderer/generated/expanded-requalification-v9.json)
retains 197 of the 200 native additions at all four profiles and demotes the
same three; the [v10 diagnostic selection](../../tools/qualification/manifests/expanded-v10.json)
records that set without changing the original manifest.

The complete clean census at `9f983df9` measured the nested positioned
continuation repair across all four profiles. One comparison became exact,
one neighboring image changed but remains different, and none regressed. All
22,924 Chromium oracle identities and decoded image hashes stayed fixed. The
remaining 1,715 differences and 948 unreviewed residual IDs keep the renderer
gate red. The [multicolumn investigation](../renderer/multicol-nested-positioned-continuation.md)
records the reduced reproducer and remaining seam. The focused and primitive
matrices stayed 640/640 and 960/960 exact. All 1,600 Open UI and Chromium
decoded hashes stayed fixed from the prior clean raster run.

The clean expanded run at the same checkpoint measured 22,006/23,724 exact,
1,718 different, and zero errors. All 200 native final-state additions kept
their prior four-profile statuses and all 800 Open UI/Chromium decoded hashes:
197 remain exact and three remain demoted. The
[v8 ledger](../renderer/generated/expanded-requalification-v8.json) and
[v9 diagnostic selection](../../tools/qualification/manifests/expanded-v9.json)
record this without changing the original manifest or Chromium oracle.

The earlier clean expanded run at `d73077e7` gained ten exact
`css_break` comparisons and made no other Open UI image changes. The 200
native final-state additions kept their prior statuses and decoded hashes;
197 remain exact at all four required profiles. The
[v3 requalification ledger](../renderer/generated/expanded-requalification-v3.json)
and [v4 diagnostic selection](../../tools/qualification/manifests/expanded-v4.json)
record this without changing the original manifest or Chromium oracle.

The [manual hardening run](https://github.com/zhuowcui/open-ui/actions/runs/36235389665)
on checkpoint `e555c442` records every job: X11/Mesa and pure-Wayland
conformance passed; native C UBSan passed; Rust 1.85 MSRV failed on dependency
metadata; Miri failed when the Engine test called Skia C FFI; AddressSanitizer,
LeakSanitizer, and fuzz failed on process-exit Fontconfig allocations. The
ordinary [PR CI run](https://github.com/zhuowcui/open-ui/actions/runs/36235359772)
passed Rust parity, Python accountability, platform conformance, and both
format checks. Its frozen replay was still running when that status was
written; this replay is now diagnostic. Later check results must be recorded
before qualification.

The next [hardening validation](https://github.com/zhuowcui/open-ui/actions/runs/36236727149)
on checkpoint `68db3c74` passed Rust 1.85 headless/Linux checks, the Miri C
handle test, and native C UBSan. ASan and LSan still report Fontconfig
allocations after all 17 FFI tests pass; fuzz stops on the same allocation
path. Its Linux platform smoke job was still running when this status was
written.

The later [hardening run](https://github.com/zhuowcui/open-ui/actions/runs/36313866551)
on checkpoint `73502f9d` passed MSRV, Miri, X11/Mesa and pure-Wayland
platform smoke, and C UBSan. All 18 FFI tests passed under both sanitizers,
then LeakSanitizer and AddressSanitizer reported 10,476 bytes in 236
Fontconfig allocations at process exit. Fuzz stopped on a 2,606-byte
Fontconfig allocation report.

The [manual hardening run](https://github.com/zhuowcui/open-ui/actions/runs/36365378115)
on the current `ceebef52` evidence checkpoint passed X11/Mesa and pure-Wayland
platform smoke, Miri handles, and C UBSan. AddressSanitizer and LeakSanitizer
again failed after all 18 ABI tests passed: each reported 10,476 bytes in 236
Fontconfig allocations at process exit. Fuzz failed on 2,606 bytes in 59
Fontconfig allocations. The MSRV check passed. These failures keep hardening
unqualified.

The optional frozen replay checks the archived Open UI bytes; it does not prove
those bytes equal Chromium. The historical pixel comparator accepted
per-channel differences up to 4 and excluded the rightmost 15 pixels. At least
186 of its 5,731 reported passes have a nonzero channel delta in the compared
area. The
archived result totals and generated kickoff baseline remain immutable
historical records, not zero-tolerance qualification evidence or pixel gates.
See the [frozen oracle audit](../renderer/frozen-oracle-audit.md).

The archived image for
`wpt/css_backgrounds/background-image-gradient-interpolation-repaint-ref`
differs from the pinned Chromium pixels at 800×600@1. That old Open UI image
is not a pixel target. The [historical evidence](../renderer/generated/frozen-oracle-audit-v1.json)
preserves the inputs and measured difference. Its old blocked status records
the retired two-gate policy; it is not the current release decision.

The final-head [PR CI run](https://github.com/zhuowcui/open-ui/actions/runs/36237911865)
passed its other jobs but failed the obsolete frozen replay and strict
historical-exactness steps. Those steps have been replaced by archive
integrity and metadata checks. The four-profile Chromium matrix remains a
separate, failing release gate; green PR CI alone does not qualify pixels.

## Release decision

Do not publish or tag v0.2 final while any required row is open. A release
candidate may be packaged for qualification. Final publication requires a
clean Git tree, two identical artifact runs for both architectures, all hosted
and lab gates, signed checksums/provenance, and recorded crates.io/native SDK
installation evidence.

Known technical gaps are maintained in
[`unsupported-features.md`](unsupported-features.md) and the implementation
area documents for compositor, animations, accessibility, and hardening.
