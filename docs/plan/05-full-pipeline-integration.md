# Sub-Project 5: Full Pipeline Integration

> Wire the complete rendering pipeline end-to-end: C API → element tree → style → layout → paint → composite → pixels on screen.

## Objective

SP3 got the code compiling. SP4 created the DOM adapter and C API for building element trees and querying layout. SP5 connects the **full pipeline**: elements flow through Blink's paint system, through `PaintArtifactCompositor` into cc/'s layer tree, through tiling and rasterization, and finally to pixels in a window.

**End result:** A C program creates elements, sets styles, and sees them rendered in a GPU-accelerated window at 60fps — using Chromium's real paint and compositing code.

## Architecture

```
C API (oui_element_create, oui_element_set_*)
    │
    ▼
DOM Adapter → ComputedStyle → LayoutObject
    │
    ▼
Blink Paint (DisplayItem recording)
    │
    ▼
PaintArtifactCompositor (Blink → cc/ bridge)
    │
    ▼
cc/ Layer Tree → Tile Manager → Raster Workers (Skia GPU)
    │
    ▼
Frame → Swap Buffers → Window
```

## Tasks

### Phase A: Paint Pipeline Activation

1. **A1: Paint controller integration** — Hook Blink's `PaintController` to our element tree. After layout, trigger `Document::Paint()` which records DisplayItems for each LayoutObject.

2. **A2: Display item verification** — Verify that our styled elements produce correct display items. A red `<div>` should produce a `DrawingDisplayItem` with a red rectangle.

3. **A3: Paint property trees** — Verify that transform, clip, and effect paint property nodes are correctly built from our element styles (e.g., `opacity: 0.5` → EffectPaintPropertyNode).

### Phase B: Compositor Bridge

4. **B1: PaintArtifactCompositor activation** — Feed Blink's `PaintArtifact` (display items + paint chunks + property trees) into `PaintArtifactCompositor::Update()`. This produces cc::LayerList + cc::PropertyTrees.

5. **B2: LayerTreeHost setup** — Initialize cc/ `LayerTreeHost` with our compositor settings (single process, no viz, direct GL). Attach the layer list from PaintArtifactCompositor.

6. **B3: Direct compositing** — Replace viz/ display compositor with direct-to-surface compositing. cc/ produces frames → swap buffers directly. No IPC, no surface aggregation.

### Phase C: GPU Rasterization

7. **C1: GPU context** — Initialize GPU context (EGL/GLX on Linux). Create the raster surface that cc/ tiles rasterize into.

8. **C2: Tile rasterization** — Verify cc/'s tile manager creates tiles for our layers, schedules rasterization on worker threads, and produces rasterized tile textures.

9. **C3: Frame assembly** — Verify cc/ assembles rasterized tiles into a complete frame and presents to the window surface.

### Phase D: Window Integration

10. **D1: Window creation** — Integrate with platform windowing (X11/Wayland). Create a window with GL context suitable for cc/ output.

11. **D2: Resize handling** — Window resize → update viewport → re-layout → re-composite.

12. **D3: Input event routing** — Mouse/keyboard events → hit test against element tree → deliver to correct element.

13. **D4: VSync** — cc/ frame scheduler syncs with display VSync for smooth rendering.

### Phase E: End-to-End Verification

14. **E1: Render-to-image** — Render element tree to offscreen surface → save as PNG → pixel-compare against Chromium.

15. **E2: Interactive window** — Element tree rendered in a live window. Mouse hover highlights elements. Resize reflows.

16. **E3: Performance** — Measure frame time for various element counts. Target 60fps for typical UI (~1000 elements).

## Deliverables

| Deliverable | Description |
|---|---|
| Full pipeline working | Style → Layout → Paint → Composite → Pixels |
| `examples/hello_window.c` | First windowed application via C API |
| `examples/styled_boxes.c` | Colored/styled boxes rendered via full pipeline |
| `tests/pipeline/` | End-to-end pixel comparison tests |
| Window + input integration | Interactive windowed rendering |

## Success Criteria

- [ ] Red `<div>` renders as red rectangle in window — pixel-perfect match vs Chromium
- [ ] Nested flexbox layout renders correctly in window
- [ ] Text renders with correct font, size, color
- [ ] Opacity, transforms, clips work through compositor
- [ ] 60fps rendering for 1000-element tree
- [ ] Window resize triggers re-layout and re-render
- [ ] Mouse hit testing works through the element tree
