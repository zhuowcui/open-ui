// src/skia/oui_sk_paint.cc — Paint C API wrappers

#include "src/skia/oui_sk_types_internal.h"

extern "C" {

OuiSkPaint oui_sk_paint_create(void) {
    auto* p = new(std::nothrow) OuiSkPaint_t();
    if (!p) return nullptr;
    p->paint.setAntiAlias(true);
    return p;
}

OuiSkPaint oui_sk_paint_clone(OuiSkPaint paint) {
    if (!paint) return nullptr;
    auto* p = new(std::nothrow) OuiSkPaint_t();
    if (!p) return nullptr;
    p->paint = paint->paint;
    return p;
}

void oui_sk_paint_destroy(OuiSkPaint paint) {
    delete paint;
}

void oui_sk_paint_set_color(OuiSkPaint paint, OuiSkColor color) {
    if (!paint) return;
    paint->paint.setColor(to_sk_color(color));
}

void oui_sk_paint_set_color4f(OuiSkPaint paint, OuiSkColor4f color) {
    if (!paint) return;
    paint->paint.setColor4f(to_sk_color4f(color));
}

OuiSkColor oui_sk_paint_get_color(OuiSkPaint paint) {
    if (!paint) return {0, 0, 0, 0};
    return from_sk_color(paint->paint.getColor());
}

void oui_sk_paint_set_alpha(OuiSkPaint paint, uint8_t alpha) {
    if (!paint) return;
    paint->paint.setAlpha(alpha);
}

void oui_sk_paint_set_style(OuiSkPaint paint, OuiSkPaintStyle style) {
    if (!paint) return;
    paint->paint.setStyle(static_cast<SkPaint::Style>(style));
}

void oui_sk_paint_set_stroke_width(OuiSkPaint paint, float width) {
    if (!paint) return;
    paint->paint.setStrokeWidth(width);
}

void oui_sk_paint_set_stroke_cap(OuiSkPaint paint, OuiSkStrokeCap cap) {
    if (!paint) return;
    paint->paint.setStrokeCap(static_cast<SkPaint::Cap>(cap));
}

void oui_sk_paint_set_stroke_join(OuiSkPaint paint, OuiSkStrokeJoin join) {
    if (!paint) return;
    paint->paint.setStrokeJoin(static_cast<SkPaint::Join>(join));
}

void oui_sk_paint_set_stroke_miter(OuiSkPaint paint, float miter) {
    if (!paint) return;
    paint->paint.setStrokeMiter(miter);
}

void oui_sk_paint_set_anti_alias(OuiSkPaint paint, bool aa) {
    if (!paint) return;
    paint->paint.setAntiAlias(aa);
}

void oui_sk_paint_set_blend_mode(OuiSkPaint paint, OuiSkBlendMode mode) {
    if (!paint) return;
    paint->paint.setBlendMode(to_sk_blend_mode(mode));
}

void oui_sk_paint_set_shader(OuiSkPaint paint, OuiSkShader shader) {
    if (!paint) return;
    paint->paint.setShader(shader ? shader->shader : nullptr);
}

void oui_sk_paint_set_image_filter(OuiSkPaint paint, OuiSkImageFilter filter) {
    if (!paint) return;
    paint->paint.setImageFilter(filter ? filter->filter : nullptr);
}

void oui_sk_paint_set_mask_filter(OuiSkPaint paint, OuiSkMaskFilter filter) {
    if (!paint) return;
    paint->paint.setMaskFilter(filter ? filter->filter : nullptr);
}

void oui_sk_paint_set_color_filter(OuiSkPaint paint, OuiSkColorFilter filter) {
    if (!paint) return;
    paint->paint.setColorFilter(filter ? filter->filter : nullptr);
}

}  // extern "C"
