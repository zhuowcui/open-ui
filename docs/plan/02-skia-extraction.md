# Sub-Project 2: Skia Extraction & C API

> Chromium's Skia configuration building standalone with a clean C API for 2D graphics, rendering to a Linux window.

## Objective

Produce `libopenui_skia.so` — a standalone shared library that exposes Chromium's Skia graphics capabilities through a stable C API. This is the lowest layer of the stack and the foundation everything else paints into.

## Why Chromium's Skia?

Upstream Skia (skia.org) is already usable standalone, but Chromium's configuration adds:
- Optimized GPU backend configuration for real-world rendering
- Specific patches for performance and correctness
- Integration with Chromium's GPU process architecture
- Font fallback and text shaping configuration tuned for production

We start from Chromium's Skia and produce a C API that hides all C++ complexity behind opaque handles.

## Tasks

### 2.1 Skia Standalone Build (from SP1 POC → production)

- Promote the SP1 Skia POC into a proper build target
- Configure GPU backends:
  - **Vulkan** (primary — modern, performant, well-supported on Linux)
  - **OpenGL/EGL** (fallback — wider hardware support)
- Include all image codecs (PNG, JPEG, WebP, GIF)
- Include font backends (FreeType + FontConfig on Linux)
- Include text shaping (HarfBuzz + ICU subset)
- Build as a shared library with exported C symbols
- Strip internal symbols — only `openui_skia_*` symbols exported

### 2.2 C API Design (`include/openui/openui_skia.h`)

**Design principles:**
- Handle-based: all objects are opaque pointers (`OuiSkCanvas*`, `OuiSkPaint*`, etc.)
- No C++ in public headers
- Error handling via return codes (`OuiSkStatus`)
- Thread safety documented per function
- Resource lifecycle: explicit create/destroy pairs

**API surface:**

```c
// === Context & Surface ===
OuiSkStatus oui_sk_gpu_context_create(OuiSkGpuContext** ctx, OuiSkBackend backend);
OuiSkStatus oui_sk_surface_create_gpu(OuiSkSurface** surface, OuiSkGpuContext* ctx, int width, int height);
OuiSkStatus oui_sk_surface_create_raster(OuiSkSurface** surface, int width, int height);
OuiSkCanvas* oui_sk_surface_get_canvas(OuiSkSurface* surface);
void         oui_sk_surface_destroy(OuiSkSurface* surface);

// === Canvas ===
void oui_sk_canvas_save(OuiSkCanvas* canvas);
void oui_sk_canvas_restore(OuiSkCanvas* canvas);
void oui_sk_canvas_translate(OuiSkCanvas* canvas, float dx, float dy);
void oui_sk_canvas_scale(OuiSkCanvas* canvas, float sx, float sy);
void oui_sk_canvas_rotate(OuiSkCanvas* canvas, float degrees);
void oui_sk_canvas_clip_rect(OuiSkCanvas* canvas, const OuiSkRect* rect);
void oui_sk_canvas_clip_path(OuiSkCanvas* canvas, const OuiSkPath* path);
void oui_sk_canvas_clear(OuiSkCanvas* canvas, OuiSkColor color);

// === Drawing ===
void oui_sk_canvas_draw_rect(OuiSkCanvas* canvas, const OuiSkRect* rect, const OuiSkPaint* paint);
void oui_sk_canvas_draw_rrect(OuiSkCanvas* canvas, const OuiSkRRect* rrect, const OuiSkPaint* paint);
void oui_sk_canvas_draw_circle(OuiSkCanvas* canvas, float cx, float cy, float r, const OuiSkPaint* paint);
void oui_sk_canvas_draw_path(OuiSkCanvas* canvas, const OuiSkPath* path, const OuiSkPaint* paint);
void oui_sk_canvas_draw_text(OuiSkCanvas* canvas, const char* text, size_t len,
                              float x, float y, const OuiSkFont* font, const OuiSkPaint* paint);
void oui_sk_canvas_draw_image(OuiSkCanvas* canvas, const OuiSkImage* image,
                               float x, float y, const OuiSkPaint* paint);

// === Paint ===
OuiSkPaint* oui_sk_paint_create(void);
void        oui_sk_paint_destroy(OuiSkPaint* paint);
void        oui_sk_paint_set_color(OuiSkPaint* paint, OuiSkColor color);
void        oui_sk_paint_set_style(OuiSkPaint* paint, OuiSkPaintStyle style);
void        oui_sk_paint_set_stroke_width(OuiSkPaint* paint, float width);
void        oui_sk_paint_set_anti_alias(OuiSkPaint* paint, bool aa);
void        oui_sk_paint_set_blend_mode(OuiSkPaint* paint, OuiSkBlendMode mode);
void        oui_sk_paint_set_shader(OuiSkPaint* paint, OuiSkShader* shader);
void        oui_sk_paint_set_image_filter(OuiSkPaint* paint, OuiSkImageFilter* filter);

// === Path ===
OuiSkPath* oui_sk_path_create(void);
void       oui_sk_path_destroy(OuiSkPath* path);
void       oui_sk_path_move_to(OuiSkPath* path, float x, float y);
void       oui_sk_path_line_to(OuiSkPath* path, float x, float y);
void       oui_sk_path_quad_to(OuiSkPath* path, float cx, float cy, float x, float y);
void       oui_sk_path_cubic_to(OuiSkPath* path, float c1x, float c1y, float c2x, float c2y, float x, float y);
void       oui_sk_path_close(OuiSkPath* path);

// === Font & Text ===
OuiSkFont* oui_sk_font_create(const char* family, float size, OuiSkFontStyle style);
void       oui_sk_font_destroy(OuiSkFont* font);
OuiSkStatus oui_sk_text_measure(const OuiSkFont* font, const char* text, size_t len, OuiSkRect* bounds);
// Text shaping (HarfBuzz) for complex scripts
OuiSkTextBlob* oui_sk_text_shape(const char* text, size_t len, const OuiSkFont* font, float width);

// === Image ===
OuiSkStatus oui_sk_image_decode(OuiSkImage** image, const void* data, size_t len);
OuiSkStatus oui_sk_image_load_file(OuiSkImage** image, const char* path);
void        oui_sk_image_destroy(OuiSkImage* image);
int         oui_sk_image_width(const OuiSkImage* image);
int         oui_sk_image_height(const OuiSkImage* image);

// === Shaders & Effects ===
OuiSkShader* oui_sk_shader_linear_gradient(const OuiSkPoint* pts, const OuiSkColor* colors,
                                            const float* positions, int count);
OuiSkShader* oui_sk_shader_radial_gradient(const OuiSkPoint* center, float radius,
                                            const OuiSkColor* colors, const float* positions, int count);
OuiSkImageFilter* oui_sk_image_filter_blur(float sigmaX, float sigmaY);
OuiSkImageFilter* oui_sk_image_filter_drop_shadow(float dx, float dy, float sigmaX, float sigmaY, OuiSkColor color);
```

### 2.3 Platform Window Integration

Minimal windowing layer to display rendered content. This becomes `src/platform/`.

**Linux X11 (via xcb):**
- Window creation with GL/Vulkan surface
- Basic event loop (expose, resize, close)
- Keyboard and mouse event capture
- VSync-aware present

**Linux Wayland:**
- `wl_surface` + `wl_shell_surface` (or xdg-shell)
- EGL/Vulkan surface binding
- Frame callbacks for VSync

**Abstraction:**
```c
// Platform-agnostic window API
OuiSkStatus oui_window_create(OuiWindow** window, int width, int height, const char* title);
OuiSkStatus oui_window_get_surface(OuiWindow* window, OuiSkSurface** surface);
OuiSkStatus oui_window_present(OuiWindow* window);
bool        oui_window_poll_event(OuiWindow* window, OuiEvent* event);
void        oui_window_destroy(OuiWindow* window);
```

### 2.4 Validation & Benchmarks

**Render test suite:**
- Geometric primitives (rect, rrect, circle, path)
- Text rendering (Latin, CJK, Arabic/RTL, emoji)
- Image drawing (decode + composite)
- Gradients (linear, radial, sweep)
- Clipping (rect, path, complex)
- Blend modes
- Transforms (translate, rotate, scale, perspective)
- Filters (blur, drop shadow)

**Performance benchmarks:**
- Frame time for rendering N shapes
- Text shaping throughput
- Image decode + draw latency
- GPU memory usage
- Compare against: direct Skia C++ API (overhead of C wrapper should be < 1%)

**Correctness:**
- Pixel-level comparison against reference images
- Fuzzing the C API for memory safety

## Deliverables

| Deliverable | Description |
|---|---|
| `libopenui_skia.so` | Shared library with C API |
| `include/openui/openui_skia.h` | Public C header |
| `examples/skia_hello.c` | Hello world: window with shapes and text |
| `examples/skia_gallery.c` | Comprehensive rendering gallery |
| `tests/skia/` | Render test suite with reference images |
| `benchmarks/skia/` | Performance benchmarks |

## Success Criteria

- [ ] Render colored text and shapes to a Linux window via pure C API
- [ ] Vulkan and OpenGL backends both work
- [ ] Text shaping handles Latin, CJK, and Arabic correctly
- [ ] C API overhead vs. direct C++ is < 1% on benchmarks
- [ ] No memory leaks (validated with ASan/LSan)
- [ ] API documentation complete for all public functions
