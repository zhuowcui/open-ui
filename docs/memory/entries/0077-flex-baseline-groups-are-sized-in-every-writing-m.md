---
id: 0077
title: Flex baseline groups are sized in every writing mode
tags: layout, flex, baseline, line-clamp, vertical
status: active
created: 2026-10-24
updated: 2026-10-24
refs: bindings/rust/openui-layout/src/flex/algorithm.rs, docs/progress/flex-baseline-group-2026-10-24.md
---

compute_line_cross_sizes now adds max baseline ascent + descent for baseline-aligned row items in vertical writing modes and lines with line-clamped items (Blink sizes every baseline-sharing line). A clamped item's last baseline is capped at its cross edge; a clamped item with no content baseline is skipped (pinned by baseline_line_cross_size_preserves_legacy_line_clamp_measurement). Missing baselines must be synthesized exactly as final placement does (central for upright/mixed vertical, block-start for sideways vertical-lr); synthesizing at the cross edge lost baseline-synthesis-003/004. Census 21,699 -> 21,701, 0 losses; fixed line-clamp-003 (owner was flex sizing, not the clamp marker). Chrome geometry can be probed with the Windows harness (windows_chrome.WindowsChrome + compare_windows_text page builder).
