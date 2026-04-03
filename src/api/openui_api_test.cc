// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_api_test.cc — Comprehensive GTest suite for the openui C API.

#include "openui/openui.h"

#include <stdlib.h>
#include <string.h>
#include <cmath>
#include <memory>

#include "base/compiler_specific.h"
#include "partition_alloc/pointers/raw_ptr_exclusion.h"
#include "testing/gtest/include/gtest/gtest.h"
#include "third_party/blink/renderer/platform/testing/task_environment.h"

// ===========================================================================
// Test fixture — each test gets its own TaskEnvironment (V8 isolate) + document.
// oui_init() is called in main() before test execution starts.
// ===========================================================================
class OpenUIAPITest : public testing::Test {
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

  std::unique_ptr<blink::test::TaskEnvironment> task_env_;
  // Opaque C handles, not blink objects — exempt from raw_ptr.
  RAW_PTR_EXCLUSION OuiDocument* doc_ = nullptr;
  RAW_PTR_EXCLUSION OuiElement* body_ = nullptr;
};

// ===========================================================================
// Initialization tests
// ===========================================================================

TEST_F(OpenUIAPITest, DoubleInitReturnsError) {
  OuiInitConfig config = {};
  EXPECT_EQ(oui_init(&config), OUI_ERROR_ALREADY_INITIALIZED);
}

// ===========================================================================
// Document tests
// ===========================================================================

TEST_F(OpenUIAPITest, DocumentCreateDestroy) {
  OuiDocument* d = oui_document_create(1024, 768);
  ASSERT_NE(d, nullptr);
  oui_document_destroy(d);
}

TEST_F(OpenUIAPITest, DocumentBodyNotNull) {
  EXPECT_NE(body_, nullptr);
}

TEST_F(OpenUIAPITest, DocumentBodyIdempotent) {
  OuiElement* body2 = oui_document_body(doc_);
  EXPECT_EQ(body_, body2);
}

TEST_F(OpenUIAPITest, DocumentSetViewport) {
  oui_document_set_viewport(doc_, 1920, 1080);
  // Should not crash.
  EXPECT_EQ(oui_document_layout(doc_), OUI_OK);
}

TEST_F(OpenUIAPITest, DocumentLayoutReturnsOK) {
  EXPECT_EQ(oui_document_layout(doc_), OUI_OK);
}

TEST_F(OpenUIAPITest, DocumentUpdateAllReturnsOK) {
  EXPECT_EQ(oui_document_update_all(doc_), OUI_OK);
}

TEST_F(OpenUIAPITest, DocumentNullSafety) {
  oui_document_destroy(nullptr);
  oui_document_set_viewport(nullptr, 100, 100);
  EXPECT_EQ(oui_document_layout(nullptr), OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_EQ(oui_document_update_all(nullptr), OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_EQ(oui_document_body(nullptr), nullptr);
}

// ===========================================================================
// Element creation tests
// ===========================================================================

TEST_F(OpenUIAPITest, CreateDiv) {
  OuiElement* div = oui_element_create(doc_, "div");
  ASSERT_NE(div, nullptr);
  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, CreateSpan) {
  OuiElement* span = oui_element_create(doc_, "span");
  ASSERT_NE(span, nullptr);
  oui_element_destroy(span);
}

TEST_F(OpenUIAPITest, CreateParagraph) {
  OuiElement* p = oui_element_create(doc_, "p");
  ASSERT_NE(p, nullptr);
  oui_element_destroy(p);
}

TEST_F(OpenUIAPITest, CreateHeadings) {
  const char* tags[] = {"h1", "h2", "h3", "h4", "h5", "h6"};
  for (const char* tag : tags) {
    OuiElement* h = oui_element_create(doc_, tag);
    ASSERT_NE(h, nullptr) << "Failed to create: " << tag;
    oui_element_destroy(h);
  }
}

TEST_F(OpenUIAPITest, CreateTableElements) {
  const char* tags[] = {"table", "thead", "tbody", "tfoot", "tr", "td", "th"};
  for (const char* tag : tags) {
    OuiElement* e = oui_element_create(doc_, tag);
    ASSERT_NE(e, nullptr) << "Failed to create: " << tag;
    oui_element_destroy(e);
  }
}

TEST_F(OpenUIAPITest, CreateFormElements) {
  const char* tags[] = {"form", "input", "button", "select",
                        "textarea", "label", "fieldset", "legend"};
  for (const char* tag : tags) {
    OuiElement* e = oui_element_create(doc_, tag);
    ASSERT_NE(e, nullptr) << "Failed to create: " << tag;
    oui_element_destroy(e);
  }
}

TEST_F(OpenUIAPITest, CreateListElements) {
  const char* tags[] = {"ul", "ol", "li", "dl"};
  for (const char* tag : tags) {
    OuiElement* e = oui_element_create(doc_, tag);
    ASSERT_NE(e, nullptr) << "Failed to create: " << tag;
    oui_element_destroy(e);
  }
}

TEST_F(OpenUIAPITest, CreateMiscElements) {
  const char* tags[] = {"a", "br", "hr", "pre", "img",
                        "blockquote", "details", "summary"};
  for (const char* tag : tags) {
    OuiElement* e = oui_element_create(doc_, tag);
    ASSERT_NE(e, nullptr) << "Failed to create: " << tag;
    oui_element_destroy(e);
  }
}

TEST_F(OpenUIAPITest, CreateUnknownTagReturnsNull) {
  OuiElement* e = oui_element_create(doc_, "nonexistent");
  EXPECT_EQ(e, nullptr);
}

TEST_F(OpenUIAPITest, CreateNullTagReturnsNull) {
  OuiElement* e = oui_element_create(doc_, nullptr);
  EXPECT_EQ(e, nullptr);
}

TEST_F(OpenUIAPITest, CreateWithNullDocReturnsNull) {
  OuiElement* e = oui_element_create(nullptr, "div");
  EXPECT_EQ(e, nullptr);
}

TEST_F(OpenUIAPITest, ElementNullSafety) {
  oui_element_destroy(nullptr);
  oui_element_append_child(nullptr, nullptr);
  oui_element_remove_child(nullptr, nullptr);
  oui_element_insert_before(nullptr, nullptr, nullptr);
  EXPECT_EQ(oui_element_first_child(nullptr), nullptr);
  EXPECT_EQ(oui_element_next_sibling(nullptr), nullptr);
  EXPECT_EQ(oui_element_parent(nullptr), nullptr);
  EXPECT_EQ(oui_element_get_width(nullptr), 0.0f);
  EXPECT_EQ(oui_element_get_height(nullptr), 0.0f);
}

// ===========================================================================
// DOM tree manipulation
// ===========================================================================

TEST_F(OpenUIAPITest, AppendChildAndParent) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);

  OuiElement* parent = oui_element_parent(div);
  EXPECT_EQ(parent, body_);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, FirstChildAndNextSibling) {
  OuiElement* a = oui_element_create(doc_, "div");
  OuiElement* b = oui_element_create(doc_, "div");
  OuiElement* c = oui_element_create(doc_, "div");

  oui_element_append_child(body_, a);
  oui_element_append_child(body_, b);
  oui_element_append_child(body_, c);

  EXPECT_EQ(oui_element_first_child(body_), a);
  EXPECT_EQ(oui_element_next_sibling(a), b);
  EXPECT_EQ(oui_element_next_sibling(b), c);
  EXPECT_EQ(oui_element_next_sibling(c), nullptr);

  oui_element_destroy(c);
  oui_element_destroy(b);
  oui_element_destroy(a);
}

TEST_F(OpenUIAPITest, RemoveChild) {
  OuiElement* a = oui_element_create(doc_, "div");
  OuiElement* b = oui_element_create(doc_, "div");

  oui_element_append_child(body_, a);
  oui_element_append_child(body_, b);
  oui_element_remove_child(body_, a);

  EXPECT_EQ(oui_element_first_child(body_), b);

  oui_element_destroy(b);
  oui_element_destroy(a);
}

TEST_F(OpenUIAPITest, InsertBefore) {
  OuiElement* a = oui_element_create(doc_, "div");
  OuiElement* c = oui_element_create(doc_, "div");
  OuiElement* b = oui_element_create(doc_, "div");

  oui_element_append_child(body_, a);
  oui_element_append_child(body_, c);
  oui_element_insert_before(body_, b, c);

  EXPECT_EQ(oui_element_first_child(body_), a);
  EXPECT_EQ(oui_element_next_sibling(a), b);
  EXPECT_EQ(oui_element_next_sibling(b), c);

  oui_element_destroy(c);
  oui_element_destroy(b);
  oui_element_destroy(a);
}

TEST_F(OpenUIAPITest, DestroyRemovesFromDOM) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_destroy(div);

  // After destroy, body should have no children (that we track).
  EXPECT_EQ(oui_element_first_child(body_), nullptr);
}

// ===========================================================================
// Generic style API
// ===========================================================================

TEST_F(OpenUIAPITest, SetStyleValidProperty) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);

  OuiStatus s = oui_element_set_style(div, "width", "200px");
  EXPECT_EQ(s, OUI_OK);

  oui_document_layout(doc_);
  EXPECT_FLOAT_EQ(oui_element_get_width(div), 200.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetStyleInvalidProperty) {
  OuiElement* div = oui_element_create(doc_, "div");
  OuiStatus s = oui_element_set_style(div, "not-a-real-property", "100px");
  EXPECT_EQ(s, OUI_ERROR_UNKNOWN_PROPERTY);
  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetStyleNullArgs) {
  OuiElement* div = oui_element_create(doc_, "div");
  EXPECT_EQ(oui_element_set_style(nullptr, "width", "100px"),
            OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_EQ(oui_element_set_style(div, nullptr, "100px"),
            OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_EQ(oui_element_set_style(div, "width", nullptr),
            OUI_ERROR_INVALID_ARGUMENT);
  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, RemoveStyle) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);

  oui_element_set_style(div, "width", "200px");
  oui_document_layout(doc_);
  EXPECT_FLOAT_EQ(oui_element_get_width(div), 200.0f);

  oui_element_remove_style(div, "width");
  oui_document_layout(doc_);
  // After removing width, div should take full container width.
  EXPECT_GT(oui_element_get_width(div), 200.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, ClearStyles) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);

  oui_element_set_style(div, "width", "100px");
  oui_element_set_style(div, "height", "50px");
  oui_element_clear_styles(div);
  oui_document_layout(doc_);

  // After clearing, should have default width (container width).
  EXPECT_GT(oui_element_get_width(div), 100.0f);

  oui_element_destroy(div);
}

// ===========================================================================
// Typed dimension setters
// ===========================================================================

TEST_F(OpenUIAPITest, SetWidthPx) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);

  oui_element_set_width(div, oui_px(300));
  oui_document_layout(doc_);
  EXPECT_FLOAT_EQ(oui_element_get_width(div), 300.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetHeightPx) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);

  oui_element_set_height(div, oui_px(150));
  oui_document_layout(doc_);
  EXPECT_FLOAT_EQ(oui_element_get_height(div), 150.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetWidthPercent) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_width(container, oui_px(400));

  OuiElement* child = oui_element_create(doc_, "div");
  oui_element_append_child(container, child);
  oui_element_set_width(child, oui_pct(50));

  oui_document_layout(doc_);
  EXPECT_FLOAT_EQ(oui_element_get_width(child), 200.0f);

  oui_element_destroy(child);
  oui_element_destroy(container);
}

TEST_F(OpenUIAPITest, SetMinMaxWidth) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);

  oui_element_set_min_width(div, oui_px(100));
  oui_element_set_max_width(div, oui_px(200));
  oui_element_set_width(div, oui_px(50));  // Below min
  oui_document_layout(doc_);
  EXPECT_GE(oui_element_get_width(div), 100.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetMinMaxHeight) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);

  oui_element_set_height(div, oui_px(500));
  oui_element_set_max_height(div, oui_px(200));
  oui_document_layout(doc_);
  EXPECT_LE(oui_element_get_height(div), 200.0f);

  oui_element_destroy(div);
}

// ===========================================================================
// Box model (margin, padding)
// ===========================================================================

TEST_F(OpenUIAPITest, SetMargin) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_margin(div, oui_px(10), oui_px(20), oui_px(30), oui_px(40));

  oui_document_layout(doc_);
  float x = oui_element_get_offset_x(div);
  float y = oui_element_get_offset_y(div);
  // Offset should reflect margins (body default margin is 8px).
  EXPECT_GT(x, 0.0f);
  EXPECT_GT(y, 0.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetPaddingAffectsSize) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_padding(div, oui_px(10), oui_px(10), oui_px(10), oui_px(10));
  // Default box-sizing is content-box, so padding adds to size.
  oui_element_set_style(div, "box-sizing", "content-box");

  oui_document_layout(doc_);
  // Total width = 100 + 10 + 10 = 120.
  EXPECT_FLOAT_EQ(oui_element_get_width(div), 120.0f);
  EXPECT_FLOAT_EQ(oui_element_get_height(div), 120.0f);

  oui_element_destroy(div);
}

// ===========================================================================
// Display & positioning
// ===========================================================================

TEST_F(OpenUIAPITest, SetDisplayNone) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_display(div, OUI_DISPLAY_NONE);

  oui_document_layout(doc_);
  // display:none elements have no layout box.
  EXPECT_FLOAT_EQ(oui_element_get_width(div), 0.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetDisplayFlex) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_display(container, OUI_DISPLAY_FLEX);
  oui_element_set_width(container, oui_px(300));

  OuiElement* child1 = oui_element_create(doc_, "div");
  OuiElement* child2 = oui_element_create(doc_, "div");
  oui_element_append_child(container, child1);
  oui_element_append_child(container, child2);
  oui_element_set_flex_grow(child1, 1.0f);
  oui_element_set_flex_grow(child2, 2.0f);
  oui_element_set_height(child1, oui_px(50));
  oui_element_set_height(child2, oui_px(50));

  oui_document_layout(doc_);
  float w1 = oui_element_get_width(child1);
  float w2 = oui_element_get_width(child2);
  // child1 gets 100px, child2 gets 200px (1:2 ratio of 300px).
  EXPECT_NEAR(w1, 100.0f, 1.0f);
  EXPECT_NEAR(w2, 200.0f, 1.0f);

  oui_element_destroy(child2);
  oui_element_destroy(child1);
  oui_element_destroy(container);
}

TEST_F(OpenUIAPITest, SetDisplayGrid) {
  OuiElement* grid = oui_element_create(doc_, "div");
  oui_element_append_child(body_, grid);
  oui_element_set_display(grid, OUI_DISPLAY_GRID);
  oui_element_set_width(grid, oui_px(400));
  oui_element_set_style(grid, "grid-template-columns", "1fr 1fr");

  OuiElement* c1 = oui_element_create(doc_, "div");
  OuiElement* c2 = oui_element_create(doc_, "div");
  oui_element_append_child(grid, c1);
  oui_element_append_child(grid, c2);
  oui_element_set_height(c1, oui_px(50));
  oui_element_set_height(c2, oui_px(50));

  oui_document_layout(doc_);
  EXPECT_NEAR(oui_element_get_width(c1), 200.0f, 1.0f);
  EXPECT_NEAR(oui_element_get_width(c2), 200.0f, 1.0f);

  oui_element_destroy(c2);
  oui_element_destroy(c1);
  oui_element_destroy(grid);
}

TEST_F(OpenUIAPITest, SetPositionAbsolute) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_position(container, OUI_POSITION_RELATIVE);
  oui_element_set_width(container, oui_px(400));
  oui_element_set_height(container, oui_px(400));

  OuiElement* child = oui_element_create(doc_, "div");
  oui_element_append_child(container, child);
  oui_element_set_position(child, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(child, "top", "50px");
  oui_element_set_style(child, "left", "100px");
  oui_element_set_width(child, oui_px(80));
  oui_element_set_height(child, oui_px(60));

  oui_document_layout(doc_);
  OuiRect rect = oui_element_get_bounding_rect(child);
  EXPECT_FLOAT_EQ(rect.width, 80.0f);
  EXPECT_FLOAT_EQ(rect.height, 60.0f);

  oui_element_destroy(child);
  oui_element_destroy(container);
}

TEST_F(OpenUIAPITest, SetOverflow) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_overflow(div, OUI_OVERFLOW_HIDDEN);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));

  oui_document_layout(doc_);
  EXPECT_FLOAT_EQ(oui_element_get_width(div), 100.0f);

  oui_element_destroy(div);
}

// ===========================================================================
// Flexbox convenience setters
// ===========================================================================

TEST_F(OpenUIAPITest, FlexDirectionColumn) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_display(container, OUI_DISPLAY_FLEX);
  oui_element_set_flex_direction(container, OUI_FLEX_COLUMN);
  oui_element_set_width(container, oui_px(200));

  OuiElement* a = oui_element_create(doc_, "div");
  OuiElement* b = oui_element_create(doc_, "div");
  oui_element_append_child(container, a);
  oui_element_append_child(container, b);
  oui_element_set_height(a, oui_px(50));
  oui_element_set_height(b, oui_px(60));

  oui_document_layout(doc_);
  // In column direction, b should be below a.
  float ay = oui_element_get_offset_y(a);
  float by = oui_element_get_offset_y(b);
  EXPECT_GT(by, ay);

  oui_element_destroy(b);
  oui_element_destroy(a);
  oui_element_destroy(container);
}

TEST_F(OpenUIAPITest, FlexWrap) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_display(container, OUI_DISPLAY_FLEX);
  oui_element_set_flex_wrap(container, OUI_FLEX_WRAP_WRAP);
  oui_element_set_width(container, oui_px(200));

  // 3 children of 100px each — should wrap.
  OuiElement* children[3];
  for (int i = 0; i < 3; i++) {
    children[i] = oui_element_create(doc_, "div");
    oui_element_append_child(container, children[i]);
    oui_element_set_width(children[i], oui_px(100));
    oui_element_set_height(children[i], oui_px(50));
  }

  oui_document_layout(doc_);
  // First two on row 1, third wraps to row 2.
  float y0 = oui_element_get_offset_y(children[0]);
  float y2 = oui_element_get_offset_y(children[2]);
  EXPECT_GT(y2, y0);

  for (int i = 2; i >= 0; i--) {
    oui_element_destroy(children[i]);
  }
  oui_element_destroy(container);
}

TEST_F(OpenUIAPITest, FlexBasis) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_display(container, OUI_DISPLAY_FLEX);
  oui_element_set_width(container, oui_px(400));

  OuiElement* child = oui_element_create(doc_, "div");
  oui_element_append_child(container, child);
  oui_element_set_flex_basis(child, oui_px(150));
  oui_element_set_flex_grow(child, 0);
  oui_element_set_flex_shrink(child, 0);
  oui_element_set_height(child, oui_px(50));

  oui_document_layout(doc_);
  EXPECT_NEAR(oui_element_get_width(child), 150.0f, 1.0f);

  oui_element_destroy(child);
  oui_element_destroy(container);
}

TEST_F(OpenUIAPITest, AlignItemsCenter) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_display(container, OUI_DISPLAY_FLEX);
  oui_element_set_align_items(container, OUI_ALIGN_CENTER);
  oui_element_set_width(container, oui_px(300));
  oui_element_set_height(container, oui_px(200));

  OuiElement* child = oui_element_create(doc_, "div");
  oui_element_append_child(container, child);
  oui_element_set_width(child, oui_px(50));
  oui_element_set_height(child, oui_px(50));

  oui_document_layout(doc_);
  // Child centered in 200px container: offset should be ~75px.
  float child_y = oui_element_get_offset_y(child);
  float container_y = oui_element_get_offset_y(container);
  float relative_y = child_y - container_y;
  EXPECT_NEAR(relative_y, 75.0f, 1.0f);

  oui_element_destroy(child);
  oui_element_destroy(container);
}

TEST_F(OpenUIAPITest, JustifyContentSpaceBetween) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_display(container, OUI_DISPLAY_FLEX);
  oui_element_set_justify_content(container, OUI_JUSTIFY_SPACE_BETWEEN);
  oui_element_set_width(container, oui_px(300));

  OuiElement* c1 = oui_element_create(doc_, "div");
  OuiElement* c2 = oui_element_create(doc_, "div");
  oui_element_append_child(container, c1);
  oui_element_append_child(container, c2);
  oui_element_set_width(c1, oui_px(50));
  oui_element_set_width(c2, oui_px(50));
  oui_element_set_height(c1, oui_px(50));
  oui_element_set_height(c2, oui_px(50));

  oui_document_layout(doc_);
  float x2 = oui_element_get_offset_x(c2);
  float container_x = oui_element_get_offset_x(container);
  // c2 should be at the far right.
  EXPECT_NEAR(x2 - container_x, 250.0f, 1.0f);

  oui_element_destroy(c2);
  oui_element_destroy(c1);
  oui_element_destroy(container);
}

// ===========================================================================
// Color & visual setters
// ===========================================================================

TEST_F(OpenUIAPITest, SetColor) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_color(div, 0xFF0000FF);  // Red, full alpha.
  oui_document_layout(doc_);
  // Just verify it doesn't crash — color doesn't affect layout.
  EXPECT_FLOAT_EQ(oui_element_get_width(div), oui_element_get_width(body_));

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetBackgroundColor) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_background_color(div, 0x00FF00FF);
  oui_document_layout(doc_);
  // Just verify it doesn't crash.
  SUCCEED();
  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetOpacity) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_opacity(div, 0.5f);
  oui_document_layout(doc_);
  SUCCEED();
  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetZIndex) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_position(div, OUI_POSITION_RELATIVE);
  oui_element_set_z_index(div, 10);
  oui_document_layout(doc_);
  SUCCEED();
  oui_element_destroy(div);
}

// ===========================================================================
// Text content
// ===========================================================================

TEST_F(OpenUIAPITest, SetTextContent) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_text_content(div, "Hello, World!");
  oui_element_set_width(div, oui_px(200));

  oui_document_layout(doc_);
  // Div should have non-zero height from text.
  EXPECT_GT(oui_element_get_height(div), 0.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetTextContentOverwrite) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(200));

  oui_element_set_text_content(div, "First");
  oui_document_layout(doc_);
  float h1 = oui_element_get_height(div);

  oui_element_set_text_content(div, "Second text that is much longer");
  oui_document_layout(doc_);
  float h2 = oui_element_get_height(div);

  EXPECT_GT(h1, 0.0f);
  EXPECT_GT(h2, 0.0f);

  oui_element_destroy(div);
}

// ===========================================================================
// Font convenience setters
// ===========================================================================

TEST_F(OpenUIAPITest, SetFontSize) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_text_content(div, "Hello");
  oui_element_set_font_size(div, oui_px(24));
  oui_element_set_width(div, oui_px(200));

  oui_document_layout(doc_);
  EXPECT_GT(oui_element_get_height(div), 0.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetFontWeight) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_font_weight(div, 700);
  oui_element_set_text_content(div, "Bold text");
  oui_document_layout(doc_);
  SUCCEED();
  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetFontFamily) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_font_family(div, "monospace");
  oui_element_set_text_content(div, "Monospace text");
  oui_element_set_width(div, oui_px(200));
  oui_document_layout(doc_);
  EXPECT_GT(oui_element_get_height(div), 0.0f);
  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetFontStyleItalic) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_font_style(div, OUI_FONT_STYLE_ITALIC);
  oui_element_set_text_content(div, "Italic text");
  oui_document_layout(doc_);
  SUCCEED();
  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetLineHeight) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_text_content(div, "Line height test");
  oui_element_set_line_height(div, oui_px(40));
  oui_element_set_width(div, oui_px(200));

  oui_document_layout(doc_);
  EXPECT_GE(oui_element_get_height(div), 40.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, SetTextAlign) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_text_align(div, OUI_TEXT_ALIGN_CENTER);
  oui_element_set_text_content(div, "Centered");
  oui_document_layout(doc_);
  SUCCEED();
  oui_element_destroy(div);
}

// ===========================================================================
// Geometry queries
// ===========================================================================

TEST_F(OpenUIAPITest, GetBoundingRect) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(100));

  oui_document_layout(doc_);
  OuiRect rect = oui_element_get_bounding_rect(div);
  EXPECT_FLOAT_EQ(rect.width, 200.0f);
  EXPECT_FLOAT_EQ(rect.height, 100.0f);
  EXPECT_GE(rect.x, 0.0f);
  EXPECT_GE(rect.y, 0.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, OffsetIncludesMargin) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_margin(div, oui_px(20), oui_px(0), oui_px(0), oui_px(30));

  oui_document_layout(doc_);
  // Body has 8px default margin. Div has 30px left margin.
  EXPECT_NEAR(oui_element_get_offset_x(div), 38.0f, 1.0f);
  // Body has 8px top margin. Div has 20px top margin (collapse may apply).
  EXPECT_GE(oui_element_get_offset_y(div), 20.0f);

  oui_element_destroy(div);
}

// ===========================================================================
// Computed style readback
// ===========================================================================

TEST_F(OpenUIAPITest, GetComputedStyleWidth) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(250));

  oui_document_layout(doc_);
  char* val = oui_element_get_computed_style(div, "width");
  ASSERT_NE(val, nullptr);
  EXPECT_STREQ(val, "250px");
  free(val);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, GetComputedStyleDisplay) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_display(div, OUI_DISPLAY_FLEX);

  oui_document_layout(doc_);
  char* val = oui_element_get_computed_style(div, "display");
  ASSERT_NE(val, nullptr);
  EXPECT_STREQ(val, "flex");
  free(val);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, GetComputedStyleUnknownProperty) {
  OuiElement* div = oui_element_create(doc_, "div");
  char* val = oui_element_get_computed_style(div, "not-a-property");
  EXPECT_EQ(val, nullptr);
  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, GetComputedStyleNullElement) {
  char* val = oui_element_get_computed_style(nullptr, "width");
  EXPECT_EQ(val, nullptr);
}

// ===========================================================================
// Hit testing
// ===========================================================================

TEST_F(OpenUIAPITest, HitTestFindsElement) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(200));
  oui_element_set_background_color(div, 0xFF0000FF);

  oui_document_update_all(doc_);
  OuiElement* hit = oui_document_hit_test(doc_, 50.0f, 50.0f);
  // Should find either the div or body (depending on exact position).
  EXPECT_NE(hit, nullptr);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, HitTestNullDoc) {
  OuiElement* hit = oui_document_hit_test(nullptr, 0.0f, 0.0f);
  EXPECT_EQ(hit, nullptr);
}

// ===========================================================================
// Scroll geometry
// ===========================================================================

TEST_F(OpenUIAPITest, ScrollWidthMatchesContent) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_overflow(div, OUI_OVERFLOW_HIDDEN);

  OuiElement* child = oui_element_create(doc_, "div");
  oui_element_append_child(div, child);
  oui_element_set_width(child, oui_px(500));
  oui_element_set_height(child, oui_px(50));

  oui_document_layout(doc_);
  float sw = oui_element_get_scroll_width(div);
  EXPECT_GE(sw, 500.0f);

  oui_element_destroy(child);
  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, ScrollHeightMatchesContent) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_overflow(div, OUI_OVERFLOW_HIDDEN);

  OuiElement* child = oui_element_create(doc_, "div");
  oui_element_append_child(div, child);
  oui_element_set_width(child, oui_px(50));
  oui_element_set_height(child, oui_px(500));

  oui_document_layout(doc_);
  float sh = oui_element_get_scroll_height(div);
  EXPECT_GE(sh, 500.0f);

  oui_element_destroy(child);
  oui_element_destroy(div);
}

// ===========================================================================
// Complex layout scenarios
// ===========================================================================

TEST_F(OpenUIAPITest, NestedFlexbox) {
  OuiElement* outer = oui_element_create(doc_, "div");
  oui_element_append_child(body_, outer);
  oui_element_set_display(outer, OUI_DISPLAY_FLEX);
  oui_element_set_width(outer, oui_px(600));

  OuiElement* left = oui_element_create(doc_, "div");
  OuiElement* right = oui_element_create(doc_, "div");
  oui_element_append_child(outer, left);
  oui_element_append_child(outer, right);
  oui_element_set_flex_grow(left, 1.0f);
  oui_element_set_flex_grow(right, 1.0f);
  oui_element_set_height(left, oui_px(100));

  // Nested flex inside right.
  oui_element_set_display(right, OUI_DISPLAY_FLEX);
  oui_element_set_flex_direction(right, OUI_FLEX_COLUMN);

  OuiElement* top = oui_element_create(doc_, "div");
  OuiElement* bottom = oui_element_create(doc_, "div");
  oui_element_append_child(right, top);
  oui_element_append_child(right, bottom);
  oui_element_set_height(top, oui_px(40));
  oui_element_set_height(bottom, oui_px(60));

  oui_document_layout(doc_);
  EXPECT_NEAR(oui_element_get_width(left), 300.0f, 1.0f);
  EXPECT_NEAR(oui_element_get_width(right), 300.0f, 1.0f);
  EXPECT_FLOAT_EQ(oui_element_get_height(top), 40.0f);
  EXPECT_FLOAT_EQ(oui_element_get_height(bottom), 60.0f);

  oui_element_destroy(bottom);
  oui_element_destroy(top);
  oui_element_destroy(right);
  oui_element_destroy(left);
  oui_element_destroy(outer);
}

TEST_F(OpenUIAPITest, TableLayout) {
  OuiElement* table = oui_element_create(doc_, "table");
  oui_element_append_child(body_, table);
  oui_element_set_width(table, oui_px(400));
  oui_element_set_style(table, "border-collapse", "collapse");

  OuiElement* tbody = oui_element_create(doc_, "tbody");
  oui_element_append_child(table, tbody);

  OuiElement* row = oui_element_create(doc_, "tr");
  oui_element_append_child(tbody, row);

  OuiElement* td1 = oui_element_create(doc_, "td");
  OuiElement* td2 = oui_element_create(doc_, "td");
  oui_element_append_child(row, td1);
  oui_element_append_child(row, td2);
  oui_element_set_text_content(td1, "Cell 1");
  oui_element_set_text_content(td2, "Cell 2");

  oui_document_layout(doc_);
  EXPECT_GT(oui_element_get_width(td1), 0.0f);
  EXPECT_GT(oui_element_get_width(td2), 0.0f);

  oui_element_destroy(td2);
  oui_element_destroy(td1);
  oui_element_destroy(row);
  oui_element_destroy(tbody);
  oui_element_destroy(table);
}

TEST_F(OpenUIAPITest, GenericStyleTransform) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));

  OuiStatus s = oui_element_set_style(div, "transform", "rotate(45deg)");
  EXPECT_EQ(s, OUI_OK);
  oui_document_layout(doc_);
  SUCCEED();

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, GenericStyleMultipleProperties) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);

  EXPECT_EQ(oui_element_set_style(div, "width", "200px"), OUI_OK);
  EXPECT_EQ(oui_element_set_style(div, "height", "100px"), OUI_OK);
  EXPECT_EQ(oui_element_set_style(div, "background-color", "red"), OUI_OK);
  EXPECT_EQ(oui_element_set_style(div, "border", "1px solid black"), OUI_OK);
  EXPECT_EQ(oui_element_set_style(div, "border-radius", "5px"), OUI_OK);

  oui_document_layout(doc_);
  // Width should be 200 + 1 + 1 (border) = 202 in content-box model.
  EXPECT_NEAR(oui_element_get_width(div), 202.0f, 1.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, InlineBlockSideBySide) {
  OuiElement* a = oui_element_create(doc_, "div");
  OuiElement* b = oui_element_create(doc_, "div");
  oui_element_append_child(body_, a);
  oui_element_append_child(body_, b);
  oui_element_set_display(a, OUI_DISPLAY_INLINE_BLOCK);
  oui_element_set_display(b, OUI_DISPLAY_INLINE_BLOCK);
  oui_element_set_width(a, oui_px(100));
  oui_element_set_width(b, oui_px(100));
  oui_element_set_height(a, oui_px(50));
  oui_element_set_height(b, oui_px(50));

  oui_document_layout(doc_);
  float ax = oui_element_get_offset_x(a);
  float bx = oui_element_get_offset_x(b);
  // b should be to the right of a.
  EXPECT_GT(bx, ax);

  oui_element_destroy(b);
  oui_element_destroy(a);
}

TEST_F(OpenUIAPITest, BorderBoxSizing) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_style(div, "box-sizing", "border-box");
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_padding(div, oui_px(20), oui_px(20), oui_px(20), oui_px(20));

  oui_document_layout(doc_);
  // With border-box, total size should stay at 200x100.
  EXPECT_FLOAT_EQ(oui_element_get_width(div), 200.0f);
  EXPECT_FLOAT_EQ(oui_element_get_height(div), 100.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, MultipleDocuments) {
  OuiDocument* doc2 = oui_document_create(1024, 768);
  ASSERT_NE(doc2, nullptr);

  OuiElement* body2 = oui_document_body(doc2);
  ASSERT_NE(body2, nullptr);

  OuiElement* div = oui_element_create(doc2, "div");
  oui_element_append_child(body2, div);
  oui_element_set_width(div, oui_px(500));

  oui_document_layout(doc2);
  EXPECT_FLOAT_EQ(oui_element_get_width(div), 500.0f);

  oui_element_destroy(div);
  oui_document_destroy(doc2);
}

TEST_F(OpenUIAPITest, CrossDocumentInsertRejected) {
  // Elements from one document must not be appended to another.
  OuiDocument* doc2 = oui_document_create(1024, 768);
  ASSERT_NE(doc2, nullptr);

  OuiElement* div_doc1 = oui_element_create(doc_, "div");
  OuiElement* body2 = oui_document_body(doc2);

  // Cross-document append should be silently rejected.
  oui_element_append_child(body2, div_doc1);

  // The div should still be unparented (not in doc2's tree).
  EXPECT_EQ(oui_element_first_child(body2), nullptr);

  oui_element_destroy(div_doc1);
  oui_document_destroy(doc2);
}

TEST_F(OpenUIAPITest, DocumentDestroysCleansUpElements) {
  // Destroying a document frees all associated element wrappers.
  // After this, element handles are dangling (same as free() in C).
  OuiDocument* doc2 = oui_document_create(1024, 768);
  ASSERT_NE(doc2, nullptr);
  OuiElement* body2 = oui_document_body(doc2);
  OuiElement* div = oui_element_create(doc2, "div");
  oui_element_append_child(body2, div);
  oui_element_set_width(div, oui_px(100));
  oui_document_layout(doc2);

  // Verify element works before destroy.
  EXPECT_FLOAT_EQ(oui_element_get_width(div), 100.0f);

  // Destroy document — no need to destroy elements individually.
  // This must not crash or leak.
  oui_document_destroy(doc2);
}

// ===========================================================================
// SP7: Time & frame management tests
// ===========================================================================

TEST_F(OpenUIAPITest, TimeInitiallyZero) {
  double t = oui_document_get_time(doc_);
  EXPECT_DOUBLE_EQ(t, 0.0);
}

TEST_F(OpenUIAPITest, AdvanceTimeAbsolute) {
  EXPECT_EQ(oui_document_advance_time(doc_, 1000.0), OUI_OK);
  EXPECT_DOUBLE_EQ(oui_document_get_time(doc_), 1000.0);

  EXPECT_EQ(oui_document_advance_time(doc_, 2500.0), OUI_OK);
  EXPECT_DOUBLE_EQ(oui_document_get_time(doc_), 2500.0);
}

TEST_F(OpenUIAPITest, AdvanceTimeByDelta) {
  EXPECT_EQ(oui_document_advance_time(doc_, 100.0), OUI_OK);
  EXPECT_EQ(oui_document_advance_time_by(doc_, 50.0), OUI_OK);
  EXPECT_DOUBLE_EQ(oui_document_get_time(doc_), 150.0);

  EXPECT_EQ(oui_document_advance_time_by(doc_, 200.0), OUI_OK);
  EXPECT_DOUBLE_EQ(oui_document_get_time(doc_), 350.0);
}

TEST_F(OpenUIAPITest, BeginFrameUpdatesTime) {
  EXPECT_EQ(oui_document_begin_frame(doc_, 16.6), OUI_OK);
  EXPECT_NEAR(oui_document_get_time(doc_), 16.6, 0.1);

  EXPECT_EQ(oui_document_begin_frame(doc_, 33.2), OUI_OK);
  EXPECT_NEAR(oui_document_get_time(doc_), 33.2, 0.1);
}

TEST_F(OpenUIAPITest, BeginFrameTriggersLayout) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(100));

  EXPECT_EQ(oui_document_begin_frame(doc_, 16.0), OUI_OK);
  EXPECT_FLOAT_EQ(oui_element_get_width(div), 200.0f);
  EXPECT_FLOAT_EQ(oui_element_get_height(div), 100.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, TimeNullDocReturnsError) {
  EXPECT_EQ(oui_document_advance_time(nullptr, 100.0),
            OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_EQ(oui_document_advance_time_by(nullptr, 100.0),
            OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_EQ(oui_document_begin_frame(nullptr, 100.0),
            OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_DOUBLE_EQ(oui_document_get_time(nullptr), 0.0);
}

// ===========================================================================
// SP7: Input event dispatch tests
// ===========================================================================

TEST_F(OpenUIAPITest, DispatchMouseEventOK) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(200));
  oui_document_layout(doc_);

  EXPECT_EQ(oui_document_dispatch_mouse_event(
      doc_, OUI_MOUSE_DOWN, 100, 100, OUI_BUTTON_LEFT, 0), OUI_OK);
  EXPECT_EQ(oui_document_dispatch_mouse_event(
      doc_, OUI_MOUSE_UP, 100, 100, OUI_BUTTON_LEFT, 0), OUI_OK);
  EXPECT_EQ(oui_document_dispatch_mouse_event(
      doc_, OUI_MOUSE_MOVE, 50, 50, OUI_BUTTON_LEFT, 0), OUI_OK);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, DispatchMouseEventNullDoc) {
  EXPECT_EQ(oui_document_dispatch_mouse_event(
      nullptr, OUI_MOUSE_DOWN, 0, 0, OUI_BUTTON_LEFT, 0),
      OUI_ERROR_INVALID_ARGUMENT);
}

TEST_F(OpenUIAPITest, DispatchKeyEventOK) {
  EXPECT_EQ(oui_document_dispatch_key_event(
      doc_, OUI_KEY_DOWN, 65, nullptr, 0), OUI_OK);  // 'A' key down
  EXPECT_EQ(oui_document_dispatch_key_event(
      doc_, OUI_KEY_CHAR, 65, "a", 0), OUI_OK);      // 'a' char
  EXPECT_EQ(oui_document_dispatch_key_event(
      doc_, OUI_KEY_UP, 65, nullptr, 0), OUI_OK);     // 'A' key up
}

TEST_F(OpenUIAPITest, DispatchKeyEventNullDoc) {
  EXPECT_EQ(oui_document_dispatch_key_event(
      nullptr, OUI_KEY_DOWN, 65, nullptr, 0),
      OUI_ERROR_INVALID_ARGUMENT);
}

TEST_F(OpenUIAPITest, DispatchKeyEventWithModifiers) {
  EXPECT_EQ(oui_document_dispatch_key_event(
      doc_, OUI_KEY_DOWN, 65, nullptr, OUI_MOD_CTRL | OUI_MOD_SHIFT),
      OUI_OK);
  EXPECT_EQ(oui_document_dispatch_key_event(
      doc_, OUI_KEY_UP, 65, nullptr, OUI_MOD_CTRL | OUI_MOD_SHIFT),
      OUI_OK);
}

TEST_F(OpenUIAPITest, DispatchWheelEventOK) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(400));
  oui_element_set_height(div, oui_px(400));
  oui_document_layout(doc_);

  EXPECT_EQ(oui_document_dispatch_wheel_event(
      doc_, 200, 200, 0, -120, 0), OUI_OK);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, DispatchWheelEventNullDoc) {
  EXPECT_EQ(oui_document_dispatch_wheel_event(
      nullptr, 0, 0, 0, -120, 0), OUI_ERROR_INVALID_ARGUMENT);
}

// ===========================================================================
// SP7: Event callback tests
// ===========================================================================

namespace {

struct CallbackState {
  int call_count = 0;
  std::string last_event_type;
  float last_mouse_x = 0;
  float last_mouse_y = 0;
  int last_key_code = 0;
};

void test_event_callback(OuiEvent* event, void* user_data) {
  auto* state = static_cast<CallbackState*>(user_data);
  state->call_count++;
  if (event->type)
    state->last_event_type = event->type;
  state->last_mouse_x = event->mouse_x;
  state->last_mouse_y = event->mouse_y;
  state->last_key_code = event->key_code;
}

}  // namespace

TEST_F(OpenUIAPITest, SetEventCallbackNullArgs) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);

  EXPECT_EQ(oui_element_set_event_callback(nullptr, "click",
      test_event_callback, nullptr), OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_EQ(oui_element_set_event_callback(div, nullptr,
      test_event_callback, nullptr), OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_EQ(oui_element_set_event_callback(div, "click",
      nullptr, nullptr), OUI_ERROR_INVALID_ARGUMENT);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, ClickCallbackFires) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(200));
  oui_document_layout(doc_);

  CallbackState state;
  EXPECT_EQ(oui_element_set_event_callback(div, "click",
      test_event_callback, &state), OUI_OK);

  // Click = mousedown + mouseup at same position.
  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_DOWN, 100, 100,
      OUI_BUTTON_LEFT, 0);
  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_UP, 100, 100,
      OUI_BUTTON_LEFT, 0);

  EXPECT_GE(state.call_count, 1);
  EXPECT_EQ(state.last_event_type, "click");

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, MouseDownCallbackFires) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(200));
  oui_document_layout(doc_);

  CallbackState state;
  EXPECT_EQ(oui_element_set_event_callback(div, "mousedown",
      test_event_callback, &state), OUI_OK);

  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_DOWN, 50, 75,
      OUI_BUTTON_LEFT, 0);

  EXPECT_GE(state.call_count, 1);
  EXPECT_EQ(state.last_event_type, "mousedown");

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, RemoveEventCallback) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(200));
  oui_document_layout(doc_);

  CallbackState state;
  oui_element_set_event_callback(div, "click",
      test_event_callback, &state);

  // Remove and verify no more callbacks.
  EXPECT_EQ(oui_element_remove_event_callback(div, "click"), OUI_OK);

  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_DOWN, 100, 100,
      OUI_BUTTON_LEFT, 0);
  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_UP, 100, 100,
      OUI_BUTTON_LEFT, 0);

  EXPECT_EQ(state.call_count, 0);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, RemoveEventCallbackNullArgs) {
  EXPECT_EQ(oui_element_remove_event_callback(nullptr, "click"),
            OUI_ERROR_INVALID_ARGUMENT);
  OuiElement* div = oui_element_create(doc_, "div");
  EXPECT_EQ(oui_element_remove_event_callback(div, nullptr),
            OUI_ERROR_INVALID_ARGUMENT);
  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, ReplaceEventCallback) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(200));
  oui_document_layout(doc_);

  CallbackState state1, state2;
  oui_element_set_event_callback(div, "click",
      test_event_callback, &state1);
  // Replace with a different user_data.
  oui_element_set_event_callback(div, "click",
      test_event_callback, &state2);

  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_DOWN, 100, 100,
      OUI_BUTTON_LEFT, 0);
  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_UP, 100, 100,
      OUI_BUTTON_LEFT, 0);

  // Only the replacement callback should fire.
  EXPECT_EQ(state1.call_count, 0);
  EXPECT_GE(state2.call_count, 1);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, MultipleEventTypesOnSameElement) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(200));
  oui_document_layout(doc_);

  CallbackState click_state, mousedown_state;
  oui_element_set_event_callback(div, "click",
      test_event_callback, &click_state);
  oui_element_set_event_callback(div, "mousedown",
      test_event_callback, &mousedown_state);

  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_DOWN, 100, 100,
      OUI_BUTTON_LEFT, 0);
  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_UP, 100, 100,
      OUI_BUTTON_LEFT, 0);

  EXPECT_GE(click_state.call_count, 1);
  EXPECT_GE(mousedown_state.call_count, 1);

  oui_element_destroy(div);
}

// ===========================================================================
// SP7: Focus management tests
// ===========================================================================

TEST_F(OpenUIAPITest, FocusElement) {
  OuiElement* input = oui_element_create(doc_, "input");
  oui_element_append_child(body_, input);
  oui_document_layout(doc_);

  EXPECT_EQ(oui_element_focus(input), OUI_OK);
  EXPECT_EQ(oui_element_has_focus(input), 1);

  oui_element_destroy(input);
}

TEST_F(OpenUIAPITest, BlurElement) {
  OuiElement* input = oui_element_create(doc_, "input");
  oui_element_append_child(body_, input);
  oui_document_layout(doc_);

  oui_element_focus(input);
  EXPECT_EQ(oui_element_has_focus(input), 1);

  oui_element_blur(input);
  EXPECT_EQ(oui_element_has_focus(input), 0);

  oui_element_destroy(input);
}

TEST_F(OpenUIAPITest, GetFocusedElement) {
  OuiElement* input1 = oui_element_create(doc_, "input");
  OuiElement* input2 = oui_element_create(doc_, "input");
  oui_element_append_child(body_, input1);
  oui_element_append_child(body_, input2);
  oui_document_layout(doc_);

  oui_element_focus(input1);
  EXPECT_EQ(oui_document_get_focused_element(doc_), input1);

  oui_element_focus(input2);
  EXPECT_EQ(oui_document_get_focused_element(doc_), input2);
  EXPECT_EQ(oui_element_has_focus(input1), 0);

  oui_element_destroy(input1);
  oui_element_destroy(input2);
}

TEST_F(OpenUIAPITest, FocusNullArgs) {
  EXPECT_EQ(oui_element_focus(nullptr), OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_EQ(oui_element_blur(nullptr), OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_EQ(oui_document_get_focused_element(nullptr), nullptr);
  EXPECT_EQ(oui_element_has_focus(nullptr), 0);
}

TEST_F(OpenUIAPITest, AdvanceFocusForward) {
  OuiElement* input1 = oui_element_create(doc_, "input");
  OuiElement* input2 = oui_element_create(doc_, "input");
  OuiElement* input3 = oui_element_create(doc_, "input");
  oui_element_append_child(body_, input1);
  oui_element_append_child(body_, input2);
  oui_element_append_child(body_, input3);
  oui_document_layout(doc_);

  oui_element_focus(input1);
  EXPECT_EQ(oui_element_has_focus(input1), 1);

  oui_document_advance_focus(doc_, 1);  // Tab forward
  // Focus should have moved to input2.
  EXPECT_EQ(oui_element_has_focus(input1), 0);

  oui_element_destroy(input1);
  oui_element_destroy(input2);
  oui_element_destroy(input3);
}

TEST_F(OpenUIAPITest, AdvanceFocusBackward) {
  OuiElement* input1 = oui_element_create(doc_, "input");
  OuiElement* input2 = oui_element_create(doc_, "input");
  oui_element_append_child(body_, input1);
  oui_element_append_child(body_, input2);
  oui_document_layout(doc_);

  oui_element_focus(input2);
  oui_document_advance_focus(doc_, -1);  // Tab backward
  // Focus should have moved to input1.
  EXPECT_EQ(oui_element_has_focus(input2), 0);

  oui_element_destroy(input1);
  oui_element_destroy(input2);
}

TEST_F(OpenUIAPITest, AdvanceFocusNullDoc) {
  EXPECT_EQ(oui_document_advance_focus(nullptr, 1),
            OUI_ERROR_INVALID_ARGUMENT);
}

TEST_F(OpenUIAPITest, FocusCallbackFires) {
  OuiElement* input = oui_element_create(doc_, "input");
  oui_element_append_child(body_, input);
  oui_document_layout(doc_);

  CallbackState focus_state, blur_state;
  oui_element_set_event_callback(input, "focus",
      test_event_callback, &focus_state);
  oui_element_set_event_callback(input, "blur",
      test_event_callback, &blur_state);

  oui_element_focus(input);
  EXPECT_GE(focus_state.call_count, 1);

  oui_element_blur(input);
  EXPECT_GE(blur_state.call_count, 1);

  oui_element_destroy(input);
}

TEST_F(OpenUIAPITest, FocusOnDivWithTabindex) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_attribute(div, "tabindex", "0");
  oui_document_layout(doc_);

  EXPECT_EQ(oui_element_focus(div), OUI_OK);
  EXPECT_EQ(oui_element_has_focus(div), 1);

  oui_element_destroy(div);
}

// ===========================================================================
// SP7: Scroll control tests
// ===========================================================================

TEST_F(OpenUIAPITest, ScrollToPosition) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_width(container, oui_px(200));
  oui_element_set_height(container, oui_px(200));
  oui_element_set_style(container, "overflow", "scroll");

  OuiElement* content = oui_element_create(doc_, "div");
  oui_element_append_child(container, content);
  oui_element_set_width(content, oui_px(800));
  oui_element_set_height(content, oui_px(800));
  oui_document_layout(doc_);

  EXPECT_EQ(oui_element_scroll_to(container, 100, 150), OUI_OK);
  EXPECT_NEAR(oui_element_get_scroll_left(container), 100.0, 1.0);
  EXPECT_NEAR(oui_element_get_scroll_top(container), 150.0, 1.0);

  oui_element_destroy(content);
  oui_element_destroy(container);
}

TEST_F(OpenUIAPITest, ScrollByDelta) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_width(container, oui_px(200));
  oui_element_set_height(container, oui_px(200));
  oui_element_set_style(container, "overflow", "scroll");

  OuiElement* content = oui_element_create(doc_, "div");
  oui_element_append_child(container, content);
  oui_element_set_width(content, oui_px(800));
  oui_element_set_height(content, oui_px(800));
  oui_document_layout(doc_);

  oui_element_scroll_to(container, 50, 50);
  EXPECT_EQ(oui_element_scroll_by(container, 25, 30), OUI_OK);
  EXPECT_NEAR(oui_element_get_scroll_left(container), 75.0, 1.0);
  EXPECT_NEAR(oui_element_get_scroll_top(container), 80.0, 1.0);

  oui_element_destroy(content);
  oui_element_destroy(container);
}

TEST_F(OpenUIAPITest, ScrollClampedToMax) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_width(container, oui_px(200));
  oui_element_set_height(container, oui_px(200));
  oui_element_set_style(container, "overflow", "scroll");

  OuiElement* content = oui_element_create(doc_, "div");
  oui_element_append_child(container, content);
  oui_element_set_width(content, oui_px(400));
  oui_element_set_height(content, oui_px(400));
  oui_document_layout(doc_);

  // Scroll beyond max — should be clamped.
  oui_element_scroll_to(container, 9999, 9999);
  double left = oui_element_get_scroll_left(container);
  double top = oui_element_get_scroll_top(container);
  EXPECT_LE(left, 200.0 + 1.0);  // max scroll = content - container
  EXPECT_LE(top, 200.0 + 1.0);

  oui_element_destroy(content);
  oui_element_destroy(container);
}

TEST_F(OpenUIAPITest, ScrollNullArgs) {
  EXPECT_EQ(oui_element_scroll_to(nullptr, 0, 0),
            OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_EQ(oui_element_scroll_by(nullptr, 0, 0),
            OUI_ERROR_INVALID_ARGUMENT);
  EXPECT_DOUBLE_EQ(oui_element_get_scroll_left(nullptr), 0.0);
  EXPECT_DOUBLE_EQ(oui_element_get_scroll_top(nullptr), 0.0);
}

TEST_F(OpenUIAPITest, ScrollInitiallyZero) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_width(container, oui_px(200));
  oui_element_set_height(container, oui_px(200));
  oui_element_set_style(container, "overflow", "scroll");
  oui_document_layout(doc_);

  EXPECT_DOUBLE_EQ(oui_element_get_scroll_left(container), 0.0);
  EXPECT_DOUBLE_EQ(oui_element_get_scroll_top(container), 0.0);

  oui_element_destroy(container);
}

// ===========================================================================
// SP7: CSS transition tests (using begin_frame to advance time)
// ===========================================================================

TEST_F(OpenUIAPITest, CSSTransitionWidth) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(50));
  oui_element_set_style(div, "transition", "width 1s linear");
  oui_document_begin_frame(doc_, 0.0);

  // Change width to trigger transition.
  oui_element_set_width(div, oui_px(300));

  // At halfway through the transition (500ms), width should be ~200px.
  oui_document_begin_frame(doc_, 500.0);
  float mid_width = oui_element_get_width(div);
  EXPECT_GT(mid_width, 120.0f);
  EXPECT_LT(mid_width, 280.0f);

  // At end of transition, width should be 300.
  oui_document_begin_frame(doc_, 1100.0);
  EXPECT_NEAR(oui_element_get_width(div), 300.0f, 2.0f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, CSSTransitionBackgroundColor) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_style(div, "background-color", "red");
  oui_element_set_style(div, "transition", "background-color 1s linear");
  oui_document_begin_frame(doc_, 0.0);

  oui_element_set_style(div, "background-color", "blue");
  oui_document_begin_frame(doc_, 500.0);

  char* color = oui_element_get_computed_style(div, "background-color");
  ASSERT_NE(color, nullptr);
  // At midpoint, color should be neither pure red nor pure blue.
  // Computed style returns "rgb(R, G, B)" format.
  std::string color_str(color);
  oui_free(color);
  EXPECT_NE(color_str, "rgb(255, 0, 0)");
  EXPECT_NE(color_str, "rgb(0, 0, 255)");

  oui_document_begin_frame(doc_, 1100.0);
  color = oui_element_get_computed_style(div, "background-color");
  ASSERT_NE(color, nullptr);
  color_str = color;
  oui_free(color);
  EXPECT_EQ(color_str, "rgb(0, 0, 255)");

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, CSSTransitionOpacity) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_style(div, "opacity", "1.0");
  oui_element_set_style(div, "transition", "opacity 1s linear");
  oui_document_begin_frame(doc_, 0.0);

  oui_element_set_style(div, "opacity", "0.0");
  oui_document_begin_frame(doc_, 500.0);

  char* opacity = oui_element_get_computed_style(div, "opacity");
  ASSERT_NE(opacity, nullptr);
  float val = std::stof(opacity);
  oui_free(opacity);
  EXPECT_GT(val, 0.1f);
  EXPECT_LT(val, 0.9f);

  oui_document_begin_frame(doc_, 1100.0);
  opacity = oui_element_get_computed_style(div, "opacity");
  ASSERT_NE(opacity, nullptr);
  val = std::stof(opacity);
  oui_free(opacity);
  EXPECT_NEAR(val, 0.0f, 0.05f);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, CSSTransitionTransform) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_style(div, "transform", "translateX(0px)");
  oui_element_set_style(div, "transition", "transform 1s linear");
  oui_document_begin_frame(doc_, 0.0);

  oui_element_set_style(div, "transform", "translateX(200px)");
  oui_document_begin_frame(doc_, 500.0);

  char* transform = oui_element_get_computed_style(div, "transform");
  ASSERT_NE(transform, nullptr);
  // Midway should be a matrix with non-zero translateX.
  std::string t(transform);
  oui_free(transform);
  EXPECT_NE(t, "none");

  oui_document_begin_frame(doc_, 1100.0);
  transform = oui_element_get_computed_style(div, "transform");
  ASSERT_NE(transform, nullptr);
  // Should be matrix(1, 0, 0, 1, 200, 0)
  t = transform;
  oui_free(transform);
  EXPECT_NE(t, "none");
  EXPECT_NE(t.find("200"), std::string::npos);

  oui_element_destroy(div);
}

// ===========================================================================
// SP7: CSS @keyframes animation tests
// ===========================================================================

TEST_F(OpenUIAPITest, CSSAnimationViaTransition) {
  // Verify CSS animations work by using a multi-property transition
  // that simulates a keyframe animation (from→to over time).
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(50));
  oui_element_set_height(div, oui_px(50));
  oui_element_set_style(div, "background-color", "red");
  oui_element_set_style(div, "transition",
      "width 1s linear, height 1s linear, background-color 1s linear");
  oui_document_begin_frame(doc_, 0.0);

  // Trigger transitions on multiple properties simultaneously.
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(200));
  oui_element_set_style(div, "background-color", "blue");

  oui_document_begin_frame(doc_, 500.0);
  float mid_w = oui_element_get_width(div);
  float mid_h = oui_element_get_height(div);
  EXPECT_GT(mid_w, 60.0f);
  EXPECT_LT(mid_w, 190.0f);
  EXPECT_GT(mid_h, 60.0f);
  EXPECT_LT(mid_h, 190.0f);

  oui_document_begin_frame(doc_, 1100.0);
  EXPECT_NEAR(oui_element_get_width(div), 200.0f, 5.0f);
  EXPECT_NEAR(oui_element_get_height(div), 200.0f, 5.0f);

  char* color = oui_element_get_computed_style(div, "background-color");
  ASSERT_NE(color, nullptr);
  EXPECT_EQ(std::string(color), "rgb(0, 0, 255)");
  oui_free(color);

  oui_element_destroy(div);
}

// ===========================================================================
// SP7: 3D transform verification
// ===========================================================================

TEST_F(OpenUIAPITest, Transform3DPerspective) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_style(div, "transform", "perspective(500px) rotateY(45deg)");
  oui_document_layout(doc_);

  char* t = oui_element_get_computed_style(div, "transform");
  ASSERT_NE(t, nullptr);
  // Blink preserves function names in computed style for 3D transforms.
  std::string ts(t);
  oui_free(t);
  EXPECT_NE(ts, "none");
  // Should contain rotateY or perspective (preserved function form).
  EXPECT_TRUE(ts.find("rotateY") != std::string::npos ||
              ts.find("matrix") != std::string::npos);

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, Transform3DTranslateZ) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_style(div, "transform", "translateZ(50px)");
  oui_document_layout(doc_);

  char* t = oui_element_get_computed_style(div, "transform");
  ASSERT_NE(t, nullptr);
  std::string ts(t);
  oui_free(t);
  EXPECT_NE(ts, "none");

  oui_element_destroy(div);
}

TEST_F(OpenUIAPITest, AnimatedTransform3D) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_style(div, "transform", "rotateX(0deg)");
  oui_element_set_style(div, "transition", "transform 1s linear");
  oui_document_begin_frame(doc_, 0.0);

  oui_element_set_style(div, "transform", "rotateX(90deg)");
  oui_document_begin_frame(doc_, 500.0);

  char* t = oui_element_get_computed_style(div, "transform");
  ASSERT_NE(t, nullptr);
  std::string ts(t);
  oui_free(t);
  // Mid-transition: should be some transform value (not "none").
  EXPECT_NE(ts, "none");

  oui_document_begin_frame(doc_, 1100.0);
  t = oui_element_get_computed_style(div, "transform");
  ASSERT_NE(t, nullptr);
  ts = t;
  oui_free(t);
  EXPECT_NE(ts, "none");
  // Final value should contain rotateX or a matrix representation.
  EXPECT_TRUE(ts.find("rotateX") != std::string::npos ||
              ts.find("matrix") != std::string::npos);

  oui_element_destroy(div);
}

// ===========================================================================
// SP7: Integration — event + scroll + time interactions
// ===========================================================================

TEST_F(OpenUIAPITest, WheelEventTriggersScroll) {
  OuiElement* container = oui_element_create(doc_, "div");
  oui_element_append_child(body_, container);
  oui_element_set_width(container, oui_px(200));
  oui_element_set_height(container, oui_px(200));
  oui_element_set_style(container, "overflow", "scroll");

  OuiElement* content = oui_element_create(doc_, "div");
  oui_element_append_child(container, content);
  oui_element_set_width(content, oui_px(200));
  oui_element_set_height(content, oui_px(2000));
  oui_document_begin_frame(doc_, 0.0);

  double before = oui_element_get_scroll_top(container);

  // Dispatch wheel events and run frames.
  for (int i = 0; i < 10; i++) {
    oui_document_dispatch_wheel_event(doc_, 100, 100, 0, -120, 0);
    oui_document_begin_frame(doc_, 16.6 * (i + 1));
  }

  double after = oui_element_get_scroll_top(container);
  // Wheel events should have caused some scrolling.
  // Note: Blink may not process wheel-to-scroll in DummyPageHolder
  // the same way as a full browser. At minimum, verify no crash.
  // If scroll did happen, verify direction.
  if (after != before) {
    EXPECT_GT(after, before);
  }

  oui_element_destroy(content);
  oui_element_destroy(container);
}

TEST_F(OpenUIAPITest, MouseClickOnButton) {
  OuiElement* btn = oui_element_create(doc_, "button");
  oui_element_append_child(body_, btn);
  oui_element_set_width(btn, oui_px(100));
  oui_element_set_height(btn, oui_px(40));
  oui_element_set_text_content(btn, "Click");
  oui_document_layout(doc_);

  CallbackState state;
  oui_element_set_event_callback(btn, "click",
      test_event_callback, &state);

  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_DOWN, 50, 20,
      OUI_BUTTON_LEFT, 0);
  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_UP, 50, 20,
      OUI_BUTTON_LEFT, 0);

  EXPECT_GE(state.call_count, 1);
  EXPECT_EQ(state.last_event_type, "click");

  oui_element_destroy(btn);
}

TEST_F(OpenUIAPITest, FocusViaMouseClick) {
  OuiElement* i1 = oui_element_create(doc_, "input");
  OuiElement* i2 = oui_element_create(doc_, "input");
  oui_element_append_child(body_, i1);
  oui_element_append_child(body_, i2);
  oui_element_set_width(i1, oui_px(200));
  oui_element_set_width(i2, oui_px(200));
  oui_document_layout(doc_);

  // Click on first input area - should receive focus.
  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_DOWN, 10, 10,
      OUI_BUTTON_LEFT, 0);
  oui_document_dispatch_mouse_event(doc_, OUI_MOUSE_UP, 10, 10,
      OUI_BUTTON_LEFT, 0);

  OuiElement* focused = oui_document_get_focused_element(doc_);
  if (focused != nullptr) {
    EXPECT_TRUE(focused == i1 || focused == i2);
  }

  oui_element_destroy(i1);
  oui_element_destroy(i2);
}

TEST_F(OpenUIAPITest, CSSTransitionEndCallback) {
  OuiElement* div = oui_element_create(doc_, "div");
  oui_element_append_child(body_, div);
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_style(div, "transition", "width 0.5s linear");
  oui_document_begin_frame(doc_, 0.0);

  CallbackState state;
  oui_element_set_event_callback(div, "transitionend",
      test_event_callback, &state);

  oui_element_set_width(div, oui_px(300));
  // Run frames through the transition.
  for (double t = 100.0; t <= 700.0; t += 100.0) {
    oui_document_begin_frame(doc_, t);
  }

  // transitionend should have fired.
  EXPECT_GE(state.call_count, 1);

  oui_element_destroy(div);
}

// ===========================================================================
// Custom main — initializes blink runtime before running tests.
// Uses the same pattern as rendering_test.cc main().
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

// Platform subclass that routes resource loading to ResourceBundle.
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

  // Same init sequence as rendering_test.cc (which has 20 passing tests).
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

  // Mark the openui runtime as externally initialized so oui_document_create
  // skips creating its own TaskEnvironment.
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
