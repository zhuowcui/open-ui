// src/skia/oui_sk_types_internal.h
// Internal type definitions — maps opaque C handles to Skia C++ objects.
// NOT part of the public API.

#ifndef OUI_SK_TYPES_INTERNAL_H_
#define OUI_SK_TYPES_INTERNAL_H_

#include <memory>
#include <new>

#include "include/openui/openui_skia.h"

#include "include/core/SkCanvas.h"
#include "include/core/SkColorFilter.h"
#include "include/core/SkFont.h"
#include "include/core/SkFontMgr.h"
#include "include/core/SkImage.h"
#include "include/core/SkImageFilter.h"
#include "include/core/SkMaskFilter.h"
#include "include/core/SkPaint.h"
#include "include/core/SkPath.h"
#include "include/core/SkPathBuilder.h"
#include "include/core/SkRefCnt.h"
#include "include/core/SkShader.h"
#include "include/core/SkSurface.h"
#include "include/core/SkPicture.h"
#include "include/core/SkTextBlob.h"

// Forward declare to avoid pulling paragraph headers into every TU
namespace skia { namespace textlayout { class Paragraph; } }

#include "include/core/SkTypeface.h"

#ifdef SK_GANESH
#include "include/gpu/ganesh/GrDirectContext.h"
#endif

// Internal structures behind opaque handles.
// Each wraps the corresponding Skia smart pointer.

struct OuiSkCanvas_t {
    SkCanvas* canvas;  // Borrowed from surface — not owned
};

struct OuiSkSurface_t {
    sk_sp<SkSurface> surface;
    OuiSkCanvas_t canvas_handle;
    bool canvas_valid;

    OuiSkSurface_t() : canvas_valid(false) {}
};

struct OuiSkPaint_t {
    SkPaint paint;
};

struct OuiSkPath_t {
    SkPathBuilder builder;
    SkPath snapshot;
    bool dirty;

    OuiSkPath_t() : dirty(true) {}

    const SkPath& path() {
        if (dirty) {
            snapshot = builder.snapshot();
            dirty = false;
        }
        return snapshot;
    }
};

struct OuiSkFont_t {
    SkFont font;
};

struct OuiSkTypeface_t {
    sk_sp<SkTypeface> typeface;
};

struct OuiSkTextBlob_t {
    sk_sp<SkTextBlob> blob;
    std::unique_ptr<skia::textlayout::Paragraph> paragraph;  // For shaped text
    size_t text_length;  // Length of shaped text (for updateForegroundPaint range)
};

struct OuiSkImage_t {
    sk_sp<SkImage> image;
};

struct OuiSkShader_t {
    sk_sp<SkShader> shader;
};

struct OuiSkImageFilter_t {
    sk_sp<SkImageFilter> filter;
};

struct OuiSkMaskFilter_t {
    sk_sp<SkMaskFilter> filter;
};

struct OuiSkColorFilter_t {
    sk_sp<SkColorFilter> filter;
};

struct OuiSkGpuContext_t {
#ifdef SK_GANESH
    sk_sp<GrDirectContext> context;
#endif
};

// Helper: convert OuiSkColor to SkColor
static inline SkColor to_sk_color(OuiSkColor c) {
    return SkColorSetARGB(c.a, c.r, c.g, c.b);
}

// Helper: convert SkColor to OuiSkColor
static inline OuiSkColor from_sk_color(SkColor c) {
    OuiSkColor out;
    out.r = SkColorGetR(c);
    out.g = SkColorGetG(c);
    out.b = SkColorGetB(c);
    out.a = SkColorGetA(c);
    return out;
}

// Helper: convert OuiSkColor4f to SkColor4f
static inline SkColor4f to_sk_color4f(OuiSkColor4f c) {
    return {c.r, c.g, c.b, c.a};
}

// Helper: convert OuiSkRect to SkRect
static inline SkRect to_sk_rect(const OuiSkRect& r) {
    return SkRect::MakeLTRB(r.left, r.top, r.right, r.bottom);
}

// Helper: convert SkRect to OuiSkRect
static inline OuiSkRect from_sk_rect(const SkRect& r) {
    OuiSkRect out;
    out.left = r.fLeft;
    out.top = r.fTop;
    out.right = r.fRight;
    out.bottom = r.fBottom;
    return out;
}

// Helper: convert OuiSkRRect to SkRRect
static inline SkRRect to_sk_rrect(const OuiSkRRect& r) {
    SkRRect rr;
    rr.setRectXY(to_sk_rect(r.rect), r.rx, r.ry);
    return rr;
}

// Helper: convert OuiSkMatrix to SkMatrix
static inline SkMatrix to_sk_matrix(const OuiSkMatrix& m) {
    SkMatrix mat;
    mat.set9(m.values);
    return mat;
}

// Helper: convert OuiSkBlendMode to SkBlendMode
static inline SkBlendMode to_sk_blend_mode(OuiSkBlendMode mode) {
    return static_cast<SkBlendMode>(mode);
}

// Helper: convert OuiSkTileMode to SkTileMode
static inline SkTileMode to_sk_tile_mode(OuiSkTileMode mode) {
    return static_cast<SkTileMode>(mode);
}

// Global font manager (lazily initialized)
sk_sp<SkFontMgr> oui_sk_get_font_manager();

#endif  // OUI_SK_TYPES_INTERNAL_H_
