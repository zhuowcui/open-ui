# Open UI v0.2 release qualification

The v0.2 source contract and artifact pipeline are implemented. This document
distinguishes verified repository evidence from release-lab work that cannot be
claimed by source code alone.

| Gate | Current evidence | State |
|---|---|---|
| Frozen headless rendering | fresh 5,731/5,731 byte replay, zero tolerance | pass |
| Accountability | 7/7 over 7,673 rows | pass |
| Rust workspace and docs | full locked workspace suite | pass |
| Rust/C application contract | 36 scenarios, 84 frozen exports, four C examples and C++ consumer | pass |
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

## Release decision

Do not publish or tag v0.2 final while any required row is open. A release
candidate may be packaged for qualification. Final publication requires a
clean Git tree, two identical artifact runs for both architectures, all hosted
and lab gates, signed checksums/provenance, and recorded crates.io/native SDK
installation evidence.

Known technical gaps are maintained in
[`unsupported-features.md`](unsupported-features.md) and the implementation
area documents for compositor, animations, accessibility, and hardening.
