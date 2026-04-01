# Rendering Pipeline Overview

> Architecture document for the Open UI project.
> Covers Chromium's rendering pipeline end-to-end, annotated with extraction boundaries.

## Pipeline at a Glance

Every frame Chromium produces follows this data flow:

```
                          MAIN THREAD
 ┌──────────────────────────────────────────────────────────┐
 │                                                          │
 │  DOM Tree + Stylesheets                                  │
 │       │                                                  │
 │       ▼                                                  │
 │  ┌────────────────┐   ComputedStyle                      │
 │  │ Style          │──────────────┐                       │
 │  │ Resolution     │              │                       │
 │  └────────────────┘              ▼                       │
 │                          ┌───────────────┐               │
 │                          │ Layout (NG)   │               │
 │                          │               │               │
 │                          └───────┬───────┘               │
 │                                  │ NGPhysicalFragment    │
 │                                  ▼                       │
 │                          ┌───────────────┐               │
 │                          │ Paint         │               │
 │                          │               │               │
 │                          └───────┬───────┘               │
 │                                  │ DisplayItemList +     │
 │                                  │ PropertyTrees         │
 └──────────────────────────────────┼───────────────────────┘
                                    │
                              Commit (IPC)
                                    │
                          COMPOSITOR THREAD
 ┌──────────────────────────────────┼───────────────────────┐
 │                                  ▼                       │
 │                          ┌───────────────┐               │
 │                          │ Compositing   │               │
 │                          │ (cc/)         │               │
 │                          └───────┬───────┘               │
 │                                  │ Tiles                 │
 └──────────────────────────────────┼───────────────────────┘
                                    │
                             RASTER WORKERS
 ┌──────────────────────────────────┼───────────────────────┐
 │                                  ▼                       │
 │                          ┌───────────────┐               │
 │                          │ Rasterize     │               │
 │                          │ (Skia)        │               │
 │                          └───────┬───────┘               │
 │                                  │ GPU Textures          │
 └──────────────────────────────────┼───────────────────────┘
                                    │
                                    ▼
                          ┌───────────────────┐
                          │ Display / Present │
                          │ (viz or direct)   │
                          └───────────────────┘
```

**Thread model.** Style, Layout, and Paint run on the main thread. The compositor
runs on its own thread. Rasterization fans out across a worker pool. In
Chromium, presentation goes through the `viz` service in a separate GPU
process; in Open UI we run this in-process.

---

## Stage 1: Style Resolution

### Purpose

Take a DOM element and the set of applicable CSS rules, and produce a fully
resolved `ComputedStyle` — an immutable snapshot of every CSS property's final
computed value for that element.

### Entry Point

```
third_party/blink/renderer/core/css/resolver/style_resolver.h
  class StyleResolver
    → ResolveStyle(Element&, const StyleRecalcContext&)
    → returns scoped_refptr<ComputedStyle>
```

### Inputs

| Input | Description |
|-------|-------------|
| `Element&` | The DOM node being styled. Provides tag name, attributes, class list, inline `style` attribute, pseudo-element type. |
| `StyleSheetContents` | Parsed stylesheet rules, held by `TreeScope`. |
| `StyleRecalcContext` | Parent style, container query state, animation context. |

### Key Operations

1. **Rule collection** — `ElementRuleCollector` walks all stylesheets and
   gathers matching rules. A Bloom filter (`SelectorFilter`) prunes ancestors
   early. File: `third_party/blink/renderer/core/css/resolver/element_rule_collector.h`.

2. **Cascade** — Collected declarations are sorted by origin (user-agent →
   user → author) and importance (!important reverses origin order). Within
   an origin, specificity breaks ties; within equal specificity, source order
   wins. Implemented in `CascadeResolver`.
   File: `third_party/blink/renderer/core/css/resolver/cascade_resolver.h`.

3. **Inheritance** — Properties marked as `inherited` in
   `css_properties.json5` copy from the parent's `ComputedStyle` when no
   explicit value is set. `StyleBuilder` applies the final cascade result onto
   a `ComputedStyleBuilder`.
   File: `third_party/blink/renderer/core/css/resolver/style_builder.h`.

4. **Value computation** — Lengths resolve `em`/`rem` against font size,
   percentages resolve against the containing block (deferred until layout for
   some properties), `calc()` collapses. `currentcolor` resolves against
   the computed `color` property.
   File: `third_party/blink/renderer/core/css/resolver/style_resolver.cc`.

5. **Custom properties** — `var()` references are substituted, with cycle
   detection. Registered custom properties (`@property`) go through full
   type computation.
   File: `third_party/blink/renderer/core/css/properties/css_property_ref.h`.

### Output

`ComputedStyle` — a ref-counted, immutable, interned object. Two elements
with identical computed values share the same `ComputedStyle` pointer.

Key fields consumed by later stages:

| Field | Used By |
|-------|---------|
| `Display()`, `Position()` | Layout — determines formatting context |
| `Width()`, `Height()`, `Margin()`, `Padding()` | Layout — box model |
| `GetFont()` | Layout/Paint — text shaping and rendering |
| `Transform()`, `Opacity()`, `ClipPath()` | Paint/Compositor — property trees |
| `Color()`, `BackgroundColor()` | Paint — drawing |

File: `third_party/blink/renderer/core/style/computed_style.h`.

---

## Stage 2: Layout (LayoutNG)

### Purpose

Given the `ComputedStyle`-annotated tree, compute the exact pixel position and
size of every box and text fragment.

### Entry Point

```
third_party/blink/renderer/core/layout/ng/ng_block_node.h
  class NGBlockNode : public NGLayoutInputNode
    → Layout(const NGConstraintSpace&, const NGBlockBreakToken*)
    → returns const NGLayoutResult&  (wraps NGPhysicalFragment)
```

Layout is recursive: each node receives constraints from its parent and
returns a fragment describing its geometry.

### Inputs

| Input | Description |
|-------|-------------|
| `LayoutObject` tree | Mirror of the DOM, one `LayoutObject` per styled element. Built by `LayoutTreeBuilder`. File: `third_party/blink/renderer/core/layout/layout_object.h`. |
| `ComputedStyle` | Attached to each `LayoutObject`. Drives every layout decision. |
| `NGConstraintSpace` | The available size, percentage resolution basis, writing mode, and BFC offset. File: `third_party/blink/renderer/core/layout/ng/ng_constraint_space.h`. |

### Constraint Space Model

The constraint space is LayoutNG's mechanism for passing top-down information:

```cpp
// Simplified from ng_constraint_space.h
class NGConstraintSpace {
  LogicalSize available_size;          // inline-size × block-size available
  LogicalSize percentage_resolution;   // basis for percentage resolution
  LayoutUnit fragmentainer_block_size; // for pagination / fragmentation
  WritingMode writing_mode;            // horizontal-tb, vertical-rl, etc.
  bool is_fixed_inline_size;           // true when width is already known
  bool is_shrink_to_fit;              // intrinsic sizing
};
```

A parent constructs a `NGConstraintSpace` for each child, then calls
`child.Layout(constraint_space)`. The child returns its fragment. The parent
places it.

### Layout Algorithms

Each formatting context has a dedicated algorithm class:

| Algorithm | File | Handles |
|-----------|------|---------|
| `NGBlockLayoutAlgorithm` | `ng_block_layout_algorithm.cc` | Block flow, floats, margins, BFC |
| `NGInlineLayoutAlgorithm` | `ng_inline_layout_algorithm.cc` | Inline boxes, bidi, line stacking |
| `NGFlexLayoutAlgorithm` | `ng_flex_layout_algorithm.cc` | Flexbox — multi-pass for flex-grow/shrink |
| `NGGridLayoutAlgorithm` | `ng_grid_layout_algorithm.cc` | Grid — track sizing, auto-placement |
| `NGTableLayoutAlgorithm` | `ng_table_layout_algorithm.cc` | Table — column width distribution |

All live under `third_party/blink/renderer/core/layout/ng/`.

An algorithm implements:

```cpp
class NGBlockLayoutAlgorithm : public NGLayoutAlgorithm<NGBlockNode> {
  const NGLayoutResult* Layout() override;
  MinMaxSizesResult ComputeMinMaxSizes(const MinMaxSizesFloatInput&) override;
};
```

`Layout()` produces the fragment tree. `ComputeMinMaxSizes()` is called
during intrinsic sizing (shrink-to-fit, flex basis, etc.).

### Text Layout

Text layout is a sub-pipeline within the inline algorithm:

1. **Shaping** — `HarfBuzzShaper` calls HarfBuzz to convert a run of Unicode
   code points + font into positioned glyphs. Complex scripts (Arabic, Devanagari)
   get ligatures and reordering here.
   File: `third_party/blink/renderer/platform/fonts/shaping/harfbuzz_shaper.h`.

2. **Line breaking** — `NGLineBreaker` walks shaped results and finds legal
   break opportunities (UAX #14, word boundaries, `<br>`, `overflow-wrap`).
   It fills `NGInlineItemResult` entries that the inline algorithm places.
   File: `third_party/blink/renderer/core/layout/ng/inline/ng_line_breaker.h`.

3. **Bidi** — `NGBidiParagraph` applies the Unicode Bidirectional Algorithm
   (ICU UBiDi) before shaping to determine visual run order.
   File: `third_party/blink/renderer/core/layout/ng/inline/ng_bidi_paragraph.h`.

### Output

`NGPhysicalFragment` — the immutable result of layout:

```cpp
// Simplified from ng_physical_fragment.h
class NGPhysicalFragment {
  PhysicalSize size;           // width × height in physical pixels
  PhysicalOffset offset;       // position relative to parent fragment
  const ComputedStyle& Style();
  // Children: either sub-fragments (box) or text (NGPhysicalTextFragment)
  base::span<const NGLink> Children();
};
```

The fragment tree is the single source of truth for geometry. It feeds paint.

File: `third_party/blink/renderer/core/layout/ng/ng_physical_fragment.h`.

---

## Stage 3: Paint

### Purpose

Walk the fragment tree and produce a flat list of serialized draw commands
(display items), grouped by paint phase and annotated with property tree
nodes.

### Entry Point

```
third_party/blink/renderer/platform/graphics/paint/paint_controller.h
  class PaintController
    → Manages display item list construction and caching
```

Painting is driven by `LocalFrameView::PaintTree()` which calls painters like
`NGBoxFragmentPainter`, `NGTextFragmentPainter`, etc.

File: `third_party/blink/renderer/core/paint/ng/ng_box_fragment_painter.h`.

### Display Items

A `DisplayItem` is one atomic draw operation:

```
DrawingDisplayItem      — wraps a PaintRecord (Skia draw commands)
ForeignLayerDisplayItem — embeds an external compositor layer
ScrollHitTestDisplayItem — hit-test region for scrollable areas
```

Display items are gathered into a `DisplayItemList`, an append-only
sequential list.

File: `third_party/blink/renderer/platform/graphics/paint/display_item.h`.

### Paint Chunks

Consecutive display items that share the same set of property tree nodes are
grouped into a `PaintChunk`. Each chunk records:

```cpp
struct PaintChunk {
  wtf_size_t begin_index;   // first display item index
  wtf_size_t end_index;     // one-past-end display item index
  PaintChunkProperties properties; // → transform, clip, effect, scroll nodes
  IntRect bounds;
};
```

File: `third_party/blink/renderer/platform/graphics/paint/paint_chunk.h`.

### Property Trees

Instead of nesting transform/clip/opacity in a tree of paint contexts,
Chromium uses four flat trees with shared nodes:

| Tree | Represents | Key Node Type |
|------|-----------|---------------|
| **Transform** | CSS transforms, scrolling offsets, perspective | `TransformPaintPropertyNode` |
| **Clip** | `overflow: hidden`, `clip-path`, CSS `clip` | `ClipPaintPropertyNode` |
| **Effect** | `opacity`, `filter`, `mix-blend-mode`, isolation | `EffectPaintPropertyNode` |
| **Scroll** | Scrollable overflow regions | `ScrollPaintPropertyNode` |

Each display item / paint chunk references one node from each tree. This
decouples geometry from drawing and enables the compositor to manipulate
transforms and clips independently.

File: `third_party/blink/renderer/platform/graphics/paint/property_tree_state.h`.

### Paint Invalidation

When a style or layout change occurs, `PaintInvalidator` marks affected
`LayoutObject` nodes. On the next paint, only dirty subtrees are re-painted.
`PaintController` uses a caching scheme: unchanged display items are
replayed from the previous frame's list.

File: `third_party/blink/renderer/core/paint/paint_invalidator.h`.

### Output

The paint stage produces:

1. **`DisplayItemList`** — ordered draw commands.
2. **`PaintChunkList`** — groupings with property tree references.
3. **Property trees** — transform, clip, effect, scroll.

These are the inputs to the commit phase.

---

## Stage 4: Commit to Compositor

### Purpose

Transfer the main-thread paint output to the compositor thread so that the
compositor can produce frames independently (scrolling, animations) without
blocking on main-thread work.

### Trigger

```
cc/trees/layer_tree_host.h
  class LayerTreeHost
    → SetNeedsCommit()      // schedule a commit
    → SetNeedsAnimate()     // need compositor-driven animation tick
```

`LayerTreeHost` lives on the main thread and is the entry point for pushing
data to the compositor.

### What Gets Committed

| Data | Description |
|------|-------------|
| **Paint chunks → Layers** | `PaintChunksToCcLayer` converts paint chunks into `cc::Layer` objects. Adjacent chunks with compatible property trees merge into one layer. File: `third_party/blink/renderer/platform/graphics/compositing/paint_chunks_to_cc_layer.cc`. |
| **Property trees** | Transform, clip, effect, and scroll trees are serialized into `cc::PropertyTrees`. File: `cc/trees/property_tree.h`. |
| **Display lists** | Each layer carries a `cc::DisplayItemList` (distinct from Blink's — this is cc's serialized format). File: `cc/paint/display_item_list.h`. |

### Commit Lifecycle

```
Main Thread                          Compositor Thread
     │                                      │
     │  SetNeedsCommit()                    │
     │─────────────────────────────────────▶│
     │                                      │
     │  BeginMainFrame (from scheduler)     │
     │◀─────────────────────────────────────│
     │                                      │
     │  [run style, layout, paint]          │
     │                                      │
     │  FinishCommitOnImplThread()          │
     │─────────────────────────────────────▶│
     │                                      │
     │           [compositor owns data]     │
     │                                      │
```

During commit, the main thread is blocked while `LayerTreeHost` copies its
state into `LayerTreeHostImpl` on the compositor thread. After commit
completes, the main thread is free to start the next frame's work.

File: `cc/trees/layer_tree_host.cc`, `cc/trees/proxy_main.cc`.

---

## Stage 5: Compositing (cc/)

### Purpose

Manage layers, tiles, animations, and scrolling on the compositor thread,
producing frames at VSync rate even when the main thread is busy.

### Core Class

```
cc/trees/layer_tree_host_impl.h
  class LayerTreeHostImpl
    → the compositor-thread counterpart to LayerTreeHost
    → owns the active layer tree, tile manager, and raster pool
```

### Layer Tree Activation

The compositor maintains two layer trees:

- **Pending tree** — receives fresh data from each commit. Tiles are
  rasterized against this tree.
- **Active tree** — the tree currently being drawn. Once the pending tree
  is fully rastered (or enough of it), it is *activated* — swapped into the
  active slot.

This double-buffering ensures the screen always has a fully-rastered tree
to draw from.

File: `cc/trees/layer_tree_host_impl.cc` — `ActivateSyncTree()`.

### Tile Management

Large layers are broken into tiles (typically 256×256 or 512×512 device
pixels). This enables:

- **Partial rasterization** — only visible and near-visible tiles are rastered.
- **Priority scheduling** — tiles are prioritized by distance from viewport.
- **Memory management** — off-screen tiles can be evicted.

Key classes:

```
cc/tiles/tile_manager.h      — orchestrates tile lifecycle
cc/tiles/tile.h              — one tile: state machine (idle → raster → ready)
cc/tiles/picture_layer_tiling.h — maps a layer to a grid of tiles
cc/tiles/tile_priority.h     — distance-to-viewport priority
```

### Compositor-Thread Animations

CSS animations and transitions that affect only compositor properties
(transform, opacity, filter) run entirely on the compositor thread:

```
cc/animation/animation_host.h
  class AnimationHost
    → ticks active animations each frame
    → updates property tree nodes directly
```

This is how scroll and fling animations stay at 60fps even when the main
thread is doing expensive JavaScript work.

### Compositor-Thread Scrolling

Scroll input events (touch, wheel) are handled on the compositor thread
by `InputHandler`:

```
cc/input/input_handler.h
  class InputHandler
    → ScrollBegin(), ScrollUpdate(), ScrollEnd()
    → modifies scroll offset in the scroll property tree
```

Only when a scroll event listener is registered does the compositor need to
check with the main thread (hit-test, `preventDefault()`).

### Frame Scheduling

The `Scheduler` coordinates work with VSync:

```
cc/scheduler/scheduler.h
  class Scheduler
    → receives VSync (BeginFrameSource)
    → decides when to: begin main frame, commit, raster, draw
    → state machine: IDLE → BEGIN_MAIN_FRAME → COMMIT → RASTER → DRAW
```

File: `cc/scheduler/scheduler_state_machine.h`.

---

## Stage 6: Rasterization

### Purpose

Convert display item lists into pixel data (GPU textures or bitmaps) by
drawing through Skia.

### Thread Model

Rasterization runs on a pool of worker threads managed by
`cc::RasterTaskWorkerPool` (via `base::TaskGraphRunner`). The compositor
thread enqueues raster tasks; workers execute them in priority order.

### Raster Path: GPU (OOP-R)

Chromium defaults to **Out-of-Process Rasterization (OOP-R)**: raster
commands are serialized into a command buffer and sent to the GPU process
for execution. This keeps the renderer sandboxed away from the GPU driver.

```
cc/raster/gpu_raster_buffer_provider.h
  class GpuRasterBufferProvider
    → implements RasterBufferProvider
    → serializes paint ops into a GPU command buffer
```

In Open UI, since we run in-process, we can use direct GPU rasterization
without the IPC overhead.

### Raster Path: Software

For testing or when no GPU is available:

```
cc/raster/software_raster_buffer_provider.h
  class SoftwareRasterBufferProvider
    → rasterizes into a shared-memory bitmap via SkCanvas
```

### Skia Interface

Both paths ultimately call Skia:

```
// GPU path
sk_sp<GrDirectContext> gr_context;
SkCanvas* canvas = surface->getCanvas();
cc::DisplayItemList::Raster(canvas);  // replays paint ops

// Software path
SkBitmap bitmap;
SkCanvas canvas(bitmap);
cc::DisplayItemList::Raster(&canvas);
```

Paint ops (`cc::PaintOp` subclasses in `cc/paint/paint_op.h`) are Skia draw
calls in serialized form: `DrawRectOp`, `DrawTextBlobOp`, `DrawImageOp`,
`ClipRectOp`, `SaveOp`/`RestoreOp`, etc.

### Output

Each rasterized tile produces a GPU texture (or software bitmap) stored in a
`ResourcePool`. The compositor thread uses these to assemble the final frame.

File: `cc/resources/resource_pool.h`.

---

## Stage 7: Display / Presentation

### Purpose

Assemble rastered tiles into a final frame and submit it to the display.

### Frame Assembly

`LayerTreeHostImpl::DrawLayers()` walks the active layer tree and issues draw
quads for each visible tile:

```
cc/trees/layer_tree_host_impl.h
  → DrawLayers(FrameData*)

cc/output/renderer_impl.h (or cc/output/skia_renderer.h)
  → DrawFrame(const RenderPassList&)
```

Each tile becomes a `TileDrawQuad` in a `CompositorRenderPass`. Render
passes are organized by effect tree structure (e.g., an opacity group gets
its own render pass that is blended back).

### Viz (Chromium)

In Chromium, draw quads are submitted to the `viz::DisplayCompositor` in the
GPU process via `CompositorFrameSink`:

```
components/viz/service/display/display.h
  class Display
    → aggregates frames from multiple surfaces (browser UI, renderer, video)
    → issues final GL/Vulkan draw calls
```

### Direct-to-Screen (Open UI)

We bypass viz entirely. Our presentation path:

```
Compositor DrawLayers()
    → SkiaRenderer produces GPU commands
    → SwapBuffers() / eglSwapBuffers() / vkQueuePresentKHR()
    → directly to the window system (X11/Wayland/Win32)
```

### VSync Synchronization

Frames are paced to VSync to avoid tearing and minimize latency:

```
cc/scheduler/begin_frame_source.h
  class BeginFrameSource
    → DelayBasedBeginFrameSource  (timer-based, our default)
    → ExternalBeginFrameSource    (driven by display hardware in Chromium)
```

The scheduler only begins draw work when a VSync signal arrives.

### Damage Tracking

Not every frame requires a full redraw. `DamageTracker` computes the union
of rectangles that changed since the last frame:

```
cc/trees/damage_tracker.h
  class DamageTracker
    → tracks per-layer damage rects
    → compositor only redraws the damaged region
```

This is critical for power efficiency — a blinking cursor should not
re-rasterize the entire screen.

---

## Where Open UI Cuts

The diagram below annotates each stage with the extraction boundary:

```
 ┌──────────────────────────────────────────────────────────────────┐
 │  REPLACED BY OPEN UI                                            │
 │                                                                  │
 │  ╔════════════════════════════════════════════════════════════╗  │
 │  ║  DOM + HTML Parser + CSS Parser + JavaScript Engine       ║  │
 │  ║  (Document, Element, V8, HTMLParser, CSSParser)           ║  │
 │  ║                                                            ║  │
 │  ║  → Replaced by Open UI's scene graph API (openui.h)      ║  │
 │  ║  → Application sets styles via C API, not CSS text        ║  │
 │  ║  → No HTML parsing, no JS evaluation, no networking       ║  │
 │  ╚════════════════════════════════════════════════════════════╝  │
 │                                                                  │
 ├──────────────────────────────────────────────────────────────────┤
 │  EXTRACTED INTO OPEN UI (kept, adapted)                         │
 │                                                                  │
 │  ┌──────────────────────────────────────────────────────────┐   │
 │  │  Style Resolution (libopenui_style)                      │   │
 │  │  ─ Cascade, specificity, inheritance machinery           │   │
 │  │  ─ ComputedStyle generation                              │   │
 │  │  ─ REMOVED: selector matching against DOM                │   │
 │  │  ─ ADDED: direct property-set API                        │   │
 │  └──────────────────────────────────────────────────────────┘   │
 │                                                                  │
 │  ┌──────────────────────────────────────────────────────────┐   │
 │  │  Layout Engine (libopenui_layout)                        │   │
 │  │  ─ LayoutNG: Block, Inline, Flex, Grid algorithms        │   │
 │  │  ─ NGConstraintSpace, NGPhysicalFragment                 │   │
 │  │  ─ Text shaping (HarfBuzz), line breaking                │   │
 │  │  ─ REMOVED: LayoutObject↔DOM binding                     │   │
 │  │  ─ ADDED: standalone LayoutInputNode from scene graph    │   │
 │  └──────────────────────────────────────────────────────────┘   │
 │                                                                  │
 │  ┌──────────────────────────────────────────────────────────┐   │
 │  │  Paint (part of libopenui_compositor)                    │   │
 │  │  ─ PaintController, display items, paint chunks          │   │
 │  │  ─ Property trees (transform, clip, effect, scroll)      │   │
 │  │  ─ Paint invalidation                                    │   │
 │  │  ─ Minimal changes: PaintController input comes from     │   │
 │  │    our fragment tree instead of LayoutObject              │   │
 │  └──────────────────────────────────────────────────────────┘   │
 │                                                                  │
 │  ┌──────────────────────────────────────────────────────────┐   │
 │  │  Compositor (libopenui_compositor)                       │   │
 │  │  ─ cc/ layer tree, tile manager, scheduler               │   │
 │  │  ─ Compositor-thread scrolling and animations            │   │
 │  │  ─ KEPT INTACT: this is the most self-contained piece    │   │
 │  └──────────────────────────────────────────────────────────┘   │
 │                                                                  │
 │  ┌──────────────────────────────────────────────────────────┐   │
 │  │  Rasterization (libopenui_skia)                          │   │
 │  │  ─ Skia 2D rasterization (GPU + software)               │   │
 │  │  ─ SIMPLIFIED: direct GPU context, no OOP-R IPC         │   │
 │  └──────────────────────────────────────────────────────────┘   │
 │                                                                  │
 ├──────────────────────────────────────────────────────────────────┤
 │  SIMPLIFIED                                                      │
 │                                                                  │
 │  ┌──────────────────────────────────────────────────────────┐   │
 │  │  viz/ → replaced with direct-to-screen presentation      │   │
 │  │  ─ No multi-surface aggregation                          │   │
 │  │  ─ No cross-process frame submission                     │   │
 │  │  ─ SkiaRenderer draws directly to a window surface       │   │
 │  └──────────────────────────────────────────────────────────┘   │
 │                                                                  │
 │  ┌──────────────────────────────────────────────────────────┐   │
 │  │  GPU process → runs in-process                           │   │
 │  │  ─ No GPU sandbox, no command buffer IPC                 │   │
 │  │  ─ Direct Skia GrDirectContext / Vulkan calls            │   │
 │  │  ─ OOP-R becomes direct rasterization                    │   │
 │  └──────────────────────────────────────────────────────────┘   │
 │                                                                  │
 └──────────────────────────────────────────────────────────────────┘
```

### Summary Table

| Pipeline Stage | Chromium Location | Open UI Library | Extraction Status |
|---------------|-------------------|-----------------|-------------------|
| DOM / HTML / CSS / JS | `third_party/blink/renderer/core/dom/`, `v8/` | **Removed** — replaced by `openui.h` scene graph | Replaced entirely |
| Style Resolution | `third_party/blink/renderer/core/css/resolver/` | `libopenui_style` | Extract cascade + computed style; remove selector matching |
| Layout (NG) | `third_party/blink/renderer/core/layout/ng/` | `libopenui_layout` | Extract algorithms + fragment tree; decouple from DOM |
| Paint | `third_party/blink/renderer/platform/graphics/paint/` | `libopenui_compositor` | Extract with minimal changes |
| Compositor | `cc/` | `libopenui_compositor` | Extract mostly intact |
| Rasterization | `cc/raster/`, Skia | `libopenui_skia` | Simplify to direct GPU raster |
| Display | `components/viz/` | Platform layer | Replace with direct-to-screen |

### Dependency Chain

```
libopenui_skia          ← standalone (Skia only)
       ↑
libopenui_compositor    ← depends on Skia
       ↑
libopenui_layout        ← depends on Compositor (for paint integration)
       ↑
libopenui_style         ← depends on Layout (ComputedStyle definition)
       ↑
libopenui               ← depends on all above (scene graph + public API)
```

### Key Extraction Challenges

1. **`base/` dependency** — Nearly every Chromium component depends on
   `base/` (threading, ref-counting, containers, logging). We must extract a
   minimal subset. See [ADR-003](../adr/003-base-extraction-strategy.md).

2. **DOM entanglement in style** — `StyleResolver` fundamentally assumes a
   DOM `Element`. We need an adapter layer that presents our scene graph
   nodes as something `StyleResolver` can consume, or we rewrite the entry
   point.

3. **LayoutObject ↔ DOM coupling** — `LayoutObject` has back-pointers to
   `Node`, and layout reads DOM state (e.g., `<input>` type affects intrinsic
   size). These must be severed.

4. **Blink-specific types in cc/** — Some cc/ interfaces use Blink types
   (e.g., `WebLayerTreeView`). The cc/ extraction needs to identify and
   replace these with standalone equivalents.

5. **Thread model** — Chromium's multi-process architecture means IPC
   boundaries that we can collapse. But the main-thread / compositor-thread
   split is essential for performance and must be preserved.

---

## References

- [Chromium Rendering Architecture](https://chromium.googlesource.com/chromium/src/+/main/docs/how_cc_works.md) — official cc/ documentation.
- [Life of a Pixel](https://docs.google.com/presentation/d/1boPxbgNrTU0ddsc144rcXayGA_WF53k96imRH8Mp34Y) — Chrome team talk on the full pipeline.
- [LayoutNG Design](https://chromium.googlesource.com/chromium/src/+/main/third_party/blink/renderer/core/layout/ng/README.md) — LayoutNG architecture.
- [BlinkOn: Property Trees](https://docs.google.com/presentation/d/1V7gCqKR-edNdRDv0bDnJa_uEs6iARAU2h5WhgxHyejQ) — property tree system.
