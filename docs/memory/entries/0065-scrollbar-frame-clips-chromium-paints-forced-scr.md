---
id: 0065
title: Scrollbar frame clips: Chromium paints forced-scrollbar parts pixel-rounded; layout keeps CSS thickness
tags: scrollbar, paint, chromium, oracle, overflow-controls
status: active
created: 2026-10-20
updated: 2026-10-21
refs: docs/progress/native-scrollbar-frame-clip-2026-10-20.md, tools/qualification/capture_native_scrollbar_oracle.py, bindings/rust/openui-engine/tests/native_resize_control_clip.rs
---

On 20 October 2026 the native_resize_control_clip guard failure was resolved. Pinned Chromium 147.0.7727.50 (fresh capture reproducing all 50 historical ordinary-Div frames) paints ink outside the guard's pixel-centre border rule (e.g. row 57 at 1.25), so that rule was wrong; the guard now uses Chromium's measured ink extent per state. Chromium layout keeps scrollbar thickness at 15/10 CSS px at every scale (CDP probe: client 85x65 / 90x70, 85.25x65.5 for a fractional box), so Blink's ClampRound device thickness affects paint geometry only. Forced bars without a scroll range are ordinary pictures whose parts round to physical pixels independently; paint them directly under the device transform without antialiasing, never offscreen with fractional-origin resampling. 46/50 frames exact; four residual compositor edge cases remain. The source_layer_raster viewport arrow test needs Skia built with -DSK_ENABLE_LEGACY_SHADERCONTEXT, as the pinned parity profile does; since 21 October 2026 every hosted configuration sets it.
