/*
 * Open UI — Skia C API
 *
 * Stable C ABI for 2D graphics rendering powered by Skia.
 * All types are opaque handles. All functions use create/destroy lifecycle.
 * Thread safety: objects are NOT thread-safe. Use one object per thread,
 * or synchronize externally.
 *
 * Error handling: functions that can fail return OuiSkStatus.
 * Functions that return handles return NULL on failure.
 */

#ifndef OPENUI_SKIA_H_
#define OPENUI_SKIA_H_

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#  if defined(OUI_SK_BUILDING_DLL)
#    define OUI_SK_API __declspec(dllexport)
#  else
#    define OUI_SK_API __declspec(dllimport)
#  endif
#else
#  if defined(OUI_SK_BUILDING_DLL)
#    define OUI_SK_API __attribute__((visibility("default")))
#  else
#    define OUI_SK_API
#  endif
#endif

/* ─── Status codes ──────────────────────────────────────────────── */

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

/* ─── Opaque handle types ───────────────────────────────────────── */

typedef struct OuiSkSurface_t*      OuiSkSurface;
typedef struct OuiSkCanvas_t*       OuiSkCanvas;
typedef struct OuiSkPaint_t*        OuiSkPaint;
typedef struct OuiSkPath_t*         OuiSkPath;
typedef struct OuiSkFont_t*         OuiSkFont;
typedef struct OuiSkTypeface_t*     OuiSkTypeface;
typedef struct OuiSkTextBlob_t*     OuiSkTextBlob;
typedef struct OuiSkImage_t*        OuiSkImage;
typedef struct OuiSkShader_t*       OuiSkShader;
typedef struct OuiSkImageFilter_t*  OuiSkImageFilter;
typedef struct OuiSkMaskFilter_t*   OuiSkMaskFilter;
typedef struct OuiSkColorFilter_t*  OuiSkColorFilter;
typedef struct OuiSkGpuContext_t*   OuiSkGpuContext;
typedef struct OuiWindow_t*         OuiWindow;

/* ─── Value types ───────────────────────────────────────────────── */

typedef struct {
    float x, y;
} OuiSkPoint;

typedef struct {
    float width, height;
} OuiSkSize;

typedef struct {
    float left, top, right, bottom;
} OuiSkRect;

typedef struct {
    OuiSkRect rect;
    float rx, ry;
} OuiSkRRect;

typedef struct {
    uint8_t r, g, b, a;
} OuiSkColor;

typedef struct {
    float r, g, b, a;
} OuiSkColor4f;

typedef struct {
    /* Row-major 3x3 affine matrix:
     * | scale_x  skew_x   trans_x |
     * | skew_y   scale_y  trans_y |
     * | persp_0  persp_1  persp_2 |
     */
    float values[9];
} OuiSkMatrix;

/* ─── Enums ─────────────────────────────────────────────────────── */

typedef enum {
    OUI_SK_PAINT_STYLE_FILL = 0,
    OUI_SK_PAINT_STYLE_STROKE = 1,
    OUI_SK_PAINT_STYLE_FILL_AND_STROKE = 2,
} OuiSkPaintStyle;

typedef enum {
    OUI_SK_BLEND_MODE_CLEAR = 0,
    OUI_SK_BLEND_MODE_SRC,
    OUI_SK_BLEND_MODE_DST,
    OUI_SK_BLEND_MODE_SRC_OVER,
    OUI_SK_BLEND_MODE_DST_OVER,
    OUI_SK_BLEND_MODE_SRC_IN,
    OUI_SK_BLEND_MODE_DST_IN,
    OUI_SK_BLEND_MODE_SRC_OUT,
    OUI_SK_BLEND_MODE_DST_OUT,
    OUI_SK_BLEND_MODE_SRC_ATOP,
    OUI_SK_BLEND_MODE_DST_ATOP,
    OUI_SK_BLEND_MODE_XOR,
    OUI_SK_BLEND_MODE_PLUS,
    OUI_SK_BLEND_MODE_MODULATE,
    OUI_SK_BLEND_MODE_SCREEN,
    OUI_SK_BLEND_MODE_OVERLAY,
    OUI_SK_BLEND_MODE_DARKEN,
    OUI_SK_BLEND_MODE_LIGHTEN,
    OUI_SK_BLEND_MODE_COLOR_DODGE,
    OUI_SK_BLEND_MODE_COLOR_BURN,
    OUI_SK_BLEND_MODE_HARD_LIGHT,
    OUI_SK_BLEND_MODE_SOFT_LIGHT,
    OUI_SK_BLEND_MODE_DIFFERENCE,
    OUI_SK_BLEND_MODE_EXCLUSION,
    OUI_SK_BLEND_MODE_MULTIPLY,
    OUI_SK_BLEND_MODE_HUE,
    OUI_SK_BLEND_MODE_SATURATION,
    OUI_SK_BLEND_MODE_COLOR,
    OUI_SK_BLEND_MODE_LUMINOSITY,
} OuiSkBlendMode;

typedef enum {
    OUI_SK_TILE_MODE_CLAMP = 0,
    OUI_SK_TILE_MODE_REPEAT = 1,
    OUI_SK_TILE_MODE_MIRROR = 2,
    OUI_SK_TILE_MODE_DECAL = 3,
} OuiSkTileMode;

typedef enum {
    OUI_SK_FONT_STYLE_NORMAL = 0,
    OUI_SK_FONT_STYLE_BOLD = 1,
    OUI_SK_FONT_STYLE_ITALIC = 2,
    OUI_SK_FONT_STYLE_BOLD_ITALIC = 3,
} OuiSkFontStylePreset;

typedef enum {
    OUI_SK_TEXT_ALIGN_LEFT = 0,
    OUI_SK_TEXT_ALIGN_CENTER = 1,
    OUI_SK_TEXT_ALIGN_RIGHT = 2,
} OuiSkTextAlign;

typedef enum {
    OUI_SK_TEXT_DIRECTION_LTR = 0,
    OUI_SK_TEXT_DIRECTION_RTL = 1,
} OuiSkTextDirection;

typedef enum {
    OUI_SK_IMAGE_FORMAT_PNG = 0,
    OUI_SK_IMAGE_FORMAT_JPEG = 1,
    OUI_SK_IMAGE_FORMAT_WEBP = 2,
} OuiSkImageFormat;

typedef enum {
    OUI_SK_BLUR_STYLE_NORMAL = 0,
    OUI_SK_BLUR_STYLE_SOLID = 1,
    OUI_SK_BLUR_STYLE_OUTER = 2,
    OUI_SK_BLUR_STYLE_INNER = 3,
} OuiSkBlurStyle;

typedef enum {
    OUI_SK_STROKE_CAP_BUTT = 0,
    OUI_SK_STROKE_CAP_ROUND = 1,
    OUI_SK_STROKE_CAP_SQUARE = 2,
} OuiSkStrokeCap;

typedef enum {
    OUI_SK_STROKE_JOIN_MITER = 0,
    OUI_SK_STROKE_JOIN_ROUND = 1,
    OUI_SK_STROKE_JOIN_BEVEL = 2,
} OuiSkStrokeJoin;

typedef enum {
    OUI_SK_BACKEND_CPU = 0,
    OUI_SK_BACKEND_GL = 1,
    OUI_SK_BACKEND_VULKAN = 2,
    OUI_SK_BACKEND_AUTO = 3,
} OuiSkBackend;

/* Font metrics returned by oui_sk_font_get_metrics */
typedef struct {
    float ascent;
    float descent;
    float leading;
} OuiSkFontMetrics;

/* ─── Window / Event types ──────────────────────────────────────── */

typedef enum {
    OUI_EVENT_NONE = 0,
    OUI_EVENT_QUIT,
    OUI_EVENT_WINDOW_RESIZED,
    OUI_EVENT_WINDOW_EXPOSED,
    OUI_EVENT_MOUSE_MOTION,
    OUI_EVENT_MOUSE_BUTTON_DOWN,
    OUI_EVENT_MOUSE_BUTTON_UP,
    OUI_EVENT_MOUSE_WHEEL,
    OUI_EVENT_KEY_DOWN,
    OUI_EVENT_KEY_UP,
    OUI_EVENT_TEXT_INPUT,
} OuiEventType;

typedef struct {
    OuiEventType type;
    union {
        struct { int width; int height; } resize;
        struct { float x; float y; } mouse_motion;
        struct { float x; float y; int button; } mouse_button;
        struct { float dx; float dy; } mouse_wheel;
        struct { uint32_t keycode; uint32_t scancode; bool repeat; } key;
        struct { char text[32]; } text_input;
    };
} OuiEvent;

/* ─── Surface & GPU Context ─────────────────────────────────────── */

OUI_SK_API OuiSkSurface oui_sk_surface_create_raster(int width, int height);
OUI_SK_API OuiSkSurface oui_sk_surface_create_gpu(
    OuiSkGpuContext gpu_ctx, int width, int height);
OUI_SK_API void oui_sk_surface_destroy(OuiSkSurface surface);
OUI_SK_API OuiSkCanvas oui_sk_surface_get_canvas(OuiSkSurface surface);
    /* Returns a borrowed pointer — do NOT free or destroy. Valid until surface
       is destroyed. Calling this multiple times returns the same pointer. */
OUI_SK_API OuiSkStatus oui_sk_surface_read_pixels(
    OuiSkSurface surface, void* dst, size_t dst_row_bytes,
    int src_x, int src_y, int width, int height);
OUI_SK_API OuiSkImage oui_sk_surface_make_image_snapshot(OuiSkSurface surface);

OUI_SK_API OuiSkGpuContext oui_sk_gpu_context_create_gl(void);
OUI_SK_API void oui_sk_gpu_context_destroy(OuiSkGpuContext ctx);

/* ─── Canvas ────────────────────────────────────────────────────── */

OUI_SK_API int  oui_sk_canvas_save(OuiSkCanvas canvas);
OUI_SK_API void oui_sk_canvas_restore(OuiSkCanvas canvas);
OUI_SK_API void oui_sk_canvas_restore_to_count(OuiSkCanvas canvas, int save_count);
OUI_SK_API int  oui_sk_canvas_get_save_count(OuiSkCanvas canvas);

OUI_SK_API void oui_sk_canvas_translate(OuiSkCanvas canvas, float dx, float dy);
OUI_SK_API void oui_sk_canvas_scale(OuiSkCanvas canvas, float sx, float sy);
OUI_SK_API void oui_sk_canvas_rotate(OuiSkCanvas canvas, float degrees);
OUI_SK_API void oui_sk_canvas_skew(OuiSkCanvas canvas, float sx, float sy);
OUI_SK_API void oui_sk_canvas_concat_matrix(
    OuiSkCanvas canvas, const OuiSkMatrix* matrix);

OUI_SK_API void oui_sk_canvas_clip_rect(
    OuiSkCanvas canvas, const OuiSkRect* rect, bool anti_alias);
OUI_SK_API void oui_sk_canvas_clip_rrect(
    OuiSkCanvas canvas, const OuiSkRRect* rrect, bool anti_alias);
OUI_SK_API void oui_sk_canvas_clip_path(
    OuiSkCanvas canvas, OuiSkPath path, bool anti_alias);

OUI_SK_API void oui_sk_canvas_clear(OuiSkCanvas canvas, OuiSkColor color);

OUI_SK_API void oui_sk_canvas_draw_rect(
    OuiSkCanvas canvas, const OuiSkRect* rect, OuiSkPaint paint);
OUI_SK_API void oui_sk_canvas_draw_rrect(
    OuiSkCanvas canvas, const OuiSkRRect* rrect, OuiSkPaint paint);
OUI_SK_API void oui_sk_canvas_draw_circle(
    OuiSkCanvas canvas, float cx, float cy, float radius, OuiSkPaint paint);
OUI_SK_API void oui_sk_canvas_draw_oval(
    OuiSkCanvas canvas, const OuiSkRect* rect, OuiSkPaint paint);
OUI_SK_API void oui_sk_canvas_draw_path(
    OuiSkCanvas canvas, OuiSkPath path, OuiSkPaint paint);
OUI_SK_API void oui_sk_canvas_draw_line(
    OuiSkCanvas canvas, float x0, float y0, float x1, float y1,
    OuiSkPaint paint);
OUI_SK_API void oui_sk_canvas_draw_point(
    OuiSkCanvas canvas, float x, float y, OuiSkPaint paint);

OUI_SK_API void oui_sk_canvas_draw_image(
    OuiSkCanvas canvas, OuiSkImage image, float x, float y,
    OuiSkPaint paint);
OUI_SK_API void oui_sk_canvas_draw_image_rect(
    OuiSkCanvas canvas, OuiSkImage image,
    const OuiSkRect* src, const OuiSkRect* dst, OuiSkPaint paint);

OUI_SK_API void oui_sk_canvas_draw_text(
    OuiSkCanvas canvas, const char* text, size_t len,
    float x, float y, OuiSkFont font, OuiSkPaint paint);
OUI_SK_API void oui_sk_canvas_draw_text_blob(
    OuiSkCanvas canvas, OuiSkTextBlob blob,
    float x, float y, OuiSkPaint paint);

/* ─── Paint ─────────────────────────────────────────────────────── */

OUI_SK_API OuiSkPaint oui_sk_paint_create(void);
OUI_SK_API OuiSkPaint oui_sk_paint_clone(OuiSkPaint paint);
OUI_SK_API void oui_sk_paint_destroy(OuiSkPaint paint);

OUI_SK_API void oui_sk_paint_set_color(OuiSkPaint paint, OuiSkColor color);
OUI_SK_API void oui_sk_paint_set_color4f(OuiSkPaint paint, OuiSkColor4f color);
OUI_SK_API OuiSkColor oui_sk_paint_get_color(OuiSkPaint paint);
OUI_SK_API void oui_sk_paint_set_alpha(OuiSkPaint paint, uint8_t alpha);
OUI_SK_API void oui_sk_paint_set_style(OuiSkPaint paint, OuiSkPaintStyle style);
OUI_SK_API void oui_sk_paint_set_stroke_width(OuiSkPaint paint, float width);
OUI_SK_API void oui_sk_paint_set_stroke_cap(OuiSkPaint paint, OuiSkStrokeCap cap);
OUI_SK_API void oui_sk_paint_set_stroke_join(OuiSkPaint paint, OuiSkStrokeJoin join);
OUI_SK_API void oui_sk_paint_set_stroke_miter(OuiSkPaint paint, float miter);
OUI_SK_API void oui_sk_paint_set_anti_alias(OuiSkPaint paint, bool aa);
OUI_SK_API void oui_sk_paint_set_blend_mode(OuiSkPaint paint, OuiSkBlendMode mode);
OUI_SK_API void oui_sk_paint_set_shader(OuiSkPaint paint, OuiSkShader shader);
OUI_SK_API void oui_sk_paint_set_image_filter(
    OuiSkPaint paint, OuiSkImageFilter filter);
OUI_SK_API void oui_sk_paint_set_mask_filter(
    OuiSkPaint paint, OuiSkMaskFilter filter);
OUI_SK_API void oui_sk_paint_set_color_filter(
    OuiSkPaint paint, OuiSkColorFilter filter);

/* ─── Path ──────────────────────────────────────────────────────── */

OUI_SK_API OuiSkPath oui_sk_path_create(void);
OUI_SK_API OuiSkPath oui_sk_path_clone(OuiSkPath path);
OUI_SK_API void oui_sk_path_destroy(OuiSkPath path);

OUI_SK_API void oui_sk_path_move_to(OuiSkPath path, float x, float y);
OUI_SK_API void oui_sk_path_line_to(OuiSkPath path, float x, float y);
OUI_SK_API void oui_sk_path_quad_to(
    OuiSkPath path, float cx, float cy, float x, float y);
OUI_SK_API void oui_sk_path_cubic_to(
    OuiSkPath path, float c1x, float c1y, float c2x, float c2y,
    float x, float y);
OUI_SK_API void oui_sk_path_arc_to(
    OuiSkPath path, const OuiSkRect* oval,
    float start_angle, float sweep_angle, bool force_move_to);
OUI_SK_API void oui_sk_path_close(OuiSkPath path);
OUI_SK_API void oui_sk_path_reset(OuiSkPath path);

OUI_SK_API OuiSkPath oui_sk_path_from_svg_string(const char* svg);
OUI_SK_API OuiSkRect oui_sk_path_get_bounds(OuiSkPath path);
OUI_SK_API bool oui_sk_path_contains(OuiSkPath path, float x, float y);

/* ─── Font & Text ───────────────────────────────────────────────── */

OUI_SK_API OuiSkTypeface oui_sk_typeface_create_from_name(
    const char* family_name, OuiSkFontStylePreset style);
OUI_SK_API OuiSkTypeface oui_sk_typeface_create_from_file(
    const char* path, int index);
OUI_SK_API OuiSkTypeface oui_sk_typeface_create_from_data(
    const void* data, size_t size, int index);
OUI_SK_API void oui_sk_typeface_destroy(OuiSkTypeface typeface);

OUI_SK_API OuiSkFont oui_sk_font_create(OuiSkTypeface typeface, float size);
OUI_SK_API void oui_sk_font_destroy(OuiSkFont font);
OUI_SK_API void oui_sk_font_set_size(OuiSkFont font, float size);
OUI_SK_API float oui_sk_font_get_size(OuiSkFont font);
OUI_SK_API OuiSkFontMetrics oui_sk_font_get_metrics(OuiSkFont font);
OUI_SK_API float oui_sk_font_measure_text(
    OuiSkFont font, const char* text, size_t len);

OUI_SK_API OuiSkTextBlob oui_sk_text_blob_create(
    const char* text, size_t len, OuiSkFont font);
OUI_SK_API OuiSkTextBlob oui_sk_text_shape(
    const char* text, size_t len, OuiSkFont font,
    float width, OuiSkTextAlign align, OuiSkTextDirection dir);
OUI_SK_API void oui_sk_text_blob_destroy(OuiSkTextBlob blob);

/* ─── Image ─────────────────────────────────────────────────────── */

OUI_SK_API OuiSkImage oui_sk_image_decode(
    const void* data, size_t size);
OUI_SK_API OuiSkImage oui_sk_image_load_file(const char* path);
OUI_SK_API void oui_sk_image_destroy(OuiSkImage image);
OUI_SK_API int oui_sk_image_width(OuiSkImage image);
OUI_SK_API int oui_sk_image_height(OuiSkImage image);

OUI_SK_API OuiSkStatus oui_sk_image_encode(
    OuiSkImage image, OuiSkImageFormat format, int quality,
    void** out_data, size_t* out_size);
OUI_SK_API void oui_sk_image_encode_free(void* data);

/* ─── Shader ────────────────────────────────────────────────────── */

OUI_SK_API OuiSkShader oui_sk_shader_linear_gradient(
    OuiSkPoint start, OuiSkPoint end,
    const OuiSkColor* colors, const float* positions, int count,
    OuiSkTileMode tile_mode);
OUI_SK_API OuiSkShader oui_sk_shader_radial_gradient(
    OuiSkPoint center, float radius,
    const OuiSkColor* colors, const float* positions, int count,
    OuiSkTileMode tile_mode);
OUI_SK_API OuiSkShader oui_sk_shader_sweep_gradient(
    OuiSkPoint center,
    const OuiSkColor* colors, const float* positions, int count);
OUI_SK_API OuiSkShader oui_sk_shader_image(
    OuiSkImage image, OuiSkTileMode tile_x, OuiSkTileMode tile_y);
OUI_SK_API void oui_sk_shader_destroy(OuiSkShader shader);

/* ─── Image Filter ──────────────────────────────────────────────── */

OUI_SK_API OuiSkImageFilter oui_sk_image_filter_blur(
    float sigma_x, float sigma_y, OuiSkTileMode tile_mode);
OUI_SK_API OuiSkImageFilter oui_sk_image_filter_drop_shadow(
    float dx, float dy, float sigma_x, float sigma_y, OuiSkColor color);
OUI_SK_API OuiSkImageFilter oui_sk_image_filter_color_filter(
    OuiSkColorFilter color_filter);
OUI_SK_API OuiSkImageFilter oui_sk_image_filter_compose(
    OuiSkImageFilter outer, OuiSkImageFilter inner);
OUI_SK_API void oui_sk_image_filter_destroy(OuiSkImageFilter filter);

/* ─── Mask Filter ───────────────────────────────────────────────── */

OUI_SK_API OuiSkMaskFilter oui_sk_mask_filter_blur(
    OuiSkBlurStyle style, float sigma);
OUI_SK_API void oui_sk_mask_filter_destroy(OuiSkMaskFilter filter);

/* ─── Color Filter ──────────────────────────────────────────────── */

OUI_SK_API OuiSkColorFilter oui_sk_color_filter_blend(
    OuiSkColor color, OuiSkBlendMode mode);
OUI_SK_API OuiSkColorFilter oui_sk_color_filter_matrix(
    const float matrix[20]);
OUI_SK_API OuiSkColorFilter oui_sk_color_filter_compose(
    OuiSkColorFilter outer, OuiSkColorFilter inner);
OUI_SK_API void oui_sk_color_filter_destroy(OuiSkColorFilter filter);

/* ─── Window (SDL3, temporary — replaced in SP6) ────────────────── */

OUI_SK_API OuiWindow oui_window_create(
    const char* title, int width, int height, OuiSkBackend backend);
OUI_SK_API void oui_window_destroy(OuiWindow window);
OUI_SK_API OuiSkCanvas oui_window_get_canvas(OuiWindow window);
    /* Returns a borrowed pointer — do NOT free or destroy. Valid until window
       is destroyed or resized. */
OUI_SK_API void oui_window_present(OuiWindow window);
OUI_SK_API bool oui_window_poll_event(OuiWindow window, OuiEvent* event);
OUI_SK_API OuiSkSize oui_window_get_size(OuiWindow window);
OUI_SK_API float oui_window_get_dpi_scale(OuiWindow window);

/* ─── Utility ───────────────────────────────────────────────────── */

OUI_SK_API OuiSkColor oui_sk_color_make(uint8_t r, uint8_t g, uint8_t b, uint8_t a);
OUI_SK_API OuiSkColor4f oui_sk_color4f_make(float r, float g, float b, float a);
OUI_SK_API OuiSkRect oui_sk_rect_make(float l, float t, float r, float b);
OUI_SK_API OuiSkRect oui_sk_rect_make_xywh(float x, float y, float w, float h);
OUI_SK_API OuiSkRRect oui_sk_rrect_make(OuiSkRect rect, float rx, float ry);
OUI_SK_API OuiSkMatrix oui_sk_matrix_identity(void);
OUI_SK_API const char* oui_sk_status_string(OuiSkStatus status);

#ifdef __cplusplus
}
#endif

#endif /* OPENUI_SKIA_H_ */
