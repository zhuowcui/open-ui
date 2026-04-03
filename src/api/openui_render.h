// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_render.h — Internal header for offscreen rasterization.
// Provides functions to rasterize a document's paint output into pixels.

#ifndef OPENUI_OPENUI_RENDER_H_
#define OPENUI_OPENUI_RENDER_H_

#include <vector>

#include "openui/openui.h"
#include "openui/openui_impl.h"
#include "third_party/skia/include/core/SkBitmap.h"

// Rasterize the document's current paint output into an SkBitmap.
// Runs UpdateAllLifecyclePhasesForTest(), gets the PaintRecord, and
// replays it onto a raster SkCanvas backed by the returned SkBitmap.
// The bitmap uses kN32_SkColorType (BGRA on little-endian).
// Returns true on success, false if paint record is unavailable.
bool OpenUIRasterize(OuiDocumentImpl* doc_impl, SkBitmap* out_bitmap);

// Convert an SkBitmap (kN32_SkColorType / BGRA) to an RGBA pixel buffer.
// Returns a heap-allocated buffer that the caller must free().
// Sets out_size to width * height * 4.
uint8_t* OpenUIBitmapToRGBA(const SkBitmap& bitmap, size_t* out_size);

// Encode an SkBitmap to PNG in memory.
// Returns the PNG data in out_data, or empty on failure.
bool OpenUIEncodePNG(const SkBitmap& bitmap, std::vector<uint8_t>* out_data);

#endif  // OPENUI_OPENUI_RENDER_H_
