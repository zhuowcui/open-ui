# Open UI Skia C API Reference

> **Header**: `include/openui/openui_skia.h`
> **Library**: `libopenui_skia.so` / `openui_skia.dll`

## Overview

The Open UI Skia C API provides a stable, portable C ABI for 2D graphics rendering
powered by Google's Skia library. All types are opaque handles managed through
`create`/`destroy` lifecycle functions.

### Design Principles

- **Opaque handles**: All complex types are pointers to hidden structs. Callers
  never access fields directly.
- **Borrowed vs owned pointers**: Functions returning handles via `_create` or
  `_make_` transfer ownership — caller must destroy. Functions like
  `oui_sk_surface_get_canvas` return **borrowed** pointers — do NOT free.
- **NULL on failure**: All handle-returning functions return `NULL` on error.
- **Status codes**: Functions that can fail in multiple ways return `OuiSkStatus`.
- **Thread safety**: Individual objects are NOT thread-safe. Synchronize externally
  or use one object per thread.

---

## Status Codes

```c
typedef enum {
    OUI_SK_OK = 0,
    OUI_SK_ERROR_INVALID_ARGUMENT = 1,
    OUI_SK_ERROR_NULL_POINTER = 2,
    OUI_SK_ERROR_OUT_OF_MEMORY = 3,
    OUI_SK_ERROR_GPU_INIT_FAILED = 4,
    OUI_SK_ERROR_SURFACE_CREATION_FAILED = 5,
    OUI_SK_ERROR_ENCODE_FAILED = 6,
    OUI_SK_ERROR_DECODE_FAILED = 7,
    OUI_SK_ERROR_FILE_NOT_FOUND = 8,
    OUI_SK_ERROR_FONT_NOT_FOUND = 9,
    OUI_SK_ERROR_BACKEND_NOT_AVAILABLE = 10,
    OUI_SK_ERROR_WINDOW_CREATION_FAILED = 11,
    OUI_SK_ERROR_UNKNOWN = 99,
} OuiSkStatus;
```

Use `oui_sk_status_string(status)` to get a human-readable error message.

---

## Opaque Handle Types

| Type | Description | Lifecycle |
|------|-------------|-----------|
| `OuiSkSurface` | Drawing target (raster or GPU) | `create_raster`/`create_gpu` → `destroy` |
| `OuiSkCanvas` | Drawing context (borrowed from surface/window) | **Borrowed** — do NOT free |
| `OuiSkPaint` | Fill/stroke/effect configuration | `create` → `destroy` |
| `OuiSkPath` | Vector path (lines, curves, arcs) | `create` → `destroy` |
| `OuiSkFont` | Sized font for text rendering | `create` → `destroy` |
| `OuiSkTypeface` | Font family + style | `create_from_name`/`file`/`data` → `destroy` |
| `OuiSkTextBlob` | Pre-shaped text for drawing | `create`/`shape` → `destroy` |
| `OuiSkImage` | Decoded bitmap image | `decode`/`load_file`/`make_image_snapshot` → `destroy` |
| `OuiSkShader` | Gradient / image fill shader | `linear_gradient`/`radial`/`sweep`/`image` → `destroy` |
| `OuiSkImageFilter` | Post-processing filter (blur, shadow) | `blur`/`drop_shadow`/`compose` → `destroy` |
| `OuiSkMaskFilter` | Alpha mask filter | `blur` → `destroy` |
| `OuiSkColorFilter` | Color transformation filter | `blend`/`matrix`/`compose` → `destroy` |
| `OuiSkGpuContext` | GPU rendering context | `create_gl` → `destroy` |
| `OuiWindow` | Native window (X11/GLX) | `create` → `destroy` |

---

## Value Types

### OuiSkColor (8-bit RGBA)
```c
typedef struct { uint8_t r, g, b, a; } OuiSkColor;
OuiSkColor oui_sk_color_make(uint8_t r, uint8_t g, uint8_t b, uint8_t a);
```

### OuiSkColor4f (float RGBA, 0.0–1.0)
```c
typedef struct { float r, g, b, a; } OuiSkColor4f;
OuiSkColor4f oui_sk_color4f_make(float r, float g, float b, float a);
```

### OuiSkPoint
```c
typedef struct { float x, y; } OuiSkPoint;
```

### OuiSkSize
```c
typedef struct { float width, height; } OuiSkSize;
```

### OuiSkRect
```c
typedef struct { float left, top, right, bottom; } OuiSkRect;
OuiSkRect oui_sk_rect_make(float l, float t, float r, float b);
OuiSkRect oui_sk_rect_make_xywh(float x, float y, float w, float h);
```

### OuiSkRRect (rounded rectangle)
```c
typedef struct { OuiSkRect rect; float rx, ry; } OuiSkRRect;
OuiSkRRect oui_sk_rrect_make(OuiSkRect rect, float rx, float ry);
```

### OuiSkMatrix (3×3 affine)
```c
typedef struct { float values[9]; } OuiSkMatrix;
OuiSkMatrix oui_sk_matrix_identity(void);
```
Row-major layout: `[scale_x, skew_x, trans_x, skew_y, scale_y, trans_y, persp_0, persp_1, persp_2]`

---

## Surface & GPU Context

### oui_sk_surface_create_raster
```c
OuiSkSurface oui_sk_surface_create_raster(int width, int height);
```
Creates a CPU-backed drawing surface. Returns NULL if width/height are invalid or allocation fails.

### oui_sk_surface_create_gpu
```c
OuiSkSurface oui_sk_surface_create_gpu(OuiSkGpuContext gpu_ctx, int width, int height);
```
Creates a GPU-backed drawing surface. Requires a valid GPU context.

### oui_sk_surface_destroy
```c
void oui_sk_surface_destroy(OuiSkSurface surface);
```
Frees all resources. Invalidates any borrowed canvas pointer.

### oui_sk_surface_get_canvas
```c
OuiSkCanvas oui_sk_surface_get_canvas(OuiSkSurface surface);
```
Returns a **borrowed** canvas pointer. Do NOT free or destroy it. Valid until the surface is destroyed. Calling multiple times returns the same pointer.

### oui_sk_surface_read_pixels
```c
OuiSkStatus oui_sk_surface_read_pixels(
    OuiSkSurface surface, void* dst, size_t dst_row_bytes,
    int src_x, int src_y, int width, int height);
```
Copies pixel data from the surface into `dst`. Pixel format is N32 premultiplied alpha (BGRA on x86, RGBA on ARM).

### oui_sk_surface_make_image_snapshot
```c
OuiSkImage oui_sk_surface_make_image_snapshot(OuiSkSurface surface);
```
Creates an immutable image from the current surface contents. Caller must destroy the returned image.

### oui_sk_gpu_context_create_gl
```c
OuiSkGpuContext oui_sk_gpu_context_create_gl(void);
```
Creates an OpenGL GPU context (tries EGL first, then GLX). Returns NULL if no GL backend is available.

### oui_sk_gpu_context_destroy
```c
void oui_sk_gpu_context_destroy(OuiSkGpuContext ctx);
```

---

## Canvas

The canvas is the primary drawing interface. It supports transforms, clipping, and drawing primitives.

### State Management
```c
int  oui_sk_canvas_save(OuiSkCanvas canvas);
void oui_sk_canvas_restore(OuiSkCanvas canvas);
void oui_sk_canvas_restore_to_count(OuiSkCanvas canvas, int save_count);
int  oui_sk_canvas_get_save_count(OuiSkCanvas canvas);
```

### Transforms
```c
void oui_sk_canvas_translate(OuiSkCanvas canvas, float dx, float dy);
void oui_sk_canvas_scale(OuiSkCanvas canvas, float sx, float sy);
void oui_sk_canvas_rotate(OuiSkCanvas canvas, float degrees);
void oui_sk_canvas_skew(OuiSkCanvas canvas, float sx, float sy);
void oui_sk_canvas_concat_matrix(OuiSkCanvas canvas, const OuiSkMatrix* matrix);
```

### Clipping
```c
void oui_sk_canvas_clip_rect(OuiSkCanvas canvas, const OuiSkRect* rect, bool anti_alias);
void oui_sk_canvas_clip_rrect(OuiSkCanvas canvas, const OuiSkRRect* rrect, bool anti_alias);
void oui_sk_canvas_clip_path(OuiSkCanvas canvas, OuiSkPath path, bool anti_alias);
```

### Drawing
```c
void oui_sk_canvas_clear(OuiSkCanvas canvas, OuiSkColor color);
void oui_sk_canvas_draw_rect(OuiSkCanvas canvas, const OuiSkRect* rect, OuiSkPaint paint);
void oui_sk_canvas_draw_rrect(OuiSkCanvas canvas, const OuiSkRRect* rrect, OuiSkPaint paint);
void oui_sk_canvas_draw_circle(OuiSkCanvas canvas, float cx, float cy, float radius, OuiSkPaint paint);
void oui_sk_canvas_draw_oval(OuiSkCanvas canvas, const OuiSkRect* rect, OuiSkPaint paint);
void oui_sk_canvas_draw_path(OuiSkCanvas canvas, OuiSkPath path, OuiSkPaint paint);
void oui_sk_canvas_draw_line(OuiSkCanvas canvas, float x0, float y0, float x1, float y1, OuiSkPaint paint);
void oui_sk_canvas_draw_point(OuiSkCanvas canvas, float x, float y, OuiSkPaint paint);
void oui_sk_canvas_draw_image(OuiSkCanvas canvas, OuiSkImage image, float x, float y, OuiSkPaint paint);
void oui_sk_canvas_draw_image_rect(OuiSkCanvas canvas, OuiSkImage image,
    const OuiSkRect* src, const OuiSkRect* dst, OuiSkPaint paint);
void oui_sk_canvas_draw_text(OuiSkCanvas canvas, const char* text, size_t len,
    float x, float y, OuiSkFont font, OuiSkPaint paint);
void oui_sk_canvas_draw_text_blob(OuiSkCanvas canvas, OuiSkTextBlob blob,
    float x, float y, OuiSkPaint paint);
```

---

## Paint

Paint controls fill/stroke style, color, anti-aliasing, blend mode, and attached effects.

```c
OuiSkPaint oui_sk_paint_create(void);
OuiSkPaint oui_sk_paint_clone(OuiSkPaint paint);
void       oui_sk_paint_destroy(OuiSkPaint paint);

void oui_sk_paint_set_color(OuiSkPaint paint, OuiSkColor color);
void oui_sk_paint_set_color4f(OuiSkPaint paint, OuiSkColor4f color);
OuiSkColor oui_sk_paint_get_color(OuiSkPaint paint);
void oui_sk_paint_set_alpha(OuiSkPaint paint, uint8_t alpha);
void oui_sk_paint_set_style(OuiSkPaint paint, OuiSkPaintStyle style);
void oui_sk_paint_set_stroke_width(OuiSkPaint paint, float width);
void oui_sk_paint_set_stroke_cap(OuiSkPaint paint, OuiSkStrokeCap cap);
void oui_sk_paint_set_stroke_join(OuiSkPaint paint, OuiSkStrokeJoin join);
void oui_sk_paint_set_stroke_miter(OuiSkPaint paint, float miter);
void oui_sk_paint_set_anti_alias(OuiSkPaint paint, bool aa);
void oui_sk_paint_set_blend_mode(OuiSkPaint paint, OuiSkBlendMode mode);
void oui_sk_paint_set_shader(OuiSkPaint paint, OuiSkShader shader);
void oui_sk_paint_set_image_filter(OuiSkPaint paint, OuiSkImageFilter filter);
void oui_sk_paint_set_mask_filter(OuiSkPaint paint, OuiSkMaskFilter filter);
void oui_sk_paint_set_color_filter(OuiSkPaint paint, OuiSkColorFilter filter);
```

---

## Path

Paths describe vector geometry using lines, quadratic/cubic Bézier curves, and arcs.

```c
OuiSkPath oui_sk_path_create(void);
OuiSkPath oui_sk_path_clone(OuiSkPath path);
void      oui_sk_path_destroy(OuiSkPath path);

void oui_sk_path_move_to(OuiSkPath path, float x, float y);
void oui_sk_path_line_to(OuiSkPath path, float x, float y);
void oui_sk_path_quad_to(OuiSkPath path, float cx, float cy, float x, float y);
void oui_sk_path_cubic_to(OuiSkPath path, float c1x, float c1y,
    float c2x, float c2y, float x, float y);
void oui_sk_path_arc_to(OuiSkPath path, const OuiSkRect* oval,
    float start_angle, float sweep_angle, bool force_move_to);
void oui_sk_path_close(OuiSkPath path);
void oui_sk_path_reset(OuiSkPath path);

OuiSkPath oui_sk_path_from_svg_string(const char* svg);
OuiSkRect oui_sk_path_get_bounds(OuiSkPath path);
bool      oui_sk_path_contains(OuiSkPath path, float x, float y);
```

---

## Font & Text

### Typeface (font family)
```c
OuiSkTypeface oui_sk_typeface_create_from_name(
    const char* family_name, OuiSkFontStylePreset style);
OuiSkTypeface oui_sk_typeface_create_from_file(const char* path, int index);
OuiSkTypeface oui_sk_typeface_create_from_data(
    const void* data, size_t size, int index);
void oui_sk_typeface_destroy(OuiSkTypeface typeface);
```

Uses system FontConfig to resolve font names. Supports loading from file or in-memory data.

### Font (sized typeface)
```c
OuiSkFont oui_sk_font_create(OuiSkTypeface typeface, float size);
void oui_sk_font_destroy(OuiSkFont font);
void oui_sk_font_set_size(OuiSkFont font, float size);
float oui_sk_font_get_size(OuiSkFont font);
OuiSkFontMetrics oui_sk_font_get_metrics(OuiSkFont font);
float oui_sk_font_measure_text(OuiSkFont font, const char* text, size_t len);
```

### Text Blob
```c
OuiSkTextBlob oui_sk_text_blob_create(
    const char* text, size_t len, OuiSkFont font);
```
Creates a simple single-line text blob.

```c
OuiSkTextBlob oui_sk_text_shape(
    const char* text, size_t len, OuiSkFont font,
    float width, OuiSkTextAlign align, OuiSkTextDirection dir);
```
Creates a paragraph-shaped text blob with line wrapping, alignment, and bidirectional text support. Uses SkParagraph + ICU internally. Pass `width <= 0` for single-line (no wrapping).

```c
void oui_sk_text_blob_destroy(OuiSkTextBlob blob);
```

---

## Image

### Decoding
```c
OuiSkImage oui_sk_image_decode(const void* data, size_t size);
OuiSkImage oui_sk_image_load_file(const char* path);
```
Supports PNG, JPEG, WebP. Returns NULL on failure.

### Properties
```c
int oui_sk_image_width(OuiSkImage image);
int oui_sk_image_height(OuiSkImage image);
```

### Encoding
```c
OuiSkStatus oui_sk_image_encode(
    OuiSkImage image, OuiSkImageFormat format, int quality,
    void** out_data, size_t* out_size);
void oui_sk_image_encode_free(void* data);
```
Encodes to PNG, JPEG, or WebP. `quality` is 0–100 (ignored for PNG). Caller must
free encoded data with `oui_sk_image_encode_free`.

### Cleanup
```c
void oui_sk_image_destroy(OuiSkImage image);
```

---

## Shaders

### Gradients
```c
OuiSkShader oui_sk_shader_linear_gradient(
    OuiSkPoint start, OuiSkPoint end,
    const OuiSkColor* colors, const float* positions, int count,
    OuiSkTileMode tile_mode);

OuiSkShader oui_sk_shader_radial_gradient(
    OuiSkPoint center, float radius,
    const OuiSkColor* colors, const float* positions, int count,
    OuiSkTileMode tile_mode);

OuiSkShader oui_sk_shader_sweep_gradient(
    OuiSkPoint center,
    const OuiSkColor* colors, const float* positions, int count);
```

### Image shader
```c
OuiSkShader oui_sk_shader_image(
    OuiSkImage image, OuiSkTileMode tile_x, OuiSkTileMode tile_y);
```

### Cleanup
```c
void oui_sk_shader_destroy(OuiSkShader shader);
```

---

## Filters

### Image Filters
```c
OuiSkImageFilter oui_sk_image_filter_blur(
    float sigma_x, float sigma_y, OuiSkTileMode tile_mode);
OuiSkImageFilter oui_sk_image_filter_drop_shadow(
    float dx, float dy, float sigma_x, float sigma_y, OuiSkColor color);
OuiSkImageFilter oui_sk_image_filter_color_filter(OuiSkColorFilter color_filter);
OuiSkImageFilter oui_sk_image_filter_compose(
    OuiSkImageFilter outer, OuiSkImageFilter inner);
void oui_sk_image_filter_destroy(OuiSkImageFilter filter);
```

### Mask Filters
```c
OuiSkMaskFilter oui_sk_mask_filter_blur(OuiSkBlurStyle style, float sigma);
void oui_sk_mask_filter_destroy(OuiSkMaskFilter filter);
```

### Color Filters
```c
OuiSkColorFilter oui_sk_color_filter_blend(OuiSkColor color, OuiSkBlendMode mode);
OuiSkColorFilter oui_sk_color_filter_matrix(const float matrix[20]);
OuiSkColorFilter oui_sk_color_filter_compose(
    OuiSkColorFilter outer, OuiSkColorFilter inner);
void oui_sk_color_filter_destroy(OuiSkColorFilter filter);
```

---

## Window (X11/GLX)

Native windowing for on-screen rendering. Currently supports X11 with GLX.

```c
OuiWindow oui_window_create(
    const char* title, int width, int height, OuiSkBackend backend);
void oui_window_destroy(OuiWindow window);
```

**Backend options**: `OUI_SK_BACKEND_CPU`, `OUI_SK_BACKEND_GL`, `OUI_SK_BACKEND_AUTO`

```c
OuiSkCanvas oui_window_get_canvas(OuiWindow window);
```
Returns a **borrowed** canvas. Do NOT free. Valid until window destroy or resize.

```c
void oui_window_present(OuiWindow window);
bool oui_window_poll_event(OuiWindow window, OuiEvent* event);
OuiSkSize oui_window_get_size(OuiWindow window);
float oui_window_get_dpi_scale(OuiWindow window);
```

### Event Types
```c
OUI_EVENT_NONE, OUI_EVENT_QUIT, OUI_EVENT_WINDOW_RESIZED,
OUI_EVENT_WINDOW_EXPOSED, OUI_EVENT_MOUSE_MOTION,
OUI_EVENT_MOUSE_BUTTON_DOWN, OUI_EVENT_MOUSE_BUTTON_UP,
OUI_EVENT_MOUSE_WHEEL, OUI_EVENT_KEY_DOWN, OUI_EVENT_KEY_UP,
OUI_EVENT_TEXT_INPUT
```

---

## Quick Start

```c
#include "openui/openui_skia.h"
#include <stdio.h>

int main(void) {
    // Create a 400x300 raster surface
    OuiSkSurface surface = oui_sk_surface_create_raster(400, 300);
    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);

    // Clear to white
    oui_sk_canvas_clear(canvas, oui_sk_color_make(255, 255, 255, 255));

    // Draw a blue rectangle
    OuiSkPaint paint = oui_sk_paint_create();
    oui_sk_paint_set_color(paint, oui_sk_color_make(0, 120, 215, 255));
    OuiSkRect rect = oui_sk_rect_make_xywh(50, 50, 200, 100);
    oui_sk_canvas_draw_rect(canvas, &rect, paint);

    // Save to PNG
    OuiSkImage image = oui_sk_surface_make_image_snapshot(surface);
    void* data; size_t size;
    oui_sk_image_encode(image, OUI_SK_IMAGE_FORMAT_PNG, 100, &data, &size);

    FILE* f = fopen("output.png", "wb");
    fwrite(data, 1, size, f);
    fclose(f);

    // Cleanup
    oui_sk_image_encode_free(data);
    oui_sk_image_destroy(image);
    oui_sk_paint_destroy(paint);
    oui_sk_surface_destroy(surface);
    return 0;
}
```

Compile:
```bash
clang -std=c11 -I/path/to/open-ui -o myapp myapp.c -L/path/to/out -lopenui_skia
```
