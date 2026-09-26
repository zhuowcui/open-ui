---
id: 0051
title: SP17 W2B–W4 closes the remaining actionable writing-mode cohort
tags: sp17, writing-mode, bidi, vertical-text, fallback, flex, fragmentation, overflow, sizing, accountability, handoff
status: active
created: 2026-08-27
updated: 2026-08-27
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/inline/items_builder.rs, bindings/rust/openui-text/src/shaping/shaper.rs, bindings/rust/openui-paint/src/painter.rs, tools/wpt/sp17_w2b_w4_targets.json, tools/wpt/sp17_w2b_w4_focused_ids.json, tools/accountability/data/pixel_comparison/results/summary.json
---

W2B–W4 closes one atomic 132-ID cohort: every remaining non-exact ID in
SP17's frozen actionable ledger plus the `css-flexbox-test1` test/reference
pair. The target partition is 43 flexbox, 37 fragmentation, 22 sizing, 20
overflow, five multicol, three backgrounds, one float, and one positioned ID.

Bidi is authoritative from styled inline collection through line breaking and
visual placement. Text is split at bidi, script, grapheme-safe fallback, and
vertical-orientation boundaries before shaping. Upright runs use vertical
OpenType substitutions and advances; sideways runs preserve horizontal
shaping. Fragment orientation, baselines, ruby, text-combine, decorations,
emphasis, shadows, clipping, and emoji metadata flow into paint without CSS or
Unicode rediscovery.

OpenUI and the manifest-scoped Chromium fontconfig register the same ordered,
byte-pinned Droid CJK, Noto Devanagari, and Noto Color Emoji faces. Their hashes
are `27db42b79d0846f6fd01b3d6a8233df9a8a5ece80b042299dc4174c48213ffd3`,
`b1dffa1fccb30dc45287111834a9db15c652b05d4d67201abe73e67717017590`, and
`72a635cb3d2f3524c51620cdde406b217204e8a6a06c6a096ff8ed4b5fd6e27b`.

Only the 132 target builders were surgically regenerated. The target manifest
is pinned at `0085f0df34162f355c1f2a24deae01967cf52f1753049f27a32ec7425e3ce089`.
The 325-ID proof contains all 193 earlier SP17 promotions plus the new targets;
it is pinned at `78efe59229615167e9603c6e40295c3eca0937c2f973f1097cd98a545d2793d3`
and finishes 325/325 exact with zero errors.

Exactly 121 formerly unported targets became runnable and 11 pinned failures
became exact without a non-target status or mismatch change. The authoritative
complete run is 3,889 runnable, 3,619 exact, 270 failures, and zero errors.
There are 3,784 unported rows, 517 live `needs_writing_mode` rows, 945 unported
`sp13_multicol` rows, and 1,025 text-manifest IDs. The validator requires 325
promotions and preserves the 73-ID SP13-R later-promotion allowlist. The
committed `summary.json` SHA-256 is
`2021d915414b470edddf51ff266ae7494240f7e5b60ede4410452da4c46784ad`.
