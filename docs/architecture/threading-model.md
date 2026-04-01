# Threading Model

> How Chromium's rendering pipeline distributes work across threads, and how Open UI replicates this architecture for jank-free compositing and 60fps frame production.

## Overview

Chromium's rendering pipeline is fundamentally multi-threaded. The main thread runs application logic, style, layout, and paint recording, while a dedicated compositor thread handles animations, scroll physics, and frame scheduling independently. A pool of raster workers parallelizes tile rasterization via Skia. This separation is the reason Chromium can scroll smoothly even when JavaScript or layout is blocking the main thread.

Open UI extracts this proven architecture. Understanding the threading model is prerequisite knowledge for every sub-project — it dictates API design (which functions are main-thread-only), data ownership (what gets copied during commit), and synchronization strategy.

## Thread Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Open UI Process                                  │
│                                                                         │
│  ┌──────────────┐   commit    ┌──────────────────┐                      │
│  │  Main Thread  │──────────▶│ Compositor Thread  │                      │
│  │              │  BeginMain  │                    │                      │
│  │  • App code  │◀───────────│  • LayerTreeHost   │                      │
│  │  • Scene     │  Frame      │    Impl            │                      │
│  │    graph     │            │  • Scheduler       │                      │
│  │  • Style     │            │  • Animations      │                      │
│  │  • Layout    │            │  • Scroll physics  │                      │
│  │  • Paint     │            │  • Frame assembly  │                      │
│  │    recording │            │  • Damage tracking │                      │
│  └──────────────┘            └────────┬───────────┘                      │
│                                       │ raster tasks                     │
│                              ┌────────▼───────────┐                      │
│                              │ Raster Worker Pool  │                      │
│                              │                     │                      │
│                              │  ┌───┐ ┌───┐ ┌───┐ │                      │
│                              │  │ W1│ │ W2│ │ W3│ │   (CPU count − 2)   │
│                              │  └───┘ └───┘ └───┘ │                      │
│                              │  • Tile raster     │                      │
│                              │  • Skia backend    │                      │
│                              └─────────────────────┘                      │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  Chromium also has (simplified away in Open UI):                  │   │
│  │  • GPU thread/process — GPU command buffer dispatch              │   │
│  │  • I/O thread — IPC, Mojo, network (not needed for rendering)   │   │
│  │  • viz display compositor — multi-process surface aggregation    │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

### Chromium's Full Thread Set (for Reference)

| Thread | Chromium Source | Role | Open UI |
|---|---|---|---|
| Main (Blink) thread | `content/renderer/render_thread_impl.cc` | DOM, style, layout, paint, JS | ✅ App code, scene graph, style, layout, paint |
| Compositor thread | `cc/trees/single_thread_proxy.cc`, `cc/trees/proxy_impl.cc` | Layer tree impl, animations, scheduling, frame assembly | ✅ Full extraction |
| Raster workers | `cc/raster/categorized_worker_pool.cc` | Parallel tile rasterization | ✅ Full extraction |
| GPU thread | `gpu/ipc/service/gpu_channel_manager.cc` | GPU command buffer dispatch | ❌ Not needed — we run GPU ops in-process on compositor thread |
| I/O thread | `content/browser/browser_thread_impl.cc` | IPC, Mojo, network | ❌ Not needed — no network, no IPC |
| viz display compositor | `components/viz/service/display/display.cc` | Surface aggregation, multi-client compositing | ❌ Replaced by direct-to-screen compositing |

## Main Thread

The main thread is where application code runs. In Chromium it hosts Blink (DOM, CSS, JavaScript). In Open UI it hosts the application's scene graph mutations, style resolution, and layout computation.

### Responsibilities

| Concern | Chromium | Open UI |
|---|---|---|
| Application logic | JavaScript via V8 | User code (C/Rust/Python via C API) |
| Scene graph | DOM tree | `OuiNode` retained tree |
| Style resolution | `blink::StyleResolver` (`third_party/blink/renderer/core/css/resolver/style_resolver.cc`) | `oui_style_resolve()` |
| Layout | LayoutNG (`third_party/blink/renderer/core/layout/ng/`) | `oui_layout_compute()` |
| Paint recording | `blink::PaintController` (`third_party/blink/renderer/platform/graphics/paint/paint_controller.cc`) | `oui_paint_record()` → `cc::PaintRecord` |
| Layer tree (host side) | `cc::LayerTreeHost` (`cc/trees/layer_tree_host.cc`) | Extracted directly |

### Task Runner

The main thread runs a `base::SingleThreadTaskRunner` backed by a `base::RunLoop`. All main-thread work — application callbacks, style resolution, layout, paint — executes as tasks on this runner.

```
// Chromium source reference
// base/task/single_thread_task_runner.h
// base/run_loop.h

// Posting a task to the main thread:
main_task_runner_->PostTask(FROM_HERE, base::BindOnce(&DoWork));
```

**Key files:**
- `base/task/single_thread_task_runner.h` — Interface for posting to a single thread
- `base/task/sequenced_task_runner.h` — Sequenced (ordered) task execution
- `base/run_loop.h` — Event loop that pumps the task queue
- `base/message_loop/message_pump.h` — Platform-specific event pump (epoll on Linux, kqueue on macOS)

### Main Thread Blocking Concern

If the main thread blocks (long layout, expensive paint, slow application callback), the compositor thread continues independently — delivering smooth scroll and animations. This is the fundamental reason for the split. The main thread must never be required for ongoing frame production.

## Compositor Thread

The compositor thread owns the "impl-side" copy of the layer tree and is responsible for producing frames. It runs independently of the main thread and can scroll, animate, and composite even when the main thread is blocked.

### Responsibilities

1. **Layer tree management** — Owns `cc::LayerTreeHostImpl` with pending and active layer tree copies
2. **Animation tick** — Evaluates `cc::Animation` keyframes per frame, updates transform/opacity/filter
3. **Scroll physics** — Processes scroll input, applies overscroll elasticity
4. **Tile scheduling** — Decides which tiles to rasterize, at what priority, dispatches to raster pool
5. **Damage tracking** — Computes dirty regions from property changes and tile updates
6. **Frame assembly** — Builds the `viz::CompositorFrame` (or equivalent) from the active tree
7. **GPU submission** — Submits the assembled frame for display

### Key Source Files

| File | Purpose |
|---|---|
| `cc/trees/layer_tree_host_impl.cc` | Compositor-thread tree owner; drives draw, manage tiles |
| `cc/trees/layer_tree_impl.cc` | Single layer tree snapshot (pending or active) |
| `cc/scheduler/scheduler.cc` | Frame production state machine |
| `cc/scheduler/begin_frame_source.cc` | VSync signal source |
| `cc/tiles/tile_manager.cc` | Tile lifecycle orchestration |
| `cc/animation/animation_host.cc` | Animation tick dispatch |
| `cc/trees/property_tree.cc` | Transform/clip/effect/scroll hierarchies |

### Single-Threaded vs Multi-Threaded Compositing

Chromium supports two compositing modes, controlled by the proxy layer between `LayerTreeHost` (main) and `LayerTreeHostImpl` (compositor):

**`cc::SingleThreadProxy`** (`cc/trees/single_thread_proxy.cc`):
- Main and compositor logic run on the **same thread**
- Used for: software rendering fallback, tests, some Android WebView configurations
- Simpler but no jank isolation — blocked main thread = dropped frames
- Commit is synchronous (just a function call)

**`cc::ThreadProxy`** (split across `cc/trees/proxy_main.cc` and `cc/trees/proxy_impl.cc`):
- Compositor logic runs on a **dedicated thread**
- Used for: normal hardware-accelerated rendering (the common path)
- Full jank isolation — compositor thread produces frames independently
- Commit is asynchronous: main thread pushes state, compositor thread pulls it
- `ProxyMain` runs on the main thread; `ProxyImpl` runs on the compositor thread

**Open UI decision:** We extract `ThreadProxy` as the primary mode, with `SingleThreadProxy` available for testing and simpler deployments. The C API defaults to multi-threaded compositing.

### Scheduler

`cc::Scheduler` (`cc/scheduler/scheduler.cc`) is a state machine that coordinates frame production timing:

```
cc::Scheduler states (simplified):
┌─────────────────────────────────────────────────────┐
│                                                     │
│  IDLE ──▶ BEGIN_MAIN_FRAME_SENT ──▶ COMMIT         │
│    ▲           │                        │            │
│    │           │ (main busy)            ▼            │
│    │           ▼                    ACTIVATE         │
│    │     WAIT_FOR_MAIN              (when raster     │
│    │           │                     ready)          │
│    │           │                        │            │
│    │           ▼                        ▼            │
│    └──────── DRAW ◀──────────────── READY_TO_DRAW   │
│                                                     │
└─────────────────────────────────────────────────────┘
```

Key scheduler interfaces:
- `BeginFrameSource` — Provides VSync timing signals
- `SchedulerClient` — Callbacks invoked by the scheduler (`ScheduledActionBeginMainFrame`, `ScheduledActionCommit`, `ScheduledActionActivateSyncTree`, `ScheduledActionDraw`)
- `BeginFrameArgs` — Carries the frame deadline and interval

**Key files:**
- `cc/scheduler/scheduler.h` / `.cc` — State machine implementation
- `cc/scheduler/scheduler_state_machine.h` / `.cc` — Pure state machine logic (no I/O)
- `cc/scheduler/begin_frame_source.h` — VSync source interface and implementations

## Raster Worker Pool

Tile rasterization is the most parallelizable part of the rendering pipeline. Chromium uses a dedicated thread pool where each worker independently rasterizes tiles using Skia.

### Architecture

```
Compositor Thread                    Worker Pool
┌──────────────────┐               ┌─────────────────────────┐
│  TileManager     │               │                         │
│                  │  dispatch     │  Worker 0: Tile A       │
│  Priority queue  │──────────────▶│  Worker 1: Tile B       │
│  of tiles to     │               │  Worker 2: Tile C       │
│  rasterize       │◀──────────────│  Worker 3: (idle)       │
│                  │  completion   │                         │
│  Tile tracker    │  callbacks    │  Each worker:           │
│  (state per tile)│               │  1. Dequeue tile        │
└──────────────────┘               │  2. Lock Skia surface   │
                                   │  3. Rasterize via Skia  │
                                   │  4. Signal completion   │
                                   └─────────────────────────┘
```

### Key Source Files

| File | Purpose |
|---|---|
| `cc/raster/categorized_worker_pool.h` / `.cc` | Thread pool implementation |
| `cc/raster/raster_buffer_provider.h` | Interface for providing raster targets |
| `cc/raster/gpu_raster_buffer_provider.cc` | GPU-backed raster (OOP-R) |
| `cc/raster/software_raster_buffer_provider.cc` | CPU-backed raster (software fallback) |
| `cc/tiles/tile_manager.h` / `.cc` | Tile lifecycle, priority, scheduling |
| `cc/tiles/tile.h` | Individual tile state |
| `cc/tiles/picture_layer_tiling.h` / `.cc` | Tiling grid for a layer at a given scale |

### CategorizedWorkerPool

`cc::CategorizedWorkerPool` (`cc/raster/categorized_worker_pool.cc`) is the thread pool for raster work:

- **Pool size:** Typically `max(1, cpu_count - 2)`, capped at a platform-specific maximum. Determined by `base::SysInfo::NumberOfProcessors()` at startup.
- **Task categories:** Tasks are tagged with a category (`NONCONCURRENT_FOREGROUND`, `FOREGROUND`, `BACKGROUND`) to control priority and concurrency.
- **No shared mutable state:** Each worker operates on its own tile. The tile's backing store (bitmap or GPU texture) is exclusively owned by the worker during rasterization. No locks are held between workers.
- **Completion notification:** When a worker finishes a tile, it signals the `TileManager` on the compositor thread via task posting.

### Rasterization Modes

| Mode | Provider | Description | Open UI |
|---|---|---|---|
| **GPU raster (OOP-R)** | `GpuRasterBufferProvider` | Rasterize using Skia-on-GPU via the GPU process command buffer. Out-of-process raster. | Simplified: in-process Skia GPU via Vulkan/GL |
| **Software raster** | `SoftwareRasterBufferProvider` | Rasterize to CPU bitmaps, upload to GPU as textures. | ✅ Supported as fallback |
| **Zero-copy** | `ZeroCopyRasterBufferProvider` | Rasterize directly into shared memory GPU buffers (GBM/dma-buf on Linux). | Future optimization |

## Synchronization Points

The three threads communicate through well-defined synchronization points. Understanding these is critical for correctness — data races here cause flicker, stale frames, or crashes.

### 1. BeginMainFrame (Compositor → Main)

The compositor thread signals the main thread that it should produce new content for the next frame.

```
Compositor Thread                     Main Thread
       │                                   │
       │  PostTask(BeginMainFrame)          │
       │──────────────────────────────────▶│
       │                                   │ Style resolution
       │                                   │ Layout computation
       │                                   │ Paint recording
       │                                   │ Layer tree updates
       │                                   │
       │                                   │ (main frame complete)
       │         ReadyToCommit             │
       │◀──────────────────────────────────│
       │                                   │
```

**Source:** `cc::ProxyMain::BeginMainFrame()` (`cc/trees/proxy_main.cc`), called via task posted by `cc::ProxyImpl::ScheduledActionSendBeginMainFrame()`.

The main thread can take arbitrarily long to respond. The compositor doesn't wait — it continues producing frames from the last committed state. If the main thread finishes before the next VSync, the new content is committed. If not, the compositor reuses old content (the animation/scroll is still smooth).

### 2. Commit (Main → Compositor)

The commit copies the main thread's layer tree state to the compositor thread. This is the primary synchronization point.

```
Main Thread                          Compositor Thread
       │                                   │
       │  (main frame work complete)       │
       │                                   │
       │    ── blocked during commit ──    │
       │         PushProperties()          │
       │──────────────────────────────────▶│
       │         Copy layer tree           │
       │         Copy property trees       │
       │         Copy paint data           │
       │         (into pending tree)       │
       │◀──────────────────────────────────│
       │    ── main thread unblocked ──    │
       │                                   │
```

**What gets copied:**
- Layer tree structure (adds/removes/reorders)
- Property tree nodes (transform, clip, effect, scroll)
- Paint content (`cc::PaintRecord` display lists)
- Scroll offsets, layer bounds, and other properties

**Source:** `cc::LayerTreeHost::FinishCommitOnImplThread()` (`cc/trees/layer_tree_host.cc`), `cc::LayerTreeImpl::PushPropertiesTo()` (`cc/trees/layer_tree_impl.cc`).

The main thread is **blocked** during the commit copy. This is the only time the main thread is blocked by the compositor. Keeping commit fast is critical — it should be a memcpy-like operation, not a computation.

### 3. Activation (Compositor Thread Internal)

After commit, the copied state lives in the **pending tree**. Activation promotes it to the **active tree** once rasterization is ready.

```
Compositor Thread (internal)

   Pending Tree                    Active Tree
   ┌─────────────────┐           ┌─────────────────┐
   │ New layer state  │           │ Current drawing  │
   │ from last commit │──────────▶│ state            │
   │                  │ activate  │                  │
   │ Tiles may not be │           │ All tiles are    │
   │ rasterized yet   │           │ rasterized       │
   └─────────────────┘           └─────────────────┘
```

**Why two trees?** The pending tree may reference tiles that haven't been rasterized yet. Drawing from it would show blank tiles. The active tree only activates once all required tiles are ready, guaranteeing complete frames.

**Source:** `cc::LayerTreeHostImpl::ActivateSyncTree()` (`cc/trees/layer_tree_host_impl.cc`).

### 4. Draw / Submit (Compositor Thread → Display)

The compositor assembles a frame from the active tree and submits it:

1. Walk the active tree's property trees to compute visible regions
2. Build a render pass list with draw quads for each visible tile
3. Submit to the display (Vulkan swapchain, EGL surface, or software buffer)

**Source:** `cc::LayerTreeHostImpl::DrawLayers()` (`cc/trees/layer_tree_host_impl.cc`).

### Locking and Synchronization Primitives

| Primitive | Header | Usage in Compositor |
|---|---|---|
| `base::Lock` | `base/synchronization/lock.h` | Protects shared data during commit |
| `base::AutoLock` | `base/synchronization/lock.h` | RAII lock guard |
| `base::WaitableEvent` | `base/synchronization/waitable_event.h` | Main thread waits for commit completion |
| `base::ConditionVariable` | `base/synchronization/condition_variable.h` | Worker pool task signaling |
| `DCHECK_CALLED_ON_VALID_THREAD()` | `base/threading/thread_checker.h` | Compile/runtime assertion that code runs on the expected thread |
| `DCHECK_CALLED_ON_VALID_SEQUENCE()` | `base/sequence_checker.h` | Assert correct sequence (weaker than thread — allows thread migration) |

**Pattern: Thread checker macros.** Chromium annotates classes with thread affinity:

```cpp
// From cc/trees/layer_tree_host.h (main-thread-only class):
class LayerTreeHost {
 public:
  // ...
 private:
  THREAD_CHECKER(thread_checker_);
};

// In every method:
void LayerTreeHost::SetRootLayer(scoped_refptr<Layer> root) {
  DCHECK_CALLED_ON_VALID_THREAD(thread_checker_);
  // ...
}
```

This pattern is critical for Open UI — our C API functions that are main-thread-only must assert thread affinity.

## Data Ownership Model

Correct threading requires clear ownership rules. Each piece of data is owned by exactly one thread at any given time. Shared access is achieved through copying at commit time, not through concurrent access.

### Main-Thread-Only Data

These are touched only on the main thread. No synchronization needed.

| Data | Chromium Type | Chromium Source |
|---|---|---|
| Scene graph (DOM) | `blink::Node`, `blink::Element` | `third_party/blink/renderer/core/dom/` |
| Style data | `blink::ComputedStyle` | `third_party/blink/renderer/core/style/computed_style.h` |
| Layout tree | `blink::LayoutObject` | `third_party/blink/renderer/core/layout/layout_object.h` |
| Paint controller | `blink::PaintController` | `third_party/blink/renderer/platform/graphics/paint/paint_controller.h` |
| Layer tree (host side) | `cc::LayerTreeHost`, `cc::Layer` | `cc/trees/layer_tree_host.h`, `cc/layers/layer.h` |
| Paint records | `cc::PaintRecord` | `cc/paint/paint_record.h` |

### Compositor-Thread-Only Data

These are touched only on the compositor thread. No synchronization needed.

| Data | Chromium Type | Chromium Source |
|---|---|---|
| Layer tree impl (active) | `cc::LayerTreeImpl` (active) | `cc/trees/layer_tree_impl.h` |
| Layer tree impl (pending) | `cc::LayerTreeImpl` (pending) | `cc/trees/layer_tree_impl.h` |
| Tile textures / bitmaps | `cc::Tile`, `cc::TileDrawInfo` | `cc/tiles/tile.h` |
| Animation state | `cc::AnimationHost`, `cc::Animation` | `cc/animation/animation_host.h` |
| Scheduler state | `cc::Scheduler`, `cc::SchedulerStateMachine` | `cc/scheduler/scheduler.h` |
| Scroll state | `cc::ScrollTree` (active) | `cc/trees/scroll_tree.h` |

### Copied During Commit (Shared via Copy)

These are the data that get copied from the main thread's representation to the compositor thread's representation during the commit synchronization point.

| Data | Main Side | Compositor Side | Copy Mechanism |
|---|---|---|---|
| Layer tree structure | `cc::Layer` tree | `cc::LayerImpl` tree | `PushPropertiesTo()` per layer |
| Property trees | `cc::PropertyTrees` | `cc::PropertyTrees` (pending) | `PropertyTrees::PushOpacityIfNeeded()`, etc. |
| Paint content | `cc::PaintRecord` | `cc::RasterSource` | `PictureLayer::PushPropertiesTo()` copies the recording ref |
| Scroll offsets | `cc::ScrollTree` | `cc::ScrollTree` (pending) | `ScrollTree::PushScrollUpdatesFromMainThread()` |
| Layer bounds, transforms | `cc::Layer` properties | `cc::LayerImpl` properties | `Layer::PushPropertiesTo()` |

### Thread-Safe Utility Types

Some `base/` types are designed for cross-thread use:

| Type | Header | Use Case |
|---|---|---|
| `base::RefCountedThreadSafe<T>` | `base/memory/ref_counted.h` | Reference counting safe from any thread |
| `base::SequenceBound<T>` | `base/threading/sequence_bound.h` | Own an object on a specific sequence; call methods via posted tasks |
| `base::AtomicFlag` | `base/synchronization/atomic_flag.h` | Lock-free boolean flag |
| `base::subtle::Atomic32` | `base/atomicops.h` | Low-level atomic operations |
| `scoped_refptr<T>` | `base/memory/scoped_refptr.h` | Smart pointer for ref-counted objects (thread safety depends on T) |

## Frame Production Pipeline

This is the complete timing diagram for a single frame, showing how work flows across threads relative to VSync signals.

```
Time ──────────────────────────────────────────────────────────────────────▶

VSync N                                              VSync N+1
  │                                                     │
  ▼                                                     ▼
  ┊                                                     ┊
  ┊  Compositor Thread                                  ┊
  ┊  ┌─────────────────┐                                ┊
  ┊  │ BeginFrame       │                                ┊
  ┊  │ • Check if main  │                                ┊
  ┊  │   frame needed   │                                ┊
  ┊  └────────┬────────┘                                ┊
  ┊           │ PostTask                                 ┊
  ┊           ▼                                          ┊
  ┊  Main Thread                                        ┊
  ┊  ┌─────────────────────────────────────┐            ┊
  ┊  │ BeginMainFrame                       │            ┊
  ┊  │ 1. Animate (requestAnimationFrame)  │            ┊
  ┊  │ 2. Style resolution                 │            ┊
  ┊  │ 3. Layout                           │            ┊
  ┊  │ 4. Paint recording                  │            ┊
  ┊  │ 5. Layer tree update                │            ┊
  ┊  └────────────────────┬────────────────┘            ┊
  ┊                       │                              ┊
  ┊  ┌────────────────────▼──────┐                      ┊
  ┊  │ Commit (main blocked)     │                      ┊
  ┊  │ • PushProperties to       │                      ┊
  ┊  │   pending tree            │                      ┊
  ┊  └────────────────────┬──────┘                      ┊
  ┊                       │                              ┊
  ┊  Raster Workers       │                              ┊
  ┊  ┌────────────────────▼──────────────────────┐      ┊
  ┊  │ Rasterize new/dirty tiles (parallel)      │      ┊
  ┊  │  W0: ████ Tile A                          │      ┊
  ┊  │  W1: ██████ Tile B                        │      ┊
  ┊  │  W2: ███ Tile C                           │      ┊
  ┊  └────────────────────┬──────────────────────┘      ┊
  ┊                       │                              ┊
  ┊  Compositor Thread    │                              ┊
  ┊  ┌────────────────────▼──────┐                      ┊
  ┊  │ Activate                   │                      ┊
  ┊  │ • Pending tree → Active    │                      ┊
  ┊  └────────────────────┬──────┘                      ┊
  ┊                       │                              ┊
  ┊  ┌────────────────────▼──────┐                      ┊
  ┊  │ Draw & Submit              │                      ┊
  ┊  │ • Build render passes      │                      ┊
  ┊  │ • Submit to display        │  ◀── before deadline ┊
  ┊  └───────────────────────────┘                      ┊
  ┊                                                     ┊
```

### Pipeline Latency

In the best case, content produced in frame N appears on screen at VSync N+1 (one frame of latency). When the main thread is slow, the compositor reuses old content — frames are still produced at VSync rate, but main-thread updates arrive later.

### Compositor-Only Frames

When no main thread work is needed (e.g., ongoing compositor-driven animation or scroll), the compositor skips BeginMainFrame entirely:

```
VSync N                          VSync N+1
  │                                │
  ▼                                ▼
  Compositor Thread only:
  ┌──────────┐ ┌─────────┐ ┌──────────────┐
  │ BeginFrame│ │ Animate │ │ Draw & Submit│
  │           │ │ Scroll  │ │              │
  └──────────┘ └─────────┘ └──────────────┘

  Main Thread: (idle — not involved)
```

This is the key to jank-free scrolling and animation.

## Key `base/` Threading Primitives

These are the Chromium `base/` types that underpin the entire threading model. Open UI extracts these as part of the `base/` extraction (see [ADR 003](../adr/003-base-extraction-strategy.md)).

### `base::Thread`

A named thread with its own message loop and task runner.

```cpp
// base/threading/thread.h
// Creates a platform thread with a MessagePump for task execution.

auto compositor_thread = std::make_unique<base::Thread>("Compositor");
base::Thread::Options options;
options.message_pump_type = base::MessagePumpType::DEFAULT;
compositor_thread->StartWithOptions(std::move(options));

// Post work to it:
compositor_thread->task_runner()->PostTask(
    FROM_HERE, base::BindOnce(&DoCompositorWork));
```

**Key files:**
- `base/threading/thread.h` / `.cc` — Thread with message loop
- `base/threading/platform_thread.h` — Low-level platform thread (pthread_create wrapper)
- `base/threading/thread_id_name_manager.h` — Maps thread IDs to names (for debugging)

### `base::TaskRunner` / `base::SequencedTaskRunner` / `base::SingleThreadTaskRunner`

The abstraction for posting work to a thread or sequence.

```cpp
// base/task/task_runner.h        — Unordered task execution
// base/task/sequenced_task_runner.h — Ordered execution (sequence)
// base/task/single_thread_task_runner.h — Ordered + same thread

// Post a one-shot task:
task_runner->PostTask(FROM_HERE, base::BindOnce(&Func, arg));

// Post with delay:
task_runner->PostDelayedTask(FROM_HERE, base::BindOnce(&Func), base::Milliseconds(16));

// Post and reply (cross-thread):
base::PostTaskAndReplyWithResult(
    worker_runner.get(), FROM_HERE,
    base::BindOnce(&ComputeOnWorker),
    base::BindOnce(&HandleResultOnMain));
```

**Hierarchy:**
```
base::TaskRunner           (no ordering guarantees)
  └── base::SequencedTaskRunner   (tasks run in order, possibly migrating threads)
        └── base::SingleThreadTaskRunner   (tasks run in order on one thread)
```

### `base::OnceCallback` / `base::RepeatingCallback`

Typed, move-only (Once) or copyable (Repeating) callbacks that bind arguments.

```cpp
// base/functional/callback.h
// base/functional/bind.h

base::OnceCallback<void(int)> cb = base::BindOnce(&Func, extra_arg);
std::move(cb).Run(42);

base::RepeatingCallback<int()> rcb = base::BindRepeating(&GetValue);
int v = rcb.Run();  // Can call multiple times
```

**Key files:**
- `base/functional/callback.h` — Callback type declarations
- `base/functional/bind.h` — `BindOnce`, `BindRepeating`
- `base/functional/callback_helpers.h` — `DoNothing()`, `NullCallback()`

### `base::RunLoop`

The event loop that pumps tasks from the task queue.

```cpp
// base/run_loop.h

base::RunLoop run_loop;
// ... post tasks that eventually call run_loop.QuitClosure() ...
run_loop.Run();  // Blocks until quit
```

### `base::WaitableEvent`

Cross-thread signaling. One thread waits, another signals.

```cpp
// base/synchronization/waitable_event.h

base::WaitableEvent event;

// Thread A (waiting):
event.Wait();  // Blocks until signaled

// Thread B (signaling):
event.Signal();  // Unblocks Thread A
```

Used during commit: the main thread blocks on a `WaitableEvent` until the compositor thread finishes copying state.

### `base::Lock` / `base::AutoLock`

Mutex and RAII lock guard.

```cpp
// base/synchronization/lock.h

base::Lock lock_;

{
  base::AutoLock auto_lock(lock_);
  shared_data_.push_back(item);
}
```

Used sparingly in the compositor — the design prefers copying over sharing.

### `base::SequenceChecker` / `base::ThreadChecker`

Assertions that code runs on the expected sequence or thread.

```cpp
// base/sequence_checker.h
// base/threading/thread_checker.h

SEQUENCE_CHECKER(sequence_checker_);

void OnMainThread() {
  DCHECK_CALLED_ON_VALID_SEQUENCE(sequence_checker_);
  // ...
}
```

**Difference:** `SequenceChecker` allows a sequence to migrate between threads (common with thread pools). `ThreadChecker` requires the exact same thread. For the compositor and main thread, use `ThreadChecker`.

## Open UI Threading Design

Open UI replicates Chromium's threading architecture with targeted simplifications that remove browser-specific complexity while preserving the rendering performance characteristics.

### Thread Map

| Thread | Responsibility | Implementation |
|---|---|---|
| **Main thread** | App code, scene graph, style, layout, paint | Application's thread; runs `oui_run_loop()` or integrates with app's event loop |
| **Compositor thread** | Animations, scroll, frame scheduling, compositing | Created internally by `oui_compositor_create()` |
| **Raster workers** | Parallel tile rasterization via Skia | `CategorizedWorkerPool`, created by compositor |

### Simplifications vs Chromium

| Chromium | Open UI | Rationale |
|---|---|---|
| GPU process with command buffer IPC | In-process Vulkan/GL on compositor thread | No multi-process security boundary needed |
| `viz/` display compositor for surface aggregation | Direct-to-swapchain compositing | Single client, no multi-process surfaces |
| I/O thread for Mojo IPC and network | Not needed | No browser IPC, no network stack |
| Browser thread, utility processes | Not needed | Library, not a browser |
| `base::ThreadPool` (general-purpose) | `CategorizedWorkerPool` only | Only need raster parallelism |

### C API Threading Contract

The C API enforces clear threading rules:

```c
// Main-thread-only functions (majority of the API):
// Must be called from the thread that called oui_init().
OuiStatus oui_node_create(OuiContext* ctx, OuiNodeType type, OuiNode** node);
OuiStatus oui_node_set_style(OuiNode* node, const OuiStyleDecl* style);
OuiStatus oui_layout_compute(OuiContext* ctx);
OuiStatus oui_compositor_commit(OuiCompositor* comp);

// Thread-safe functions (callable from any thread):
// Explicitly documented as thread-safe.
void      oui_ref(OuiHandle handle);
void      oui_unref(OuiHandle handle);
OuiStatus oui_compositor_request_frame(OuiCompositor* comp);  // just posts a task

// Callbacks specify their thread:
// Paint callbacks     → called on raster worker threads
// Animation callbacks → called on compositor thread
// Event callbacks     → called on main thread
typedef void (*OuiPaintCallback)(OuiSkCanvas* canvas, const OuiRect* dirty, void* userdata);
typedef void (*OuiFrameCallback)(double frame_time_ms, void* userdata);
typedef void (*OuiEventCallback)(const OuiEvent* event, void* userdata);
```

**Enforcement:** Debug builds use `DCHECK_CALLED_ON_VALID_THREAD()` inside every main-thread-only function. Release builds omit the check for performance.

### Open UI Frame Loop

```
Application                        Open UI Internals
───────────                        ──────────────────

oui_init()                         Creates main-thread task runner
    │
oui_compositor_create()            Spawns compositor thread + raster pool
    │
oui_node_create(...)               Builds scene graph (main thread)
oui_node_set_style(...)
    │
oui_layout_compute()               Runs style → layout → paint (main thread)
    │
oui_compositor_commit()            Pushes to compositor (main blocked briefly)
    │                                  │
    │                                  ▼ Compositor thread:
    │                                  Rasterize tiles (raster pool)
    │                                  Activate
    │                                  Draw & submit to display
    │
oui_run_loop()                     Main event loop — pumps tasks, handles
    │                              BeginMainFrame requests from compositor
    │
    ▼ (repeat each frame)
```

### Integration with Application Event Loops

Applications may have their own event loop (GTK, Qt, game loop). Open UI supports two modes:

1. **Owned loop**: `oui_run_loop()` — Open UI runs the main loop. Simplest.
2. **External loop**: Application calls `oui_pump_tasks()` periodically from its own loop, processing pending Open UI tasks (BeginMainFrame, callbacks, etc.).

Both modes use `base::RunLoop` internally; the external loop mode wraps it with a foreign `MessagePump` adapter.

## Summary of Cross-Thread Data Flow

```
Main Thread                     Compositor Thread              Raster Workers
────────────                    ──────────────────             ──────────────
Scene Graph (owned)
Style Data (owned)
Layout Tree (owned)
Paint Records (owned)
                   ──commit──▶
cc::Layer tree                  cc::LayerImpl tree (pending)
                                  ──activate──▶
                                cc::LayerImpl tree (active)
                                Tile Manager (owned)
                                Animation State (owned)
                                Scheduler (owned)
                                                    ──dispatch──▶
                                                               Tile + Skia surface
                                                    ◀──done────
                                Tile textures (owned)
                                  ──draw──▶ Display
```

## References

- `cc/trees/layer_tree_host.h` — Main-thread tree owner
- `cc/trees/layer_tree_host_impl.h` — Compositor-thread tree owner
- `cc/trees/proxy_main.h` / `cc/trees/proxy_impl.h` — Thread proxy (main/impl sides)
- `cc/trees/single_thread_proxy.h` — Single-threaded compositor path
- `cc/scheduler/scheduler.h` — Frame production state machine
- `cc/scheduler/scheduler_state_machine.h` — Pure state logic
- `cc/raster/categorized_worker_pool.h` — Raster thread pool
- `cc/tiles/tile_manager.h` — Tile lifecycle management
- `cc/animation/animation_host.h` — Animation dispatch
- `base/threading/thread.h` — Named thread with message loop
- `base/task/single_thread_task_runner.h` — Task runner interface
- `base/synchronization/lock.h` — Mutex
- `base/synchronization/waitable_event.h` — Cross-thread signaling
- `base/threading/thread_checker.h` — Thread affinity assertions
- [ADR 003: Extract `base/` First, Shim Later](../adr/003-base-extraction-strategy.md)
- [Sub-Project 3: Compositor Extraction](../plan/03-compositor-extraction.md)
