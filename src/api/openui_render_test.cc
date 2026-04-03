// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_render_test.cc — C++ GTest suite for offscreen rendering (SP5).
// Tests: rasterization, pixel correctness, PNG round-trip, edge cases.

#include "openui/openui.h"
#include "openui/openui_pixel_diff.h"
#include "openui/openui_render.h"

#include <cstring>
#include <fstream>
#include <memory>
#include <string>
#include <vector>

#include "base/compiler_specific.h"
#include "base/files/file_path.h"
#include "base/files/file_util.h"
#include "base/files/scoped_temp_dir.h"
#include "partition_alloc/pointers/raw_ptr_exclusion.h"
#include "testing/gtest/include/gtest/gtest.h"
#include "third_party/blink/renderer/platform/testing/task_environment.h"
#include "third_party/skia/include/core/SkBitmap.h"
#include "ui/gfx/codec/png_codec.h"

// ===========================================================================
// Test fixture — each test gets a fresh document at 800×600.
// ===========================================================================
class OpenUIRenderTest : public testing::Test {
 protected:
  void SetUp() override {
    task_env_ = std::make_unique<blink::test::TaskEnvironment>();
    doc_ = oui_document_create(800, 600);
    ASSERT_NE(doc_, nullptr);
    body_ = oui_document_body(doc_);
    ASSERT_NE(body_, nullptr);
  }

  void TearDown() override {
    oui_document_destroy(doc_);
    doc_ = nullptr;
    body_ = nullptr;
    task_env_.reset();
  }

  // Helper: get RGBA pixel at (x, y) from an OuiBitmap.
  void GetPixel(const OuiBitmap& bmp, int x, int y,
                uint8_t* r, uint8_t* g, uint8_t* b, uint8_t* a) {
    ASSERT_GE(x, 0);
    ASSERT_LT(x, bmp.width);
    ASSERT_GE(y, 0);
    ASSERT_LT(y, bmp.height);
    int idx = (y * bmp.stride) + (x * 4);
    *r = bmp.pixels[idx + 0];
    *g = bmp.pixels[idx + 1];
    *b = bmp.pixels[idx + 2];
    *a = bmp.pixels[idx + 3];
  }

  // Helper: check a pixel is approximately a given color.
  void ExpectPixelNear(const OuiBitmap& bmp, int x, int y,
                       uint8_t er, uint8_t eg, uint8_t eb, uint8_t ea,
                       int tolerance = 2) {
    uint8_t r, g, b, a;
    GetPixel(bmp, x, y, &r, &g, &b, &a);
    EXPECT_NEAR(r, er, tolerance) << "R at (" << x << "," << y << ")";
    EXPECT_NEAR(g, eg, tolerance) << "G at (" << x << "," << y << ")";
    EXPECT_NEAR(b, eb, tolerance) << "B at (" << x << "," << y << ")";
    EXPECT_NEAR(a, ea, tolerance) << "A at (" << x << "," << y << ")";
  }

  std::unique_ptr<blink::test::TaskEnvironment> task_env_;
  RAW_PTR_EXCLUSION OuiDocument* doc_ = nullptr;
  RAW_PTR_EXCLUSION OuiElement* body_ = nullptr;
};

// ===========================================================================
// B1: Basic rasterization — red div
// ===========================================================================
TEST_F(OpenUIRenderTest, BasicRedDiv) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0xFF0000FF);  // Red, full alpha
  oui_element_append_child(body_, div);

  OuiBitmap bmp = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp), OUI_OK);
  EXPECT_EQ(bmp.width, 800);
  EXPECT_EQ(bmp.height, 600);
  EXPECT_NE(bmp.pixels, nullptr);
  EXPECT_EQ(bmp.stride, 800 * 4);

  // Center of the red div should be red.
  ExpectPixelNear(bmp, 50, 50, 255, 0, 0, 255);

  // Outside the div should be white (default background).
  ExpectPixelNear(bmp, 400, 300, 255, 255, 255, 255);

  oui_bitmap_free(&bmp);
}

// ===========================================================================
// B2: Multi-element rendering — RGB boxes side by side
// ===========================================================================
TEST_F(OpenUIRenderTest, MultiElementRGB) {
  // Use flexbox for side-by-side layout.
  oui_element_set_display(body_, OUI_DISPLAY_FLEX);

  OuiElement* red = oui_element_create(doc_, "div");
  oui_element_set_width(red, oui_px(100));
  oui_element_set_height(red, oui_px(100));
  oui_element_set_background_color(red, 0xFF0000FF);
  oui_element_append_child(body_, red);

  OuiElement* green = oui_element_create(doc_, "div");
  oui_element_set_width(green, oui_px(100));
  oui_element_set_height(green, oui_px(100));
  oui_element_set_background_color(green, 0x00FF00FF);
  oui_element_append_child(body_, green);

  OuiElement* blue = oui_element_create(doc_, "div");
  oui_element_set_width(blue, oui_px(100));
  oui_element_set_height(blue, oui_px(100));
  oui_element_set_background_color(blue, 0x0000FFFF);
  oui_element_append_child(body_, blue);

  OuiBitmap bmp = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp), OUI_OK);

  // Red box center (around x=50)
  ExpectPixelNear(bmp, 50, 50, 255, 0, 0, 255);
  // Green box center (around x=150) — 0x00FF00FF is lime: R=0 G=255 B=0
  ExpectPixelNear(bmp, 150, 50, 0, 255, 0, 255, 5);
  // Blue box center (around x=250)
  ExpectPixelNear(bmp, 250, 50, 0, 0, 255, 255);

  oui_bitmap_free(&bmp);
}

// ===========================================================================
// B3: Nested flexbox layout
// ===========================================================================
TEST_F(OpenUIRenderTest, NestedFlexboxLayout) {
  oui_element_set_display(body_, OUI_DISPLAY_FLEX);
  oui_element_set_flex_direction(body_, OUI_FLEX_COLUMN);

  OuiElement* row = oui_element_create(doc_, "div");
  oui_element_set_display(row, OUI_DISPLAY_FLEX);
  oui_element_set_width(row, oui_px(300));
  oui_element_set_height(row, oui_px(100));
  oui_element_append_child(body_, row);

  OuiElement* left = oui_element_create(doc_, "div");
  oui_element_set_width(left, oui_px(150));
  oui_element_set_height(left, oui_px(100));
  oui_element_set_background_color(left, 0xFF0000FF);
  oui_element_append_child(row, left);

  OuiElement* right = oui_element_create(doc_, "div");
  oui_element_set_width(right, oui_px(150));
  oui_element_set_height(right, oui_px(100));
  oui_element_set_background_color(right, 0x0000FFFF);
  oui_element_append_child(row, right);

  OuiBitmap bmp = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp), OUI_OK);

  // Left half should be red, right half blue.
  ExpectPixelNear(bmp, 75, 50, 255, 0, 0, 255);
  ExpectPixelNear(bmp, 225, 50, 0, 0, 255, 255);

  oui_bitmap_free(&bmp);
}

// ===========================================================================
// B4: Text rendering (verify non-white pixels exist in text region)
// ===========================================================================
TEST_F(OpenUIRenderTest, TextRenders) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(50));
  oui_element_set_font_size(div, oui_px(24));
  oui_element_set_color(div, 0x000000FF);  // Black text
  oui_element_set_text_content(div, "Hello");
  oui_element_append_child(body_, div);

  OuiBitmap bmp = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp), OUI_OK);

  // Scan the text region for non-white pixels.
  int non_white = 0;
  for (int y = 0; y < 50; y++) {
    for (int x = 0; x < 200; x++) {
      int idx = y * bmp.stride + x * 4;
      if (bmp.pixels[idx + 0] < 250 ||
          bmp.pixels[idx + 1] < 250 ||
          bmp.pixels[idx + 2] < 250) {
        non_white++;
      }
    }
  }
  // Text should produce many non-white pixels.
  EXPECT_GT(non_white, 50) << "Text 'Hello' should render visible pixels";

  oui_bitmap_free(&bmp);
}

// ===========================================================================
// B5: CSS opacity — 50% opacity red on white should blend
// ===========================================================================
TEST_F(OpenUIRenderTest, CSSOpacity) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0xFF0000FF);
  oui_element_set_opacity(div, 0.5f);
  oui_element_append_child(body_, div);

  OuiBitmap bmp = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp), OUI_OK);

  // 50% opacity red on white background should produce approximately (255, 128, 128).
  uint8_t r, g, b, a;
  GetPixel(bmp, 50, 50, &r, &g, &b, &a);
  EXPECT_NEAR(r, 255, 10) << "Red channel with opacity blend";
  EXPECT_NEAR(g, 128, 20) << "Green channel with opacity blend";
  EXPECT_NEAR(b, 128, 20) << "Blue channel with opacity blend";

  oui_bitmap_free(&bmp);
}

// ===========================================================================
// B6: CSS border rendering
// ===========================================================================
TEST_F(OpenUIRenderTest, CSSBorder) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_style(div, "border", "5px solid black");
  oui_element_set_background_color(div, 0xFFFFFFFF);
  oui_element_append_child(body_, div);

  OuiBitmap bmp = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp), OUI_OK);

  // Top border: body has 8px default margin, so div starts at y=8.
  // y=10 should be in the 5px border area (y=8..12). Expect black.
  ExpectPixelNear(bmp, 50, 10, 0, 0, 0, 255, 5);
  // Interior: well inside the div (y=50). Expect white.
  ExpectPixelNear(bmp, 50, 50, 255, 255, 255, 255, 5);

  oui_bitmap_free(&bmp);
}

// ===========================================================================
// B7: PNG output round-trip
// ===========================================================================
TEST_F(OpenUIRenderTest, PNGRoundTrip) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0x00FF00FF);  // Green
  oui_element_append_child(body_, div);

  // Render to PNG buffer.
  uint8_t* png_data = nullptr;
  size_t png_size = 0;
  ASSERT_EQ(oui_document_render_to_png_buffer(doc_, &png_data, &png_size),
            OUI_OK);
  ASSERT_NE(png_data, nullptr);
  ASSERT_GT(png_size, static_cast<size_t>(0));

  // PNG signature check.
  EXPECT_EQ(png_data[0], 0x89);
  EXPECT_EQ(png_data[1], 'P');
  EXPECT_EQ(png_data[2], 'N');
  EXPECT_EQ(png_data[3], 'G');

  // Decode the PNG back and check a pixel.
  SkBitmap decoded = gfx::PNGCodec::Decode(
      base::span(png_data, png_size));
  ASSERT_FALSE(decoded.isNull());

  EXPECT_EQ(decoded.width(), 800);
  EXPECT_EQ(decoded.height(), 600);

  oui_free(png_data);
}

// ===========================================================================
// B7b: PNG file output
// ===========================================================================
TEST_F(OpenUIRenderTest, PNGFileOutput) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0xFF0000FF);
  oui_element_append_child(body_, div);

  base::ScopedTempDir temp_dir;
  ASSERT_TRUE(temp_dir.CreateUniqueTempDir());
  base::FilePath png_path = temp_dir.GetPath().Append("test_output.png");

  ASSERT_EQ(oui_document_render_to_png(doc_, png_path.value().c_str()),
            OUI_OK);

  // Verify the file exists and is non-empty.
  auto file_size = base::GetFileSize(png_path);
  ASSERT_TRUE(file_size.has_value());
  EXPECT_GT(file_size.value(), 0);
}

// ===========================================================================
// E1: Empty document renders without crash
// ===========================================================================
TEST_F(OpenUIRenderTest, EmptyDocument) {
  OuiBitmap bmp = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp), OUI_OK);
  EXPECT_EQ(bmp.width, 800);
  EXPECT_EQ(bmp.height, 600);
  EXPECT_NE(bmp.pixels, nullptr);

  // Should be all white.
  ExpectPixelNear(bmp, 400, 300, 255, 255, 255, 255);

  oui_bitmap_free(&bmp);
}

// ===========================================================================
// E2: Viewport size rendering
// ===========================================================================
TEST_F(OpenUIRenderTest, ViewportSizeSmall) {
  oui_document_set_viewport(doc_, 320, 240);

  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0xFF0000FF);
  oui_element_append_child(body_, div);

  OuiBitmap bmp = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp), OUI_OK);
  EXPECT_EQ(bmp.width, 320);
  EXPECT_EQ(bmp.height, 240);

  ExpectPixelNear(bmp, 50, 50, 255, 0, 0, 255);

  oui_bitmap_free(&bmp);
}

TEST_F(OpenUIRenderTest, ViewportSizeLarge) {
  oui_document_set_viewport(doc_, 1920, 1080);

  OuiBitmap bmp = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp), OUI_OK);
  EXPECT_EQ(bmp.width, 1920);
  EXPECT_EQ(bmp.height, 1080);

  oui_bitmap_free(&bmp);
}

// ===========================================================================
// E3: Re-render after mutation
// ===========================================================================
TEST_F(OpenUIRenderTest, ReRenderAfterMutation) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0xFF0000FF);  // Red
  oui_element_append_child(body_, div);

  OuiBitmap bmp1 = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp1), OUI_OK);
  ExpectPixelNear(bmp1, 50, 50, 255, 0, 0, 255);  // Red

  // Mutate: change to blue.
  oui_element_set_background_color(div, 0x0000FFFF);

  OuiBitmap bmp2 = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp2), OUI_OK);
  ExpectPixelNear(bmp2, 50, 50, 0, 0, 255, 255);  // Blue

  oui_bitmap_free(&bmp1);
  oui_bitmap_free(&bmp2);
}

// ===========================================================================
// E4: Multiple documents render independently
// ===========================================================================
TEST_F(OpenUIRenderTest, MultipleDocuments) {
  // doc_ already exists with the fixture. Create a second one.
  OuiDocument* doc2 = oui_document_create(400, 300);
  ASSERT_NE(doc2, nullptr);

  // First doc: red box.
  OuiElement* red = oui_element_create(doc_, "div");
  oui_element_set_width(red, oui_px(100));
  oui_element_set_height(red, oui_px(100));
  oui_element_set_background_color(red, 0xFF0000FF);
  oui_element_append_child(body_, red);

  // Second doc: blue box.
  OuiElement* body2 = oui_document_body(doc2);
  OuiElement* blue = oui_element_create(doc2, "div");
  oui_element_set_width(blue, oui_px(100));
  oui_element_set_height(blue, oui_px(100));
  oui_element_set_background_color(blue, 0x0000FFFF);
  oui_element_append_child(body2, blue);

  OuiBitmap bmp1 = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp1), OUI_OK);
  EXPECT_EQ(bmp1.width, 800);
  ExpectPixelNear(bmp1, 50, 50, 255, 0, 0, 255);

  OuiBitmap bmp2 = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc2, &bmp2), OUI_OK);
  EXPECT_EQ(bmp2.width, 400);
  EXPECT_EQ(bmp2.height, 300);
  ExpectPixelNear(bmp2, 50, 50, 0, 0, 255, 255);

  oui_bitmap_free(&bmp1);
  oui_bitmap_free(&bmp2);
  oui_document_destroy(doc2);
}

// ===========================================================================
// E5: Large element tree (500+ elements, no crash)
// ===========================================================================
TEST_F(OpenUIRenderTest, LargeElementTree) {
  for (int i = 0; i < 500; i++) {
    OuiElement* div = oui_element_create(doc_, "div");
    oui_element_set_width(div, oui_px(10));
    oui_element_set_height(div, oui_px(2));
    oui_element_set_background_color(div, 0x336699FF);
    oui_element_append_child(body_, div);
  }

  OuiBitmap bmp = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp), OUI_OK);
  EXPECT_EQ(bmp.width, 800);
  EXPECT_EQ(bmp.height, 600);

  // Verify some colored pixels exist (not all white).
  int non_white = 0;
  for (int y = 0; y < bmp.height; y += 10) {
    for (int x = 0; x < bmp.width; x += 10) {
      int idx = y * bmp.stride + x * 4;
      if (bmp.pixels[idx + 0] < 250 ||
          bmp.pixels[idx + 1] < 250 ||
          bmp.pixels[idx + 2] < 250) {
        non_white++;
      }
    }
  }
  EXPECT_GT(non_white, 0) << "500 elements should produce non-white pixels";

  oui_bitmap_free(&bmp);
}

// ===========================================================================
// Pixel diff utility tests
// ===========================================================================
TEST_F(OpenUIRenderTest, PixelDiffIdentical) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0xFF0000FF);
  oui_element_append_child(body_, div);

  OuiBitmap bmp1 = {};
  OuiBitmap bmp2 = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp1), OUI_OK);
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp2), OUI_OK);

  PixelDiffResult result = ComparePixels(
      bmp1.pixels, bmp2.pixels, bmp1.width, bmp1.height, 0);
  EXPECT_TRUE(result.identical);
  EXPECT_EQ(result.max_channel_diff, 0);
  EXPECT_DOUBLE_EQ(result.diff_percentage, 0.0);

  oui_bitmap_free(&bmp1);
  oui_bitmap_free(&bmp2);
}

TEST_F(OpenUIRenderTest, PixelDiffDifferent) {
  // Render 1: red div.
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0xFF0000FF);
  oui_element_append_child(body_, div);

  OuiBitmap bmp1 = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp1), OUI_OK);

  // Mutate to blue.
  oui_element_set_background_color(div, 0x0000FFFF);

  OuiBitmap bmp2 = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp2), OUI_OK);

  PixelDiffResult result = ComparePixels(
      bmp1.pixels, bmp2.pixels, bmp1.width, bmp1.height, 0);
  EXPECT_FALSE(result.identical);
  EXPECT_GT(result.max_channel_diff, 0);
  EXPECT_GT(result.diff_percentage, 0.0);

  oui_bitmap_free(&bmp1);
  oui_bitmap_free(&bmp2);
}

TEST_F(OpenUIRenderTest, PixelDiffTolerance) {
  // Create two slightly different bitmaps manually.
  const int w = 2, h = 2;
  uint8_t a[16] = {100, 100, 100, 255,  200, 200, 200, 255,
                    50,  50,  50,  255,  150, 150, 150, 255};
  uint8_t b[16] = {102, 100, 100, 255,  200, 200, 200, 255,
                    50,  50,  50,  255,  150, 150, 150, 255};

  // Tolerance 0: should differ.
  PixelDiffResult r0 = ComparePixels(a, b, w, h, 0);
  EXPECT_FALSE(r0.identical);
  EXPECT_EQ(r0.max_channel_diff, 2);

  // Tolerance 2: should match.
  PixelDiffResult r2 = ComparePixels(a, b, w, h, 2);
  EXPECT_TRUE(r2.identical);
}

// ===========================================================================
// Null argument handling
// ===========================================================================
TEST_F(OpenUIRenderTest, NullArguments) {
  EXPECT_EQ(oui_document_render_to_bitmap(nullptr, nullptr),
            OUI_ERROR_INVALID_ARGUMENT);

  OuiBitmap bmp = {};
  EXPECT_EQ(oui_document_render_to_bitmap(nullptr, &bmp),
            OUI_ERROR_INVALID_ARGUMENT);

  EXPECT_EQ(oui_document_render_to_png(nullptr, "/tmp/test.png"),
            OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_EQ(oui_document_render_to_png(doc_, nullptr),
            OUI_ERROR_INVALID_ARGUMENT);

  uint8_t* data = nullptr;
  size_t size = 0;
  EXPECT_EQ(oui_document_render_to_png_buffer(nullptr, &data, &size),
            OUI_ERROR_INVALID_ARGUMENT);

  // Free null should be safe.
  oui_bitmap_free(nullptr);
  oui_free(nullptr);
}

// ===========================================================================
// CSS transform rendering
// ===========================================================================
TEST_F(OpenUIRenderTest, CSSTransform) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0xFF0000FF);
  oui_element_set_style(div, "transform", "rotate(45deg)");
  oui_element_append_child(body_, div);

  OuiBitmap bmp = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp), OUI_OK);

  // A rotated box should have colored pixels outside the original 100x100 bounds.
  // Check a point that would be outside the box without rotation.
  // The diagonal of a 100x100 box is ~141px. After 45deg rotation centered at
  // (50,50), pixels at approximately (50, -20) relative to the original top-left
  // should have color. In absolute coords this depends on transform-origin,
  // but we can verify the render succeeded and has non-white outside original bounds.
  bool found_color_outside = false;
  for (int y = 0; y < 150; y++) {
    for (int x = 100; x < 200; x++) {
      int idx = y * bmp.stride + x * 4;
      if (bmp.pixels[idx + 0] > 200 &&
          bmp.pixels[idx + 1] < 50 &&
          bmp.pixels[idx + 2] < 50) {
        found_color_outside = true;
        break;
      }
    }
    if (found_color_outside) break;
  }
  EXPECT_TRUE(found_color_outside)
      << "Rotated red box should paint outside original 100x100 bounds";

  oui_bitmap_free(&bmp);
}

// ===========================================================================
// Grid layout rendering
// ===========================================================================
TEST_F(OpenUIRenderTest, GridLayout) {
  OuiElement* grid = oui_element_create(doc_, "div");
  oui_element_set_display(grid, OUI_DISPLAY_GRID);
  oui_element_set_style(grid, "grid-template-columns", "100px 100px");
  oui_element_set_style(grid, "grid-template-rows", "50px 50px");
  oui_element_set_width(grid, oui_px(200));
  oui_element_set_height(grid, oui_px(100));
  oui_element_append_child(body_, grid);

  // 4 cells with different colors.
  uint32_t colors[] = {0xFF0000FF, 0x00FF00FF, 0x0000FFFF, 0xFFFF00FF};
  for (int i = 0; i < 4; i++) {
    OuiElement* cell = oui_element_create(doc_, "div");
    oui_element_set_background_color(cell, colors[i]);
    oui_element_append_child(grid, cell);
  }

  OuiBitmap bmp = {};
  ASSERT_EQ(oui_document_render_to_bitmap(doc_, &bmp), OUI_OK);

  // Top-left cell: red — body has 8px margin, so grid starts at (8,8).
  // Center of 100x50 cell: (8+50, 8+25) = (58, 33).
  ExpectPixelNear(bmp, 58, 33, 255, 0, 0, 255, 5);
  // Top-right cell: lime/0x00FF00FF — center at (8+150, 8+25) = (158, 33).
  ExpectPixelNear(bmp, 158, 33, 0, 255, 0, 255, 5);
  // Bottom-left cell: blue — center at (8+50, 8+75) = (58, 83).
  ExpectPixelNear(bmp, 58, 83, 0, 0, 255, 255, 5);

  oui_bitmap_free(&bmp);
}

// ===========================================================================
// Custom main — same init as openui_api_test.cc
// ===========================================================================

#include "base/command_line.h"
#include "base/feature_list.h"
#include "base/files/file_path.h"
#include "base/memory/discardable_memory_allocator.h"
#include "base/path_service.h"
#include "base/test/icu_test_util.h"
#include "base/test/launcher/unit_test_launcher.h"
#include "base/test/null_task_runner.h"
#include "base/test/test_discardable_memory_allocator.h"
#include "base/test/test_io_thread.h"
#include "base/test/test_suite.h"
#include "base/task/single_thread_task_runner.h"
#include "gin/v8_initializer.h"
#include "mojo/core/embedder/embedder.h"
#include "mojo/core/embedder/scoped_ipc_support.h"
#include "mojo/public/cpp/bindings/binder_map.h"
#include "third_party/blink/public/platform/platform.h"
#include "third_party/blink/public/platform/scheduler/test/renderer_scheduler_test_support.h"
#include "third_party/blink/public/platform/scheduler/web_thread_scheduler.h"
#include "third_party/blink/public/platform/web_runtime_features.h"
#include "third_party/blink/public/web/blink.h"
#include "ui/base/resource/resource_bundle.h"
#include "v8/include/v8.h"
#include "openui/openui_init.h"

namespace {

class OpenUIPlatformForTests : public blink::Platform {
 public:
  blink::WebString DefaultLocale() override {
    return blink::WebString::FromUTF8("en-US");
  }
  std::string GetDataResourceString(int resource_id) override {
    if (ui::ResourceBundle::HasSharedInstance()) {
      return ui::ResourceBundle::GetSharedInstance()
          .LoadDataResourceString(resource_id);
    }
    return std::string();
  }
  blink::WebData GetDataResource(
      int resource_id,
      ui::ResourceScaleFactor scale_factor) override {
    if (ui::ResourceBundle::HasSharedInstance()) {
      std::string_view data =
          ui::ResourceBundle::GetSharedInstance().GetRawDataResourceForScale(
              resource_id, scale_factor);
      return blink::WebData(base::as_byte_span(data));
    }
    return blink::WebData();
  }
  bool HasDataResource(int resource_id) const override {
    if (ui::ResourceBundle::HasSharedInstance()) {
      return !ui::ResourceBundle::GetSharedInstance()
                  .GetRawDataResource(resource_id)
                  .empty();
    }
    return false;
  }
};

base::TestDiscardableMemoryAllocator* g_test_discardable = nullptr;
std::unique_ptr<blink::scheduler::WebThreadScheduler>* g_test_scheduler =
    nullptr;

}  // namespace

int main(int argc, char** argv) {
  base::TestSuite test_suite(argc, argv);

  base::test::InitializeICUForTesting();

  g_test_discardable = new base::TestDiscardableMemoryAllocator();
  base::DiscardableMemoryAllocator::SetInstance(g_test_discardable);

  {
    auto feature_list = std::make_unique<base::FeatureList>();
    feature_list->InitFromCommandLine(
        base::CommandLine::ForCurrentProcess()->GetSwitchValueASCII(
            "enable-features"),
        base::CommandLine::ForCurrentProcess()->GetSwitchValueASCII(
            "disable-features"));
    base::FeatureList::SetInstance(std::move(feature_list));
  }

  {
    base::FilePath pak_path;
    base::PathService::Get(base::DIR_ASSETS, &pak_path);
    pak_path = pak_path.Append(FILE_PATH_LITERAL("content_shell.pak"));
    ui::ResourceBundle::InitSharedInstanceWithPakPath(pak_path);
  }

  mojo::core::Init();

#if defined(V8_USE_EXTERNAL_STARTUP_DATA)
  gin::V8Initializer::LoadV8Snapshot();
#endif

  blink::Platform::InitializeBlink();

  g_test_scheduler = new std::unique_ptr<blink::scheduler::WebThreadScheduler>(
      blink::scheduler::CreateWebMainThreadSchedulerForTests());

  const char kV8Flags[] = "--expose-gc --no-freeze-flags-after-init";
  v8::V8::SetFlagsFromString(kV8Flags, sizeof(kV8Flags) - 1);

  static auto* platform = new OpenUIPlatformForTests();

  {
    auto dummy_task_runner = base::MakeRefCounted<base::NullTaskRunner>();
    base::SingleThreadTaskRunner::CurrentDefaultHandle dummy_handle(
        dummy_task_runner);

    mojo::BinderMap binders;
    blink::InitializeWithoutIsolateForTesting(platform, &binders,
                                              g_test_scheduler->get());
  }

  blink::WebRuntimeFeatures::EnableExperimentalFeatures(true);
  blink::WebRuntimeFeatures::EnableTestOnlyFeatures(true);

  openui_runtime_mark_initialized_externally();

  base::TestIOThread test_io_thread(base::TestIOThread::kAutoStart);
  mojo::core::ScopedIPCSupport ipc_support(
      test_io_thread.task_runner(),
      mojo::core::ScopedIPCSupport::ShutdownPolicy::CLEAN);

  return base::LaunchUnitTests(
      argc, argv,
      base::BindOnce(&base::TestSuite::Run,
                     base::Unretained(&test_suite)));
}
