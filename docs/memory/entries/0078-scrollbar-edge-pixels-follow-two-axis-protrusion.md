---
id: 0078
title: Scrollbar edge pixels follow two-axis protrusion
tags: paint, scrollbar, clip, device-scale, chromium-oracle
status: superseded
created: 2026-10-24
updated: 2026-10-25
refs: bindings/rust/openui-paint/src/painter.rs, docs/progress/scrollbar-two-axis-clip-2026-10-24.md
---

Forced scrollbar pictures clip to the enclosing device pixels of the border box only when the control frames protrude in both axes (OverflowControlsFrameGeometry::protrudes_both_axes); one-axis protrusion keeps the snapped clip. Thin vs Auto is not the discriminator (thin w1 h5/h8/h10 paint the partial column; auto w1 h16+ does not). Pinned frames 46 -> 47 exact. Remaining residuals: #fdfdfd/#fefefe last row/column (likely composited PaintedScrollbarLayer nine-patch bilinear stretching; Blink composites when MayCompositeScrollbar: ScrollsOverflow && Maximum != 0), irregular one-axis partial columns at 1.75, and CSS-vs-device thin thickness (10 css vs round(12.5)=13 device). Windows CfT matches Linux for Auto and arrow-free Thin frames; Thin arrows differ on Windows, so do not use Windows Thin arrow frames as Linux evidence. Chromium sources at the pinned tag are fetchable from chromium.googlesource.com/.../+/refs/tags/147.0.7727.50/<path>?format=TEXT (base64).
Superseded 2026-10-25 by 0079: the two-axis rule approximated Chromium's composited-scroller path.
