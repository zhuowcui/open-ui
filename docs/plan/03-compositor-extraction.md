# Sub-Project 3: Compositor Extraction

> Extract Chromium's compositor (`cc/`) for GPU-accelerated layer compositing, tiling, damage tracking, and compositor-thread animations.

## Objective

Produce `libopenui_compositor.so` — a standalone compositor library that manages a tree of layers, handles tiling and rasterization scheduling, performs damage tracking for partial updates, and runs animations/scrolling on a dedicated compositor thread. This is the component that makes Chromium scroll butter-smooth and animate at 60fps even when the main thread is busy.

## Background: What `cc/` Does in Chromium

Chromium's `cc/` (often called "Chrome Compositor" or just "cc") is the layer between Blink's paint output and the actual pixels on screen. Its key responsibilities:

1. **Layer tree management** — Maintains a tree of `cc::Layer` objects, each representing a composited surface (e.g., a scrolling container, a transformed element, a video)
2. **Property trees** — Efficient representation of transform, clip, effect (opacity/filter), and scroll hierarchies
3. **Tiling** — Breaks large layers into tiles for efficient rasterization and GPU memory management
4. **Rasterization scheduling** — Decides which tiles to rasterize, at what priority, using a thread pool
5. **Damage tracking** — Computes the minimal region that needs redrawing when something changes
6. **Compositor-thread animations** — Runs animations and scroll physics on the compositor thread, independent of main thread jank
7. **Frame scheduling** — Coordinates with VSync to produce frames at the right time

## Tasks

### 3.1 Extract `cc/` Core Components

**Layer system (`cc/layers/`):**
- `cc::Layer` — Base layer type
- `cc::PictureLayer` — Rasterized content layer (most common)
- `cc::SolidColorLayer` — Optimization for solid color regions
- `cc::SurfaceLayer` — Embedded surface (for offscreen/video content)
- `cc::TextureLayer` — Externally managed GPU texture
- Property trees (`cc/trees/property_tree.h`) — Transform, clip, effect, scroll trees

**Tile management (`cc/tiles/`):**
- `TileManager` — Orchestrates tile lifecycle
- `Tile` — Individual tile with raster state
- `PictureLayerTiling` — Tiling grid for a layer at a given scale
- `RasterTaskController` — Schedules raster work on thread pool

**Scheduling (`cc/scheduler/`):**
- `Scheduler` — Frame production timing
- `BeginFrameSource` — VSync signal provider

**Trees (`cc/trees/`):**
- `LayerTreeHost` — Main-thread tree owner
- `LayerTreeImpl` — Compositor-thread tree (pending/active)
- `LayerTreeHostImpl` — Compositor-thread tree management + drawing

**Animation:**
- `cc/animation/` — Keyframe animations, scroll animations, animation timelines
- Runs entirely on compositor thread

### 3.2 Dependency Extraction

The compositor depends on several Chromium subsystems that we need to extract or replace:

| Dependency | What cc/ uses | Extraction strategy |
|---|---|---|
| `base/` | Threading, task runners, callbacks, time, containers | Extract minimal subset (from SP1 analysis) |
| `gpu/` | GPU command buffer, context management, texture allocation | Simplify — we run in-process, no GPU process needed. Direct Vulkan/GL instead of command buffer. |
| `viz/` | Display compositor, surface aggregation | **Replace entirely** — cc/ normally feeds into viz for multi-process compositing. We composite directly. |
| `ui/gfx/` | Geometry types, transforms, color, GPU fences | Extract needed types |
| `skia/` | Rasterization backend | Already extracted in SP2 |

**Key simplification: No GPU process.** Chromium runs the compositor in the renderer process and the display compositor in the GPU process, communicating via IPC. We run everything in-process, eliminating the entire `viz/` service layer and GPU command buffer. This is a major simplification.

### 3.3 Decouple from Blink

In Chromium, Blink feeds the compositor through specific interfaces:
- `cc::Layer` creation and tree building
- Paint output as `cc::PaintRecord` (display lists)
- Property tree updates from style/layout changes

We need to replace Blink as the compositor's client:
- Our C API becomes the new "Blink" — it creates layers, provides paint content, and updates property trees
- Replace `cc::PaintRecord` input with our Skia C API output (or a display list we define)
- Remove all `#include` paths that reach into Blink

### 3.4 C API Design (`include/openui/openui_compositor.h`)

```c
// === Compositor lifecycle ===
OuiStatus oui_compositor_create(OuiCompositor** comp, OuiSkGpuContext* gpu_ctx);
void      oui_compositor_destroy(OuiCompositor* comp);
OuiStatus oui_compositor_set_viewport(OuiCompositor* comp, int width, int height, float scale_factor);
OuiStatus oui_compositor_set_vsync_source(OuiCompositor* comp, OuiVSyncSource* source);

// === Layer management ===
OuiLayer* oui_layer_create_picture(OuiCompositor* comp);   // Rasterized content
OuiLayer* oui_layer_create_solid_color(OuiCompositor* comp, OuiSkColor color);
OuiLayer* oui_layer_create_surface(OuiCompositor* comp);   // Embedded surface
void      oui_layer_destroy(OuiLayer* layer);

// === Layer tree ===
void oui_layer_add_child(OuiLayer* parent, OuiLayer* child);
void oui_layer_remove_child(OuiLayer* parent, OuiLayer* child);
void oui_layer_set_root(OuiCompositor* comp, OuiLayer* root);

// === Layer properties ===
void oui_layer_set_bounds(OuiLayer* layer, int width, int height);
void oui_layer_set_position(OuiLayer* layer, float x, float y);
void oui_layer_set_transform(OuiLayer* layer, const OuiTransform* transform);
void oui_layer_set_opacity(OuiLayer* layer, float opacity);
void oui_layer_set_clip(OuiLayer* layer, const OuiRect* clip);
void oui_layer_set_mask(OuiLayer* layer, OuiLayer* mask_layer);
void oui_layer_set_scrollable(OuiLayer* layer, OuiSize content_size);
void oui_layer_set_scroll_offset(OuiLayer* layer, float x, float y);

// === Content invalidation ===
// For picture layers: tell the compositor what to rasterize
typedef void (*OuiPaintCallback)(OuiSkCanvas* canvas, const OuiRect* dirty_rect, void* userdata);
void oui_layer_set_paint_callback(OuiLayer* layer, OuiPaintCallback cb, void* userdata);
void oui_layer_invalidate_rect(OuiLayer* layer, const OuiRect* rect);  // Mark region dirty

// === Animations ===
OuiAnimation* oui_animation_create(OuiLayer* target, OuiAnimProperty property);
void oui_animation_add_keyframe(OuiAnimation* anim, float time, float value, OuiEasing easing);
void oui_animation_start(OuiAnimation* anim);
void oui_animation_cancel(OuiAnimation* anim);

// === Frame production ===
// The compositor runs its own thread. These control the commit cycle.
OuiStatus oui_compositor_commit(OuiCompositor* comp);  // Push main-thread changes to compositor thread
OuiStatus oui_compositor_present(OuiCompositor* comp, OuiWindow* window);  // Display to window
```

### 3.5 Threading Architecture

Match Chromium's proven model:

```
Main Thread              Compositor Thread           Raster Workers
┌──────────┐            ┌──────────────────┐        ┌──────────┐
│ App code │            │ Animation tick   │        │ Tile     │
│ Scene    │──commit──▶ │ Scroll physics   │        │ raster   │
│ changes  │            │ Damage compute   │──work──▶│ (Skia)   │
│          │            │ Tile scheduling  │◀─done──│          │
│          │            │ Frame assembly   │        │          │
│          │            │ GPU submit       │        │          │
└──────────┘            └──────────────────┘        └──────────┘
```

- **Main thread**: Application code mutates the layer tree and commits
- **Compositor thread**: Processes commits, runs animations, schedules rasterization, submits GPU work
- **Raster workers**: Thread pool that rasterizes tiles using Skia

### 3.6 Validation & Benchmarks

**Correctness tests:**
- Single layer rendering
- Multi-layer compositing (overlapping, z-order)
- Transform compositing (rotate, scale, translate, perspective)
- Opacity and blend modes
- Clip regions (rect, rounded rect)
- Scroll offset and scroll containers
- Damage tracking (invalidate a rect → only that rect re-rasterized)
- Tiling (large layers correctly tiled and assembled)

**Performance tests:**
- 60fps with 100 animated layers
- Scroll smoothness (frame time variance < 2ms)
- Damage tracking efficiency (1 dirty tile → 1 tile re-rasterized, not full frame)
- Memory usage: GPU texture memory for tiled layers
- Rasterization throughput on worker threads

## Deliverables

| Deliverable | Description |
|---|---|
| `libopenui_compositor.so` | Compositor shared library |
| `include/openui/openui_compositor.h` | Public C header |
| `examples/compositor_layers.c` | Multi-layer compositing demo |
| `examples/compositor_scroll.c` | Smooth scrolling demo |
| `examples/compositor_animation.c` | Animated layers demo |
| `tests/compositor/` | Correctness test suite |
| `benchmarks/compositor/` | Performance benchmarks |

## Success Criteria

- [ ] Multi-layer compositing renders correctly via C API
- [ ] Compositor-thread animations run at 60fps independent of main thread
- [ ] Scrolling is smooth (< 2ms frame time variance)
- [ ] Damage tracking works (partial invalidation → partial re-raster)
- [ ] No GPU memory leaks
- [ ] Main thread can be blocked for 100ms without dropping compositor frames

## Key Risks

- **`viz/` replacement is non-trivial**: The display compositor in viz handles surface aggregation and multi-client compositing. We need to replace this with direct-to-screen compositing, which requires understanding exactly what viz provides.
- **GPU resource management**: Chromium's GPU process manages GPU memory across multiple renderer processes. Without it, we need our own resource management that avoids GPU memory exhaustion.
- **Threading correctness**: The compositor's threading model is complex with many synchronization points. Extracting it without introducing races requires careful attention.
