---
id: 0013
title: SP13 residuals (paused): ~48 hard fragmentation/multicol tails
tags: sp13, residuals, paused
status: active
created: 2026-07-08
updated: 2026-07-08
refs: bindings/rust/pixel-compare/src/wpt/wpt_css_break.rs
---

~48 hard, heterogeneous SP13 fragmentation/multicol residuals remain, paused behind the text track. Sample tail candidates for later: overflowed-block-with-no-room-after-000/001, trailing-child-margin-003, tall-float-pushed-to-next-fragmentainer-003, overflow-clip-017, box-decoration-break-clone-005.tentative (fails on paint ORDERING of red cloned bottom border over abspos green child — geometry correct). Use `pixel_compare debug wpt/css_break/<id>` to dump fragment trees before attempting fixes.
