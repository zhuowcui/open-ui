---
id: 0053
title: SP19 W1A closes the float and inline repair partition
tags: sp19 float inline intrinsic-sizing margin-collapse accountability handoff
status: active
created: 2026-08-31
updated: 2026-08-31
refs: bindings/rust/openui-layout/src/inline/algorithm.rs, bindings/rust/openui-layout/src/block.rs, tools/wpt/generate_sp19_closure.py
---

SP19 W1A repairs all 33 float/inline targets without changing an SP18 exact
non-target. The shared fixes cover source-order float placement, nowrap rewind,
full-height line opportunities, zero-height float coordinates, nested floats in
anonymous inline runs, block-in-inline margin propagation, shrink-to-fit
float-plus-line contributions, and two-estimate BFC relayout/separation.

The live proof is 4,091 exact, 48 failures, and zero errors across 4,139 runnable
rows. The release 33-ID partition is 33/33 exact, the repaired-baseline manifest
retains SHA-256 `b8392afabf69482e051efb41ebdbb7a4d37b17909bfc2b41f45c540764608b8a`,
and both SP19 no-write checks report `wave=w1a_float_inline`. Verification also
passes the locked style/text/layout/paint matrix, 150 historical Python tests,
release `pixel_compare`, deterministic mapping/deferred/report regeneration,
formatting and diff checks, and the PNG-backed repository audit 7/7.
