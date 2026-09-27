# Open UI v0.2 release qualification

The v0.2 source contract and artifact pipeline are implemented. This document
distinguishes verified repository evidence from release-lab work that cannot be
claimed by source code alone.

| Area | Current evidence | State |
|---|---|---|
| Historical Open UI archive | Archive and records are byte-pinned; optional [replay](../renderer/generated/frozen-replay-v1.json) found 5,549/5,731 unchanged, 182 changed | provenance pass; replay diagnostic |
| Chromium pixel target | Pinned Chromium 147 is the sole expected output for the declared renderer tests | see matrix below |
| Four-profile renderer matrix | 21,179/22,924 exact, 1,745 different, zero errors in the [latest clean census](../renderer/generated/four-profile-census-v12.json) | fail |
| Focused and primitive raster | 640/640 focused exact; 960/960 primitive exact in the [latest raster index](../renderer/generated/focused-primitive-raster-v13.json) | pass |
| Direct Ganesh raster | Clean Mesa llvmpipe [comparison](../renderer/generated/ganesh-raster-comparison-v1.json): 408/640 focused and 624/960 primitive exact; CPU remains the qualification backend | unpromoted |
| Expanded native final-state fixtures | Clean full expanded run retains 197 of 200 exact additions and [demotes three](../renderer/generated/expanded-requalification-v2.json); 21,966/23,724 total comparisons exact, zero errors. None of the 36 original pending cases met all four profiles; no JavaScript is run by Open UI | open |
| Accountability | 7/7 over 7,673 rows | pass |
| Rust workspace and docs | full locked workspace suite | pass |
| Rust 1.85 MSRV | [hosted hardening](https://github.com/zhuowcui/open-ui/actions/runs/36313866551): locked headless and Linux checks passed at the prior pushed checkpoint | pass |
| Rust/C application contract | 37 scenarios, 102 existing exports, five C examples and C++ consumer; owned C accessibility-tree snapshots now exported, native C window loop still open | partial |
| Native element interaction | Public Rust `Document` and `Element` APIs cover lookup, mutation, callbacks, activation, focus, scrolling, and controls; browser-style operations needed by applications must be exposed through native APIs | core implemented; remaining API coverage review open |
| C-owned X11/Wayland application loop | no exported run/request-exit platform lifecycle yet | open |
| C platform accessibility | owned full-tree snapshots, node metadata/relations/focus, and changed/removed IDs export from the shared engine; automated AT-SPI operation in a C window remains unqualified | open |
| Generated sources | style, ABI, migration, closure generators are read-only clean | pass |
| No-work frame | zero layout, paint, and raster on unchanged snapshots | pass |
| Mutation ownership | 10,000-iteration soak, no owned-object leak | pass |
| Local performance smoke | 0.108 ms p95, 308 UI-thread animation fps, 1.389% RSS growth | non-qualifying pass |
| X11/Wayland software and Mesa GL | [hosted hardening](https://github.com/zhuowcui/open-ui/actions/runs/36313866551) passed both smoke paths at the prior pushed checkpoint; final-head rerun pending | provisional pass |
| Miri C handle ownership | [hosted hardening](https://github.com/zhuowcui/open-ui/actions/runs/36313866551): opaque-handle ownership test passed under pinned Miri | pass |
| ASan/LSan/fuzz | [hosted hardening](https://github.com/zhuowcui/open-ui/actions/runs/36313866551): all 18 FFI tests passed, then both sanitizers reported 10,476 bytes through Fontconfig at process exit; fuzz stopped on a 2,606-byte Fontconfig allocation report | fail |
| Native C UBSan | [hosted hardening](https://github.com/zhuowcui/open-ui/actions/runs/36313866551): ABI consumers passed | pass |
| x86-64/AArch64 SDK, deb, rpm | deterministic source pipeline and tag matrix | pending tag build |
| Clean Ubuntu/Fedora install | release workflow consumer jobs | pending tag build |
| Physical GPU/context loss | release-lab profile | open |
| 100 compositor animations during 250 ms UI stall | retained animation layers required | open |
| AT-SPI inspect/operate | release-lab accessibility session | open |
| Two signed reproducible builds | tag workflow, keyless signatures/attestations | open |
| crates.io publication | credentials and final release approval | open |

The checked-in performance artifact is a WSL2 smoke result and explicitly has
`qualification: false`. It must not be relabeled as reference-machine evidence.

The complete clean census at `e942aebc` measured the multicolumn start-clip
repair across all four profiles. Ten comparisons became exact, none regressed,
and all Chromium decoded image hashes stayed fixed. The remaining 1,745
differences and 968 unreviewed residual IDs keep the renderer gate red.

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
Fontconfig allocation report. These jobs remain failed; the current paint
checkpoint has not yet had a separate hardening rerun.

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
