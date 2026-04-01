// Open UI — Skia Proof of Concept
//
// Validates that Skia can be built and used outside of Chromium's full build.
// Renders shapes, text, and gradients to a PNG file.
//
// Build:
//   ninja -C out/Debug skia_poc
//
// Run:
//   ./out/Debug/skia_poc
//   # → writes output.png

// TODO(sp1-e3): Uncomment and complete once Skia is building with our build system.
// This file is a template showing what the POC will do.

#if 0  // Enable once Skia headers are available

#include "include/core/SkCanvas.h"
#include "include/core/SkColor.h"
#include "include/core/SkFont.h"
#include "include/core/SkFontMgr.h"
#include "include/core/SkPaint.h"
#include "include/core/SkPath.h"
#include "include/core/SkRRect.h"
#include "include/core/SkSurface.h"
#include "include/core/SkTextBlob.h"
#include "include/encode/SkPngEncoder.h"

#include <cstdio>

constexpr int kWidth = 800;
constexpr int kHeight = 600;

// Draw a star path centered at (cx, cy) with given outer and inner radii.
SkPath make_star(float cx, float cy, float outer_r, float inner_r, int points) {
  SkPath path;
  float angle = -SK_ScalarPI / 2;  // Start at top
  float step = SK_ScalarPI / points;

  path.moveTo(cx + outer_r * cosf(angle), cy + outer_r * sinf(angle));
  for (int i = 1; i < points * 2; i++) {
    angle += step;
    float r = (i % 2 == 0) ? outer_r : inner_r;
    path.lineTo(cx + r * cosf(angle), cy + r * sinf(angle));
  }
  path.close();
  return path;
}

int main() {
  // 1. Create a raster surface
  auto info = SkImageInfo::MakeN32Premul(kWidth, kHeight);
  auto surface = SkSurfaces::Raster(info);
  if (!surface) {
    fprintf(stderr, "ERROR: Failed to create Skia surface\n");
    return 1;
  }

  SkCanvas* canvas = surface->getCanvas();

  // 2. Clear background to dark blue
  canvas->clear(SkColorSetRGB(0x1a, 0x1a, 0x2e));

  // 3. Draw a red rounded rectangle
  {
    SkPaint paint;
    paint.setColor(SkColorSetRGB(0xe9, 0x45, 0x60));
    paint.setAntiAlias(true);

    SkRRect rrect;
    rrect.setRectXY(SkRect::MakeXYWH(50, 50, 300, 200), 16, 16);
    canvas->drawRRect(rrect, paint);
  }

  // 4. Draw a gradient-filled circle
  {
    SkPoint pts[] = {{400, 150}, {600, 350}};
    SkColor colors[] = {SkColorSetRGB(0x0f, 0x34, 0x60), SkColorSetRGB(0x53, 0x34, 0x83)};
    auto shader = SkGradientShader::MakeLinear(pts, colors, nullptr, 2, SkTileMode::kClamp);

    SkPaint paint;
    paint.setAntiAlias(true);
    paint.setShader(shader);
    canvas->drawCircle(500, 250, 100, paint);
  }

  // 5. Draw "Hello, Open UI!" text
  {
    auto font_mgr = SkFontMgr::RefDefault();
    auto typeface = font_mgr->matchFamilyStyle("sans-serif", SkFontStyle::Normal());
    SkFont font(typeface, 48);

    SkPaint paint;
    paint.setColor(SK_ColorWHITE);
    paint.setAntiAlias(true);

    auto blob = SkTextBlob::MakeFromString("Hello, Open UI!", font);
    canvas->drawTextBlob(blob, 50, 350, paint);
  }

  // 6. Draw a star path
  {
    SkPath star = make_star(650, 500, 60, 25, 5);
    SkPaint paint;
    paint.setColor(SkColorSetRGB(0xff, 0xd7, 0x00));
    paint.setAntiAlias(true);
    canvas->drawPath(star, paint);

    // Star outline
    SkPaint stroke;
    stroke.setColor(SkColorSetRGB(0xff, 0xa5, 0x00));
    stroke.setAntiAlias(true);
    stroke.setStyle(SkPaint::kStroke_Style);
    stroke.setStrokeWidth(2);
    canvas->drawPath(star, stroke);
  }

  // 7. Encode to PNG and write to file
  auto image = surface->makeImageSnapshot();
  auto data = SkPngEncoder::Encode(nullptr, image.get(), {});
  if (!data) {
    fprintf(stderr, "ERROR: Failed to encode PNG\n");
    return 1;
  }

  FILE* f = fopen("output.png", "wb");
  if (!f) {
    fprintf(stderr, "ERROR: Failed to open output.png for writing\n");
    return 1;
  }
  fwrite(data->data(), 1, data->size(), f);
  fclose(f);

  printf("Success! Written output.png (%zu bytes)\n", data->size());
  printf("  - Dark blue background\n");
  printf("  - Red rounded rectangle (50,50 300x200)\n");
  printf("  - Gradient circle (center 500,250 r=100)\n");
  printf("  - 'Hello, Open UI!' text at (50,350)\n");
  printf("  - Gold star at (650,500)\n");
  return 0;
}

#else

#include <cstdio>

int main() {
  printf("Skia POC — placeholder\n");
  printf("\n");
  printf("This POC will render shapes, text, and gradients to output.png\n");
  printf("once Skia is building with our build system.\n");
  printf("\n");
  printf("See the #if 0 block in this file for the actual implementation.\n");
  printf("\n");
  printf("Next steps:\n");
  printf("  1. Complete SP1-A2: Chromium reference checkout\n");
  printf("  2. Complete SP1-A4: Sparse-checkout submodule\n");
  printf("  3. Complete SP1-D3: Integrate submodule in build\n");
  printf("  4. Enable the Skia code in this file\n");
  return 0;
}

#endif
