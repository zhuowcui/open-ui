/* Copyright 2025 The Open UI Authors
 * SPDX-License-Identifier: BSD-3-Clause
 *
 * openui_c_test.c — Pure C consumer test proving the C ABI works.
 * This file is compiled as C (not C++) to verify the header is C-compatible.
 */

#include "openui/openui.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int g_passed = 0;
static int g_failed = 0;

#define CHECK(cond, msg)                                 \
  do {                                                   \
    if (cond) {                                          \
      g_passed++;                                        \
    } else {                                             \
      g_failed++;                                        \
      fprintf(stderr, "FAIL [%s:%d]: %s\n", __FILE__,   \
              __LINE__, msg);                             \
    }                                                    \
  } while (0)

#define CHECK_NEAR(a, b, eps, msg)                       \
  CHECK(fabs((double)(a) - (double)(b)) <= (eps), msg)

#define CHECK_EQ(a, b, msg) CHECK((a) == (b), msg)
#define CHECK_NE(a, b, msg) CHECK((a) != (b), msg)
#define CHECK_GT(a, b, msg) CHECK((a) > (b), msg)
#define CHECK_GE(a, b, msg) CHECK((a) >= (b), msg)

/* Note: this test is intended to be run AFTER oui_init() has already been
 * called by the test runner. In a standalone scenario, it would call
 * oui_init() itself. Since it links to libopenui which depends on the
 * blink test infrastructure that requires special init sequencing,
 * we rely on the test harness having initialized the runtime. */

static void test_value_constructors(void) {
  OuiLength px = oui_px(100);
  CHECK_EQ(px.unit, OUI_UNIT_PX, "oui_px unit");
  CHECK_NEAR(px.value, 100.0f, 0.001f, "oui_px value");

  OuiLength pct = oui_pct(50);
  CHECK_EQ(pct.unit, OUI_UNIT_PERCENT, "oui_pct unit");

  OuiLength em = oui_em(2);
  CHECK_EQ(em.unit, OUI_UNIT_EM, "oui_em unit");

  OuiLength rem = oui_rem(1.5f);
  CHECK_EQ(rem.unit, OUI_UNIT_REM, "oui_rem unit");

  OuiLength vw = oui_vw(100);
  CHECK_EQ(vw.unit, OUI_UNIT_VW, "oui_vw unit");

  OuiLength vh = oui_vh(100);
  CHECK_EQ(vh.unit, OUI_UNIT_VH, "oui_vh unit");

  OuiLength fr = oui_fr(1);
  CHECK_EQ(fr.unit, OUI_UNIT_FR, "oui_fr unit");

  OuiLength a = oui_auto();
  CHECK_EQ(a.unit, OUI_UNIT_AUTO, "oui_auto unit");

  OuiLength n = oui_none();
  CHECK_EQ(n.unit, OUI_UNIT_NONE, "oui_none unit");
}

static void test_document_create_destroy(OuiDocument* doc) {
  CHECK_NE(doc, (OuiDocument*)NULL, "doc created");

  OuiElement* body = oui_document_body(doc);
  CHECK_NE(body, (OuiElement*)NULL, "body not null");

  OuiElement* body2 = oui_document_body(doc);
  CHECK_EQ(body, body2, "body idempotent");
}

static void test_element_create(OuiDocument* doc) {
  OuiElement* div = oui_element_create(doc, "div");
  CHECK_NE(div, (OuiElement*)NULL, "div created");
  oui_element_destroy(div);

  OuiElement* bad = oui_element_create(doc, "nonexistent");
  CHECK_EQ(bad, (OuiElement*)NULL, "unknown tag null");

  OuiElement* null_tag = oui_element_create(doc, NULL);
  CHECK_EQ(null_tag, (OuiElement*)NULL, "null tag null");
}

static void test_dom_tree(OuiDocument* doc) {
  OuiElement* body = oui_document_body(doc);
  OuiElement* a = oui_element_create(doc, "div");
  OuiElement* b = oui_element_create(doc, "div");

  oui_element_append_child(body, a);
  oui_element_append_child(body, b);

  CHECK_EQ(oui_element_first_child(body), a, "first child is a");
  CHECK_EQ(oui_element_next_sibling(a), b, "next sibling of a is b");
  CHECK_EQ(oui_element_parent(a), body, "parent of a is body");

  oui_element_destroy(b);
  oui_element_destroy(a);
}

static void test_flexbox_layout(OuiDocument* doc) {
  OuiElement* body = oui_document_body(doc);
  OuiElement* container = oui_element_create(doc, "div");
  oui_element_append_child(body, container);
  oui_element_set_display(container, OUI_DISPLAY_FLEX);
  oui_element_set_width(container, oui_px(300));

  OuiElement* c1 = oui_element_create(doc, "div");
  OuiElement* c2 = oui_element_create(doc, "div");
  OuiElement* c3 = oui_element_create(doc, "div");
  oui_element_append_child(container, c1);
  oui_element_append_child(container, c2);
  oui_element_append_child(container, c3);

  oui_element_set_flex_grow(c1, 1.0f);
  oui_element_set_flex_grow(c2, 1.0f);
  oui_element_set_flex_grow(c3, 1.0f);
  oui_element_set_height(c1, oui_px(50));
  oui_element_set_height(c2, oui_px(50));
  oui_element_set_height(c3, oui_px(50));

  oui_document_layout(doc);

  float w1 = oui_element_get_width(c1);
  float w2 = oui_element_get_width(c2);
  float w3 = oui_element_get_width(c3);

  CHECK_NEAR(w1, 100.0f, 1.0f, "flex child 1 width");
  CHECK_NEAR(w2, 100.0f, 1.0f, "flex child 2 width");
  CHECK_NEAR(w3, 100.0f, 1.0f, "flex child 3 width");

  oui_element_destroy(c3);
  oui_element_destroy(c2);
  oui_element_destroy(c1);
  oui_element_destroy(container);
}

static void test_grid_layout(OuiDocument* doc) {
  OuiElement* body = oui_document_body(doc);
  OuiElement* grid = oui_element_create(doc, "div");
  oui_element_append_child(body, grid);
  oui_element_set_display(grid, OUI_DISPLAY_GRID);
  oui_element_set_width(grid, oui_px(400));
  oui_element_set_style(grid, "grid-template-columns", "1fr 1fr");

  OuiElement* c1 = oui_element_create(doc, "div");
  OuiElement* c2 = oui_element_create(doc, "div");
  oui_element_append_child(grid, c1);
  oui_element_append_child(grid, c2);
  oui_element_set_height(c1, oui_px(50));
  oui_element_set_height(c2, oui_px(50));

  oui_document_layout(doc);

  CHECK_NEAR(oui_element_get_width(c1), 200.0f, 1.0f, "grid cell 1 width");
  CHECK_NEAR(oui_element_get_width(c2), 200.0f, 1.0f, "grid cell 2 width");

  oui_element_destroy(c2);
  oui_element_destroy(c1);
  oui_element_destroy(grid);
}

static void test_generic_style(OuiDocument* doc) {
  OuiElement* body = oui_document_body(doc);
  OuiElement* div = oui_element_create(doc, "div");
  oui_element_append_child(body, div);

  OuiStatus s = oui_element_set_style(div, "width", "250px");
  CHECK_EQ(s, OUI_OK, "set style width ok");

  s = oui_element_set_style(div, "not-a-property", "100px");
  CHECK_EQ(s, OUI_ERROR_UNKNOWN_PROPERTY, "unknown property error");

  oui_document_layout(doc);
  CHECK_NEAR(oui_element_get_width(div), 250.0f, 0.1f, "generic style width");

  oui_element_destroy(div);
}

static void test_text_content(OuiDocument* doc) {
  OuiElement* body = oui_document_body(doc);
  OuiElement* div = oui_element_create(doc, "div");
  oui_element_append_child(body, div);
  oui_element_set_width(div, oui_px(200));
  oui_element_set_text_content(div, "Hello from C!");

  oui_document_layout(doc);
  CHECK_GT(oui_element_get_height(div), 0.0f, "text gives height");

  oui_element_destroy(div);
}

static void test_computed_style(OuiDocument* doc) {
  OuiElement* body = oui_document_body(doc);
  OuiElement* div = oui_element_create(doc, "div");
  oui_element_append_child(body, div);
  oui_element_set_width(div, oui_px(300));

  oui_document_layout(doc);
  char* val = oui_element_get_computed_style(div, "width");
  CHECK_NE(val, (char*)NULL, "computed style not null");
  if (val) {
    CHECK_EQ(strcmp(val, "300px"), 0, "computed width is 300px");
    free(val);
  }

  oui_element_destroy(div);
}

static void test_border_box(OuiDocument* doc) {
  OuiElement* body = oui_document_body(doc);
  OuiElement* div = oui_element_create(doc, "div");
  oui_element_append_child(body, div);
  oui_element_set_style(div, "box-sizing", "border-box");
  oui_element_set_width(div, oui_px(200));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_padding(div, oui_px(10), oui_px(10), oui_px(10), oui_px(10));

  oui_document_layout(doc);
  CHECK_NEAR(oui_element_get_width(div), 200.0f, 0.1f,
             "border-box width stays 200");
  CHECK_NEAR(oui_element_get_height(div), 100.0f, 0.1f,
             "border-box height stays 100");

  oui_element_destroy(div);
}

int main(int argc, char** argv) {
  (void)argc;
  (void)argv;

  /* Value constructors don't need init. */
  test_value_constructors();

  /* Initialize the runtime. */
  OuiInitConfig config;
  memset(&config, 0, sizeof(config));
  OuiStatus status = oui_init(&config);
  if (status != OUI_OK) {
    fprintf(stderr, "FATAL: oui_init failed: %d\n", status);
    return 1;
  }

  /* All tests create their own document. */
  {
    OuiDocument* doc = oui_document_create(800, 600);
    test_document_create_destroy(doc);
    oui_document_destroy(doc);
  }
  {
    OuiDocument* doc = oui_document_create(800, 600);
    test_element_create(doc);
    oui_document_destroy(doc);
  }
  {
    OuiDocument* doc = oui_document_create(800, 600);
    test_dom_tree(doc);
    oui_document_destroy(doc);
  }
  {
    OuiDocument* doc = oui_document_create(800, 600);
    test_flexbox_layout(doc);
    oui_document_destroy(doc);
  }
  {
    OuiDocument* doc = oui_document_create(800, 600);
    test_grid_layout(doc);
    oui_document_destroy(doc);
  }
  {
    OuiDocument* doc = oui_document_create(800, 600);
    test_generic_style(doc);
    oui_document_destroy(doc);
  }
  {
    OuiDocument* doc = oui_document_create(800, 600);
    test_text_content(doc);
    oui_document_destroy(doc);
  }
  {
    OuiDocument* doc = oui_document_create(800, 600);
    test_computed_style(doc);
    oui_document_destroy(doc);
  }
  {
    OuiDocument* doc = oui_document_create(800, 600);
    test_border_box(doc);
    oui_document_destroy(doc);
  }

  oui_shutdown();

  printf("\n=== C Test Results: %d passed, %d failed ===\n",
         g_passed, g_failed);
  return g_failed > 0 ? 1 : 0;
}
