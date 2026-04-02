// Minimal smoke test: can we link against Chromium's rendering libraries?
#include "base/at_exit.h"
#include "base/command_line.h"
#include "base/logging.h"
#include "third_party/skia/include/core/SkCanvas.h"
#include "third_party/skia/include/core/SkSurface.h"
#include "cc/paint/paint_canvas.h"
#include "cc/paint/skia_paint_canvas.h"
#include "cc/paint/paint_flags.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/gfx/geometry/size.h"

int main(int argc, char** argv) {
  base::AtExitManager at_exit;
  base::CommandLine::Init(argc, argv);

  // Test 1: Create a Skia surface
  auto surface = SkSurfaces::Raster(
      SkImageInfo::MakeN32Premul(800, 600));
  if (!surface) {
    LOG(ERROR) << "Failed to create Skia surface";
    return 1;
  }

  // Test 2: Use cc::PaintCanvas
  cc::SkiaPaintCanvas paint_canvas(surface->getCanvas());
  cc::PaintFlags flags;
  flags.setColor(SK_ColorRED);
  flags.setAntiAlias(true);
  paint_canvas.drawRect(SkRect::MakeXYWH(10, 10, 100, 50), flags);

  // Test 3: Use gfx types
  gfx::Rect rect(0, 0, 800, 600);
  gfx::Size size = rect.size();

  LOG(INFO) << "OpenUI smoke test passed! Surface: "
            << size.width() << "x" << size.height();
  return 0;
}
