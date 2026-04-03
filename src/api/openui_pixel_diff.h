// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_pixel_diff.h — Pixel comparison utility for render tests.
// Compares two RGBA bitmaps and reports differences.

#ifndef OPENUI_OPENUI_PIXEL_DIFF_H_
#define OPENUI_OPENUI_PIXEL_DIFF_H_

#include <stddef.h>
#include <stdint.h>

// Result of comparing two bitmaps pixel-by-pixel.
struct PixelDiffResult {
  bool identical;               // True if all pixels match within tolerance.
  int max_channel_diff;         // Maximum single-channel difference found.
  double diff_percentage;       // Percentage of pixels that differ (0.0–100.0).
  int differing_pixel_count;    // Absolute count of differing pixels.
  int total_pixel_count;        // Total pixels compared.
};

// Compare two RGBA pixel buffers of the same dimensions.
// |tolerance| is per-channel: if all channels of a pixel differ by at most
// |tolerance|, that pixel is considered matching (0 = exact match).
// Returns a PixelDiffResult. If dimensions don't match, returns non-identical
// with max_channel_diff = 256.
PixelDiffResult ComparePixels(const uint8_t* pixels_a,
                              const uint8_t* pixels_b,
                              int width,
                              int height,
                              int tolerance = 0);

// Compare two SkBitmaps (both kN32_SkColorType).
// Converts to RGBA internally and delegates to ComparePixels.
class SkBitmap;
PixelDiffResult CompareBitmaps(const SkBitmap& bitmap_a,
                               const SkBitmap& bitmap_b,
                               int tolerance = 0);

#endif  // OPENUI_OPENUI_PIXEL_DIFF_H_
