# Sub-Project 6: Unified Pipeline & Scene Graph

> Wire all four layers into a single pipeline and build the retained scene graph with declarative API.

## Objective

Produce `libopenui.so` — the unified UI framework library that developers actually use. This is the "product" layer: a retained scene graph with a declarative API that internally drives the Style → Layout → Compositor → Skia pipeline. Think of it as the Flutter `Widget` tree or SwiftUI `View` hierarchy, but backed by Chromium's rendering engine.

## Architecture

```
Application Code
       │
       ▼
┌──────────────────────────────────────────────────┐
│               Scene Graph (this layer)            │
│  ┌──────────┐  ┌──────────┐  ┌────────────────┐ │
│  │ Node Tree│  │ Diffing  │  │ Event Dispatch │ │
│  │ Manager  │  │ Engine   │  │ & Hit Testing  │ │
│  └────┬─────┘  └────┬─────┘  └───────┬────────┘ │
│       │              │                │           │
│       ▼              ▼                ▼           │
│  ┌──────────────────────────────────────────┐    │
│  │          Pipeline Orchestrator           │    │
│  │  Style Resolve → Layout → Paint → Commit │    │
│  └──────────────────────────────────────────┘    │
├──────────┬──────────┬───────────────┬────────────┤
│  Style   │  Layout  │  Compositor   │    Skia    │
│  (SP5)   │  (SP4)   │    (SP3)      │   (SP2)    │
└──────────┴──────────┴───────────────┴────────────┘
```

## Tasks

### 6.1 Scene Graph Node System

**Node types:**

| Type | Purpose | Example |
|---|---|---|
| `OUI_NODE_BOX` | Generic container with visual properties | div-like container |
| `OUI_NODE_TEXT` | Text content | Label, paragraph |
| `OUI_NODE_IMAGE` | Image display | Icon, photo |
| `OUI_NODE_SCROLL` | Scrollable container | List, content area |
| `OUI_NODE_CUSTOM` | User-rendered content (direct Skia access) | Charts, canvas, games |
| `OUI_NODE_CLIP` | Clipping container | Rounded corners, masks |

**Node properties:**

Each node has:
- **Identity**: Stable key for diffing (optional, auto-generated if not provided)
- **Style rules**: Inline style + class-like style rules
- **Children**: Ordered list of child nodes
- **Event handlers**: Callbacks for input events
- **Lifecycle hooks**: Mount, update, unmount callbacks
- **User data**: Arbitrary pointer for application state

**Property change tracking:**
- Each property has a generation counter
- When a property changes, its generation increments
- Pipeline stages compare generations to skip unchanged work
- Dirty flags propagate up the tree: `STYLE_DIRTY`, `LAYOUT_DIRTY`, `PAINT_DIRTY`, `COMPOSITE_DIRTY`

### 6.2 Declarative API Design (`include/openui/openui.h`)

**Approach: Snapshot-based declarative API**

The application builds a tree descriptor each "frame" (or when state changes). The framework diffs the new descriptor against the current tree and applies minimal mutations.

```c
// === Application Lifecycle ===
OuiApp*    oui_app_create(const OuiAppConfig* config);
OuiStatus  oui_app_run(OuiApp* app);         // Enters main event loop
void       oui_app_request_update(OuiApp* app); // Request a re-render
void       oui_app_quit(OuiApp* app);
void       oui_app_destroy(OuiApp* app);

// === Window ===
OuiWindow* oui_window_create(OuiApp* app, const OuiWindowConfig* config);
void       oui_window_set_root(OuiWindow* window, OuiNodeDesc* root);
void       oui_window_destroy(OuiWindow* window);

// === Node Descriptors (declarative tree building) ===
// These describe what the UI *should* look like. The framework diffs and applies.
OuiNodeDesc* oui_desc_box(const char* key);
OuiNodeDesc* oui_desc_text(const char* key, const char* content);
OuiNodeDesc* oui_desc_image(const char* key, OuiImageSource source);
OuiNodeDesc* oui_desc_scroll(const char* key);
OuiNodeDesc* oui_desc_custom(const char* key, OuiCustomPaintFn paint_fn, void* userdata);

// === Node Descriptor Configuration (builder pattern) ===
OuiNodeDesc* oui_desc_style(OuiNodeDesc* desc, OuiStyleRule* style);
OuiNodeDesc* oui_desc_on_click(OuiNodeDesc* desc, OuiEventCallback cb, void* userdata);
OuiNodeDesc* oui_desc_on_scroll(OuiNodeDesc* desc, OuiScrollCallback cb, void* userdata);
OuiNodeDesc* oui_desc_on_key(OuiNodeDesc* desc, OuiKeyCallback cb, void* userdata);
OuiNodeDesc* oui_desc_on_hover(OuiNodeDesc* desc, OuiHoverCallback cb, void* userdata);
OuiNodeDesc* oui_desc_on_focus(OuiNodeDesc* desc, OuiFocusCallback cb, void* userdata);
OuiNodeDesc* oui_desc_children(OuiNodeDesc* desc, OuiNodeDesc** children, size_t count);
OuiNodeDesc* oui_desc_userdata(OuiNodeDesc* desc, void* data, OuiDestroyFn destroy);

// === Example Usage ===
// Build UI tree declaratively:
OuiNodeDesc* build_ui(AppState* state) {
    OuiNodeDesc* button = oui_desc_box("submit-btn");
    oui_desc_style(button, state->is_hovered ? style_btn_hover : style_btn);
    oui_desc_on_click(button, on_submit_click, state);
    OuiNodeDesc* label = oui_desc_text("submit-label", "Submit");
    oui_desc_style(label, style_btn_text);
    oui_desc_children(button, (OuiNodeDesc*[]){ label }, 1);
    return button;
}
```

### 6.3 Diffing Engine

When the application provides a new tree descriptor:
1. **Key matching**: Match old nodes to new descriptors by key
2. **Property comparison**: Detect which properties changed
3. **Tree mutations**: Insert, remove, reorder children
4. **Minimal pipeline invalidation**: Changed properties → appropriate dirty flags

This is analogous to React's reconciliation or Flutter's widget diffing.

### 6.4 Pipeline Orchestrator

Coordinates the full render pipeline on each frame:

```
1. Process input events → dispatch to nodes
2. Application code runs (may call oui_app_request_update)
3. If update requested:
   a. Call application's build_ui() → new tree descriptor
   b. Diff against current tree → mutations
   c. Style resolve (only dirty nodes)
   d. Layout compute (only dirty subtrees)
   e. Paint (only dirty layers)
   f. Commit to compositor thread
4. Compositor thread:
   a. Process commit (activate pending tree)
   b. Animate (advance compositor animations)
   c. Rasterize dirty tiles (on worker threads)
   d. Composite and present to screen
```

**Frame budget tracking:**
- Target 16.67ms per frame (60fps)
- If main thread work exceeds budget, compositor thread still animates/scrolls
- Pipeline stages report their time for profiling

### 6.5 Event & Input System

**Event types:**
- Mouse: move, button down/up, click, double-click, wheel
- Keyboard: key down/up, character input
- Touch: begin, move, end, cancel
- Focus: focus in/out, tab navigation
- Scroll: scroll begin, scroll update, scroll end
- Window: resize, close, DPI change

**Event dispatch:**
1. Platform event → coordinate transform → hit test through scene graph
2. Dispatch to target node and bubble up through ancestors
3. Event handlers can `consume` events to stop propagation
4. Focus management: tab order, focus trapping

**Gesture recognition:**
- Tap (down + up within threshold)
- Long press (down + hold)
- Drag (down + move beyond threshold)
- Scroll (wheel or touch pan)
- Pinch-to-zoom (multi-touch)

### 6.6 Threading Model

```
Main Thread                  Compositor Thread              Raster Pool
┌──────────────────┐        ┌────────────────────┐        ┌──────────┐
│ Event loop       │        │ Frame scheduling   │        │ Worker 1 │
│ App callbacks    │        │ Animation tick     │        │ Worker 2 │
│ build_ui()       │─commit→│ Scroll physics     │─work──→│ Worker 3 │
│ Style resolve    │        │ Property animation │←done───│ Worker 4 │
│ Layout compute   │        │ Tile scheduling    │        └──────────┘
│ Paint dispatch   │        │ Composite + present│
└──────────────────┘        └────────────────────┘
```

**Contract:**
- Main thread owns the scene graph (reads and writes)
- Compositor thread owns its copy of the layer tree (after commit)
- Commit is a synchronization point: main thread pushes changes, compositor thread activates
- Raster workers are stateless: rasterize a tile, return result
- Animations can run on either thread (compositor animations for scroll/transform/opacity)

### 6.7 Accessibility

Generate an accessibility tree from the scene graph:

- Map node types to accessibility roles (Box→group, Text→static text, Button→button, etc.)
- Expose text content, labels, descriptions
- Keyboard navigation and focus management
- Announce changes to screen readers

**Linux integration:**
- AT-SPI2 (Assistive Technology Service Provider Interface)
- Generate AT-SPI objects from our accessibility tree
- Handle AT-SPI requests (get text, get bounds, perform action)

### 6.8 Application Configuration

```c
typedef struct {
    const char* app_name;
    const char* app_id;
    OuiRendererBackend preferred_backend;  // Vulkan, OpenGL, Software
    int thread_pool_size;                   // 0 = auto (CPU count - 2)
    bool enable_vsync;
    bool enable_accessibility;
    OuiLogLevel log_level;
    OuiLogCallback log_callback;
} OuiAppConfig;

typedef struct {
    const char* title;
    int width;
    int height;
    int min_width, min_height;
    int max_width, max_height;
    bool resizable;
    bool decorated;
    float scale_factor;                     // 0 = auto-detect
    OuiBuildUiFn build_ui;                  // Called when update is needed
    void* userdata;                         // Passed to build_ui
} OuiWindowConfig;
```

## Deliverables

| Deliverable | Description |
|---|---|
| `libopenui.so` | Unified framework library (includes all layers) |
| `include/openui/openui.h` | Public C API — the main developer-facing header |
| `examples/hello_world.c` | Simple window with text and a button |
| `examples/counter.c` | Stateful counter app |
| `examples/todo_app.c` | Todo list application |
| `examples/layout_gallery.c` | Showcase of all layout modes |
| `examples/animation_demo.c` | Animated transitions |
| `tests/scene_graph/` | Diffing, event dispatch, pipeline tests |
| `benchmarks/pipeline/` | End-to-end frame time benchmarks |

## Success Criteria

- [ ] Hello world app renders correctly (text + colored box)
- [ ] Counter app works (click button → number updates → re-renders)
- [ ] Todo app works (add, remove, toggle items with smooth animations)
- [ ] 60fps maintained with 1,000 visible nodes
- [ ] Compositor-thread scroll/animation runs at 60fps even if main thread blocks for 100ms
- [ ] Accessibility: screen reader can read a simple app
- [ ] Memory: 10,000 node app uses < 50MB RAM
- [ ] Event dispatch: click, keyboard, scroll events routed correctly
