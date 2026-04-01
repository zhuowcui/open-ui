// src/skia/oui_sk_path.cc — Path C API wrappers

#include "src/skia/oui_sk_types_internal.h"

#include "include/utils/SkParsePath.h"

extern "C" {

OuiSkPath oui_sk_path_create(void) {
    auto* _p = new(std::nothrow) OuiSkPath_t();
    if (!_p) return nullptr;
    return _p;
}

OuiSkPath oui_sk_path_clone(OuiSkPath path) {
    if (!path) return nullptr;
    auto* p = new(std::nothrow) OuiSkPath_t();
    if (!p) return nullptr;
    // Snapshot the source path and create a new builder from it
    const SkPath& src = path->path();
    p->builder = SkPathBuilder(src);
    p->dirty = true;
    return p;
}

void oui_sk_path_destroy(OuiSkPath path) {
    delete path;
}

void oui_sk_path_move_to(OuiSkPath path, float x, float y) {
    if (!path) return;
    path->builder.moveTo(x, y);
    path->dirty = true;
}

void oui_sk_path_line_to(OuiSkPath path, float x, float y) {
    if (!path) return;
    path->builder.lineTo(x, y);
    path->dirty = true;
}

void oui_sk_path_quad_to(
    OuiSkPath path, float cx, float cy, float x, float y) {
    if (!path) return;
    path->builder.quadTo(cx, cy, x, y);
    path->dirty = true;
}

void oui_sk_path_cubic_to(
    OuiSkPath path, float c1x, float c1y, float c2x, float c2y,
    float x, float y) {
    if (!path) return;
    path->builder.cubicTo(c1x, c1y, c2x, c2y, x, y);
    path->dirty = true;
}

void oui_sk_path_arc_to(
    OuiSkPath path, const OuiSkRect* oval,
    float start_angle, float sweep_angle, bool force_move_to) {
    if (!path || !oval) return;
    SkRect sk_oval = to_sk_rect(*oval);
    path->builder.arcTo(sk_oval, start_angle, sweep_angle, force_move_to);
    path->dirty = true;
}

void oui_sk_path_close(OuiSkPath path) {
    if (!path) return;
    path->builder.close();
    path->dirty = true;
}

void oui_sk_path_reset(OuiSkPath path) {
    if (!path) return;
    path->builder.reset();
    path->dirty = true;
}

OuiSkPath oui_sk_path_from_svg_string(const char* svg) {
    if (!svg) return nullptr;

    SkPath sk_path;
    if (!SkParsePath::FromSVGString(svg, &sk_path)) {
        return nullptr;
    }

    auto* p = new(std::nothrow) OuiSkPath_t();
    if (!p) return nullptr;
    p->builder = SkPathBuilder(sk_path);
    p->dirty = true;
    return p;
}

OuiSkRect oui_sk_path_get_bounds(OuiSkPath path) {
    if (!path) return {0, 0, 0, 0};
    return from_sk_rect(path->path().getBounds());
}

bool oui_sk_path_contains(OuiSkPath path, float x, float y) {
    if (!path) return false;
    return path->path().contains(x, y);
}

}  // extern "C"
