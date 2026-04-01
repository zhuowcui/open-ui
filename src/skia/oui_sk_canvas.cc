// src/skia/oui_sk_canvas.cc — Canvas C API wrappers

#include "src/skia/oui_sk_types_internal.h"

#include "include/core/SkCanvas.h"
#include "include/core/SkSamplingOptions.h"
#include "modules/skparagraph/include/Paragraph.h"

extern "C" {

int oui_sk_canvas_save(OuiSkCanvas canvas) {
    if (!canvas || !canvas->canvas) return 0;
    return canvas->canvas->save();
}

void oui_sk_canvas_restore(OuiSkCanvas canvas) {
    if (!canvas || !canvas->canvas) return;
    canvas->canvas->restore();
}

void oui_sk_canvas_restore_to_count(OuiSkCanvas canvas, int save_count) {
    if (!canvas || !canvas->canvas) return;
    canvas->canvas->restoreToCount(save_count);
}

int oui_sk_canvas_get_save_count(OuiSkCanvas canvas) {
    if (!canvas || !canvas->canvas) return 0;
    return canvas->canvas->getSaveCount();
}

void oui_sk_canvas_translate(OuiSkCanvas canvas, float dx, float dy) {
    if (!canvas || !canvas->canvas) return;
    canvas->canvas->translate(dx, dy);
}

void oui_sk_canvas_scale(OuiSkCanvas canvas, float sx, float sy) {
    if (!canvas || !canvas->canvas) return;
    canvas->canvas->scale(sx, sy);
}

void oui_sk_canvas_rotate(OuiSkCanvas canvas, float degrees) {
    if (!canvas || !canvas->canvas) return;
    canvas->canvas->rotate(degrees);
}

void oui_sk_canvas_skew(OuiSkCanvas canvas, float sx, float sy) {
    if (!canvas || !canvas->canvas) return;
    canvas->canvas->skew(sx, sy);
}

void oui_sk_canvas_concat_matrix(OuiSkCanvas canvas, const OuiSkMatrix* matrix) {
    if (!canvas || !canvas->canvas || !matrix) return;
    canvas->canvas->concat(to_sk_matrix(*matrix));
}

void oui_sk_canvas_clip_rect(
    OuiSkCanvas canvas, const OuiSkRect* rect, bool anti_alias) {
    if (!canvas || !canvas->canvas || !rect) return;
    canvas->canvas->clipRect(to_sk_rect(*rect), anti_alias);
}

void oui_sk_canvas_clip_rrect(
    OuiSkCanvas canvas, const OuiSkRRect* rrect, bool anti_alias) {
    if (!canvas || !canvas->canvas || !rrect) return;
    canvas->canvas->clipRRect(to_sk_rrect(*rrect), anti_alias);
}

void oui_sk_canvas_clip_path(
    OuiSkCanvas canvas, OuiSkPath path, bool anti_alias) {
    if (!canvas || !canvas->canvas || !path) return;
    canvas->canvas->clipPath(path->path(), anti_alias);
}

void oui_sk_canvas_clear(OuiSkCanvas canvas, OuiSkColor color) {
    if (!canvas || !canvas->canvas) return;
    canvas->canvas->clear(to_sk_color(color));
}

void oui_sk_canvas_draw_rect(
    OuiSkCanvas canvas, const OuiSkRect* rect, OuiSkPaint paint) {
    if (!canvas || !canvas->canvas || !rect || !paint) return;
    canvas->canvas->drawRect(to_sk_rect(*rect), paint->paint);
}

void oui_sk_canvas_draw_rrect(
    OuiSkCanvas canvas, const OuiSkRRect* rrect, OuiSkPaint paint) {
    if (!canvas || !canvas->canvas || !rrect || !paint) return;
    canvas->canvas->drawRRect(to_sk_rrect(*rrect), paint->paint);
}

void oui_sk_canvas_draw_circle(
    OuiSkCanvas canvas, float cx, float cy, float radius, OuiSkPaint paint) {
    if (!canvas || !canvas->canvas || !paint) return;
    canvas->canvas->drawCircle(cx, cy, radius, paint->paint);
}

void oui_sk_canvas_draw_oval(
    OuiSkCanvas canvas, const OuiSkRect* rect, OuiSkPaint paint) {
    if (!canvas || !canvas->canvas || !rect || !paint) return;
    canvas->canvas->drawOval(to_sk_rect(*rect), paint->paint);
}

void oui_sk_canvas_draw_path(
    OuiSkCanvas canvas, OuiSkPath path, OuiSkPaint paint) {
    if (!canvas || !canvas->canvas || !path || !paint) return;
    canvas->canvas->drawPath(path->path(), paint->paint);
}

void oui_sk_canvas_draw_line(
    OuiSkCanvas canvas, float x0, float y0, float x1, float y1,
    OuiSkPaint paint) {
    if (!canvas || !canvas->canvas || !paint) return;
    canvas->canvas->drawLine(x0, y0, x1, y1, paint->paint);
}

void oui_sk_canvas_draw_point(
    OuiSkCanvas canvas, float x, float y, OuiSkPaint paint) {
    if (!canvas || !canvas->canvas || !paint) return;
    canvas->canvas->drawPoint(x, y, paint->paint);
}

void oui_sk_canvas_draw_image(
    OuiSkCanvas canvas, OuiSkImage image, float x, float y,
    OuiSkPaint paint) {
    if (!canvas || !canvas->canvas || !image) return;
    const SkPaint* p = paint ? &paint->paint : nullptr;
    canvas->canvas->drawImage(image->image, x, y, SkSamplingOptions(), p);
}

void oui_sk_canvas_draw_image_rect(
    OuiSkCanvas canvas, OuiSkImage image,
    const OuiSkRect* src, const OuiSkRect* dst, OuiSkPaint paint) {
    if (!canvas || !canvas->canvas || !image || !dst) return;

    SkRect sk_dst = to_sk_rect(*dst);
    const SkPaint* p = paint ? &paint->paint : nullptr;

    if (src) {
        SkRect sk_src = to_sk_rect(*src);
        canvas->canvas->drawImageRect(
            image->image, sk_src, sk_dst, SkSamplingOptions(), p,
            SkCanvas::kStrict_SrcRectConstraint);
    } else {
        canvas->canvas->drawImageRect(
            image->image, sk_dst, SkSamplingOptions(), p);
    }
}

void oui_sk_canvas_draw_text(
    OuiSkCanvas canvas, const char* text, size_t len,
    float x, float y, OuiSkFont font, OuiSkPaint paint) {
    if (!canvas || !canvas->canvas || !text || !font || !paint) return;

    sk_sp<SkTextBlob> blob = SkTextBlob::MakeFromText(
        text, len, font->font, SkTextEncoding::kUTF8);
    if (blob) {
        canvas->canvas->drawTextBlob(blob, x, y, paint->paint);
    }
}

void oui_sk_canvas_draw_text_blob(
    OuiSkCanvas canvas, OuiSkTextBlob blob,
    float x, float y, OuiSkPaint paint) {
    if (!canvas || !canvas->canvas || !blob || !paint) return;

    if (blob->paragraph) {
        // Shaped paragraph — apply user's paint color at draw time
        blob->paragraph->updateForegroundPaint(0, blob->text_length, paint->paint);
        canvas->canvas->save();
        canvas->canvas->translate(x, y);
        blob->paragraph->paint(canvas->canvas, 0, 0);
        canvas->canvas->restore();
    } else if (blob->blob) {
        canvas->canvas->drawTextBlob(blob->blob, x, y, paint->paint);
    }
}

}  // extern "C"
