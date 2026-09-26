# Open UI v0.2 release qualification

The v0.2 source contract and artifact pipeline are implemented. This document
distinguishes verified repository evidence from release-lab work that cannot be
claimed by source code alone.

| Gate | Current evidence | State |
|---|---|---|
| Frozen headless replay | 5,549/5,731 byte-identical on clean checkpoint `5acc962a`; 182 changed | fail |
| Original Chromium exactness | 186 historical passes report nonzero channel deltas; comparator omitted the rightmost 15 pixels | blocked |
| Four-profile renderer matrix | 21,108/22,924 exact in the last complete diagnostic census | open |
| Expanded deterministic manifest | 200 admitted cases; 36 lowered cases await exact qualification | open |
| Accountability | 7/7 over 7,673 rows | pass |
| Rust workspace and docs | full locked workspace suite | pass |
| Rust/C application contract | 36 scenarios, 93 existing exports, four C examples and C++ consumer | pass |
| C-owned X11/Wayland application loop | no exported run/request-exit platform lifecycle yet | open |
| C platform accessibility | retained setters/actions exist; full adapter tree is not exported | open |
| Generated sources | style, ABI, migration, closure generators are read-only clean | pass |
| No-work frame | zero layout, paint, and raster on unchanged snapshots | pass |
| Mutation ownership | 10,000-iteration soak, no owned-object leak | pass |
| Local performance smoke | 0.108 ms p95, 308 UI-thread animation fps, 1.389% RSS growth | non-qualifying pass |
| X11/Wayland software and Mesa GL | hosted smoke workflow | pending first hosted run |
| Miri/sanitizers/fuzz | scheduled workflow | pending first scheduled run |
| x86-64/AArch64 SDK, deb, rpm | deterministic source pipeline and tag matrix | pending tag build |
| Clean Ubuntu/Fedora install | release workflow consumer jobs | pending tag build |
| Physical GPU/context loss | release-lab profile | open |
| 100 compositor animations during 250 ms UI stall | retained animation layers required | open |
| AT-SPI inspect/operate | release-lab accessibility session | open |
| Two signed reproducible builds | tag workflow, keyless signatures/attestations | open |
| crates.io publication | credentials and final release approval | open |

The checked-in performance artifact is a WSL2 smoke result and explicitly has
`qualification: false`. It must not be relabeled as reference-machine evidence.

The frozen replay checks the archived Open UI bytes; it does not prove those
bytes equal Chromium. The historical pixel comparator accepted per-channel
differences up to 4 and excluded the rightmost 15 pixels. At least 186 of its
5,731 reported passes have a nonzero channel delta in the compared area. The
archived result totals and generated kickoff baseline remain immutable
historical records, not zero-tolerance qualification evidence. See the
[frozen oracle audit](../renderer/frozen-oracle-audit.md).

## Release decision

Do not publish or tag v0.2 final while any required row is open. A release
candidate may be packaged for qualification. Final publication requires a
clean Git tree, two identical artifact runs for both architectures, all hosted
and lab gates, signed checksums/provenance, and recorded crates.io/native SDK
installation evidence.

Known technical gaps are maintained in
[`unsupported-features.md`](unsupported-features.md) and the implementation
area documents for compositor, animations, accessibility, and hardening.
