# Native scroll notifications

Rust and C applications receive scroll notifications through the shared native
Document event pipeline. Open UI executes no JavaScript. Offset changes remain
immediate; callbacks run on the owning thread during dispatch_pending_events or
the next native frame, after Engine and task borrows are released.

The Engine queues ordered, coalesced live handle identities from programmatic,
accessibility, wheel, smooth-animation, layout-clamp and snap offset producers.
The native task dispatcher captures a bounded notification batch. Callback
scrolls await a later turn; recursive pumping does no work. No-op/clamped
requests queue nothing. Removed generations cannot notify a reused handle.
The captured batch and the next turn have separate coalescing identities.
A callback may queue a later event for an undelivered captured target. Unwinding
does not duplicate an already delivered event or discard either batch. Unchanged pumping does no layout or raster work.

Element scroll is nonbubbling and noncancelable. Capture follows the ordinary
native event route. The framework viewport represents document scroll and
uses a bubbling notification. Explicit application Event options remain owned
by the application. The C event and document-pump interfaces remain unchanged.

Pinned Chromium 147.0.7727.50 source specifies the corresponding behavior:
[Document::EnqueueScrollEventForNode](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/core/dom/document.cc),
lines 6535–6543, distinguishes document from element propagation and coalesces
per-frame targets.
[PaintLayerScrollableArea](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/core/paint/paint_layer_scrollable_area.cc),
lines 485–500 and 623–633, schedules changed offsets and defers layout clamps
until a later frame.
[ScriptedAnimationController](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/core/dom/scripted_animation_controller.cc),
lines 127–159, separates a captured frame batch from newly queued events.
The native pump provides application-controlled task turns;
this source change does not claim independent compositor animation delivery.

The retained source receipt is native-scroll-notification-source-v4430/
primary-source-receipt.json. Required qualification includes the public Rust
scroll-event guards, the existing C/C++ directional-scroll callback consumers,
selection task resumption, unchanged pixels and the complete native workspace.
The source checkpoint alone does not qualify renderer or release gates.
