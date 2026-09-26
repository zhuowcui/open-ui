# Open UI v0.2 release qualification

The v0.2 source contract and artifact pipeline are implemented. This document
distinguishes verified repository evidence from release-lab work that cannot be
claimed by source code alone.

| Gate | Current evidence | State |
|---|---|---|
| Frozen headless replay | 5,549/5,731 byte-identical in the [complete replay](../renderer/generated/frozen-replay-v1.json); 182 changed | fail |
| Original Chromium exactness | 203 frozen images differ from pinned Chromium; one minimal case differs by 4,348 pixels | blocked: conflicts with frozen replay |
| Four-profile renderer matrix | 21,111/22,924 exact, 1,813 different, zero errors in the [fresh census](../renderer/generated/four-profile-census-v2.json) | fail |
| Focused and primitive raster | 640/640 focused exact; 884/960 primitive exact with four unowned residual IDs | fail |
| Expanded deterministic manifest | Prior v1 admitted 200; fresh run retains 197 exact additions and [demotes three](../renderer/generated/expanded-requalification-v1.json). None of the 36 original pending cases met all four profiles | open |
| Accountability | 7/7 over 7,673 rows | pass |
| Rust workspace and docs | full locked workspace suite | pass |
| Rust 1.85 MSRV | compatible string-boundary implementation is committed. The next hosted run found locked Wayland/Zbus dependencies requiring Rust 1.86/1.87; the lockfile now selects versions declaring 1.85 support, with hosted rerun pending | open |
| Rust/C application contract | 36 scenarios, 93 existing exports, four C examples and C++ consumer | pass |
| C-owned X11/Wayland application loop | no exported run/request-exit platform lifecycle yet | open |
| C platform accessibility | retained setters/actions exist; full adapter tree is not exported | open |
| Generated sources | style, ABI, migration, closure generators are read-only clean | pass |
| No-work frame | zero layout, paint, and raster on unchanged snapshots | pass |
| Mutation ownership | 10,000-iteration soak, no owned-object leak | pass |
| Local performance smoke | 0.108 ms p95, 308 UI-thread animation fps, 1.389% RSS growth | non-qualifying pass |
| X11/Wayland software and Mesa GL | hosted PR hardening passed both smoke paths on the pushed evidence checkpoint; final-head rerun pending | provisional pass |
| Miri/sanitizers/fuzz | latest hardening run: Miri reached unsupported Skia C FFI; an actual opaque-handle registry test now avoids that FFI, with hosted rerun pending. ASan, LSan, and fuzz still report Fontconfig allocations at exit. Native C UBSan passed | fail |
| x86-64/AArch64 SDK, deb, rpm | deterministic source pipeline and tag matrix | pending tag build |
| Clean Ubuntu/Fedora install | release workflow consumer jobs | pending tag build |
| Physical GPU/context loss | release-lab profile | open |
| 100 compositor animations during 250 ms UI stall | retained animation layers required | open |
| AT-SPI inspect/operate | release-lab accessibility session | open |
| Two signed reproducible builds | tag workflow, keyless signatures/attestations | open |
| crates.io publication | credentials and final release approval | open |

The checked-in performance artifact is a WSL2 smoke result and explicitly has
`qualification: false`. It must not be relabeled as reference-machine evidence.

The [manual hardening run](https://github.com/zhuowcui/open-ui/actions/runs/36235389665)
on checkpoint `e555c442` records every job: X11/Mesa and pure-Wayland
conformance passed; native C UBSan passed; Rust 1.85 MSRV failed on dependency
metadata; Miri failed when the Engine test called Skia C FFI; AddressSanitizer,
LeakSanitizer, and fuzz failed on process-exit Fontconfig allocations. The
ordinary [PR CI run](https://github.com/zhuowcui/open-ui/actions/runs/36235359772)
passed Rust parity, Python accountability, platform conformance, and both
format checks. Its strict frozen replay was still running when this status was
written. Later check results must be recorded before qualification.

The frozen replay checks the archived Open UI bytes; it does not prove those
bytes equal Chromium. The historical pixel comparator accepted per-channel
differences up to 4 and excluded the rightmost 15 pixels. At least 186 of its
5,731 reported passes have a nonzero channel delta in the compared area. The
archived result totals and generated kickoff baseline remain immutable
historical records, not zero-tolerance qualification evidence. See the
[frozen oracle audit](../renderer/frozen-oracle-audit.md).

The archived image for
`wpt/css_backgrounds/background-image-gradient-interpolation-repaint-ref`
differs from the pinned Chromium pixels at 800×600@1. Since frozen replay
requires those archived bytes and the original matrix requires Chromium's
pixels for the same fixture, the two mandatory gates cannot both pass. This
is a release-contract blocker even if the current renderer's other residuals
are repaired. The [minimal proof](../renderer/generated/frozen-oracle-audit-v1.json)
preserves the inputs and measured difference.

## Release decision

Do not publish or tag v0.2 final while any required row is open. A release
candidate may be packaged for qualification. Final publication requires a
clean Git tree, two identical artifact runs for both architectures, all hosted
and lab gates, signed checksums/provenance, and recorded crates.io/native SDK
installation evidence.

Known technical gaps are maintained in
[`unsupported-features.md`](unsupported-features.md) and the implementation
area documents for compositor, animations, accessibility, and hardening.
