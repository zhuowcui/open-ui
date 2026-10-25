---
id: 0079
title: Forced scrollbars follow Chromium scroller compositing
tags: paint, scrollbar, compositing, cc, device-scale, chromium-oracle, tracing
status: active
created: 2026-10-25
updated: 2026-10-25
refs: bindings/rust/openui-paint/src/painter.rs, docs/progress/scrollbar-compositing-model-2026-10-25.md
---

Element scrollbar frames use Blink integer geometry (PixelSnappedBorderBoxSize at the rounded paint offset, bars anchored at the far edge; an oversized bar leaves the box on the start/top side). When frames protrude (OverflowControlsClip exists): composited scroller -> enclosing scissor plus track-colour fill of partial edge texels (cc ClearForOpaqueRaster / solid quad); otherwise the snapped clip is antialiased in the page layer. Composited = Blink overflow (ScrollWidth.Round() > unclamped client.Round()) and (dsf >= 1.5, LCD text ignored, or empty scroll area). 50/50 pinned frames and 1,600/1,600 Linux probes exact. Method: Windows CfT DevTools Tracing with disabled-by-default-cc.debug(+quads, display_items, viz.quads) gives layer trees and display-list SkPictures (decode CLIP_RECT params bit 4 = AA); pick the last cc::LayerTreeHostImpl snapshot that contains a LayoutView layer (others are browser UI). Headless uses viz SoftwareRenderer; layer space is CSS px rastered at the device scale.
