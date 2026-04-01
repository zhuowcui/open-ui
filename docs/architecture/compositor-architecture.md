# Compositor Architecture (cc/)

> Architecture document for the Open UI project.
> Deep technical reference on Chromium's compositor layer (`cc/`), annotated
> with extraction boundaries and the proposed C ABI surface.

---

## Table of Contents

1. [Compositor Overview](#1-compositor-overview)
2. [Layer Tree Architecture](#2-layer-tree-architecture)
3. [Tile Management & Rasterization](#3-tile-management--rasterization)
4. [Frame Scheduling (Scheduler State Machine)](#4-frame-scheduling-scheduler-state-machine)
5. [Drawing & Output](#5-drawing--output)
6. [Animations](#6-animations)
7. [Scrolling](#7-scrolling)
8. [Key Dependencies](#8-key-dependencies)
9. [Open UI Extraction Strategy](#9-open-ui-extraction-strategy)
10. [Source File Reference](#source-file-reference)

---

## 1. Compositor Overview

### Purpose

The compositor (`cc/`) is the stage of the rendering pipeline that sits between
paint and display.  Its responsibilities:

- **Frame production** — deciding *when* to draw and coordinating the main
  thread, compositor thread, and raster workers.
- **Layer management** — maintaining a tree of composited layers with
  transform, clip, opacity, and scroll state.
- **Off-main-thread work** — driving scroll, pinch-zoom, and CSS animations
  on the compositor thread so the main thread can remain free for JavaScript.
- **Tiled rasterization** — breaking painted content into tiles, scheduling
  their rasterization in priority order, and uploading results to the GPU.
- **Frame assembly** — producing a `CompositorFrame` (a set of render passes
  and draw quads) that describes how to composite every layer into the final
  image.

### Location

All compositor code lives under `cc/` in the Chromium source tree:

```
cc/
├── animation/        # Animation host, timelines, keyframe models
├── base/             # Math helpers, region, switches
├── input/            # Compositor-side input handling (scroll, pinch)
├── layers/           # Layer and LayerImpl types
├── metrics/          # Frame-sequence trackers, UMA reporting
├── paint/            # Display item list, paint worklet
├── raster/           # Raster buffer providers, tile rasterisation
├── resources/        # Resource pool, transferable resource
├── scheduler/        # Scheduler, state machine, begin-frame source
├── slim/             # Experimental slim compositor (subset)
├── tiles/            # TileManager, Tile, TilePriority
├── trees/            # LayerTreeHost(Impl), property trees, tree sync
└── viz/              # Client-side viz integration (frame sink)
```

### Key Classes

| Class | Thread | Role |
|-------|--------|------|
| `LayerTreeHost` | Main | Owns the main-thread layer tree; entry point for paint invalidation and property changes. |
| `LayerTreeHostImpl` | Compositor | Owns the active & pending trees; drives raster, animation ticking, draw. |
| `ProxyMain` / `ProxyImpl` | Both | Thread-hopping proxies that ferry messages between `LayerTreeHost` and `LayerTreeHostImpl`. |
| `Scheduler` | Compositor | State machine that decides when to request main frames, commit, activate, and draw. |
| `TileManager` | Compositor | Manages tile lifecycle: create, prioritize, raster, evict. |
| `AnimationHost` | Both | Maintains animation timelines; ticks on compositor thread. |

### Single-Process vs Multi-Process

In Chromium's production architecture the compositor produces a
`CompositorFrame` that is shipped via Mojo IPC to the **viz** service running
in the GPU process.  Viz aggregates frames from multiple clients (browser UI,
each renderer) and issues the final GL/Vulkan draw calls.

```
  Chromium (multi-process)
  ┌───────────────┐        Mojo         ┌──────────────┐
  │ Renderer      │  CompositorFrame    │ GPU process   │
  │  cc/ ─────────┼────────────────────►│  viz/         │
  │               │                     │  Display-     │
  │               │                     │  Compositor   │
  └───────────────┘                     └──────┬───────┘
                                               │
                                          Skia / GL
                                               │
                                               ▼
                                        ┌────────────┐
                                        │ Window /   │
                                        │ Surface    │
                                        └────────────┘
```

**Open UI eliminates this split.** The compositor runs in the same process as
the application and renders directly to a caller-provided surface:

```
  Open UI (single-process, in-process compositor)
  ┌──────────────────────────────────────────────┐
  │ Application process                          │
  │                                              │
  │  Main thread        Compositor thread        │
  │  ┌──────────┐       ┌─────────────────┐      │
  │  │ Layout / │ commit│ cc/ scheduler   │      │
  │  │ Paint    │──────►│ tile manager    │      │
  │  └──────────┘       │ draw ──► Skia ──┼──►Surface
  │                     └─────────────────┘      │
  └──────────────────────────────────────────────┘
```

No Mojo, no GPU process, no viz aggregation.  `cc/` draws directly to an
`SkCanvas` backed by a GPU or software surface the embedder owns.

---

## 2. Layer Tree Architecture

### Main-Thread vs Compositor-Thread Layers

The compositor maintains **two parallel representations** of the layer tree:

| Aspect | `cc::Layer` | `cc::LayerImpl` |
|--------|-------------|-----------------|
| Thread | Main | Compositor |
| Header | `cc/layers/layer.h` | `cc/layers/layer_impl.h` |
| Lifetime | Persistent across frames | Rebuilt on each commit (pending tree) then activated |
| Mutability | Freely mutated by paint code | Read-only after activation; next frame's changes go into pending tree |

During **Commit** the main-thread tree is serialised into a pending
`LayerImpl` tree on the compositor thread.  Once rasterisation completes the
pending tree is **activated** and becomes the new active tree used for drawing.

```
  Main thread                   Compositor thread
  ┌──────────┐                  ┌──────────────────────┐
  │ Layer    │     Commit       │  Pending LayerImpl   │
  │ tree     │─────────────────►│  tree                │
  │ (mutable)│                  │                      │
  └──────────┘                  └─────────┬────────────┘
                                          │ Activate
                                          ▼
                                ┌──────────────────────┐
                                │  Active LayerImpl    │
                                │  tree (used for draw)│
                                └──────────────────────┘
```

### Layer Types

```cpp
// cc/layers/ — each subclass overrides AppendQuads() to emit draw quads.
class PictureLayer    : public Layer;       // Painted content (display items → tiles)
class SolidColorLayer : public Layer;       // Flat color rect (no raster needed)
class SurfaceLayer    : public Layer;       // Embeds another CompositorFrame (viz surface)
class TextureLayer    : public Layer;       // Externally-provided GPU texture
class VideoLayer      : public Layer;       // Video frame overlay
class ScrollbarLayer  : public Layer;       // Painted or solid-color scrollbar
class MirrorLayer     : public Layer;       // Reflects another layer's content
```

For Open UI the primary types are **PictureLayer** (all general-purpose
content), **SolidColorLayer** (backgrounds, color fills), and
**TextureLayer** (embedder-provided images / video).  `SurfaceLayer` exists
only for viz embedding and will be removed.

### Property Trees

Chromium replaced per-layer transform/clip/opacity/scroll properties with
**shared property trees** in 2016.  Each layer stores integer *node IDs* into
four trees:

```
  PropertyTrees  (cc/trees/property_tree.h)
  ├── TransformTree   — 4×4 matrix per node, parent pointer
  ├── ClipTree        — clip rect per node, parent pointer
  ├── EffectTree      — opacity, blend mode, filters, render surface info
  └── ScrollTree      — scroll offset, bounds, user-scrollable axes
```

Advantages:

- **O(1) lookup** for any ancestor property via the tree.
- **Shared nodes** — siblings with the same transform share one node.
- **Efficient diff** — only changed nodes need recomputation.

```cpp
// Simplified from cc/trees/property_tree.h
struct TransformNode {
  int id;
  int parent_id;
  gfx::Transform local;       // local matrix relative to parent
  gfx::Transform to_screen;   // cached world-space matrix
  bool needs_local_transform_update;
};

struct EffectNode {
  int id;
  int parent_id;
  float opacity;
  SkBlendMode blend_mode;
  cc::FilterOperations filters;
  bool has_render_surface;     // true → isolated group
};
```

### Tree Synchronization (Commit)

During Commit (`TreeSynchronizer::SynchronizeTrees()`):

1. Main-thread `Layer` tree is walked.  For each layer, a corresponding
   `LayerImpl` is created or reused in the pending tree.
2. **Property trees** are deep-copied into the pending tree's
   `PropertyTrees` instance.
3. **Display item lists** on `PictureLayer`s are transferred (swapped, not
   copied) to `PictureLayerImpl`.
4. Layer properties (bounds, masks, etc.) are pushed via `Layer::PushPropertiesTo()`.

The main thread is **blocked** during commit.  Minimising commit time is
critical for animation smoothness.

---

## 3. Tile Management & Rasterization

### Overview

`PictureLayerImpl` breaks its display item list into a grid of **tiles**. Each
tile is a fixed-size bitmap (default 256×256 on most platforms) that is
independently rasterised and uploaded to the GPU.

```
  PictureLayerImpl content area
  ┌───────┬───────┬───────┬───────┐
  │ Tile  │ Tile  │ Tile  │ Tile  │  ← row 0
  │ (0,0) │ (1,0) │ (2,0) │ (3,0) │
  ├───────┼───────┼───────┼───────┤
  │ Tile  │ Tile  │ Tile  │ Tile  │  ← row 1
  │ (0,1) │ (1,1) │ (2,1) │ (3,1) │
  ├───────┼───────┼───────┼───────┤
  │ (vis) │ (vis) │       │       │  ← viewport covers (0-1, 2)
  │ (0,2) │ (1,2) │ (2,2) │ (3,2) │
  └───────┴───────┴───────┴───────┘
        ▲
        │ tiles inside/near viewport are rasterised first
```

### Key Classes

| Class | File | Purpose |
|-------|------|---------|
| `TileManager` | `cc/tiles/tile_manager.h` | Central controller: prioritizes, schedules raster tasks, tracks memory. |
| `Tile` | `cc/tiles/tile.h` | One rasterisable unit; holds draw info, priority, resource. |
| `TilePriority` | `cc/tiles/tile_priority.h` | Encodes distance-to-viewport, resolution level, required-for-activation/draw flags. |
| `PrioritizedTile` | `cc/tiles/prioritized_tile.h` | A `Tile*` + resolved priority, fed to the raster queue. |
| `RasterBufferProvider` | `cc/raster/raster_buffer_provider.h` | Abstract interface for raster backends. |

### Priority Calculation

```cpp
// cc/tiles/tile_priority.h (simplified)
struct TilePriority {
  enum PriorityBin {
    NOW,             // visible right now
    SOON,            //%.5 screen away, or needed for activation
    EVENTUALLY       // far away / low-res
  };
  PriorityBin priority_bin;
  float distance_to_visible;   // in CSS pixels
  TileResolution resolution;   // HIGH_RESOLUTION, LOW_RESOLUTION
};
```

`TileManager` sorts tiles into a raster queue (NOW → SOON → EVENTUALLY) and
feeds them to worker threads.

### Raster Modes

**Software raster (CPU)**
- `BitmapRasterBufferProvider` → `SkCanvas` backed by shared-memory bitmap.
- Content rasterised on worker threads, result is an `SkBitmap`.

**GPU raster (OOP-R)**
- `GpuRasterBufferProvider` → raster operations are serialised into
  `gpu::raster::RasterInterface` command buffers, sent to the GPU process,
  replayed on a real `SkCanvas` there.
- "Out-of-process rasterization" — the renderer never touches the GPU
  directly.

**For Open UI** we use **direct Skia raster** only:

- Worker threads rasterise into `SkSurface`s (CPU or GPU-backed).
- No command buffer serialisation, no OOP-R, no GPU process.
- A single `SkiaDirectRasterBufferProvider` (new class) hands each worker
  an `SkCanvas` and collects the resulting `SkImage` / texture handle.

```
  Chromium OOP-R path (removed in Open UI)
  ┌────────────┐  cmd buf  ┌───────────┐  Skia  ┌─────────┐
  │ Worker     │──────────►│ GPU proc  │───────►│ Texture │
  │ (renderer) │           │ (decode)  │        │         │
  └────────────┘           └───────────┘        └─────────┘

  Open UI direct path
  ┌────────────┐  SkCanvas  ┌─────────┐
  │ Worker     │───────────►│ Texture │
  │            │   (direct) │ / Bitmap│
  └────────────┘            └─────────┘
```

### Tile Grid Sizing

| Platform | Default tile size | Notes |
|----------|------------------|-------|
| Desktop  | 256 × 256        | Good balance of raster parallelism vs overhead |
| Android  | 256 × 512        | Taller tiles for vertical scroll |
| Open UI  | Configurable     | Exposed via `OuiCompositorConfig` |

---

## 4. Frame Scheduling (Scheduler State Machine)

### Architecture

The scheduler is the **heartbeat** of the compositor.  It reacts to vsync
signals, main-thread readiness, and raster completion to advance through a
state machine that controls frame production.

```
  ┌──────────────────────────────────────────────────────┐
  │                   BeginFrameSource                   │
  │          (vsync from display or synthetic)           │
  └────────────────────────┬─────────────────────────────┘
                           │ OnBeginFrame
                           ▼
  ┌──────────────────────────────────────────────────────┐
  │                    Scheduler                         │
  │  ┌──────────────────────────────────────────────┐    │
  │  │          SchedulerStateMachine               │    │
  │  │                                              │    │
  │  │  current state ──► action to perform         │    │
  │  │  + deadlines   ──► when to perform it        │    │
  │  └──────────────────────────────────────────────┘    │
  │                                                      │
  │  Calls into LayerTreeHostImpl:                       │
  │    - BeginMainFrame()                                │
  │    - Commit()                                        │
  │    - ActivateSyncTree()                              │
  │    - PrepareToDraw() / DrawLayers()                  │
  └──────────────────────────────────────────────────────┘
```

### State Machine States

The state machine (defined in `cc/scheduler/scheduler_state_machine.h`)
tracks several orthogonal pieces of state.  The most important progression
for a full frame:

```
  IDLE
    │
    │  BeginFrame (vsync arrives)
    ▼
  BEGIN_MAIN_FRAME_SENT
    │
    │  Main thread finishes (NotifyReadyToCommit)
    ▼
  READY_TO_COMMIT
    │
    │  Commit (sync trees)
    ▼
  WAITING_FOR_ACTIVATION
    │
    │  All required-for-activation tiles rasterised
    ▼
  READY_TO_ACTIVATE
    │
    │  Activate (pending → active)
    ▼
  READY_TO_DRAW
    │
    │  Draw (produce CompositorFrame / render to surface)
    ▼
  IDLE
```

Key fields inside `SchedulerStateMachine`:

```cpp
// cc/scheduler/scheduler_state_machine.h (simplified)
class SchedulerStateMachine {
  BeginMainFrameState begin_main_frame_state_;
  ForcedRedrawOnTimeoutState forced_redraw_state_;
  bool needs_redraw_;
  bool needs_begin_main_frame_;
  bool commit_has_no_updates_;
  bool has_pending_tree_;
  bool active_tree_needs_first_draw_;
  bool did_draw_in_last_frame_;
  bool did_submit_in_last_frame_;
  // ...
  Action NextAction() const;   // pure function: state → action
};
```

### BeginFrame Flow

```cpp
// 1. Vsync fires
BeginFrameSource::OnBeginFrame(BeginFrameArgs args);

// 2. Scheduler decides whether to involve main thread
Scheduler::OnBeginFrameDeadline();
  → state_machine_.NextAction()
  → ACTION_SEND_BEGIN_MAIN_FRAME
  → LayerTreeHostImpl::BeginMainFrame()

// 3. Main thread does style/layout/paint, then:
ProxyMain::NotifyReadyToCommit();

// 4. Commit
LayerTreeHostImpl::CommitComplete();

// 5. Raster finishes → Activate
LayerTreeHostImpl::NotifyReadyToActivate();
LayerTreeHostImpl::ActivateSyncTree();

// 6. Draw
LayerTreeHostImpl::PrepareToDraw(&frame);
LayerTreeHostImpl::DrawLayers(&frame);
```

### Compositor-Only Frames

When only compositor-driven state has changed (e.g., an ongoing CSS transform
animation, or a scroll offset update), the scheduler skips the main thread
entirely:

```
  BeginFrame → (no BeginMainFrame) → Draw
```

This is what makes scroll and animation **jank-free** — the main thread can
be blocked on JavaScript and the compositor still produces frames at 60 fps.

---

## 5. Drawing & Output

### Render Passes and Quads

The draw phase converts the active `LayerImpl` tree into a list of
**render passes**, each containing **draw quads**.

```
  CompositorFrame
  └── RenderPassList
      ├── RenderPass 0 (root)
      │   ├── TileDrawQuad        — rasterised tile content
      │   ├── TileDrawQuad
      │   ├── SolidColorDrawQuad  — solid-color layer
      │   └── TextureDrawQuad     — external texture
      └── RenderPass 1 (filter group)
          ├── TileDrawQuad
          └── TileDrawQuad
```

Each `DrawQuad` specifies:

- The destination rect on screen.
- The source resource (tile texture, solid color, external texture ID).
- Blend mode, opacity (from the EffectTree).
- Clip rect and transform (from ClipTree / TransformTree).

```cpp
// cc/quads/draw_quad.h (simplified)
struct DrawQuad {
  Material material;               // TILED_CONTENT, SOLID_COLOR, TEXTURE, ...
  gfx::Rect rect;                 // dest rect in target space
  gfx::Rect visible_rect;
  bool needs_blending;
  SharedQuadState* shared_quad_state;  // transform, clip, opacity
};
```

### Renderers

| Renderer | File | Backend |
|----------|------|---------|
| `SkiaRenderer` | `viz/service/display/skia_renderer.h` | Skia (GL, Vulkan, Dawn) — current default |
| `SoftwareRenderer` | `viz/service/display/software_renderer.h` | CPU `SkCanvas` |
| `DirectRenderer` | `viz/service/display/direct_renderer.h` | Abstract base for both |

> Note: These renderers live in `viz/`, not `cc/`.  `cc/` produces the
> `CompositorFrame`; viz consumes it.

### CompositorFrame Structure

```cpp
// services/viz/public/mojom/compositing/compositor_frame.mojom
struct CompositorFrame {
  CompositorFrameMetadata metadata;
  vector<TransferableResource> resource_list;
  vector<RenderPass> render_pass_list;
};
```

### Open UI: Direct Rendering

Open UI **skips the CompositorFrame → viz path**.  Instead,
`LayerTreeHostImpl::DrawLayers()` is modified to iterate quads and issue
Skia draw calls directly:

```
  LayerTreeHostImpl::DrawLayers()
    │
    ▼
  For each RenderPass:
    For each DrawQuad:
      ├── TileDrawQuad    → canvas->drawImageRect(tile_image, src, dst)
      ├── SolidColorDrawQuad → canvas->drawRect(rect, paint)
      └── TextureDrawQuad → canvas->drawImageRect(texture, src, dst)
    Apply EffectNode (saveLayer for opacity/blend/filter)
    │
    ▼
  canvas->flush()  →  presented to window
```

This eliminates:
- `CompositorFrame` serialisation.
- `TransferableResource` IPC.
- viz `Display` and `DisplayScheduler`.
- The entire `SurfaceAggregator` path.

---

## 6. Animations

### Architecture

```
  cc/animation/
  ├── animation.h                # One animation (may have multiple keyframe models)
  ├── animation_host.h           # Registry of all animations; ticks every frame
  ├── animation_timeline.h       # Groups animations for an element
  ├── keyframe_model.h           # Defines property + keyframes + timing function
  ├── keyframe_effect.h          # Binds keyframe models to element properties
  └── worklet_animation.h        # Paint/animation worklet integration
```

### Key Classes

```cpp
// cc/animation/animation_host.h (simplified)
class AnimationHost {
 public:
  void RegisterAnimation(scoped_refptr<Animation>);
  void UnregisterAnimation(scoped_refptr<Animation>);

  // Called each frame by the scheduler (compositor thread).
  void TickAnimations(base::TimeTicks monotonic_time,
                      const ScrollTree& scroll_tree,
                      bool is_active_tree);

  // Push animation state during commit.
  void PushPropertiesTo(AnimationHost* impl_host);

  bool NeedsTickAnimations() const;
};

// cc/animation/keyframe_model.h (simplified)
class KeyframeModel {
  int target_property_id_;     // e.g., TRANSFORM, OPACITY, FILTER
  std::unique_ptr<AnimationCurve> curve_;   // bezier, spring, steps, ...
  base::TimeTicks start_time_;
  base::TimeDelta duration_;
  double iterations_;
  Direction direction_;
  FillMode fill_mode_;
  RunState run_state_;         // RUNNING, PAUSED, FINISHED, ABORTED, ...
};
```

### Compositor-Driven Animations

The following properties can animate **entirely on the compositor thread**,
meaning no main-thread round-trip and no risk of jank:

| Property | Why it's compositor-friendly |
|----------|------------------------------|
| `transform` | Modifying a TransformNode; no relayout. |
| `opacity` | Modifying an EffectNode; no relayout. |
| `filter` | Modifying an EffectNode; no relayout. |
| `backdrop-filter` | Same as filter, applied to backdrop. |

For these properties `AnimationHost::TickAnimations()` updates the property
tree node directly and marks the frame as needing a draw — the main thread is
never notified until the animation finishes.

### Main-Thread Animations

Animations on properties like `width`, `height`, `top`, `left`, `color`, etc.
require **layout** and therefore must run on the main thread.  The compositor
sends `BeginMainFrame` each tick, the main thread advances the animation,
re-layouts, re-paints, and commits the new layer tree.

### Animation Timelines

`AnimationTimeline` provides a grouping mechanism.  Each timeline contains
animations associated with specific elements (identified by `ElementId`).
This maps to the Web Animations API grouping model:

```
  AnimationHost
  └── AnimationTimeline (id=1)
      ├── Animation (element=A, keyframe: transform 0→360° over 2s)
      └── Animation (element=B, keyframe: opacity 1→0 over 0.5s)
  └── AnimationTimeline (id=2)
      └── WorkletAnimation (drives scroll-linked animation)
```

### Worklet Animations

`WorkletAnimation` allows user-defined JavaScript (via AnimationWorklet) to
drive animation on the compositor thread.  The worklet's `animate()` callback
runs in a worklet global scope on the compositor thread's task runner.

For Open UI, worklet animations are **out of scope** for the initial
extraction.  Standard keyframe animations are fully supported.

---

## 7. Scrolling

### Compositor-Side Scroll Handling

Low-latency scrolling is one of the compositor's most important features.
Input events (touch, wheel, trackpad) are routed to the compositor thread
**first**.  If the scroll can be handled without the main thread (no
`scroll` event listeners with `preventDefault`, no non-fast-scrollable
regions), the compositor updates the scroll offset immediately and draws.

```
  Input event (OS)
       │
       ▼
  ┌─────────────────────────┐
  │  InputHandlerProxy      │  (ui/events/blink/)
  │  (compositor thread)    │
  └───────────┬─────────────┘
              │ ScrollBegin / ScrollUpdate
              ▼
  ┌─────────────────────────┐
  │  cc::InputHandler       │  (cc/input/input_handler.h)
  │  (compositor thread)    │
  │                         │
  │  1. Hit-test ScrollTree │
  │  2. Update ScrollNode   │
  │  3. Request redraw      │
  └─────────────────────────┘
              │
              │ (scroll committed back to main thread asynchronously)
              ▼
       main thread sees updated scroll offset on next BeginMainFrame
```

### ScrollTree and ScrollNode

```cpp
// cc/trees/scroll_tree.h / property_tree.h (simplified)
struct ScrollNode {
  int id;
  int parent_id;
  gfx::PointF current_scroll_offset;
  gfx::SizeF scroll_container_bounds;
  gfx::SizeF scroll_content_bounds;
  bool user_scrollable_horizontal;
  bool user_scrollable_vertical;
  bool scrolls_inner_viewport;
  bool scrolls_outer_viewport;
  bool prevent_viewport_scrolling_from_inner;
  ElementId element_id;   // maps back to blink::Element
};
```

The `ScrollTree` is part of the property trees and is synchronised during
commit just like TransformTree et al.

### Input Event Routing

`cc::InputHandler` (implemented by `LayerTreeHostImpl`) processes:

| Method | Trigger |
|--------|---------|
| `ScrollBegin()` | Touch down / wheel start — hit-tests the ScrollTree. |
| `ScrollUpdate()` | Touch move / wheel delta — applies offset to ScrollNode. |
| `ScrollEnd()` | Touch up / wheel end — finalises scroll, starts fling if applicable. |
| `PinchGestureBegin/Update/End()` | Pinch zoom on the compositor thread. |
| `MouseMoveAt()` | Cursor updates for hover scrollbar interactions. |

### Scroll Snapping

CSS Scroll Snap is implemented partially on the compositor.  After a fling or
scroll ends, `cc::SnapFlingController` (in `cc/input/`) calculates the
snap destination from the `SnapContainerData` stored on the ScrollNode and
animates to it.

### Overscroll & Elastic Overscroll

- **Overscroll**: When the user scrolls past content bounds, an
  `OverscrollBehavior` value (`auto`, `contain`, `none`) determines whether
  the overscroll propagates or triggers the OS overscroll effect (Android
  glow, iOS bounce).
- **Elastic overscroll** (macOS): `cc::ElasticOverscrollController` applies a
  rubber-band spring effect on the compositor thread.

### Threaded Scrollbar Painting

Scrollbar layers are composited.  `cc::ScrollbarLayerImpl` (either
`PaintedScrollbarLayerImpl` or `SolidColorScrollbarLayerImpl`) draws
scrollbar tracks and thumbs.  The compositor updates thumb position based on
scroll offset each frame — no main-thread involvement.

---

## 8. Key Dependencies

### `base/` — Chromium Base Library

| Facility | Used For |
|-----------|----------|
| `base::Thread`, `base::SingleThreadTaskRunner` | Compositor thread, raster worker pool |
| `base::RepeatingCallback`, `base::OnceClosure` | All async plumbing |
| `TRACE_EVENT` macros | Performance tracing (feeds into `chrome://tracing`) |
| `base::TimeTicks`, `base::TimeDelta` | Animation timing, frame scheduling |
| `base::WaitableEvent`, `base::Lock` | Commit synchronisation |
| `base::flat_map`, `base::small_map` | Internal data structures |

**Open UI impact**: `base/` is the largest transitive dependency.  We will
provide a **shim layer** (`oui_base/`) that maps `base::` primitives to
platform equivalents (e.g., `std::thread`, `std::chrono`).

### Skia

All rasterisation goes through Skia.  Key Skia types used by `cc/`:

- `SkCanvas` — draw commands during rasterisation.
- `SkSurface` — wraps a raster or GPU-backed canvas.
- `SkImage` — immutable image (tile output).
- `SkPicture` / `SkPictureRecorder` — display list recording.
- `cc::PaintCanvas` / `cc::PaintRecord` — Chromium's Skia wrappers in
  `cc/paint/` that add serialisation support (for OOP-R) and recording.

**Open UI impact**: Skia is **kept as-is**.  It is the rendering backend.
We strip away `cc::PaintCanvas` serialisation support that only exists for
OOP-R and use Skia directly.

### `gpu/` and `viz/`

| Component | Purpose | Open UI Status |
|-----------|---------|----------------|
| `gpu::CommandBuffer` | Serialise GL/Vulkan calls to GPU process | **Remove** |
| `gpu::SharedImageInterface` | Cross-process texture sharing | **Remove** |
| `viz::CompositorFrameSink` | IPC channel for `CompositorFrame` delivery | **Remove** |
| `viz::Display` / `viz::DirectRenderer` | Final compositing in GPU process | **Remove** |
| `viz::SurfaceAggregator` | Merges frames from multiple clients | **Remove** |
| `viz::BeginFrameSource` | Vsync distribution | **Replace** with direct vsync |

### `ui/gfx`

| Type | Header | Purpose |
|------|--------|---------|
| `gfx::Rect`, `gfx::RectF` | `ui/gfx/geometry/rect.h` | Bounding boxes |
| `gfx::Size`, `gfx::SizeF` | `ui/gfx/geometry/size.h` | Layer / tile sizes |
| `gfx::Transform` | `ui/gfx/geometry/transform.h` | 4×4 matrix |
| `gfx::Vector2dF` | `ui/gfx/geometry/vector2d_f.h` | Scroll offsets, translations |
| `gfx::ColorSpace` | `ui/gfx/color_space.h` | sRGB, Display-P3, etc. |
| `gfx::BufferFormat` | `ui/gfx/buffer_types.h` | RGBA_8888, BGRA_8888, etc. |

**Open UI impact**: `ui/gfx` geometry types are lightweight and mostly
header-only.  We will **inline / vendor** them into `oui_base/` rather than
keeping a dependency on the entire `ui/` tree.

---

## 9. Open UI Extraction Strategy

### What to Keep

| Component | Source | Rationale |
|-----------|--------|-----------|
| Layer tree (`Layer`, `LayerImpl`) | `cc/layers/` | Core data model — everything builds on this. |
| Property trees | `cc/trees/property_tree.h` | Efficient transform / clip / effect / scroll representation. |
| Tile manager | `cc/tiles/` | Tiled rasterisation is essential for large content areas. |
| Scheduler + state machine | `cc/scheduler/` | Frame cadence, compositor-only frames, deadline management. |
| Animation system | `cc/animation/` | Compositor-driven animations are a key perf feature. |
| Scroll handling | `cc/input/` | Low-latency scrolling is non-negotiable for a UI framework. |
| Paint recording | `cc/paint/` (subset) | `PaintRecord` / `PaintOp` for display list representation. |

### What to Remove

| Component | Source | Why |
|-----------|--------|-----|
| viz integration | `cc/viz/`, `viz/` | No GPU process, no frame sinks. |
| OOP-R support | `cc/raster/gpu_raster_*` | No command buffer serialisation. |
| GPU process IPC | `gpu/ipc/`, `gpu/command_buffer/` | Single-process model. |
| Mojo bindings | `services/viz/public/mojom/` | No IPC. |
| Browser compositor | `content/browser/compositor/` | Open UI is not a browser. |
| SurfaceLayer / SurfaceId | `cc/layers/surface_layer*` | No viz surface embedding. |
| Delegated ink | `cc/trees/delegated_ink_*` | Browser-specific feature. |

### What to Simplify

**CompositorFrame → Direct SkCanvas draw**

Instead of building a `CompositorFrame` and shipping it to viz, the draw
phase iterates quads and directly emits Skia calls:

```
  Before (Chromium):
    LayerImpl::AppendQuads() → CompositorFrame → Mojo → viz → SkiaRenderer → Skia

  After (Open UI):
    LayerImpl::AppendQuads() → OuiDirectRenderer → SkCanvas (caller-owned surface)
```

**Resource management**

Chromium's `TransferableResource` / `SharedImage` system exists for
cross-process GPU resource sharing.  Open UI replaces this with a simple
in-process `ResourcePool` that maps resource IDs to `SkImage` handles or raw
pixel buffers.

**BeginFrameSource**

Replace viz-based `ExternalBeginFrameSource` with a simple vsync callback
from the platform windowing layer (or a synthetic timer for headless /
test modes).

### Proposed C API Surface

```c
/*--------------------------------------------------------------------
 * oui_compositor.h — public C ABI for the Open UI compositor
 *--------------------------------------------------------------------*/

/* Lifecycle */
OuiCompositor*  oui_compositor_create(const OuiCompositorConfig* config);
void            oui_compositor_destroy(OuiCompositor* comp);

/* Layer tree access */
OuiLayerTree*   oui_compositor_layer_tree(OuiCompositor* comp);

/* Layer manipulation */
OuiLayer*       oui_layer_create(OuiLayerType type);
void            oui_layer_destroy(OuiLayer* layer);
void            oui_layer_set_bounds(OuiLayer* layer, int width, int height);
void            oui_layer_set_position(OuiLayer* layer, float x, float y);
void            oui_layer_set_transform(OuiLayer* layer, const float matrix[16]);
void            oui_layer_set_opacity(OuiLayer* layer, float opacity);
void            oui_layer_set_clip_rect(OuiLayer* layer, int x, int y, int w, int h);
void            oui_layer_add_child(OuiLayer* parent, OuiLayer* child);
void            oui_layer_remove_child(OuiLayer* parent, OuiLayer* child);
void            oui_layer_invalidate_rect(OuiLayer* layer, int x, int y, int w, int h);

/* Paint callback — embedder provides content */
typedef void (*OuiPaintCallback)(OuiLayer* layer, OuiCanvas* canvas,
                                  int width, int height, void* userdata);
void            oui_layer_set_paint_callback(OuiLayer* layer,
                                              OuiPaintCallback cb,
                                              void* userdata);

/* Frame production */
void            oui_compositor_begin_frame(OuiCompositor* comp);
void            oui_compositor_commit(OuiCompositor* comp);
OuiStatus       oui_compositor_draw(OuiCompositor* comp, OuiSurface* target);

/* Animation */
OuiAnimation*   oui_animation_create(OuiLayer* layer, OuiAnimProperty prop);
void            oui_animation_set_duration(OuiAnimation* anim, double seconds);
void            oui_animation_set_keyframes(OuiAnimation* anim,
                                             const OuiKeyframe* kfs,
                                             int count);
void            oui_animation_start(OuiAnimation* anim);
void            oui_animation_cancel(OuiAnimation* anim);
void            oui_animation_destroy(OuiAnimation* anim);

/* Scrolling */
void            oui_layer_set_scrollable(OuiLayer* layer,
                                          int content_w, int content_h);
void            oui_layer_set_scroll_offset(OuiLayer* layer, float x, float y);
void            oui_layer_get_scroll_offset(OuiLayer* layer,
                                             float* x, float* y);

/* Surface (output target) */
OuiSurface*     oui_surface_create_gpu(void* native_window);
OuiSurface*     oui_surface_create_software(int width, int height);
void            oui_surface_destroy(OuiSurface* surface);
const void*     oui_surface_pixels(OuiSurface* surface);  /* software only */
```

### Extraction Difficulty Assessment

| Component | Difficulty | Notes |
|-----------|-----------|-------|
| Layer tree + property trees | 🟡 Medium | Well-isolated but deeply uses `base/` types. Need to shim ~30 `base/` headers. |
| Tile manager | 🟡 Medium | Core logic is self-contained; main effort is replacing `RasterBufferProvider` with direct-Skia variant. |
| Scheduler / state machine | 🟢 Low | Relatively pure logic.  Inputs: vsync, ready-to-commit, raster-done.  Few external deps. |
| Animation system | 🟢 Low | Clean separation.  `AnimationHost` + `KeyframeModel` have minimal deps beyond `base/` and `gfx::Transform`. |
| Scroll handling | 🟡 Medium | `InputHandler` interacts with gesture detection (touch/wheel), which pulls in `ui/events/`. |
| Paint recording (`cc/paint/`) | 🟡 Medium | `PaintOp` serialisation for OOP-R must be stripped.  The recording side is clean. |
| Raster worker pool | 🟠 Hard | Currently uses `base::TaskScheduler` with `TaskGraphRunner`.  Must reimplement with std threading or a lightweight pool. |
| Remove viz integration | 🟠 Hard | `cc/` has many call sites that reference viz types.  Need systematic ifdef/abstraction pass. |
| `base/` shim layer | 🔴 Very Hard | `base/` is pervasive.  The shim must cover threading, time, tracing, containers, string utils, and more.  This is the single largest effort. |

### Extraction Order (Recommended)

```
  Phase 1: Foundation
  ├── Vendor ui/gfx geometry types → oui_base/
  ├── Create base/ shim (threading, time, callbacks, logging)
  └── Build cc/base/ (math helpers)

  Phase 2: Core Data Structures
  ├── Extract property trees (cc/trees/property_tree.h)
  ├── Extract Layer / LayerImpl
  └── Extract TreeSynchronizer

  Phase 3: Frame Production
  ├── Extract Scheduler + SchedulerStateMachine
  ├── Implement OuiBeginFrameSource (vsync / synthetic)
  └── Extract TileManager + direct-Skia raster provider

  Phase 4: Drawing
  ├── Implement OuiDirectRenderer (quad → SkCanvas)
  ├── Wire LayerTreeHostImpl::DrawLayers() to OuiDirectRenderer
  └── Remove all CompositorFrame / viz codepaths

  Phase 5: Interactivity
  ├── Extract AnimationHost + KeyframeModel
  ├── Extract InputHandler + scroll logic
  └── Implement C API wrappers (oui_compositor.h)
```

---

## Source File Reference

| Concept | Primary Source Files |
|---------|---------------------|
| Layer tree host (main thread) | `cc/trees/layer_tree_host.h`, `cc/trees/layer_tree_host.cc` |
| Layer tree host impl (compositor) | `cc/trees/layer_tree_host_impl.h`, `cc/trees/layer_tree_host_impl.cc` |
| Proxy (thread bridge) | `cc/trees/proxy_main.h`, `cc/trees/proxy_impl.h` |
| Layer base class | `cc/layers/layer.h`, `cc/layers/layer_impl.h` |
| PictureLayer | `cc/layers/picture_layer.h`, `cc/layers/picture_layer_impl.h` |
| SolidColorLayer | `cc/layers/solid_color_layer.h`, `cc/layers/solid_color_layer_impl.h` |
| TextureLayer | `cc/layers/texture_layer.h`, `cc/layers/texture_layer_impl.h` |
| Property trees | `cc/trees/property_tree.h`, `cc/trees/property_tree.cc` |
| Tree synchroniser | `cc/trees/tree_synchronizer.h`, `cc/trees/tree_synchronizer.cc` |
| Scheduler | `cc/scheduler/scheduler.h`, `cc/scheduler/scheduler.cc` |
| Scheduler state machine | `cc/scheduler/scheduler_state_machine.h` |
| BeginFrame | `cc/scheduler/begin_frame_source.h` |
| TileManager | `cc/tiles/tile_manager.h`, `cc/tiles/tile_manager.cc` |
| Tile | `cc/tiles/tile.h` |
| TilePriority | `cc/tiles/tile_priority.h` |
| RasterBufferProvider | `cc/raster/raster_buffer_provider.h` |
| GPU raster provider | `cc/raster/gpu_raster_buffer_provider.h` |
| Bitmap raster provider | `cc/raster/bitmap_raster_buffer_provider.h` |
| Animation host | `cc/animation/animation_host.h` |
| Animation | `cc/animation/animation.h` |
| KeyframeModel | `cc/animation/keyframe_model.h` |
| AnimationTimeline | `cc/animation/animation_timeline.h` |
| InputHandler | `cc/input/input_handler.h`, `cc/input/input_handler.cc` |
| ScrollTree / ScrollNode | `cc/trees/property_tree.h` (scroll section) |
| Snap fling controller | `cc/input/snap_fling_controller.h` |
| Elastic overscroll | `cc/input/elastic_overscroll_controller.h` |
| PaintOp / PaintRecord | `cc/paint/paint_op.h`, `cc/paint/paint_record.h` |
| DrawQuad | `cc/quads/draw_quad.h` |
| TileDrawQuad | `cc/quads/tile_draw_quad.h` |
| SolidColorDrawQuad | `cc/quads/solid_color_draw_quad.h` |
| TextureDrawQuad | `cc/quads/texture_draw_quad.h` |
| RenderPass | `cc/quads/render_pass.h` |
| CompositorFrame | `services/viz/public/mojom/compositing/compositor_frame.mojom` |
| SkiaRenderer (viz) | `viz/service/display/skia_renderer.h` |
| DirectRenderer (viz) | `viz/service/display/direct_renderer.h` |
| gfx::Transform | `ui/gfx/geometry/transform.h` |
| gfx::Rect | `ui/gfx/geometry/rect.h` |
| gfx::ColorSpace | `ui/gfx/color_space.h` |

---

*Document revision: initial draft — tracks Chromium ~M130 (`cc/` layer).
Update as extraction progresses and interfaces stabilise.*
