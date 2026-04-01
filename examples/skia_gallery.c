/*
 * Open UI — Skia Rendering Gallery
 *
 * Demonstrates all major features of the Open UI Skia C API by rendering
 * labeled sections onto a single large PNG image.
 *
 * Usage: skia_gallery [output.png]
 */

#include "openui/openui_skia.h"
#include <stdio.h>
#include <string.h>
#include <math.h>

#define WIDTH  1200
#define HEIGHT 1000

/* ─── Helpers ───────────────────────────────────────────────────── */

static OuiSkColor rgb(uint8_t r, uint8_t g, uint8_t b) {
    return oui_sk_color_make(r, g, b, 255);
}

static OuiSkColor rgba(uint8_t r, uint8_t g, uint8_t b, uint8_t a) {
    return oui_sk_color_make(r, g, b, a);
}

static OuiSkRect xywh(float x, float y, float w, float h) {
    return oui_sk_rect_make_xywh(x, y, w, h);
}

static void draw_section_label(OuiSkCanvas canvas, OuiSkFont font,
                                OuiSkPaint paint, float x, float y,
                                const char* text) {
    oui_sk_paint_set_color(paint, rgb(30, 30, 30));
    oui_sk_canvas_draw_text(canvas, text, strlen(text), x, y, font, paint);
}

/* ─── Section 1: Basic Shapes ───────────────────────────────────── */

static void draw_basic_shapes(OuiSkCanvas canvas, OuiSkFont label_font,
                               OuiSkPaint label_paint, float ox, float oy) {
    draw_section_label(canvas, label_font, label_paint, ox, oy - 10,
                       "1. Basic Shapes");

    OuiSkPaint p = oui_sk_paint_create();
    oui_sk_paint_set_anti_alias(p, true);

    /* Filled rectangle */
    oui_sk_paint_set_color(p, rgb(66, 133, 244));
    OuiSkRect r1 = xywh(ox, oy, 80, 60);
    oui_sk_canvas_draw_rect(canvas, &r1, p);

    /* Stroked rectangle */
    oui_sk_paint_set_style(p, OUI_SK_PAINT_STYLE_STROKE);
    oui_sk_paint_set_stroke_width(p, 3.0f);
    oui_sk_paint_set_color(p, rgb(219, 68, 55));
    OuiSkRect r2 = xywh(ox + 100, oy, 80, 60);
    oui_sk_canvas_draw_rect(canvas, &r2, p);

    /* Filled circle */
    oui_sk_paint_set_style(p, OUI_SK_PAINT_STYLE_FILL);
    oui_sk_paint_set_color(p, rgb(244, 180, 0));
    oui_sk_canvas_draw_circle(canvas, ox + 250, oy + 30, 30, p);

    /* Rounded rect */
    oui_sk_paint_set_color(p, rgb(15, 157, 88));
    OuiSkRect r3 = xywh(ox + 300, oy, 80, 60);
    OuiSkRRect rr = oui_sk_rrect_make(r3, 12, 12);
    oui_sk_canvas_draw_rrect(canvas, &rr, p);

    /* Oval */
    oui_sk_paint_set_color(p, rgb(171, 71, 188));
    OuiSkRect r4 = xywh(ox + 400, oy, 100, 60);
    oui_sk_canvas_draw_oval(canvas, &r4, p);

    /* Line */
    oui_sk_paint_set_style(p, OUI_SK_PAINT_STYLE_STROKE);
    oui_sk_paint_set_stroke_width(p, 4.0f);
    oui_sk_paint_set_stroke_cap(p, OUI_SK_STROKE_CAP_ROUND);
    oui_sk_paint_set_color(p, rgb(0, 172, 193));
    oui_sk_canvas_draw_line(canvas, ox + 520, oy, ox + 570, oy + 60, p);

    oui_sk_paint_destroy(p);
}

/* ─── Section 2: Path Operations ────────────────────────────────── */

static void draw_paths(OuiSkCanvas canvas, OuiSkFont label_font,
                        OuiSkPaint label_paint, float ox, float oy) {
    draw_section_label(canvas, label_font, label_paint, ox, oy - 10,
                       "2. Path Operations");

    OuiSkPaint p = oui_sk_paint_create();
    oui_sk_paint_set_anti_alias(p, true);

    /* Star via manual path */
    OuiSkPath star = oui_sk_path_create();
    float cx = ox + 40, cy = oy + 30;
    for (int i = 0; i < 5; i++) {
        float angle = (float)(i * 4 * 3.14159265 / 5.0 - 3.14159265 / 2.0);
        float x = cx + 30 * cosf(angle);
        float y = cy + 30 * sinf(angle);
        if (i == 0) oui_sk_path_move_to(star, x, y);
        else        oui_sk_path_line_to(star, x, y);
    }
    oui_sk_path_close(star);
    oui_sk_paint_set_color(p, rgb(255, 87, 34));
    oui_sk_canvas_draw_path(canvas, star, p);
    oui_sk_path_destroy(star);

    /* Bezier curve */
    OuiSkPath bezier = oui_sk_path_create();
    oui_sk_path_move_to(bezier, ox + 100, oy + 60);
    oui_sk_path_cubic_to(bezier,
        ox + 120, oy - 20,
        ox + 200, oy + 80,
        ox + 220, oy);
    oui_sk_paint_set_style(p, OUI_SK_PAINT_STYLE_STROKE);
    oui_sk_paint_set_stroke_width(p, 3.0f);
    oui_sk_paint_set_color(p, rgb(33, 150, 243));
    oui_sk_canvas_draw_path(canvas, bezier, p);
    oui_sk_path_destroy(bezier);

    /* SVG path */
    OuiSkPath svg = oui_sk_path_from_svg_string(
        "M 260 10 Q 280 0 300 10 Q 320 20 300 30 Q 280 40 260 30 Z");
    if (svg) {
        oui_sk_paint_set_style(p, OUI_SK_PAINT_STYLE_FILL);
        oui_sk_paint_set_color(p, rgb(156, 39, 176));
        oui_sk_canvas_save(canvas);
        oui_sk_canvas_translate(canvas, ox - 260, oy);
        oui_sk_canvas_draw_path(canvas, svg, p);
        oui_sk_canvas_restore(canvas);
        oui_sk_path_destroy(svg);
    }

    /* Arc */
    OuiSkPath arc = oui_sk_path_create();
    OuiSkRect oval = xywh(ox + 360, oy, 60, 60);
    oui_sk_path_arc_to(arc, &oval, 0, 270, true);
    oui_sk_paint_set_style(p, OUI_SK_PAINT_STYLE_STROKE);
    oui_sk_paint_set_stroke_width(p, 4.0f);
    oui_sk_paint_set_color(p, rgb(255, 152, 0));
    oui_sk_canvas_draw_path(canvas, arc, p);
    oui_sk_path_destroy(arc);

    /* Quad bezier */
    OuiSkPath quad = oui_sk_path_create();
    oui_sk_path_move_to(quad, ox + 450, oy + 60);
    oui_sk_path_quad_to(quad, ox + 490, oy - 10, ox + 530, oy + 60);
    oui_sk_paint_set_color(p, rgb(0, 150, 136));
    oui_sk_canvas_draw_path(canvas, quad, p);
    oui_sk_path_destroy(quad);

    oui_sk_paint_destroy(p);
}

/* ─── Section 3: Paint Styles ───────────────────────────────────── */

static void draw_paint_styles(OuiSkCanvas canvas, OuiSkFont label_font,
                               OuiSkPaint label_paint, float ox, float oy) {
    draw_section_label(canvas, label_font, label_paint, ox, oy - 10,
                       "3. Paint Styles (Fill / Stroke / Both)");

    OuiSkPaint p = oui_sk_paint_create();
    oui_sk_paint_set_anti_alias(p, true);
    OuiSkRect r;

    /* Fill */
    oui_sk_paint_set_style(p, OUI_SK_PAINT_STYLE_FILL);
    oui_sk_paint_set_color(p, rgb(66, 133, 244));
    r = xywh(ox, oy, 80, 50);
    oui_sk_canvas_draw_rect(canvas, &r, p);

    /* Stroke with various widths */
    oui_sk_paint_set_style(p, OUI_SK_PAINT_STYLE_STROKE);
    oui_sk_paint_set_color(p, rgb(219, 68, 55));

    float widths[] = {1.0f, 3.0f, 6.0f};
    for (int i = 0; i < 3; i++) {
        oui_sk_paint_set_stroke_width(p, widths[i]);
        r = xywh(ox + 100 + i * 80, oy, 60, 50);
        oui_sk_canvas_draw_rect(canvas, &r, p);
    }

    /* Fill+Stroke */
    oui_sk_paint_set_style(p, OUI_SK_PAINT_STYLE_FILL_AND_STROKE);
    oui_sk_paint_set_stroke_width(p, 3.0f);
    oui_sk_paint_set_color(p, rgba(15, 157, 88, 180));
    r = xywh(ox + 360, oy, 80, 50);
    oui_sk_canvas_draw_rect(canvas, &r, p);

    /* Stroke caps: butt, round, square */
    oui_sk_paint_set_style(p, OUI_SK_PAINT_STYLE_STROKE);
    oui_sk_paint_set_stroke_width(p, 8.0f);
    oui_sk_paint_set_color(p, rgb(121, 85, 72));
    OuiSkStrokeCap caps[] = {OUI_SK_STROKE_CAP_BUTT,
                              OUI_SK_STROKE_CAP_ROUND,
                              OUI_SK_STROKE_CAP_SQUARE};
    for (int i = 0; i < 3; i++) {
        oui_sk_paint_set_stroke_cap(p, caps[i]);
        oui_sk_canvas_draw_line(canvas,
            ox + 470 + i * 40, oy + 5,
            ox + 470 + i * 40, oy + 45, p);
    }

    oui_sk_paint_destroy(p);
}

/* ─── Section 4: Gradients ──────────────────────────────────────── */

static void draw_gradients(OuiSkCanvas canvas, OuiSkFont label_font,
                            OuiSkPaint label_paint, float ox, float oy) {
    draw_section_label(canvas, label_font, label_paint, ox, oy - 10,
                       "4. Gradients (Linear / Radial / Sweep)");

    OuiSkPaint p = oui_sk_paint_create();
    oui_sk_paint_set_anti_alias(p, true);

    OuiSkColor colors3[] = {rgb(66,133,244), rgb(244,180,0), rgb(219,68,55)};
    float pos3[] = {0.0f, 0.5f, 1.0f};

    /* Linear */
    OuiSkPoint s = {ox, oy}, e = {ox + 160, oy + 60};
    OuiSkShader lin = oui_sk_shader_linear_gradient(
        s, e, colors3, pos3, 3, OUI_SK_TILE_MODE_CLAMP);
    oui_sk_paint_set_shader(p, lin);
    OuiSkRect r = xywh(ox, oy, 160, 60);
    oui_sk_canvas_draw_rect(canvas, &r, p);
    oui_sk_shader_destroy(lin);

    /* Radial */
    OuiSkPoint center = {ox + 240, oy + 30};
    OuiSkShader rad = oui_sk_shader_radial_gradient(
        center, 40, colors3, pos3, 3, OUI_SK_TILE_MODE_CLAMP);
    oui_sk_paint_set_shader(p, rad);
    oui_sk_canvas_draw_circle(canvas, ox + 240, oy + 30, 40, p);
    oui_sk_shader_destroy(rad);

    /* Sweep */
    OuiSkColor colors4[] = {rgb(66,133,244), rgb(15,157,88),
                             rgb(244,180,0), rgb(219,68,55)};
    float pos4[] = {0.0f, 0.33f, 0.66f, 1.0f};
    OuiSkPoint sc = {ox + 370, oy + 30};
    OuiSkShader sw = oui_sk_shader_sweep_gradient(
        sc, colors4, pos4, 4);
    oui_sk_paint_set_shader(p, sw);
    oui_sk_canvas_draw_circle(canvas, ox + 370, oy + 30, 40, p);
    oui_sk_shader_destroy(sw);

    oui_sk_paint_set_shader(p, NULL);
    oui_sk_paint_destroy(p);
}

/* ─── Section 5: Text Rendering ─────────────────────────────────── */

static void draw_text(OuiSkCanvas canvas, OuiSkFont label_font,
                       OuiSkPaint label_paint, float ox, float oy) {
    draw_section_label(canvas, label_font, label_paint, ox, oy - 10,
                       "5. Text Rendering");

    OuiSkTypeface tf = oui_sk_typeface_create_from_name(
        "sans-serif", OUI_SK_FONT_STYLE_NORMAL);
    OuiSkPaint p = oui_sk_paint_create();
    oui_sk_paint_set_anti_alias(p, true);

    float sizes[] = {12.0f, 18.0f, 24.0f, 36.0f};
    float y = oy;
    for (int i = 0; i < 4; i++) {
        OuiSkFont f = oui_sk_font_create(tf, sizes[i]);
        if (!f) continue;
        char buf[64];
        snprintf(buf, sizeof(buf), "Size %.0f", sizes[i]);
        oui_sk_paint_set_color(p, rgb(33, 33, 33));
        oui_sk_canvas_draw_text(canvas, buf, strlen(buf),
                                 ox, y + sizes[i], f, p);
        y += sizes[i] + 8;
        oui_sk_font_destroy(f);
    }

    /* Bold text */
    OuiSkTypeface tf_bold = oui_sk_typeface_create_from_name(
        "sans-serif", OUI_SK_FONT_STYLE_BOLD);
    if (tf_bold) {
        OuiSkFont fb = oui_sk_font_create(tf_bold, 20.0f);
        if (fb) {
            oui_sk_paint_set_color(p, rgb(0, 100, 200));
            const char* bold_text = "Bold Text";
            oui_sk_canvas_draw_text(canvas, bold_text, strlen(bold_text),
                                     ox + 200, oy + 24, fb, p);
            oui_sk_font_destroy(fb);
        }
        oui_sk_typeface_destroy(tf_bold);
    }

    /* Italic text */
    OuiSkTypeface tf_italic = oui_sk_typeface_create_from_name(
        "sans-serif", OUI_SK_FONT_STYLE_ITALIC);
    if (tf_italic) {
        OuiSkFont fi = oui_sk_font_create(tf_italic, 20.0f);
        if (fi) {
            oui_sk_paint_set_color(p, rgb(200, 50, 50));
            const char* italic_text = "Italic Text";
            oui_sk_canvas_draw_text(canvas, italic_text, strlen(italic_text),
                                     ox + 200, oy + 52, fi, p);
            oui_sk_font_destroy(fi);
        }
        oui_sk_typeface_destroy(tf_italic);
    }

    oui_sk_typeface_destroy(tf);
    oui_sk_paint_destroy(p);
}

/* ─── Section 6: Filters ────────────────────────────────────────── */

static void draw_filters(OuiSkCanvas canvas, OuiSkFont label_font,
                          OuiSkPaint label_paint, float ox, float oy) {
    draw_section_label(canvas, label_font, label_paint, ox, oy - 10,
                       "6. Filters (Blur / Shadow / Color Matrix)");

    OuiSkPaint p = oui_sk_paint_create();
    oui_sk_paint_set_anti_alias(p, true);
    OuiSkRect r;

    /* Blur filter */
    OuiSkImageFilter blur = oui_sk_image_filter_blur(
        4.0f, 4.0f, OUI_SK_TILE_MODE_CLAMP);
    oui_sk_paint_set_color(p, rgb(66, 133, 244));
    oui_sk_paint_set_image_filter(p, blur);
    r = xywh(ox, oy, 80, 50);
    oui_sk_canvas_draw_rect(canvas, &r, p);
    oui_sk_paint_set_image_filter(p, NULL);
    oui_sk_image_filter_destroy(blur);

    /* Drop shadow */
    OuiSkImageFilter shadow = oui_sk_image_filter_drop_shadow(
        4.0f, 4.0f, 3.0f, 3.0f, rgba(0, 0, 0, 128));
    oui_sk_paint_set_color(p, rgb(15, 157, 88));
    oui_sk_paint_set_image_filter(p, shadow);
    r = xywh(ox + 120, oy, 80, 50);
    oui_sk_canvas_draw_rect(canvas, &r, p);
    oui_sk_paint_set_image_filter(p, NULL);
    oui_sk_image_filter_destroy(shadow);

    /* Mask filter (blurred circle) */
    OuiSkMaskFilter mask = oui_sk_mask_filter_blur(
        OUI_SK_BLUR_STYLE_NORMAL, 3.0f);
    oui_sk_paint_set_color(p, rgb(244, 180, 0));
    oui_sk_paint_set_mask_filter(p, mask);
    oui_sk_canvas_draw_circle(canvas, ox + 280, oy + 25, 25, p);
    oui_sk_paint_set_mask_filter(p, NULL);
    oui_sk_mask_filter_destroy(mask);

    /* Color filter (sepia-ish matrix) */
    float sepia[20] = {
        0.393f, 0.769f, 0.189f, 0, 0,
        0.349f, 0.686f, 0.168f, 0, 0,
        0.272f, 0.534f, 0.131f, 0, 0,
        0,      0,      0,      1, 0,
    };
    OuiSkColorFilter cf = oui_sk_color_filter_matrix(sepia);
    oui_sk_paint_set_color(p, rgb(100, 149, 237));
    oui_sk_paint_set_color_filter(p, cf);
    r = xywh(ox + 340, oy, 80, 50);
    oui_sk_canvas_draw_rect(canvas, &r, p);
    oui_sk_paint_set_color_filter(p, NULL);
    oui_sk_color_filter_destroy(cf);

    oui_sk_paint_destroy(p);
}

/* ─── Section 7: Transforms ─────────────────────────────────────── */

static void draw_transforms(OuiSkCanvas canvas, OuiSkFont label_font,
                              OuiSkPaint label_paint, float ox, float oy) {
    draw_section_label(canvas, label_font, label_paint, ox, oy - 10,
                       "7. Transforms (Rotate / Scale / Skew)");

    OuiSkPaint p = oui_sk_paint_create();
    oui_sk_paint_set_anti_alias(p, true);
    OuiSkRect r;

    /* Rotation */
    oui_sk_canvas_save(canvas);
    oui_sk_canvas_translate(canvas, ox + 40, oy + 30);
    oui_sk_canvas_rotate(canvas, 30);
    oui_sk_paint_set_color(p, rgb(66, 133, 244));
    r = xywh(-30, -20, 60, 40);
    oui_sk_canvas_draw_rect(canvas, &r, p);
    oui_sk_canvas_restore(canvas);

    /* Scaling */
    oui_sk_canvas_save(canvas);
    oui_sk_canvas_translate(canvas, ox + 160, oy + 30);
    oui_sk_canvas_scale(canvas, 1.5f, 0.8f);
    oui_sk_paint_set_color(p, rgb(219, 68, 55));
    r = xywh(-25, -20, 50, 40);
    oui_sk_canvas_draw_rect(canvas, &r, p);
    oui_sk_canvas_restore(canvas);

    /* Skew */
    oui_sk_canvas_save(canvas);
    oui_sk_canvas_translate(canvas, ox + 280, oy + 30);
    oui_sk_canvas_skew(canvas, 0.3f, 0.0f);
    oui_sk_paint_set_color(p, rgb(15, 157, 88));
    r = xywh(-25, -20, 50, 40);
    oui_sk_canvas_draw_rect(canvas, &r, p);
    oui_sk_canvas_restore(canvas);

    /* Matrix transform (combined) */
    oui_sk_canvas_save(canvas);
    OuiSkMatrix m = oui_sk_matrix_identity();
    m.values[0] = 0.866f;  /* cos(30) */
    m.values[1] = -0.5f;   /* -sin(30) */
    m.values[2] = ox + 400;
    m.values[3] = 0.5f;    /* sin(30) */
    m.values[4] = 0.866f;  /* cos(30) */
    m.values[5] = oy + 30;
    oui_sk_canvas_concat_matrix(canvas, &m);
    oui_sk_paint_set_color(p, rgb(244, 180, 0));
    r = xywh(-25, -20, 50, 40);
    oui_sk_canvas_draw_rect(canvas, &r, p);
    oui_sk_canvas_restore(canvas);

    oui_sk_paint_destroy(p);
}

/* ─── Section 8: Clipping ───────────────────────────────────────── */

static void draw_clipping(OuiSkCanvas canvas, OuiSkFont label_font,
                            OuiSkPaint label_paint, float ox, float oy) {
    draw_section_label(canvas, label_font, label_paint, ox, oy - 10,
                       "8. Clipping (Rect / Path)");

    OuiSkPaint p = oui_sk_paint_create();
    oui_sk_paint_set_anti_alias(p, true);

    /* Rect clip — circle clipped to rectangle */
    oui_sk_canvas_save(canvas);
    OuiSkRect clip_rect = xywh(ox, oy, 80, 50);
    oui_sk_canvas_clip_rect(canvas, &clip_rect, true);
    oui_sk_paint_set_color(p, rgb(66, 133, 244));
    oui_sk_canvas_draw_circle(canvas, ox + 40, oy + 25, 40, p);
    oui_sk_canvas_restore(canvas);

    /* Path clip — rectangle clipped to triangle */
    oui_sk_canvas_save(canvas);
    OuiSkPath tri = oui_sk_path_create();
    oui_sk_path_move_to(tri, ox + 180, oy);
    oui_sk_path_line_to(tri, ox + 240, oy + 60);
    oui_sk_path_line_to(tri, ox + 120, oy + 60);
    oui_sk_path_close(tri);
    oui_sk_canvas_clip_path(canvas, tri, true);
    oui_sk_paint_set_color(p, rgb(219, 68, 55));
    OuiSkRect big = xywh(ox + 100, oy - 10, 160, 80);
    oui_sk_canvas_draw_rect(canvas, &big, p);
    oui_sk_canvas_restore(canvas);
    oui_sk_path_destroy(tri);

    /* RRect clip */
    oui_sk_canvas_save(canvas);
    OuiSkRect rr_rect = xywh(ox + 280, oy, 100, 50);
    OuiSkRRect rr_clip = oui_sk_rrect_make(rr_rect, 20, 20);
    oui_sk_canvas_clip_rrect(canvas, &rr_clip, true);

    /* Draw gradient inside the rounded clip */
    OuiSkColor gc[] = {rgb(255, 0, 0), rgb(0, 0, 255)};
    float gp[] = {0.0f, 1.0f};
    OuiSkPoint gs = {ox + 280, oy}, ge = {ox + 380, oy + 50};
    OuiSkShader gsh = oui_sk_shader_linear_gradient(
        gs, ge, gc, gp, 2, OUI_SK_TILE_MODE_CLAMP);
    oui_sk_paint_set_shader(p, gsh);
    oui_sk_canvas_draw_rect(canvas, &rr_rect, p);
    oui_sk_paint_set_shader(p, NULL);
    oui_sk_shader_destroy(gsh);
    oui_sk_canvas_restore(canvas);

    oui_sk_paint_destroy(p);
}

/* ─── Section 9: Alpha / Blending ───────────────────────────────── */

static void draw_blending(OuiSkCanvas canvas, OuiSkFont label_font,
                            OuiSkPaint label_paint, float ox, float oy) {
    draw_section_label(canvas, label_font, label_paint, ox, oy - 10,
                       "9. Alpha Blending");

    OuiSkPaint p = oui_sk_paint_create();
    oui_sk_paint_set_anti_alias(p, true);

    /* Overlapping semi-transparent circles */
    OuiSkColor circles[] = {
        rgba(255, 0, 0, 128),
        rgba(0, 255, 0, 128),
        rgba(0, 0, 255, 128),
    };
    float cx[] = {ox + 30, ox + 55, ox + 42};
    float cy[] = {oy + 20, oy + 20, oy + 42};

    for (int i = 0; i < 3; i++) {
        oui_sk_paint_set_color(p, circles[i]);
        oui_sk_canvas_draw_circle(canvas, cx[i], cy[i], 25, p);
    }

    /* Blend modes demo */
    OuiSkBlendMode modes[] = {
        OUI_SK_BLEND_MODE_MULTIPLY,
        OUI_SK_BLEND_MODE_SCREEN,
        OUI_SK_BLEND_MODE_OVERLAY,
    };
    const char* names[] = {"Multiply", "Screen", "Overlay"};

    OuiSkTypeface tf = oui_sk_typeface_create_from_name(
        "sans-serif", OUI_SK_FONT_STYLE_NORMAL);
    OuiSkFont small = oui_sk_font_create(tf, 10.0f);

    for (int i = 0; i < 3; i++) {
        float bx = ox + 120 + i * 100;

        /* Base rect */
        oui_sk_paint_set_blend_mode(p, OUI_SK_BLEND_MODE_SRC_OVER);
        oui_sk_paint_set_color(p, rgb(66, 133, 244));
        OuiSkRect base = xywh(bx, oy, 50, 40);
        oui_sk_canvas_draw_rect(canvas, &base, p);

        /* Blended rect on top */
        oui_sk_paint_set_blend_mode(p, modes[i]);
        oui_sk_paint_set_color(p, rgb(219, 68, 55));
        OuiSkRect top = xywh(bx + 15, oy + 10, 50, 40);
        oui_sk_canvas_draw_rect(canvas, &top, p);

        /* Label */
        oui_sk_paint_set_blend_mode(p, OUI_SK_BLEND_MODE_SRC_OVER);
        oui_sk_paint_set_color(p, rgb(33, 33, 33));
        if (small) {
            oui_sk_canvas_draw_text(canvas, names[i], strlen(names[i]),
                                     bx + 5, oy + 60, small, p);
        }
    }

    if (small) oui_sk_font_destroy(small);
    if (tf) oui_sk_typeface_destroy(tf);
    oui_sk_paint_destroy(p);
}

/* ─── Section 10: Image Operations ──────────────────────────────── */

static void draw_image_ops(OuiSkCanvas canvas, OuiSkFont label_font,
                             OuiSkPaint label_paint, float ox, float oy) {
    draw_section_label(canvas, label_font, label_paint, ox, oy - 10,
                       "10. Image Ops (Snapshot / Scale / Rotate)");

    /* Create a small surface, draw a pattern, snapshot as image */
    OuiSkSurface mini = oui_sk_surface_create_raster(60, 60);
    if (!mini) return;
    OuiSkCanvas mc = oui_sk_surface_get_canvas(mini);

    OuiSkPaint p = oui_sk_paint_create();
    oui_sk_paint_set_anti_alias(p, true);
    oui_sk_canvas_clear(mc, rgb(240, 240, 240));

    /* Checkerboard pattern */
    for (int y = 0; y < 4; y++) {
        for (int x = 0; x < 4; x++) {
            if ((x + y) % 2 == 0) {
                oui_sk_paint_set_color(p, rgb(100, 100, 200));
            } else {
                oui_sk_paint_set_color(p, rgb(200, 200, 255));
            }
            OuiSkRect cell = xywh((float)(x * 15), (float)(y * 15), 15, 15);
            oui_sk_canvas_draw_rect(mc, &cell, p);
        }
    }

    OuiSkImage img = oui_sk_surface_make_image_snapshot(mini);
    oui_sk_surface_destroy(mini);

    if (!img) {
        oui_sk_paint_destroy(p);
        return;
    }

    /* Draw original */
    oui_sk_canvas_draw_image(canvas, img, ox, oy, p);

    /* Draw scaled 2x */
    OuiSkRect src_r = xywh(0, 0, 60, 60);
    OuiSkRect dst_r = xywh(ox + 80, oy, 40, 40);
    oui_sk_canvas_draw_image_rect(canvas, img, &src_r, &dst_r, p);

    /* Draw rotated */
    oui_sk_canvas_save(canvas);
    oui_sk_canvas_translate(canvas, ox + 180, oy + 30);
    oui_sk_canvas_rotate(canvas, 45);
    oui_sk_canvas_draw_image(canvas, img, -30, -30, p);
    oui_sk_canvas_restore(canvas);

    oui_sk_image_destroy(img);
    oui_sk_paint_destroy(p);
}

/* ─── Main ──────────────────────────────────────────────────────── */

int main(int argc, char* argv[]) {
    const char* output = (argc > 1) ? argv[1] : "gallery.png";

    OuiSkSurface surface = oui_sk_surface_create_raster(WIDTH, HEIGHT);
    if (!surface) {
        fprintf(stderr, "Failed to create surface\n");
        return 1;
    }

    OuiSkCanvas canvas = oui_sk_surface_get_canvas(surface);
    oui_sk_canvas_clear(canvas, rgb(250, 250, 250));

    /* Title font */
    OuiSkTypeface tf = oui_sk_typeface_create_from_name(
        "sans-serif", OUI_SK_FONT_STYLE_BOLD);
    OuiSkFont title_font = oui_sk_font_create(tf, 28.0f);

    /* Section label font */
    OuiSkTypeface tf2 = oui_sk_typeface_create_from_name(
        "sans-serif", OUI_SK_FONT_STYLE_BOLD);
    OuiSkFont label_font = oui_sk_font_create(tf2, 14.0f);
    OuiSkPaint label_paint = oui_sk_paint_create();
    oui_sk_paint_set_anti_alias(label_paint, true);

    /* Title */
    oui_sk_paint_set_color(label_paint, rgb(20, 20, 20));
    const char* title = "Open UI — Skia C API Rendering Gallery";
    if (title_font) {
        oui_sk_canvas_draw_text(canvas, title, strlen(title),
                                 30, 40, title_font, label_paint);
    }

    /* Draw separator */
    OuiSkPaint sep = oui_sk_paint_create();
    oui_sk_paint_set_color(sep, rgba(0, 0, 0, 40));
    OuiSkRect sep_r = xywh(30, 52, (float)(WIDTH - 60), 1);
    oui_sk_canvas_draw_rect(canvas, &sep_r, sep);
    oui_sk_paint_destroy(sep);

    /* Sections — 2 columns, 5 rows */
    float col1 = 30, col2 = 620;
    float row_h = 100;
    float y_start = 80;

    draw_basic_shapes(canvas, label_font, label_paint, col1, y_start);
    draw_paths(canvas, label_font, label_paint, col2, y_start);

    draw_paint_styles(canvas, label_font, label_paint, col1, y_start + row_h);
    draw_gradients(canvas, label_font, label_paint, col2, y_start + row_h);

    draw_text(canvas, label_font, label_paint, col1, y_start + row_h * 2);
    draw_filters(canvas, label_font, label_paint, col2, y_start + row_h * 2);

    draw_transforms(canvas, label_font, label_paint, col1, y_start + row_h * 3 + 30);
    draw_clipping(canvas, label_font, label_paint, col2, y_start + row_h * 3 + 30);

    draw_blending(canvas, label_font, label_paint, col1, y_start + row_h * 4 + 60);
    draw_image_ops(canvas, label_font, label_paint, col2, y_start + row_h * 4 + 60);

    /* Encode and save */
    OuiSkImage snapshot = oui_sk_surface_make_image_snapshot(surface);
    void* data = NULL;
    size_t size = 0;
    OuiSkStatus status = oui_sk_image_encode(
        snapshot, OUI_SK_IMAGE_FORMAT_PNG, 100, &data, &size);

    if (status == OUI_SK_OK && data) {
        FILE* f = fopen(output, "wb");
        if (f) {
            fwrite(data, 1, size, f);
            fclose(f);
            printf("Gallery rendered to %s (%zu bytes)\n", output, size);
        } else {
            fprintf(stderr, "Cannot write %s\n", output);
        }
        oui_sk_image_encode_free(data);
    } else {
        fprintf(stderr, "Encode failed: %s\n", oui_sk_status_string(status));
    }

    /* Cleanup */
    oui_sk_image_destroy(snapshot);
    if (title_font) oui_sk_font_destroy(title_font);
    if (label_font) oui_sk_font_destroy(label_font);
    oui_sk_paint_destroy(label_paint);
    if (tf) oui_sk_typeface_destroy(tf);
    if (tf2) oui_sk_typeface_destroy(tf2);
    oui_sk_surface_destroy(surface);

    return 0;
}
