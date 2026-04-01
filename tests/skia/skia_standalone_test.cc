// tests/skia/skia_standalone_test.cc
// Verify that Skia builds and runs standalone (no Chromium dependencies).
// Renders shapes and text to a PNG file, proving the library works.

#include "include/core/SkBitmap.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkColor.h"
#include "include/core/SkFont.h"
#include "include/core/SkFontMgr.h"
#include "include/core/SkFontScanner.h"
#include "include/core/SkImage.h"
#include "include/core/SkPaint.h"
#include "include/core/SkPath.h"
#include "include/core/SkPathBuilder.h"
#include "include/core/SkRRect.h"
#include "include/core/SkStream.h"
#include "include/core/SkSurface.h"
#include "include/core/SkTypeface.h"
#include "include/encode/SkPngEncoder.h"
#include "include/effects/SkGradient.h"
#include "include/effects/SkImageFilters.h"
#include "include/ports/SkFontMgr_fontconfig.h"
#include "include/ports/SkFontScanner_FreeType.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>

static bool test_raster_surface() {
    auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(400, 300));
    if (!surface) {
        fprintf(stderr, "FAIL: Could not create raster surface\n");
        return false;
    }
    printf("  PASS: Raster surface created (400x300)\n");
    return true;
}

static bool test_draw_shapes(SkCanvas* canvas) {
    // Background
    canvas->clear(SK_ColorWHITE);

    // Filled rectangle
    SkPaint rectPaint;
    rectPaint.setColor(SkColorSetRGB(66, 133, 244));  // Google Blue
    rectPaint.setAntiAlias(true);
    canvas->drawRect(SkRect::MakeXYWH(20, 20, 160, 100), rectPaint);

    // Rounded rectangle
    SkPaint rrectPaint;
    rrectPaint.setColor(SkColorSetRGB(234, 67, 53));  // Google Red
    rrectPaint.setAntiAlias(true);
    SkRRect rrect = SkRRect::MakeRectXY(SkRect::MakeXYWH(200, 20, 160, 100), 16, 16);
    canvas->drawRRect(rrect, rrectPaint);

    // Circle
    SkPaint circlePaint;
    circlePaint.setColor(SkColorSetRGB(251, 188, 4));  // Google Yellow
    circlePaint.setAntiAlias(true);
    canvas->drawCircle(100, 200, 60, circlePaint);

    // Stroked path (triangle)
    SkPaint pathPaint;
    pathPaint.setColor(SkColorSetRGB(52, 168, 83));  // Google Green
    pathPaint.setStyle(SkPaint::kStroke_Style);
    pathPaint.setStrokeWidth(3.0f);
    pathPaint.setAntiAlias(true);

    SkPath triangle = SkPathBuilder()
        .moveTo(280, 250)
        .lineTo(350, 150)
        .lineTo(420, 250)
        .close()
        .detach();
    canvas->drawPath(triangle, pathPaint);

    printf("  PASS: Drew rect, rrect, circle, path\n");
    return true;
}

static bool test_draw_text(SkCanvas* canvas) {
    auto scanner = SkFontScanner_Make_FreeType();
    sk_sp<SkFontMgr> fontMgr = SkFontMgr_New_FontConfig(nullptr, std::move(scanner));
    if (!fontMgr) {
        fontMgr = SkFontMgr::RefEmpty();
        printf("  WARN: FontConfig not available, using empty font manager\n");
    }

    sk_sp<SkTypeface> typeface = fontMgr->matchFamilyStyle("sans-serif", SkFontStyle::Normal());
    if (!typeface) {
        typeface = fontMgr->matchFamilyStyle(nullptr, SkFontStyle::Normal());
    }
    if (!typeface) {
        printf("  SKIP: No fonts available for text test\n");
        return true;  // not a failure, just no fonts
    }

    SkFont font(typeface, 20.0f);
    font.setEdging(SkFont::Edging::kSubpixelAntiAlias);

    SkPaint textPaint;
    textPaint.setColor(SK_ColorBLACK);

    canvas->drawString("Open UI - Skia Standalone", 20, 290, font, textPaint);

    printf("  PASS: Drew text with font\n");
    return true;
}

static bool test_gradient_shader(SkCanvas* canvas) {
    SkPoint pts[] = {{20, 310}, {380, 310}};
    SkColor4f colors[] = {
        SkColor4f::FromColor(SkColorSetRGB(66, 133, 244)),
        SkColor4f::FromColor(SkColorSetRGB(234, 67, 53))
    };
    float positions[] = {0.0f, 1.0f};

    SkGradient::Colors gradColors(
        SkSpan<const SkColor4f>(colors, 2),
        SkSpan<const float>(positions, 2),
        SkTileMode::kClamp);
    SkGradient grad(gradColors, {});

    SkPaint gradPaint;
    gradPaint.setShader(SkShaders::LinearGradient(pts, grad));

    canvas->drawRect(SkRect::MakeXYWH(20, 310, 360, 30), gradPaint);

    printf("  PASS: Drew linear gradient\n");
    return true;
}

static bool test_image_filter(SkCanvas* canvas) {
    SkPaint shadowPaint;
    shadowPaint.setColor(SkColorSetRGB(66, 133, 244));
    shadowPaint.setAntiAlias(true);
    shadowPaint.setImageFilter(
        SkImageFilters::DropShadow(4, 4, 3, 3, SK_ColorBLACK, nullptr));

    canvas->drawRect(SkRect::MakeXYWH(20, 360, 120, 60), shadowPaint);

    printf("  PASS: Drew drop shadow filter\n");
    return true;
}

static bool test_encode_png(SkSurface* surface, const char* path) {
    sk_sp<SkImage> image = surface->makeImageSnapshot();
    if (!image) {
        fprintf(stderr, "FAIL: Could not snapshot surface\n");
        return false;
    }

    SkFILEWStream stream(path);
    if (!stream.isValid()) {
        fprintf(stderr, "FAIL: Could not open %s for writing\n", path);
        return false;
    }

    SkPixmap pixmap;
    if (!image->peekPixels(&pixmap)) {
        fprintf(stderr, "FAIL: Could not peek pixels\n");
        return false;
    }

    if (!SkPngEncoder::Encode(&stream, pixmap, {})) {
        fprintf(stderr, "FAIL: PNG encode failed\n");
        return false;
    }

    printf("  PASS: Encoded PNG to %s\n", path);
    return true;
}

int main(int argc, char** argv) {
    const char* output = "skia_standalone_test.png";
    if (argc > 1) {
        output = argv[1];
    }

    printf("=== Skia Standalone Test ===\n");

    int failures = 0;

    // Test 1: Raster surface creation
    printf("[1] Raster surface:\n");
    if (!test_raster_surface()) failures++;

    // Test 2-6: Rendering
    auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(440, 440));
    SkCanvas* canvas = surface->getCanvas();

    printf("[2] Shapes:\n");
    if (!test_draw_shapes(canvas)) failures++;

    printf("[3] Text:\n");
    if (!test_draw_text(canvas)) failures++;

    printf("[4] Gradient:\n");
    if (!test_gradient_shader(canvas)) failures++;

    printf("[5] Image filter:\n");
    if (!test_image_filter(canvas)) failures++;

    printf("[6] PNG encode:\n");
    if (!test_encode_png(surface.get(), output)) failures++;

    printf("\n=== Results: %d failures ===\n", failures);
    return failures > 0 ? 1 : 0;
}
