/* examples/skia_hello.c
 *
 * Open UI — Hello World example (pure C).
 *
 * Renders a scene with colored shapes, text, gradients, and shadows
 * to a PNG file using the Open UI Skia C API.
 *
 * Build:
 *   cc -std=c11 -I include -L out -lopenui_skia -o skia_hello examples/skia_hello.c
 *
 * Run:
 *   LD_LIBRARY_PATH=out ./skia_hello
 */

#include "openui/openui_skia.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void draw_scene(OuiSkCanvas canvas) {
    /* Background */
    oui_sk_canvas_clear(canvas, oui_sk_color_make(250, 250, 250, 255));

    OuiSkPaint paint = oui_sk_paint_create();

    /* ── Header bar with gradient ───────────────────────────── */
    OuiSkColor header_colors[2] = {
        oui_sk_color_make(66, 133, 244, 255),   /* Google Blue */
        oui_sk_color_make(25, 80, 180, 255),     /* Darker blue */
    };
    OuiSkShader header_grad = oui_sk_shader_linear_gradient(
        (OuiSkPoint){0, 0}, (OuiSkPoint){600, 0},
        header_colors, NULL, 2, OUI_SK_TILE_MODE_CLAMP);
    oui_sk_paint_set_shader(paint, header_grad);
    OuiSkRect header = oui_sk_rect_make_xywh(0, 0, 600, 80);
    oui_sk_canvas_draw_rect(canvas, &header, paint);
    oui_sk_paint_set_shader(paint, NULL);
    oui_sk_shader_destroy(header_grad);

    /* ── Title text ─────────────────────────────────────────── */
    OuiSkTypeface tf_bold = oui_sk_typeface_create_from_name(
        "sans-serif", OUI_SK_FONT_STYLE_BOLD);
    OuiSkFont font_title = oui_sk_font_create(tf_bold, 32);
    oui_sk_paint_set_color(paint, oui_sk_color_make(255, 255, 255, 255));
    oui_sk_canvas_draw_text(canvas, "Hello, Open UI!", 15,
                            24, 52, font_title, paint);

    /* ── Card with shadow ───────────────────────────────────── */
    OuiSkColor shadow_color = oui_sk_color_make(0, 0, 0, 60);
    OuiSkImageFilter card_shadow = oui_sk_image_filter_drop_shadow(
        0, 4, 8, 8, shadow_color);
    oui_sk_paint_set_color(paint, oui_sk_color_make(255, 255, 255, 255));
    oui_sk_paint_set_image_filter(paint, card_shadow);
    OuiSkRRect card = oui_sk_rrect_make(
        oui_sk_rect_make_xywh(30, 110, 540, 180), 12, 12);
    oui_sk_canvas_draw_rrect(canvas, &card, paint);
    oui_sk_paint_set_image_filter(paint, NULL);
    oui_sk_image_filter_destroy(card_shadow);

    /* ── Card content: shapes ───────────────────────────────── */
    /* Red circle */
    oui_sk_paint_set_color(paint, oui_sk_color_make(234, 67, 53, 255));
    oui_sk_canvas_draw_circle(canvas, 90, 180, 40, paint);

    /* Green rounded rect */
    oui_sk_paint_set_color(paint, oui_sk_color_make(52, 168, 83, 255));
    OuiSkRRect green_rr = oui_sk_rrect_make(
        oui_sk_rect_make_xywh(155, 145, 100, 70), 8, 8);
    oui_sk_canvas_draw_rrect(canvas, &green_rr, paint);

    /* Yellow star */
    OuiSkPath star = oui_sk_path_from_svg_string(
        "M 330 148 L 342 178 L 374 178 L 348 196 "
        "L 358 226 L 330 208 L 302 226 L 312 196 "
        "L 286 178 L 318 178 Z");
    if (star) {
        oui_sk_paint_set_color(paint, oui_sk_color_make(251, 188, 4, 255));
        oui_sk_canvas_draw_path(canvas, star, paint);
        oui_sk_path_destroy(star);
    }

    /* Purple oval with stroke */
    oui_sk_paint_set_color(paint, oui_sk_color_make(160, 60, 210, 200));
    OuiSkRect oval = oui_sk_rect_make_xywh(400, 145, 120, 70);
    oui_sk_canvas_draw_oval(canvas, &oval, paint);

    oui_sk_paint_set_style(paint, OUI_SK_PAINT_STYLE_STROKE);
    oui_sk_paint_set_stroke_width(paint, 3);
    oui_sk_paint_set_color(paint, oui_sk_color_make(100, 20, 160, 255));
    oui_sk_canvas_draw_oval(canvas, &oval, paint);
    oui_sk_paint_set_style(paint, OUI_SK_PAINT_STYLE_FILL);

    /* ── Card description text ──────────────────────────────── */
    OuiSkTypeface tf_normal = oui_sk_typeface_create_from_name(
        "sans-serif", OUI_SK_FONT_STYLE_NORMAL);
    OuiSkFont font_body = oui_sk_font_create(tf_normal, 16);
    oui_sk_paint_set_color(paint, oui_sk_color_make(80, 80, 80, 255));
    const char* desc = "Shapes rendered via the Skia C API — no C++ required.";
    oui_sk_canvas_draw_text(canvas, desc, strlen(desc),
                            60, 262, font_body, paint);

    /* ── Rainbow gradient bar ───────────────────────────────── */
    OuiSkColor rainbow[7] = {
        oui_sk_color_make(255,   0,   0, 255),
        oui_sk_color_make(255, 127,   0, 255),
        oui_sk_color_make(255, 255,   0, 255),
        oui_sk_color_make(  0, 255,   0, 255),
        oui_sk_color_make(  0,   0, 255, 255),
        oui_sk_color_make( 75,   0, 130, 255),
        oui_sk_color_make(148,   0, 211, 255),
    };
    OuiSkShader rainbow_grad = oui_sk_shader_linear_gradient(
        (OuiSkPoint){30, 0}, (OuiSkPoint){570, 0},
        rainbow, NULL, 7, OUI_SK_TILE_MODE_CLAMP);
    oui_sk_paint_set_shader(paint, rainbow_grad);
    OuiSkRRect bar = oui_sk_rrect_make(
        oui_sk_rect_make_xywh(30, 320, 540, 20), 10, 10);
    oui_sk_canvas_draw_rrect(canvas, &bar, paint);
    oui_sk_paint_set_shader(paint, NULL);
    oui_sk_shader_destroy(rainbow_grad);

    /* ── Footer text ────────────────────────────────────────── */
    OuiSkFont font_small = oui_sk_font_create(tf_normal, 13);
    oui_sk_paint_set_color(paint, oui_sk_color_make(150, 150, 150, 255));
    const char* footer = "Powered by Skia • libopenui_skia.so";
    oui_sk_canvas_draw_text(canvas, footer, strlen(footer),
                            30, 370, font_small, paint);

    /* ── Cleanup ────────────────────────────────────────────── */
    oui_sk_font_destroy(font_small);
    oui_sk_font_destroy(font_body);
    oui_sk_font_destroy(font_title);
    if (tf_normal) oui_sk_typeface_destroy(tf_normal);
    if (tf_bold) oui_sk_typeface_destroy(tf_bold);
    oui_sk_paint_destroy(paint);
}

int main(int argc, char** argv) {
    const char* output = "hello_openui.png";
    if (argc > 1) output = argv[1];

    printf("Open UI — Hello World\n");

    /* Create raster surface */
    OuiSkSurface surface = oui_sk_surface_create_raster(600, 400);
    if (!surface) {
        fprintf(stderr, "Failed to create surface\n");
        return 1;
    }

    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);
    if (!canvas) {
        fprintf(stderr, "Failed to get canvas\n");
        oui_sk_surface_destroy(surface);
        return 1;
    }

    /* Draw */
    draw_scene(canvas);

    /* Encode to PNG */
    OuiSkImage image = oui_sk_surface_make_image_snapshot(surface);
    void* png_data = NULL;
    size_t png_size = 0;
    OuiSkStatus status = oui_sk_image_encode(
        image, OUI_SK_IMAGE_FORMAT_PNG, 0, &png_data, &png_size);

    if (status != OUI_SK_OK) {
        fprintf(stderr, "Encode failed: %s\n", oui_sk_status_string(status));
        oui_sk_image_destroy(image);
        oui_sk_surface_destroy(surface);
        return 1;
    }

    /* Write to file */
    FILE* f = fopen(output, "wb");
    if (f) {
        fwrite(png_data, 1, png_size, f);
        fclose(f);
        printf("Wrote %zu bytes to %s\n", png_size, output);
    } else {
        fprintf(stderr, "Cannot write to %s\n", output);
    }

    /* Cleanup */
    oui_sk_image_encode_free(png_data);
    oui_sk_image_destroy(image);
    oui_sk_surface_destroy(surface);

    return 0;
}
