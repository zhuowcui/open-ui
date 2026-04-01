// src/skia/oui_sk_font.cc — Font, typeface, and text C API wrappers

#include "src/skia/oui_sk_types_internal.h"

#include "include/core/SkData.h"
#include "include/core/SkFontMgr.h"
#include "include/core/SkFontMetrics.h"
#include "include/core/SkFontScanner.h"
#include "include/core/SkFontStyle.h"
#include "include/core/SkStream.h"
#include "include/core/SkTextBlob.h"
#include "include/ports/SkFontMgr_fontconfig.h"
#include "include/ports/SkFontScanner_FreeType.h"
#include "modules/skparagraph/include/DartTypes.h"
#include "modules/skparagraph/include/FontCollection.h"
#include "modules/skparagraph/include/Paragraph.h"
#include "modules/skparagraph/include/ParagraphBuilder.h"
#include "modules/skparagraph/include/ParagraphStyle.h"
#include "modules/skparagraph/include/TextStyle.h"
#include "modules/skparagraph/include/TypefaceFontProvider.h"
#include "modules/skunicode/include/SkUnicode_icu.h"

#include <mutex>

static sk_sp<SkFontMgr> g_font_manager;
static std::once_flag g_font_manager_init;

sk_sp<SkFontMgr> oui_sk_get_font_manager() {
    std::call_once(g_font_manager_init, []() {
        auto scanner = SkFontScanner_Make_FreeType();
        g_font_manager = SkFontMgr_New_FontConfig(nullptr, std::move(scanner));
        if (!g_font_manager) {
            g_font_manager = SkFontMgr::RefEmpty();
        }
    });
    return g_font_manager;
}

static SkFontStyle to_sk_font_style(OuiSkFontStylePreset preset) {
    switch (preset) {
        case OUI_SK_FONT_STYLE_BOLD:
            return SkFontStyle::Bold();
        case OUI_SK_FONT_STYLE_ITALIC:
            return SkFontStyle::Italic();
        case OUI_SK_FONT_STYLE_BOLD_ITALIC:
            return SkFontStyle::BoldItalic();
        case OUI_SK_FONT_STYLE_NORMAL:
        default:
            return SkFontStyle::Normal();
    }
}

extern "C" {

OuiSkTypeface oui_sk_typeface_create_from_name(
    const char* family_name, OuiSkFontStylePreset style) {
    sk_sp<SkFontMgr> mgr = oui_sk_get_font_manager();
    sk_sp<SkTypeface> tf = mgr->matchFamilyStyle(
        family_name, to_sk_font_style(style));
    if (!tf) return nullptr;

    auto* t = new(std::nothrow) OuiSkTypeface_t();
    if (!t) return nullptr;
    t->typeface = std::move(tf);
    return t;
}

OuiSkTypeface oui_sk_typeface_create_from_file(const char* path, int index) {
    if (!path) return nullptr;
    sk_sp<SkFontMgr> mgr = oui_sk_get_font_manager();

    auto stream = SkStream::MakeFromFile(path);
    if (!stream) return nullptr;

    sk_sp<SkTypeface> tf = mgr->makeFromStream(std::move(stream), index);
    if (!tf) return nullptr;

    auto* t = new(std::nothrow) OuiSkTypeface_t();
    if (!t) return nullptr;
    t->typeface = std::move(tf);
    return t;
}

OuiSkTypeface oui_sk_typeface_create_from_data(
    const void* data, size_t size, int index) {
    if (!data || size == 0) return nullptr;

    try {
        sk_sp<SkFontMgr> mgr = oui_sk_get_font_manager();
        sk_sp<SkData> sk_data = SkData::MakeWithCopy(data, size);
        auto stream = std::make_unique<SkMemoryStream>(sk_data);

        sk_sp<SkTypeface> tf = mgr->makeFromStream(std::move(stream), index);
        if (!tf) return nullptr;

        auto* t = new(std::nothrow) OuiSkTypeface_t();
        if (!t) return nullptr;
        t->typeface = std::move(tf);
        return t;
    } catch (...) {
        return nullptr;
    }
}

void oui_sk_typeface_destroy(OuiSkTypeface typeface) {
    delete typeface;
}

OuiSkFont oui_sk_font_create(OuiSkTypeface typeface, float size) {
    auto* f = new(std::nothrow) OuiSkFont_t();
    if (!f) return nullptr;
    if (typeface && typeface->typeface) {
        f->font = SkFont(typeface->typeface, size);
    } else {
        f->font = SkFont(nullptr, size);
    }
    f->font.setSubpixel(true);
    f->font.setEdging(SkFont::Edging::kSubpixelAntiAlias);
    return f;
}

void oui_sk_font_destroy(OuiSkFont font) {
    delete font;
}

void oui_sk_font_set_size(OuiSkFont font, float size) {
    if (!font) return;
    font->font.setSize(size);
}

float oui_sk_font_get_size(OuiSkFont font) {
    if (!font) return 0;
    return font->font.getSize();
}

OuiSkFontMetrics oui_sk_font_get_metrics(OuiSkFont font) {
    OuiSkFontMetrics out = {0, 0, 0};
    if (!font) return out;

    SkFontMetrics metrics;
    font->font.getMetrics(&metrics);
    out.ascent = metrics.fAscent;
    out.descent = metrics.fDescent;
    out.leading = metrics.fLeading;
    return out;
}

float oui_sk_font_measure_text(
    OuiSkFont font, const char* text, size_t len) {
    if (!font || !text) return 0;
    return font->font.measureText(text, len, SkTextEncoding::kUTF8);
}

OuiSkTextBlob oui_sk_text_blob_create(
    const char* text, size_t len, OuiSkFont font) {
    if (!text || !font) return nullptr;

    sk_sp<SkTextBlob> blob = SkTextBlob::MakeFromText(
        text, len, font->font, SkTextEncoding::kUTF8);
    if (!blob) return nullptr;

    auto* b = new(std::nothrow) OuiSkTextBlob_t();
    if (!b) return nullptr;
    b->blob = std::move(blob);
    b->text_length = len;
    return b;
}

OuiSkTextBlob oui_sk_text_shape(
    const char* text, size_t len, OuiSkFont font,
    float width, OuiSkTextAlign align, OuiSkTextDirection dir) {
    if (!text || !font) return nullptr;

    try {
        using namespace skia::textlayout;

        // Map our enums to SkParagraph enums
        TextAlign sk_align;
        switch (align) {
            case OUI_SK_TEXT_ALIGN_LEFT:    sk_align = TextAlign::kLeft; break;
            case OUI_SK_TEXT_ALIGN_CENTER:  sk_align = TextAlign::kCenter; break;
            case OUI_SK_TEXT_ALIGN_RIGHT:   sk_align = TextAlign::kRight; break;
            default:                        sk_align = TextAlign::kLeft; break;
        }
        TextDirection sk_dir = (dir == OUI_SK_TEXT_DIRECTION_RTL)
            ? TextDirection::kRtl : TextDirection::kLtr;

        // Configure paragraph style
        // For single-line mode (width <= 0), force left alignment — center/right
        // against SK_ScalarMax would place glyphs far off-screen.
        ParagraphStyle para_style;
        para_style.setTextAlign((width <= 0) ? TextAlign::kLeft : sk_align);
        para_style.setTextDirection(sk_dir);

        // Font collection: register the caller's typeface so the paragraph
        // resolver can find it (critical for custom fonts loaded from file/data)
        auto font_collection = sk_make_sp<FontCollection>();
        auto tf_provider = sk_make_sp<TypefaceFontProvider>();
        SkTypeface* tf = font->font.getTypeface();
        if (tf) {
            tf_provider->registerTypeface(sk_ref_sp(tf));
        }
        font_collection->setAssetFontManager(tf_provider);
        font_collection->setDefaultFontManager(oui_sk_get_font_manager());
        font_collection->enableFontFallback();

        // Unicode support
        auto unicode = SkUnicodes::ICU::Make();
        if (!unicode) {
            // Fallback: simple single-line blob without paragraph layout
            sk_sp<SkTextBlob> blob = SkTextBlob::MakeFromText(
                text, len, font->font, SkTextEncoding::kUTF8);
            if (!blob) return nullptr;
            auto* b = new(std::nothrow) OuiSkTextBlob_t();
            if (!b) return nullptr;
            b->blob = std::move(blob);
            b->text_length = len;
            return b;
        }

        auto builder = ParagraphBuilder::make(para_style, font_collection, unicode);
        if (!builder) return nullptr;

        // Configure text style — use a placeholder paint; the actual paint is
        // applied at draw time via Paragraph::updateForegroundPaint()
        TextStyle text_style;
        if (tf) {
            SkString family_name;
            tf->getFamilyName(&family_name);
            text_style.setFontFamilies({family_name});
            text_style.setTypeface(sk_ref_sp(tf));
        }
        text_style.setFontSize(font->font.getSize());
        text_style.setForegroundPaint(SkPaint());

        builder->pushStyle(text_style);
        builder->addText(text, len);
        builder->pop();

        auto paragraph = builder->Build();
        if (!paragraph) return nullptr;

        // Layout with the given width (0 or negative = single-line, no wrapping)
        paragraph->layout(width > 0 ? width : SK_ScalarMax);

        auto* b = new(std::nothrow) OuiSkTextBlob_t();
        if (!b) return nullptr;
        b->paragraph = std::move(paragraph);
        b->text_length = len;
        return b;
    } catch (...) {
        return nullptr;
    }
}

void oui_sk_text_blob_destroy(OuiSkTextBlob blob) {
    delete blob;
}

}  // extern "C"
