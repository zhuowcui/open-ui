# Sub-Project 5: Offscreen Rendering Pipeline

> Wire the complete paint pipeline end-to-end: C API → element tree → style → layout → paint → rasterize → pixels in memory / PNG on disk.

## Objective

SP3 proved blink's rendering pipeline works (style → layout → paint). SP4 created a stable C API for building element trees and querying layout. SP5 **closes the loop** by rasterizing the paint output into actual pixels — first offscreen (render-to-image), with pixel-perfect verification against reference images.

**End result:** A C program creates elements, sets styles, and calls `oui_document_render_to_bitmap()` to get an RGBA pixel buffer (or `oui_document_render_to_png()` to write a PNG). The output is pixel-identical to what Chromium renders for the same DOM.

**What SP5 does NOT include:** Windowing, GPU compositing, input events, VSync. Those are SP6 (Window & Compositor Integration).

## Architecture

```
C API (oui_element_create, oui_element_set_*)
    │
    ▼
DOM Adapter → ComputedStyle → LayoutObject
    │
    ▼
UpdateAllLifecyclePhasesForTest()
    │
    ▼
LocalFrameView::GetPaintRecord()
    │  Returns cc::PaintRecord (flat list of paint ops)
    ▼
PaintRecord::Playback(SkCanvas*)
    │  Replays all drawing commands onto a raster canvas
    ▼
SkBitmap (width × height BGRA pixels in memory)
    │
    ├──→ oui_document_render_to_bitmap()  → caller-owned RGBA buffer
    └──→ oui_document_render_to_png()     → PNG file on disk
```

### Key Insight: No Compositor Needed

For offscreen rendering, we skip the cc/ compositor entirely. `UpdateAllLifecyclePhasesForTest()` already runs style → layout → pre-paint → paint. The resulting `PaintArtifact` contains all drawing instructions as `cc::PaintRecord`, which can be replayed directly onto a Skia raster canvas. This is the same path used by blink's layout test infrastructure for pixel comparisons.

The cc/ compositor (LayerTreeHost, TileManager, GPU rasterization) is only needed for real-time windowed rendering with hardware acceleration — that's SP6.

## New C API Functions

```c
// ─── Rendering ──────────────────────────────────────────────
typedef struct {
  uint8_t* pixels;      // RGBA pixel data (caller must free with oui_free)
  int width;
  int height;
  int stride;           // bytes per row (width * 4)
} OuiBitmap;

// Render current element tree to an RGBA bitmap
OUI_EXPORT OuiStatus oui_document_render_to_bitmap(OuiDocument* doc,
                                                    OuiBitmap* out_bitmap);

// Free bitmap pixel data returned by render_to_bitmap
OUI_EXPORT void oui_bitmap_free(OuiBitmap* bitmap);

// Render current element tree and write PNG to file
OUI_EXPORT OuiStatus oui_document_render_to_png(OuiDocument* doc,
                                                 const char* file_path);

// Render to PNG in memory (caller must free data with oui_free)
OUI_EXPORT OuiStatus oui_document_render_to_png_buffer(OuiDocument* doc,
                                                        uint8_t** out_data,
                                                        size_t* out_size);

// Free memory allocated by render_to_png_buffer
OUI_EXPORT void oui_free(void* ptr);
```

## Tasks

### Phase A: Paint Record Extraction & Rasterization (Core)

1. **A1: Implement `openui_render.cc`** — New implementation file. Core function: given an `OuiDocumentImpl`, run `UpdateAllLifecyclePhasesForTest()`, call `LocalFrameView::GetPaintRecord()`, create an SkBitmap of the viewport size, create `SkiaPaintCanvas` targeting the bitmap, call `PaintRecord::Playback()`. Handle BGRA → RGBA conversion. Return pixel buffer.

2. **A2: Implement bitmap C API** — `oui_document_render_to_bitmap()` wraps the core rasterization. `oui_bitmap_free()` frees the pixel buffer. Add to `openui.h`. Wire through `openui_impl.cc`.

3. **A3: Implement PNG output** — `oui_document_render_to_png()` and `oui_document_render_to_png_buffer()` use `gfx::PNGCodec::EncodeBGRASkBitmap()` to encode the bitmap to PNG. `oui_free()` for buffer cleanup. Add `//ui/gfx/codec` to BUILD.gn deps.

### Phase B: Unit Tests — Rasterization Correctness

4. **B1: Basic rasterization test** — Create a 100×100 red div, render to bitmap. Verify: bitmap is 800×600 (viewport), pixel at (50, 50) is red, pixel at (400, 300) is white (background).

5. **B2: Multi-element rendering test** — Red, green, blue boxes side by side. Verify each box's region has the correct color.

6. **B3: Nested layout rendering** — Flexbox container with children of different sizes. Verify layout is reflected in pixel positions.

7. **B4: Text rendering test** — Set text content, render, verify non-white pixels exist in the text region (we can't pixel-match text exactly due to font differences, but we can verify it paints something).

8. **B5: CSS visual properties** — Test opacity, background-color, border rendering in pixels. A 50% opacity red should produce a blended pixel value.

9. **B6: Transform rendering** — A rotated box should produce pixels outside the original untransformed bounds.

10. **B7: PNG output test** — Render to PNG file, read it back with `gfx::PNGCodec::Decode()`, verify pixel values match the bitmap path.

### Phase C: Pixel Comparison Infrastructure

11. **C1: Reference image generation** — Write a test helper that generates reference PNGs from known-good renders. Store reference PNGs in `openui/test_data/references/`.

12. **C2: Pixel diff utility** — Implement `openui_pixel_diff.h`: compare two bitmaps pixel-by-pixel, report max channel difference and percentage of differing pixels. Allow configurable tolerance (default: 0 for exact match).

13. **C3: Pixel comparison tests** — For each rendering test (B1–B6), save a reference PNG on first run, then compare subsequent renders against it. Fail if any pixel differs beyond tolerance.

### Phase D: C Consumer Tests

14. **D1: C rendering test** — Pure C test program calls `oui_init()`, creates a document, adds styled elements, calls `oui_document_render_to_png()`, verifies the file exists and is non-empty. Validates the entire pipeline works from C.

15. **D2: C bitmap test** — Pure C test: render to bitmap, check dimensions, spot-check pixel colors, free bitmap.

### Phase E: Integration & Edge Cases

16. **E1: Empty document rendering** — Render an empty document. Should produce a white bitmap (no crash).

17. **E2: Viewport size rendering** — Render at different viewport sizes (320×240, 1920×1080). Verify bitmap dimensions match viewport.

18. **E3: Re-render after mutation** — Create elements, render, modify styles, render again. Second render should reflect the changes.

19. **E4: Multiple documents** — Render two separate documents to bitmaps. Verify they don't interfere with each other.

20. **E5: Large element tree** — Render a tree with 500+ elements. Verify it completes without timeout or crash. Sanity-check the output has non-white pixels.

### Phase F: Build & Review

21. **F1: BUILD.gn updates** — Add `openui_render.cc`, `openui_pixel_diff.h/.cc` to `openui_lib`. Add `//ui/gfx/codec` dep. Add `openui_render_test` target. Add `openui_c_render_test` target.

22. **F2: Multi-agent review** — 3 rounds of review with Opus 4.6 + GPT 5.4. Fix all issues until both find zero problems.

## Files

```
chromium/src/openui/
├── openui.h                    (updated — new render API)
├── openui_render.h             (new — render implementation header)
├── openui_render.cc            (new — render implementation)
├── openui_pixel_diff.h         (new — pixel comparison utility)
├── openui_pixel_diff.cc        (new — pixel comparison implementation)
├── openui_render_test.cc       (new — C++ rendering + pixel tests)
├── openui_c_render_test.c      (new — C consumer render tests)
├── test_data/references/       (new — reference PNGs)
└── BUILD.gn                    (updated — new targets & deps)
```

## Deliverables

| Deliverable | Description |
|---|---|
| `oui_document_render_to_bitmap()` | Offscreen rasterization to RGBA pixels |
| `oui_document_render_to_png()` | Direct PNG file output |
| `oui_document_render_to_png_buffer()` | PNG encoding to memory buffer |
| Pixel comparison infra | Diff utility + reference image workflow |
| 20+ rendering tests | Color, layout, text, transforms, opacity |
| C consumer render tests | Full pipeline from pure C |
| Multi-agent review | 3 rounds, zero issues |

## Success Criteria

- [ ] `oui_document_render_to_bitmap()` produces correct RGBA pixels for styled elements
- [ ] Red div renders as red pixels at correct position
- [ ] Flexbox/grid layouts render with correct spatial arrangement
- [ ] CSS opacity, transforms, borders, backgrounds all rasterize correctly
- [ ] PNG output round-trips: render → write PNG → read PNG → identical pixels
- [ ] Pixel comparison tests pass against reference images
- [ ] Empty document renders without crash (white bitmap)
- [ ] Re-render after DOM mutation reflects changes
- [ ] Multiple simultaneous documents render independently
- [ ] 500-element tree renders in reasonable time
- [ ] All tests pass, all code compiles, 3 review rounds clean

## Non-Goals (Deferred to SP6)

- Windowed rendering (X11/Wayland)
- GPU compositing (cc::LayerTreeHost, TileManager)
- VSync / 60fps frame scheduling
- Input event routing
- GPU rasterization (using CPU rasterization via Skia for now)
