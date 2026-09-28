# v0.2 conformance and hardening

The versioned application suite is defined by
`tools/conformance/v02-scenarios.json`. Its verifier requires at least 30
unique scenarios, complete coverage of ten product domains, and exact parity
with the public Rust integration tests. The current suite contains 36
scenarios. It covers retained lifecycle and ownership, large mutations, core
controls, pointer and keyboard routing, Unicode editing, IME, clipboard,
focus, scrolling, typed animation timelines, accessibility, resources,
responsive layout, bidi/emoji text, and deterministic headless frames.

The first full run found a non-unwinding rust-skia assertion for mixed
Hebrew/Arabic text containing an emoji ZWJ sequence. Open UI's run handler had
allocated each Skia glyph buffer using the final metadata callback rather than
the exact per-buffer `RunInfo`. The collector now sizes and commits every run
from its corresponding callback, with both shaping-level and application-level
regressions retained.

## Memory and performance

`openui-engine` exposes read-only `EngineObjectCounts` alongside lifecycle
statistics. The release-mode `v02_perf` harness warms the allocator and raster
paths, performs 10,000 mutation-to-present interactions, measures p95 latency
and RSS growth, checks owned object counts, drives 100 animations, and proves
that an unchanged scene performs no layout, paint, or raster work.

The contractual thresholds and required environment are frozen in
`tools/performance/v02-reference-profile.json`. Results are versioned below
`tools/performance/results`. A WSL2 developer result is checked in as a smoke
artifact and explicitly has `qualification: false`; it is not evidence for
the physical-GPU, idle-event-loop, AT-SPI, or blocked-UI-thread release gates.
Those remain open until they run on the declared exclusive reference machine.

## Automated hardening

The ordinary CI verifies the scenario manifest and performance schema. The
hardening workflow runs all 40 scenarios, Mesa OpenGL on X11 through Xvfb, and
software presentation on a pure headless Wayland compositor for pull requests.
Its scheduled/manual jobs additionally compile the public headless and Linux
surfaces at the Rust 1.85 MSRV, check C opaque-handle ownership under Miri,
exercise the C ABI under AddressSanitizer and LeakSanitizer, run C consumers
under UndefinedBehaviorSanitizer, and execute bounded sessions of all five
fuzz targets.

These checks do not convert currently open product boundaries into passing
claims. In particular, the immutable scene still lacks retained per-layer
animation curves. Direct Ganesh GPU-surface replay builds but has not passed
the raster qualification matrices. Their strict W8/W9/W10 release gates remain
unqualified.
