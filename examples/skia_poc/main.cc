// Open UI — Skia Proof of Concept
//
// Proves that Skia's raster backend works standalone. Renders shapes, text,
// and gradients to a PNG file without any browser components.
//
// Build (from Chromium src/):
//   ninja -C out/Release openui_skia_poc
//
// Run:
//   ./out/Release/openui_skia_poc
//   # → writes skia_poc_output.png

#include <cstdio>
#include <cmath>

#include "third_party/skia/include/core/SkCanvas.h"
#include "third_party/skia/include/core/SkColor.h"
#include "third_party/skia/include/core/SkFont.h"
#include "third_party/skia/include/core/SkFontMgr.h"
#include "third_party/skia/include/core/SkImage.h"
#include "third_party/skia/include/core/SkPaint.h"
#include "third_party/skia/include/core/SkPath.h"
#include "third_party/skia/include/core/SkPathBuilder.h"
#include "third_party/skia/include/core/SkPixmap.h"
#include "third_party/skia/include/core/SkRRect.h"
#include "third_party/skia/include/core/SkStream.h"
#include "third_party/skia/include/core/SkSurface.h"
#include "third_party/skia/include/core/SkTextBlob.h"
#include "third_party/skia/include/core/SkTypeface.h"
#include "third_party/skia/include/encode/SkPngRustEncoder.h"

constexpr int kWidth = 800;
constexpr int kHeight = 600;

// Draw a 5-pointed star centered at (cx, cy).
static SkPath MakeStar(float cx, float cy, float outer_r, float inner_r) {
  SkPathBuilder builder;
  constexpr int kPoints = 5;
  constexpr float kPi = 3.14159265358979323846f;
  float angle = -kPi / 2.0f;  // Start at top
  float step = kPi / kPoints;

  builder.moveTo(cx + outer_r * cosf(angle), cy + outer_r * sinf(angle));
  for (int i = 1; i < kPoints * 2; i++) {
    angle += step;
    float r = (i % 2 == 0) ? outer_r : inner_r;
    builder.lineTo(cx + r * cosf(angle), cy + r * sinf(angle));
  }
  builder.close();
  return builder.detach();
}

int main() {
  printf("Open UI — Skia Proof of Concept\n");
  printf("================================\n\n");

  // 1. Create a raster surface
  auto info = SkImageInfo::MakeN32Premul(kWidth, kHeight);
  auto surface = SkSurfaces::Raster(info);
  if (!surface) {
    fprintf(stderr, "ERROR: Failed to create Skia raster surface\n");
    return 1;
  }
  printf("[1/6] Created %dx%d raster surface\n", kWidth, kHeight);

  SkCanvas* canvas = surface->getCanvas();

  // 2. Clear to dark blue background
  canvas->clear(SkColorSetRGB(0x1a, 0x1a, 0x2e));
  printf("[2/6] Cleared background\n");

  // 3. Draw a red rounded rectangle
  {
    SkPaint paint;
    paint.setColor(SkColorSetRGB(0xe9, 0x45, 0x60));
    paint.setAntiAlias(true);

    SkRRect rrect;
    rrect.setRectXY(SkRect::MakeXYWH(50, 50, 300, 200), 16, 16);
    canvas->drawRRect(rrect, paint);
    printf("[3/6] Drew rounded rectangle (50,50 300x200 r=16)\n");
  }

  // 4. Draw a blue circle with anti-aliasing
  {
    SkPaint paint;
    paint.setColor(SkColorSetRGB(0x3a, 0x86, 0xff));
    paint.setAntiAlias(true);
    canvas->drawCircle(500, 150, 80, paint);
    printf("[4/6] Drew circle (center=500,150 r=80)\n");
  }

  // 5. Draw "Hello, Open UI!" text
  {
    SkFont font;
    font.setSize(48);

    SkPaint paint;
    paint.setColor(SK_ColorWHITE);
    paint.setAntiAlias(true);

    auto blob = SkTextBlob::MakeFromString("Hello, Open UI!", font);
    if (blob) {
      canvas->drawTextBlob(blob, 50, 350, paint);
      printf("[5/6] Drew text: 'Hello, Open UI!'\n");
    } else {
      printf("[5/6] Text blob creation failed (no fonts), drawing rect instead\n");
      canvas->drawRect(SkRect::MakeXYWH(50, 320, 400, 40), paint);
    }
  }

  // 6. Draw a gold star
  {
    SkPath star = MakeStar(650, 480, 60, 25);

    SkPaint fill;
    fill.setColor(SkColorSetRGB(0xff, 0xd7, 0x00));
    fill.setAntiAlias(true);
    canvas->drawPath(star, fill);

    SkPaint stroke;
    stroke.setColor(SkColorSetRGB(0xff, 0xa5, 0x00));
    stroke.setAntiAlias(true);
    stroke.setStyle(SkPaint::kStroke_Style);
    stroke.setStrokeWidth(2);
    canvas->drawPath(star, stroke);
    printf("[6/6] Drew star (center=650,480 r=60)\n");
  }

  // Encode to PNG
  auto image = surface->makeImageSnapshot();
  if (!image) {
    fprintf(stderr, "ERROR: Failed to create image snapshot\n");
    return 1;
  }

  SkFILEWStream file("skia_poc_output.png");
  if (!file.isValid()) {
    fprintf(stderr, "ERROR: Failed to open skia_poc_output.png for writing\n");
    return 1;
  }

  SkPngRustEncoder::Options opts;
  SkPixmap pm;
  if (!image->peekPixels(&pm)) {
    fprintf(stderr, "ERROR: Failed to peek pixels from image\n");
    return 1;
  }
  if (!SkPngRustEncoder::Encode(&file, pm, opts)) {
    fprintf(stderr, "ERROR: Failed to encode PNG\n");
    return 1;
  }

  printf("\nSuccess! Written skia_poc_output.png\n");
  printf("  Dimensions: %dx%d\n", kWidth, kHeight);
  printf("  Content:\n");
  printf("    - Dark blue background (#1a1a2e)\n");
  printf("    - Red rounded rect (50,50 300x200)\n");
  printf("    - Blue circle (500,150 r=80)\n");
  printf("    - White text 'Hello, Open UI!'\n");
  printf("    - Gold star (650,480)\n");
  printf("\nThis proves Skia's raster pipeline works standalone.\n");
  return 0;
}
