// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_pixel_diff.cc — Pixel comparison implementation.

#include "openui/openui_pixel_diff.h"

#include <algorithm>
#include <cmath>
#include <cstdlib>

#include "third_party/skia/include/core/SkBitmap.h"

PixelDiffResult ComparePixels(const uint8_t* pixels_a,
                              const uint8_t* pixels_b,
                              int width,
                              int height,
                              int tolerance) {
  PixelDiffResult result = {};
  result.total_pixel_count = width * height;

  if (!pixels_a || !pixels_b || width <= 0 || height <= 0) {
    result.identical = false;
    result.max_channel_diff = 256;
    result.diff_percentage = 100.0;
    result.differing_pixel_count = result.total_pixel_count;
    return result;
  }

  int differing = 0;
  int max_diff = 0;

  for (int i = 0; i < width * height; i++) {
    int idx = i * 4;
    int dr = std::abs(static_cast<int>(pixels_a[idx + 0]) -
                      static_cast<int>(pixels_b[idx + 0]));
    int dg = std::abs(static_cast<int>(pixels_a[idx + 1]) -
                      static_cast<int>(pixels_b[idx + 1]));
    int db = std::abs(static_cast<int>(pixels_a[idx + 2]) -
                      static_cast<int>(pixels_b[idx + 2]));
    int da = std::abs(static_cast<int>(pixels_a[idx + 3]) -
                      static_cast<int>(pixels_b[idx + 3]));

    int pixel_max = std::max({dr, dg, db, da});
    max_diff = std::max(max_diff, pixel_max);

    if (pixel_max > tolerance) {
      differing++;
    }
  }

  result.max_channel_diff = max_diff;
  result.differing_pixel_count = differing;
  result.diff_percentage =
      (static_cast<double>(differing) / result.total_pixel_count) * 100.0;
  result.identical = (differing == 0);

  return result;
}

PixelDiffResult CompareBitmaps(const SkBitmap& bitmap_a,
                               const SkBitmap& bitmap_b,
                               int tolerance) {
  PixelDiffResult result = {};

  if (bitmap_a.width() != bitmap_b.width() ||
      bitmap_a.height() != bitmap_b.height()) {
    result.identical = false;
    result.max_channel_diff = 256;
    result.diff_percentage = 100.0;
    result.total_pixel_count =
        std::max(bitmap_a.width() * bitmap_a.height(),
                 bitmap_b.width() * bitmap_b.height());
    result.differing_pixel_count = result.total_pixel_count;
    return result;
  }

  int width = bitmap_a.width();
  int height = bitmap_a.height();
  result.total_pixel_count = width * height;

  if (width <= 0 || height <= 0) {
    result.identical = true;
    return result;
  }

  // Compare directly from the BGRA bitmap data (no RGBA conversion needed
  // since we're comparing corresponding channels).
  const uint8_t* src_a =
      static_cast<const uint8_t*>(bitmap_a.getPixels());
  const uint8_t* src_b =
      static_cast<const uint8_t*>(bitmap_b.getPixels());
  size_t row_bytes_a = bitmap_a.rowBytes();
  size_t row_bytes_b = bitmap_b.rowBytes();

  int differing = 0;
  int max_diff = 0;

  for (int y = 0; y < height; y++) {
    const uint8_t* row_a = src_a + y * row_bytes_a;
    const uint8_t* row_b = src_b + y * row_bytes_b;
    for (int x = 0; x < width; x++) {
      int idx = x * 4;
      int d0 = std::abs(static_cast<int>(row_a[idx + 0]) -
                        static_cast<int>(row_b[idx + 0]));
      int d1 = std::abs(static_cast<int>(row_a[idx + 1]) -
                        static_cast<int>(row_b[idx + 1]));
      int d2 = std::abs(static_cast<int>(row_a[idx + 2]) -
                        static_cast<int>(row_b[idx + 2]));
      int d3 = std::abs(static_cast<int>(row_a[idx + 3]) -
                        static_cast<int>(row_b[idx + 3]));

      int pixel_max = std::max({d0, d1, d2, d3});
      max_diff = std::max(max_diff, pixel_max);

      if (pixel_max > tolerance) {
        differing++;
      }
    }
  }

  result.max_channel_diff = max_diff;
  result.differing_pixel_count = differing;
  result.diff_percentage =
      (static_cast<double>(differing) / result.total_pixel_count) * 100.0;
  result.identical = (differing == 0);

  return result;
}
