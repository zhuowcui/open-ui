/* Copyright 2025 The Open UI Authors
 * SPDX-License-Identifier: BSD-3-Clause
 *
 * openui_c_render_test.c — Pure C consumer test for offscreen rendering (SP5).
 * Compiled as C (not C++) to verify the render API is C-compatible.
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

#define CHECK_EQ(a, b, msg) CHECK((a) == (b), msg)
#define CHECK_NE(a, b, msg) CHECK((a) != (b), msg)
#define CHECK_GT(a, b, msg) CHECK((a) > (b), msg)

/* Helper: check pixel at (x,y) in bitmap is approximately (er,eg,eb,ea). */
static void check_pixel_near(const OuiBitmap* bmp, int x, int y,
                              int er, int eg, int eb, int ea,
                              int tol, const char* label) {
  int idx;
  int r, g, b, a;
  char msg[256];

  if (x < 0 || x >= bmp->width || y < 0 || y >= bmp->height) {
    snprintf(msg, sizeof(msg), "%s: pixel (%d,%d) out of bounds", label, x, y);
    CHECK(0, msg);
    return;
  }

  idx = y * bmp->stride + x * 4;
  r = bmp->pixels[idx + 0];
  g = bmp->pixels[idx + 1];
  b = bmp->pixels[idx + 2];
  a = bmp->pixels[idx + 3];

  snprintf(msg, sizeof(msg), "%s: R expected ~%d got %d", label, er, r);
  CHECK(abs(r - er) <= tol, msg);
  snprintf(msg, sizeof(msg), "%s: G expected ~%d got %d", label, eg, g);
  CHECK(abs(g - eg) <= tol, msg);
  snprintf(msg, sizeof(msg), "%s: B expected ~%d got %d", label, eb, b);
  CHECK(abs(b - eb) <= tol, msg);
  snprintf(msg, sizeof(msg), "%s: A expected ~%d got %d", label, ea, a);
  CHECK(abs(a - ea) <= tol, msg);
}

static void test_render_to_bitmap(OuiDocument* doc) {
  OuiElement* body = oui_document_body(doc);
  OuiElement* div;
  OuiBitmap bmp;
  OuiStatus status;

  memset(&bmp, 0, sizeof(bmp));

  div = oui_element_create(doc, "div");
  CHECK_NE(div, (OuiElement*)NULL, "render div created");

  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0xFF0000FF);  /* Red */
  oui_element_append_child(body, div);

  status = oui_document_render_to_bitmap(doc, &bmp);
  CHECK_EQ(status, OUI_OK, "render_to_bitmap returns OK");
  CHECK_EQ(bmp.width, 800, "bitmap width 800");
  CHECK_EQ(bmp.height, 600, "bitmap height 600");
  CHECK_EQ(bmp.stride, 800 * 4, "bitmap stride correct");
  CHECK_NE(bmp.pixels, (uint8_t*)NULL, "bitmap pixels non-null");

  /* Red div center should be red. */
  check_pixel_near(&bmp, 50, 50, 255, 0, 0, 255, 5, "red div center");

  /* Outside should be white. */
  check_pixel_near(&bmp, 400, 300, 255, 255, 255, 255, 5, "background white");

  oui_bitmap_free(&bmp);
  CHECK_EQ(bmp.pixels, (uint8_t*)NULL, "bitmap freed");

  /* Remove the div for the next test. */
  oui_element_remove_child(body, div);
  oui_element_destroy(div);
}

static void test_render_to_png_buffer(OuiDocument* doc) {
  OuiElement* body = oui_document_body(doc);
  OuiElement* div;
  OuiStatus status;
  uint8_t* png_data = NULL;
  size_t png_size = 0;

  div = oui_element_create(doc, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0x00FF00FF);  /* Green */
  oui_element_append_child(body, div);

  status = oui_document_render_to_png_buffer(doc, &png_data, &png_size);
  CHECK_EQ(status, OUI_OK, "render_to_png_buffer returns OK");
  CHECK_NE(png_data, (uint8_t*)NULL, "png data non-null");
  CHECK_GT(png_size, (size_t)8, "png size > 8 bytes");

  /* Check PNG signature. */
  CHECK_EQ(png_data[0], 0x89, "PNG sig byte 0");
  CHECK_EQ(png_data[1], 'P', "PNG sig byte 1");
  CHECK_EQ(png_data[2], 'N', "PNG sig byte 2");
  CHECK_EQ(png_data[3], 'G', "PNG sig byte 3");

  oui_free(png_data);

  oui_element_remove_child(body, div);
  oui_element_destroy(div);
}

static void test_render_to_png_file(OuiDocument* doc) {
  OuiElement* body = oui_document_body(doc);
  OuiElement* div;
  OuiStatus status;
  FILE* f;
  long file_size;
  const char* path = "/tmp/openui_c_render_test.png";

  div = oui_element_create(doc, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0x0000FFFF);  /* Blue */
  oui_element_append_child(body, div);

  status = oui_document_render_to_png(doc, path);
  CHECK_EQ(status, OUI_OK, "render_to_png returns OK");

  /* Verify file exists and is non-empty. */
  f = fopen(path, "rb");
  CHECK_NE(f, (FILE*)NULL, "PNG file opened");
  if (f) {
    fseek(f, 0, SEEK_END);
    file_size = ftell(f);
    CHECK_GT(file_size, 0L, "PNG file non-empty");
    fclose(f);
    remove(path);
  }

  oui_element_remove_child(body, div);
  oui_element_destroy(div);
}

static void test_render_null_args(void) {
  OuiBitmap bmp;
  uint8_t* data = NULL;
  size_t size = 0;

  memset(&bmp, 0, sizeof(bmp));

  CHECK_EQ(oui_document_render_to_bitmap(NULL, &bmp),
           OUI_ERROR_INVALID_ARGUMENT,
           "bitmap null doc");
  CHECK_EQ(oui_document_render_to_png(NULL, "/tmp/x.png"),
           OUI_ERROR_INVALID_ARGUMENT,
           "png null doc");
  CHECK_EQ(oui_document_render_to_png_buffer(NULL, &data, &size),
           OUI_ERROR_INVALID_ARGUMENT,
           "png_buffer null doc");

  /* Free null should be safe. */
  oui_bitmap_free(NULL);
  oui_free(NULL);
  g_passed++;  /* If we got here, no crash. */
}

static void test_re_render_after_mutation(OuiDocument* doc) {
  OuiElement* body = oui_document_body(doc);
  OuiElement* div;
  OuiBitmap bmp1, bmp2;

  memset(&bmp1, 0, sizeof(bmp1));
  memset(&bmp2, 0, sizeof(bmp2));

  div = oui_element_create(doc, "div");
  oui_element_set_width(div, oui_px(100));
  oui_element_set_height(div, oui_px(100));
  oui_element_set_background_color(div, 0xFF0000FF);  /* Red */
  oui_element_append_child(body, div);

  CHECK_EQ(oui_document_render_to_bitmap(doc, &bmp1), OUI_OK,
           "first render OK");
  check_pixel_near(&bmp1, 50, 50, 255, 0, 0, 255, 5, "first render red");

  /* Mutate to lime (0x00FF00FF). */
  oui_element_set_background_color(div, 0x00FF00FF);

  CHECK_EQ(oui_document_render_to_bitmap(doc, &bmp2), OUI_OK,
           "second render OK");
  check_pixel_near(&bmp2, 50, 50, 0, 255, 0, 255, 10, "second render lime");

  oui_bitmap_free(&bmp1);
  oui_bitmap_free(&bmp2);
  oui_element_remove_child(body, div);
  oui_element_destroy(div);
}

static void test_empty_document_render(OuiDocument* doc) {
  OuiBitmap bmp;
  memset(&bmp, 0, sizeof(bmp));

  CHECK_EQ(oui_document_render_to_bitmap(doc, &bmp), OUI_OK,
           "empty doc render OK");
  CHECK_EQ(bmp.width, 800, "empty doc width");
  CHECK_EQ(bmp.height, 600, "empty doc height");

  /* Should be white. */
  check_pixel_near(&bmp, 400, 300, 255, 255, 255, 255, 2, "empty doc white");

  oui_bitmap_free(&bmp);
}

int main(int argc, char** argv) {
  OuiInitConfig config;
  OuiStatus init_status;
  OuiDocument* doc;

  (void)argc;
  (void)argv;

  memset(&config, 0, sizeof(config));
  init_status = oui_init(&config);
  if (init_status != OUI_OK) {
    fprintf(stderr, "oui_init() failed: %d\n", init_status);
    return 1;
  }

  doc = oui_document_create(800, 600);
  if (!doc) {
    fprintf(stderr, "oui_document_create() failed\n");
    oui_shutdown();
    return 1;
  }

  test_render_null_args();
  test_empty_document_render(doc);
  test_render_to_bitmap(doc);
  test_render_to_png_buffer(doc);
  test_render_to_png_file(doc);
  test_re_render_after_mutation(doc);

  oui_document_destroy(doc);
  oui_shutdown();

  printf("=== C Render Test Results: %d passed, %d failed ===\n",
         g_passed, g_failed);
  return g_failed > 0 ? 1 : 0;
}
