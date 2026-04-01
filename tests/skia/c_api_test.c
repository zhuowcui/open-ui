/* tests/skia/c_api_test.c
 *
 * Comprehensive C API test for libopenui_skia.
 * Pure C — no C++ headers. Tests all major API surface areas:
 * surface, canvas, paint, path, font, text, image encode/decode,
 * gradients, filters, and utility functions.
 *
 * Build: cc -o c_api_test c_api_test.c -L../../out -lopenui_skia
 */

#include "openui/openui_skia.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int g_pass = 0;
static int g_fail = 0;

#define TEST(name) printf("  %-50s ", name)
#define PASS() do { printf("PASS\n"); g_pass++; } while(0)
#define FAIL(msg) do { printf("FAIL: %s\n", msg); g_fail++; } while(0)
#define ASSERT(cond, msg) do { if (!(cond)) { FAIL(msg); return; } } while(0)

/* ─── Surface & Canvas ─────────────────────────────────────────── */

static void test_raster_surface(void) {
    TEST("Raster surface create/destroy");
    OuiSkSurface surface = oui_sk_surface_create_raster(400, 300);
    ASSERT(surface != NULL, "surface is null");
    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);
    ASSERT(canvas != NULL, "canvas is null");

    OuiSkColor white = oui_sk_color_make(255, 255, 255, 255);
    oui_sk_canvas_clear(canvas, white);

    /* Free canvas handle (not the underlying canvas) */
    oui_sk_surface_destroy(surface);
    PASS();
}

static void test_invalid_surface(void) {
    TEST("Invalid surface args return NULL");
    OuiSkSurface s1 = oui_sk_surface_create_raster(0, 100);
    OuiSkSurface s2 = oui_sk_surface_create_raster(100, -1);
    ASSERT(s1 == NULL && s2 == NULL, "should return null for invalid size");
    PASS();
}

/* ─── Canvas operations ────────────────────────────────────────── */

static void test_canvas_save_restore(void) {
    TEST("Canvas save/restore");
    OuiSkSurface surface = oui_sk_surface_create_raster(200, 200);
    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);

    int count1 = oui_sk_canvas_get_save_count(canvas);
    int saved = oui_sk_canvas_save(canvas);
    int count2 = oui_sk_canvas_get_save_count(canvas);
    ASSERT(count2 == count1 + 1, "save count should increment");

    oui_sk_canvas_translate(canvas, 10, 20);
    oui_sk_canvas_scale(canvas, 2, 2);
    oui_sk_canvas_rotate(canvas, 45);

    oui_sk_canvas_restore(canvas);
    int count3 = oui_sk_canvas_get_save_count(canvas);
    ASSERT(count3 == count1, "restore should decrement save count");

    oui_sk_surface_destroy(surface);
    PASS();
}

static void test_canvas_draw_shapes(void) {
    TEST("Canvas draw shapes");
    OuiSkSurface surface = oui_sk_surface_create_raster(400, 400);
    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);

    OuiSkColor bg = oui_sk_color_make(240, 240, 240, 255);
    oui_sk_canvas_clear(canvas, bg);

    OuiSkPaint paint = oui_sk_paint_create();
    oui_sk_paint_set_color(paint, oui_sk_color_make(66, 133, 244, 255));

    /* Rectangle */
    OuiSkRect rect = oui_sk_rect_make_xywh(10, 10, 100, 60);
    oui_sk_canvas_draw_rect(canvas, &rect, paint);

    /* Rounded rectangle */
    OuiSkRRect rrect = oui_sk_rrect_make(
        oui_sk_rect_make_xywh(120, 10, 100, 60), 10, 10);
    oui_sk_canvas_draw_rrect(canvas, &rrect, paint);

    /* Circle */
    oui_sk_paint_set_color(paint, oui_sk_color_make(234, 67, 53, 255));
    oui_sk_canvas_draw_circle(canvas, 60, 130, 40, paint);

    /* Oval */
    OuiSkRect oval_rect = oui_sk_rect_make_xywh(120, 90, 120, 80);
    oui_sk_canvas_draw_oval(canvas, &oval_rect, paint);

    /* Line */
    oui_sk_paint_set_style(paint, OUI_SK_PAINT_STYLE_STROKE);
    oui_sk_paint_set_stroke_width(paint, 3);
    oui_sk_canvas_draw_line(canvas, 10, 200, 200, 200, paint);

    /* Point */
    oui_sk_paint_set_stroke_width(paint, 10);
    oui_sk_paint_set_stroke_cap(paint, OUI_SK_STROKE_CAP_ROUND);
    oui_sk_canvas_draw_point(canvas, 250, 200, paint);

    oui_sk_paint_destroy(paint);
    oui_sk_surface_destroy(surface);
    PASS();
}

static void test_canvas_clip(void) {
    TEST("Canvas clip rect/path");
    OuiSkSurface surface = oui_sk_surface_create_raster(200, 200);
    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);

    OuiSkRect clip = oui_sk_rect_make(10, 10, 100, 100);
    oui_sk_canvas_clip_rect(canvas, &clip, true);

    OuiSkPath path = oui_sk_path_create();
    oui_sk_path_move_to(path, 50, 0);
    oui_sk_path_line_to(path, 100, 100);
    oui_sk_path_line_to(path, 0, 100);
    oui_sk_path_close(path);
    oui_sk_canvas_clip_path(canvas, path, true);

    oui_sk_path_destroy(path);
    oui_sk_surface_destroy(surface);
    PASS();
}

/* ─── Paint ────────────────────────────────────────────────────── */

static void test_paint(void) {
    TEST("Paint create/modify/clone/destroy");
    OuiSkPaint paint = oui_sk_paint_create();
    ASSERT(paint != NULL, "paint is null");

    OuiSkColor red = oui_sk_color_make(255, 0, 0, 255);
    oui_sk_paint_set_color(paint, red);
    OuiSkColor got = oui_sk_paint_get_color(paint);
    ASSERT(got.r == 255 && got.g == 0 && got.b == 0, "color mismatch");

    oui_sk_paint_set_alpha(paint, 128);
    oui_sk_paint_set_style(paint, OUI_SK_PAINT_STYLE_STROKE);
    oui_sk_paint_set_stroke_width(paint, 2.5f);
    oui_sk_paint_set_stroke_cap(paint, OUI_SK_STROKE_CAP_ROUND);
    oui_sk_paint_set_stroke_join(paint, OUI_SK_STROKE_JOIN_ROUND);
    oui_sk_paint_set_stroke_miter(paint, 4.0f);
    oui_sk_paint_set_anti_alias(paint, false);
    oui_sk_paint_set_blend_mode(paint, OUI_SK_BLEND_MODE_MULTIPLY);

    OuiSkPaint clone = oui_sk_paint_clone(paint);
    ASSERT(clone != NULL, "clone is null");
    OuiSkColor clone_color = oui_sk_paint_get_color(clone);
    ASSERT(clone_color.r == 255, "clone color wrong");

    oui_sk_paint_destroy(clone);
    oui_sk_paint_destroy(paint);
    PASS();
}

static void test_paint_null_safety(void) {
    TEST("Paint null safety");
    /* These should not crash */
    oui_sk_paint_set_color(NULL, oui_sk_color_make(0, 0, 0, 0));
    oui_sk_paint_destroy(NULL);
    OuiSkPaint clone = oui_sk_paint_clone(NULL);
    ASSERT(clone == NULL, "clone of null should be null");
    PASS();
}

/* ─── Path ──────────────────────────────────────────────────────── */

static void test_path_basic(void) {
    TEST("Path create/operations/destroy");
    OuiSkPath path = oui_sk_path_create();
    ASSERT(path != NULL, "path is null");

    oui_sk_path_move_to(path, 0, 0);
    oui_sk_path_line_to(path, 100, 0);
    oui_sk_path_line_to(path, 100, 100);
    oui_sk_path_close(path);

    OuiSkRect bounds = oui_sk_path_get_bounds(path);
    ASSERT(bounds.left == 0 && bounds.right == 100, "bounds wrong");

    bool contains = oui_sk_path_contains(path, 50, 50);
    ASSERT(contains, "path should contain (50,50)");

    bool outside = oui_sk_path_contains(path, -10, -10);
    ASSERT(!outside, "path should not contain (-10,-10)");

    oui_sk_path_destroy(path);
    PASS();
}

static void test_path_curves(void) {
    TEST("Path curves (quad, cubic, arc)");
    OuiSkPath path = oui_sk_path_create();
    oui_sk_path_move_to(path, 0, 100);
    oui_sk_path_quad_to(path, 50, 0, 100, 100);
    oui_sk_path_cubic_to(path, 120, 0, 180, 0, 200, 100);

    OuiSkRect oval = oui_sk_rect_make(220, 20, 320, 120);
    oui_sk_path_arc_to(path, &oval, 0, 270, true);

    oui_sk_path_close(path);
    oui_sk_path_destroy(path);
    PASS();
}

static void test_path_from_svg(void) {
    TEST("Path from SVG string");
    OuiSkPath path = oui_sk_path_from_svg_string("M 10 10 L 100 10 L 100 100 Z");
    ASSERT(path != NULL, "svg path is null");

    OuiSkRect bounds = oui_sk_path_get_bounds(path);
    ASSERT(bounds.left >= 9 && bounds.right <= 101, "svg path bounds wrong");

    oui_sk_path_destroy(path);

    OuiSkPath bad = oui_sk_path_from_svg_string(NULL);
    ASSERT(bad == NULL, "null svg should return null");

    PASS();
}

static void test_path_clone(void) {
    TEST("Path clone");
    OuiSkPath orig = oui_sk_path_create();
    oui_sk_path_move_to(orig, 0, 0);
    oui_sk_path_line_to(orig, 50, 50);

    OuiSkPath clone = oui_sk_path_clone(orig);
    ASSERT(clone != NULL, "clone is null");

    OuiSkRect b1 = oui_sk_path_get_bounds(orig);
    OuiSkRect b2 = oui_sk_path_get_bounds(clone);
    ASSERT(b1.right == b2.right, "clone bounds differ");

    oui_sk_path_destroy(orig);
    oui_sk_path_destroy(clone);
    PASS();
}

/* ─── Font & Text ───────────────────────────────────────────────── */

static void test_typeface_font(void) {
    TEST("Typeface and font create");
    OuiSkTypeface tf = oui_sk_typeface_create_from_name(
        "sans-serif", OUI_SK_FONT_STYLE_NORMAL);
    /* Font might be null if no fonts are installed — that's ok */

    OuiSkFont font = oui_sk_font_create(tf, 24.0f);
    ASSERT(font != NULL, "font is null");

    float size = oui_sk_font_get_size(font);
    ASSERT(size > 23.5f && size < 24.5f, "font size wrong");

    oui_sk_font_set_size(font, 16.0f);
    ASSERT(oui_sk_font_get_size(font) > 15.5f, "set_size failed");

    OuiSkFontMetrics metrics = oui_sk_font_get_metrics(font);
    /* Ascent is typically negative (above baseline) */
    (void)metrics;

    float width = oui_sk_font_measure_text(font, "Hello", 5);
    ASSERT(width > 0, "text width should be positive");

    oui_sk_font_destroy(font);
    if (tf) oui_sk_typeface_destroy(tf);
    PASS();
}

static void test_text_blob(void) {
    TEST("Text blob create and draw");
    OuiSkTypeface tf = oui_sk_typeface_create_from_name(
        "serif", OUI_SK_FONT_STYLE_NORMAL);
    OuiSkFont font = oui_sk_font_create(tf, 20.0f);

    OuiSkTextBlob blob = oui_sk_text_blob_create("Hello World", 11, font);
    ASSERT(blob != NULL, "text blob is null");

    /* Draw on a surface */
    OuiSkSurface surface = oui_sk_surface_create_raster(200, 50);
    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);
    OuiSkPaint paint = oui_sk_paint_create();
    oui_sk_paint_set_color(paint, oui_sk_color_make(0, 0, 0, 255));

    oui_sk_canvas_draw_text_blob(canvas, blob, 10, 30, paint);

    oui_sk_text_blob_destroy(blob);
    oui_sk_paint_destroy(paint);
    oui_sk_surface_destroy(surface);
    oui_sk_font_destroy(font);
    if (tf) oui_sk_typeface_destroy(tf);
    PASS();
}

/* ─── Image encode/decode ──────────────────────────────────────── */

static void test_image_encode_decode(void) {
    TEST("Image snapshot, encode PNG, decode");
    OuiSkSurface surface = oui_sk_surface_create_raster(100, 100);
    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);
    OuiSkColor blue = oui_sk_color_make(0, 0, 255, 255);
    oui_sk_canvas_clear(canvas, blue);

    OuiSkImage img = oui_sk_surface_make_image_snapshot(surface);
    ASSERT(img != NULL, "image snapshot null");
    ASSERT(oui_sk_image_width(img) == 100, "width wrong");
    ASSERT(oui_sk_image_height(img) == 100, "height wrong");

    /* Encode to PNG */
    void* png_data = NULL;
    size_t png_size = 0;
    OuiSkStatus status = oui_sk_image_encode(
        img, OUI_SK_IMAGE_FORMAT_PNG, 0, &png_data, &png_size);
    ASSERT(status == OUI_SK_OK, oui_sk_status_string(status));
    ASSERT(png_data != NULL && png_size > 0, "png encode empty");

    /* Decode the PNG back */
    OuiSkImage decoded = oui_sk_image_decode(png_data, png_size);
    ASSERT(decoded != NULL, "decode failed");
    ASSERT(oui_sk_image_width(decoded) == 100, "decoded width wrong");

    oui_sk_image_destroy(decoded);
    oui_sk_image_encode_free(png_data);
    oui_sk_image_destroy(img);
    oui_sk_surface_destroy(surface);
    PASS();
}

/* ─── Gradients ────────────────────────────────────────────────── */

static void test_linear_gradient(void) {
    TEST("Linear gradient shader");
    OuiSkColor colors[2] = {
        oui_sk_color_make(255, 0, 0, 255),
        oui_sk_color_make(0, 0, 255, 255),
    };
    float positions[2] = {0.0f, 1.0f};
    OuiSkPoint start = {0, 0};
    OuiSkPoint end = {200, 0};

    OuiSkShader shader = oui_sk_shader_linear_gradient(
        start, end, colors, positions, 2, OUI_SK_TILE_MODE_CLAMP);
    ASSERT(shader != NULL, "gradient shader null");

    OuiSkPaint paint = oui_sk_paint_create();
    oui_sk_paint_set_shader(paint, shader);

    OuiSkSurface surface = oui_sk_surface_create_raster(200, 50);
    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);
    OuiSkRect rect = oui_sk_rect_make_xywh(0, 0, 200, 50);
    oui_sk_canvas_draw_rect(canvas, &rect, paint);

    /* Remove shader */
    oui_sk_paint_set_shader(paint, NULL);

    oui_sk_shader_destroy(shader);
    oui_sk_paint_destroy(paint);
    oui_sk_surface_destroy(surface);
    PASS();
}

static void test_radial_gradient(void) {
    TEST("Radial gradient shader");
    OuiSkColor colors[3] = {
        oui_sk_color_make(255, 255, 0, 255),
        oui_sk_color_make(0, 255, 0, 255),
        oui_sk_color_make(0, 0, 255, 255),
    };
    OuiSkPoint center = {100, 100};
    OuiSkShader shader = oui_sk_shader_radial_gradient(
        center, 100, colors, NULL, 3, OUI_SK_TILE_MODE_CLAMP);
    ASSERT(shader != NULL, "radial gradient null");
    oui_sk_shader_destroy(shader);
    PASS();
}

static void test_sweep_gradient(void) {
    TEST("Sweep gradient shader");
    OuiSkColor colors[2] = {
        oui_sk_color_make(255, 0, 0, 255),
        oui_sk_color_make(0, 255, 0, 255),
    };
    OuiSkPoint center = {100, 100};
    OuiSkShader shader = oui_sk_shader_sweep_gradient(
        center, colors, NULL, 2);
    ASSERT(shader != NULL, "sweep gradient null");
    oui_sk_shader_destroy(shader);
    PASS();
}

/* ─── Filters ──────────────────────────────────────────────────── */

static void test_image_filter_blur(void) {
    TEST("Image filter blur");
    OuiSkImageFilter filter = oui_sk_image_filter_blur(
        5.0f, 5.0f, OUI_SK_TILE_MODE_CLAMP);
    ASSERT(filter != NULL, "blur filter null");

    OuiSkPaint paint = oui_sk_paint_create();
    oui_sk_paint_set_image_filter(paint, filter);
    oui_sk_paint_set_image_filter(paint, NULL);

    oui_sk_image_filter_destroy(filter);
    oui_sk_paint_destroy(paint);
    PASS();
}

static void test_image_filter_drop_shadow(void) {
    TEST("Image filter drop shadow");
    OuiSkColor black = oui_sk_color_make(0, 0, 0, 128);
    OuiSkImageFilter filter = oui_sk_image_filter_drop_shadow(
        3, 3, 4, 4, black);
    ASSERT(filter != NULL, "shadow filter null");
    oui_sk_image_filter_destroy(filter);
    PASS();
}

static void test_filter_compose(void) {
    TEST("Image filter compose");
    OuiSkImageFilter blur = oui_sk_image_filter_blur(
        2.0f, 2.0f, OUI_SK_TILE_MODE_CLAMP);
    OuiSkColor black = oui_sk_color_make(0, 0, 0, 128);
    OuiSkImageFilter shadow = oui_sk_image_filter_drop_shadow(
        2, 2, 3, 3, black);
    OuiSkImageFilter composed = oui_sk_image_filter_compose(shadow, blur);
    ASSERT(composed != NULL, "composed filter null");

    oui_sk_image_filter_destroy(composed);
    oui_sk_image_filter_destroy(shadow);
    oui_sk_image_filter_destroy(blur);
    PASS();
}

static void test_mask_filter(void) {
    TEST("Mask filter blur");
    OuiSkMaskFilter mf = oui_sk_mask_filter_blur(
        OUI_SK_BLUR_STYLE_NORMAL, 3.0f);
    ASSERT(mf != NULL, "mask filter null");

    OuiSkPaint paint = oui_sk_paint_create();
    oui_sk_paint_set_mask_filter(paint, mf);

    oui_sk_mask_filter_destroy(mf);
    oui_sk_paint_destroy(paint);
    PASS();
}

static void test_color_filter(void) {
    TEST("Color filter blend + matrix + compose");
    OuiSkColorFilter blend = oui_sk_color_filter_blend(
        oui_sk_color_make(255, 0, 0, 128), OUI_SK_BLEND_MODE_SRC_OVER);
    ASSERT(blend != NULL, "blend filter null");

    /* Grayscale matrix */
    float matrix[20] = {
        0.2126f, 0.7152f, 0.0722f, 0, 0,
        0.2126f, 0.7152f, 0.0722f, 0, 0,
        0.2126f, 0.7152f, 0.0722f, 0, 0,
        0,       0,       0,       1, 0,
    };
    OuiSkColorFilter mat = oui_sk_color_filter_matrix(matrix);
    ASSERT(mat != NULL, "matrix filter null");

    OuiSkColorFilter composed = oui_sk_color_filter_compose(blend, mat);
    ASSERT(composed != NULL, "composed color filter null");

    oui_sk_color_filter_destroy(composed);
    oui_sk_color_filter_destroy(mat);
    oui_sk_color_filter_destroy(blend);
    PASS();
}

/* ─── Utility functions ────────────────────────────────────────── */

static void test_utilities(void) {
    TEST("Utility functions");

    OuiSkColor c = oui_sk_color_make(10, 20, 30, 255);
    ASSERT(c.r == 10 && c.g == 20 && c.b == 30 && c.a == 255, "color_make");

    OuiSkColor4f c4 = oui_sk_color4f_make(0.5f, 0.25f, 0.75f, 1.0f);
    ASSERT(c4.r > 0.49f && c4.r < 0.51f, "color4f_make");

    OuiSkRect r = oui_sk_rect_make(1, 2, 3, 4);
    ASSERT(r.left == 1 && r.top == 2 && r.right == 3 && r.bottom == 4, "rect_make");

    OuiSkRect r2 = oui_sk_rect_make_xywh(10, 20, 30, 40);
    ASSERT(r2.left == 10 && r2.right == 40, "rect_make_xywh");

    OuiSkRRect rr = oui_sk_rrect_make(r2, 5, 5);
    ASSERT(rr.rx == 5, "rrect_make");

    OuiSkMatrix m = oui_sk_matrix_identity();
    ASSERT(m.values[0] == 1.0f && m.values[4] == 1.0f &&
           m.values[8] == 1.0f && m.values[1] == 0.0f, "matrix_identity");

    const char* ok_str = oui_sk_status_string(OUI_SK_OK);
    ASSERT(ok_str != NULL && strcmp(ok_str, "OK") == 0, "status_string OK");

    const char* err_str = oui_sk_status_string(OUI_SK_ERROR_NULL_POINTER);
    ASSERT(err_str != NULL, "status_string error");

    PASS();
}

/* ─── Matrix transform ─────────────────────────────────────────── */

static void test_canvas_transform(void) {
    TEST("Canvas matrix transform");
    OuiSkSurface surface = oui_sk_surface_create_raster(200, 200);
    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);

    OuiSkMatrix m = oui_sk_matrix_identity();
    m.values[2] = 50;  /* translate x by 50 */
    m.values[5] = 50;  /* translate y by 50 */
    oui_sk_canvas_concat_matrix(canvas, &m);

    oui_sk_canvas_skew(canvas, 0.1f, 0.2f);

    oui_sk_surface_destroy(surface);
    PASS();
}

/* ─── Read pixels ──────────────────────────────────────────────── */

static void test_read_pixels(void) {
    TEST("Surface read pixels");
    OuiSkSurface surface = oui_sk_surface_create_raster(10, 10);
    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);

    OuiSkColor red = oui_sk_color_make(255, 0, 0, 255);
    oui_sk_canvas_clear(canvas, red);

    uint8_t pixels[10 * 10 * 4];
    OuiSkStatus status = oui_sk_surface_read_pixels(
        surface, pixels, 10 * 4, 0, 0, 10, 10);
    ASSERT(status == OUI_SK_OK, "read_pixels failed");

    /* Pixel at (0,0) should be red.
     * N32 format on most platforms is BGRA, so:
     * pixels[0]=B, pixels[1]=G, pixels[2]=R, pixels[3]=A */
    ASSERT(pixels[2] == 255, "red channel wrong");
    ASSERT(pixels[1] == 0, "green channel wrong");
    ASSERT(pixels[0] == 0, "blue channel wrong");

    oui_sk_surface_destroy(surface);
    PASS();
}

/* ─── Full rendering pipeline ──────────────────────────────────── */

static void test_full_render(void) {
    TEST("Full render pipeline -> PNG");
    OuiSkSurface surface = oui_sk_surface_create_raster(400, 300);
    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);

    /* Background */
    oui_sk_canvas_clear(canvas, oui_sk_color_make(245, 245, 245, 255));

    /* Blue rounded rect */
    OuiSkPaint paint = oui_sk_paint_create();
    oui_sk_paint_set_color(paint, oui_sk_color_make(66, 133, 244, 255));
    OuiSkRRect rrect = oui_sk_rrect_make(
        oui_sk_rect_make_xywh(20, 20, 160, 100), 12, 12);
    oui_sk_canvas_draw_rrect(canvas, &rrect, paint);

    /* Red circle with drop shadow */
    OuiSkColor shadow_color = oui_sk_color_make(0, 0, 0, 100);
    OuiSkImageFilter shadow = oui_sk_image_filter_drop_shadow(
        3, 3, 5, 5, shadow_color);
    oui_sk_paint_set_color(paint, oui_sk_color_make(234, 67, 53, 255));
    oui_sk_paint_set_image_filter(paint, shadow);
    oui_sk_canvas_draw_circle(canvas, 300, 70, 50, paint);
    oui_sk_paint_set_image_filter(paint, NULL);
    oui_sk_image_filter_destroy(shadow);

    /* Gradient bar */
    OuiSkColor grad_colors[3] = {
        oui_sk_color_make(66, 133, 244, 255),
        oui_sk_color_make(52, 168, 83, 255),
        oui_sk_color_make(234, 67, 53, 255),
    };
    OuiSkShader grad = oui_sk_shader_linear_gradient(
        (OuiSkPoint){20, 0}, (OuiSkPoint){380, 0},
        grad_colors, NULL, 3, OUI_SK_TILE_MODE_CLAMP);
    oui_sk_paint_set_shader(paint, grad);
    OuiSkRect grad_rect = oui_sk_rect_make_xywh(20, 150, 360, 30);
    oui_sk_canvas_draw_rect(canvas, &grad_rect, paint);
    oui_sk_paint_set_shader(paint, NULL);
    oui_sk_shader_destroy(grad);

    /* Path (star) */
    OuiSkPath star = oui_sk_path_from_svg_string(
        "M 200 200 L 220 260 L 280 260 L 230 290 "
        "L 250 350 L 200 310 L 150 350 L 170 290 "
        "L 120 260 L 180 260 Z");
    if (star) {
        oui_sk_paint_set_color(paint, oui_sk_color_make(251, 188, 4, 255));
        oui_sk_canvas_draw_path(canvas, star, paint);
        oui_sk_path_destroy(star);
    }

    /* Text */
    OuiSkTypeface tf = oui_sk_typeface_create_from_name(
        "sans-serif", OUI_SK_FONT_STYLE_BOLD);
    OuiSkFont font = oui_sk_font_create(tf, 28);
    oui_sk_paint_set_color(paint, oui_sk_color_make(32, 32, 32, 255));
    oui_sk_canvas_draw_text(canvas, "Open UI", 7, 30, 280, font, paint);

    /* Encode to PNG */
    OuiSkImage img = oui_sk_surface_make_image_snapshot(surface);
    void* png_data = NULL;
    size_t png_size = 0;
    OuiSkStatus status = oui_sk_image_encode(
        img, OUI_SK_IMAGE_FORMAT_PNG, 0, &png_data, &png_size);
    ASSERT(status == OUI_SK_OK, "encode failed");
    ASSERT(png_size > 100, "png too small");

    oui_sk_image_encode_free(png_data);
    oui_sk_image_destroy(img);
    oui_sk_font_destroy(font);
    if (tf) oui_sk_typeface_destroy(tf);
    oui_sk_paint_destroy(paint);
    oui_sk_surface_destroy(surface);
    PASS();
}

/* ─── Main ──────────────────────────────────────────────────────── */

int main(void) {
    printf("=== Open UI Skia C API Test Suite ===\n\n");

    printf("[Surface & Canvas]\n");
    test_raster_surface();
    test_invalid_surface();
    test_canvas_save_restore();
    test_canvas_draw_shapes();
    test_canvas_clip();
    test_canvas_transform();
    test_read_pixels();

    printf("\n[Paint]\n");
    test_paint();
    test_paint_null_safety();

    printf("\n[Path]\n");
    test_path_basic();
    test_path_curves();
    test_path_from_svg();
    test_path_clone();

    printf("\n[Font & Text]\n");
    test_typeface_font();
    test_text_blob();

    printf("\n[Image]\n");
    test_image_encode_decode();

    printf("\n[Gradient Shaders]\n");
    test_linear_gradient();
    test_radial_gradient();
    test_sweep_gradient();

    printf("\n[Filters]\n");
    test_image_filter_blur();
    test_image_filter_drop_shadow();
    test_filter_compose();
    test_mask_filter();
    test_color_filter();

    printf("\n[Utilities]\n");
    test_utilities();

    printf("\n[Integration]\n");
    test_full_render();

    printf("\n=== Results: %d passed, %d failed ===\n", g_pass, g_fail);
    return g_fail > 0 ? 1 : 0;
}
