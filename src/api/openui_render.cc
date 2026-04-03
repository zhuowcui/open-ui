// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_render.cc — Offscreen rasterization implementation.
// Takes a document through the full paint lifecycle and replays
// the PaintRecord onto a Skia raster canvas to produce pixels.

#include "openui/openui_render.h"

#include <cstring>

#include "cc/paint/paint_record.h"
#include "cc/paint/skia_paint_canvas.h"
#include "third_party/blink/renderer/core/frame/local_frame_view.h"
#include "third_party/blink/renderer/core/paint/paint_layer.h"
#include "third_party/skia/include/core/SkBitmap.h"
#include "third_party/skia/include/core/SkCanvas.h"
#include "third_party/skia/include/core/SkColor.h"
#include "third_party/skia/include/core/SkImageInfo.h"
#include "third_party/skia/include/core/SkSurface.h"
#include "ui/gfx/codec/png_codec.h"
#include "ui/gfx/geometry/size.h"

bool OpenUIRasterize(OuiDocumentImpl* doc_impl, SkBitmap* out_bitmap) {
  if (!doc_impl || !doc_impl->page_holder || !out_bitmap) {
    return false;
  }

  blink::Document& document = doc_impl->GetDocument();
  blink::LocalFrameView* view = document.View();
  if (!view) {
    return false;
  }

  // Run the full lifecycle: style → layout → pre-paint → paint.
  view->UpdateAllLifecyclePhasesForTest();

  // Extract the paint record containing all drawing commands.
  cc::PaintRecord record = view->GetPaintRecord();

  // Get viewport dimensions for the bitmap.
  gfx::Size viewport_size = view->Size();
  int width = viewport_size.width();
  int height = viewport_size.height();
  if (width <= 0 || height <= 0) {
    return false;
  }

  // Allocate the bitmap: kN32_SkColorType is BGRA on little-endian.
  SkImageInfo info =
      SkImageInfo::MakeN32Premul(width, height);
  if (!out_bitmap->tryAllocPixels(info)) {
    return false;
  }
  out_bitmap->eraseColor(SK_ColorWHITE);

  // Create a Skia canvas targeting the bitmap and replay the paint record.
  SkCanvas canvas(*out_bitmap);
  cc::SkiaPaintCanvas paint_canvas(&canvas);
  record.Playback(&canvas);

  return true;
}

uint8_t* OpenUIBitmapToRGBA(const SkBitmap& bitmap, size_t* out_size) {
  int width = bitmap.width();
  int height = bitmap.height();
  if (width <= 0 || height <= 0 || !bitmap.getPixels()) {
    if (out_size) {
      *out_size = 0;
    }
    return nullptr;
  }

  size_t pixel_count = static_cast<size_t>(width) * height;
  size_t buffer_size = pixel_count * 4;
  uint8_t* rgba = static_cast<uint8_t*>(malloc(buffer_size));
  if (!rgba) {
    if (out_size) {
      *out_size = 0;
    }
    return nullptr;
  }

  // Convert from kN32 (BGRA on little-endian) to RGBA.
  const uint8_t* src = static_cast<const uint8_t*>(bitmap.getPixels());
  size_t src_row_bytes = bitmap.rowBytes();

  for (int y = 0; y < height; y++) {
    const uint8_t* src_row = src + y * src_row_bytes;
    uint8_t* dst_row = rgba + y * width * 4;
    for (int x = 0; x < width; x++) {
      // Source: BGRA (kN32 on little-endian)
      uint8_t b = src_row[x * 4 + 0];
      uint8_t g = src_row[x * 4 + 1];
      uint8_t r = src_row[x * 4 + 2];
      uint8_t a = src_row[x * 4 + 3];

      // Unpremultiply alpha if needed (premul → straight).
      if (a > 0 && a < 255) {
        r = static_cast<uint8_t>(std::min(255, (r * 255 + a / 2) / a));
        g = static_cast<uint8_t>(std::min(255, (g * 255 + a / 2) / a));
        b = static_cast<uint8_t>(std::min(255, (b * 255 + a / 2) / a));
      }

      // Destination: RGBA
      dst_row[x * 4 + 0] = r;
      dst_row[x * 4 + 1] = g;
      dst_row[x * 4 + 2] = b;
      dst_row[x * 4 + 3] = a;
    }
  }

  if (out_size) {
    *out_size = buffer_size;
  }
  return rgba;
}

bool OpenUIEncodePNG(const SkBitmap& bitmap, std::vector<uint8_t>* out_data) {
  if (!out_data || bitmap.drawsNothing()) {
    return false;
  }

  auto result = gfx::PNGCodec::EncodeBGRASkBitmap(bitmap,
                                                    /*discard_transparency=*/false);
  if (!result.has_value()) {
    return false;
  }

  *out_data = std::move(result.value());
  return true;
}
