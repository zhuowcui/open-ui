// openui_render_pages.cc — Recreates HTML test pages using the Open UI C API
// and renders each to a PNG file. The output is compared against Playwright
// browser screenshots for pixel-level validation.
//
// Build: Add as a GN executable target depending on :openui_lib.
// Run:   ./openui_render_pages <output_dir>

#include "openui/openui.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

// Helper to build a file path.
static std::string MakePath(const char* dir, const char* name) {
  std::string path(dir);
  if (!path.empty() && path.back() != '/') {
    path += '/';
  }
  path += name;
  return path;
}

// ─── Page 1: red_box — 200x200 red box on white ───────────────────────
static void RenderRedBox(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xFFFFFFFF);

  OuiElement* box = oui_element_create(doc, "div");
  oui_element_set_width(box, oui_px(200));
  oui_element_set_height(box, oui_px(200));
  oui_element_set_background_color(box, 0xFF0000FF);
  oui_element_append_child(body, box);

  std::string path = MakePath(output_dir, "red_box.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  red_box: %s\n", s == OUI_OK ? "OK" : "FAIL");

  oui_document_destroy(doc);
}

// ─── Page 2: rgb_flex — three 100x100 boxes side by side ──────────────
static void RenderRGBFlex(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xFFFFFFFF);
  oui_element_set_display(body, OUI_DISPLAY_FLEX);

  uint32_t colors[] = {0xFF0000FF, 0x00FF00FF, 0x0000FFFF};
  for (int i = 0; i < 3; i++) {
    OuiElement* d = oui_element_create(doc, "div");
    oui_element_set_width(d, oui_px(100));
    oui_element_set_height(d, oui_px(100));
    oui_element_set_background_color(d, colors[i]);
    oui_element_append_child(body, d);
  }

  std::string path = MakePath(output_dir, "rgb_flex.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  rgb_flex: %s\n", s == OUI_OK ? "OK" : "FAIL");

  oui_document_destroy(doc);
}

// ─── Page 3: border_box — 200x200 white box with 10px black border ───
static void RenderBorderBox(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xFFFFFFFF);

  OuiElement* box = oui_element_create(doc, "div");
  oui_element_set_width(box, oui_px(200));
  oui_element_set_height(box, oui_px(200));
  oui_element_set_background_color(box, 0xFFFFFFFF);
  oui_element_set_style(box, "border", "10px solid black");
  oui_element_set_style(box, "margin", "20px");
  oui_element_append_child(body, box);

  std::string path = MakePath(output_dir, "border_box.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  border_box: %s\n", s == OUI_OK ? "OK" : "FAIL");

  oui_document_destroy(doc);
}

// ─── Page 4: nested_flex — two rows of flex boxes ─────────────────────
static void RenderNestedFlex(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xFFFFFFFF);
  oui_element_set_display(body, OUI_DISPLAY_FLEX);
  oui_element_set_style(body, "flex-direction", "column");

  // Row 1: flex 1:2:1 ratio in 400px wide container
  OuiElement* row1 = oui_element_create(doc, "div");
  oui_element_set_display(row1, OUI_DISPLAY_FLEX);
  oui_element_set_width(row1, oui_px(400));
  oui_element_set_height(row1, oui_px(100));
  oui_element_set_background_color(row1, 0xEEEEEEFF);
  oui_element_append_child(body, row1);

  OuiElement* r1a = oui_element_create(doc, "div");
  oui_element_set_style(r1a, "flex", "1");
  oui_element_set_background_color(r1a, 0xFF6600FF);
  oui_element_append_child(row1, r1a);

  OuiElement* r1b = oui_element_create(doc, "div");
  oui_element_set_style(r1b, "flex", "2");
  oui_element_set_background_color(r1b, 0x0066FFFF);
  oui_element_append_child(row1, r1b);

  OuiElement* r1c = oui_element_create(doc, "div");
  oui_element_set_style(r1c, "flex", "1");
  oui_element_set_background_color(r1c, 0x66FF00FF);
  oui_element_append_child(row1, r1c);

  // Row 2: two 200px boxes
  OuiElement* row2 = oui_element_create(doc, "div");
  oui_element_set_display(row2, OUI_DISPLAY_FLEX);
  oui_element_set_width(row2, oui_px(400));
  oui_element_set_height(row2, oui_px(100));
  oui_element_set_background_color(row2, 0xDDDDDDFF);
  oui_element_append_child(body, row2);

  OuiElement* r2a = oui_element_create(doc, "div");
  oui_element_set_width(r2a, oui_px(200));
  oui_element_set_background_color(r2a, 0xFF00FFFF);
  oui_element_append_child(row2, r2a);

  OuiElement* r2b = oui_element_create(doc, "div");
  oui_element_set_width(r2b, oui_px(200));
  oui_element_set_background_color(r2b, 0x00FFFFFF);
  oui_element_append_child(row2, r2b);

  std::string path = MakePath(output_dir, "nested_flex.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  nested_flex: %s\n", s == OUI_OK ? "OK" : "FAIL");

  oui_document_destroy(doc);
}

// ─── Page 5: grid_colors — 3x2 grid of colored cells ─────────────────
static void RenderGridColors(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xFFFFFFFF);

  OuiElement* grid = oui_element_create(doc, "div");
  oui_element_set_display(grid, OUI_DISPLAY_GRID);
  oui_element_set_style(grid, "grid-template-columns", "100px 100px 100px");
  oui_element_set_style(grid, "grid-template-rows", "100px 100px");
  oui_element_set_width(grid, oui_px(300));
  oui_element_set_height(grid, oui_px(200));
  oui_element_append_child(body, grid);

  uint32_t colors[] = {
    0xFF0000FF, 0x00FF00FF, 0x0000FFFF,
    0xFFFF00FF, 0xFF00FFFF, 0x00FFFFFF,
  };

  for (int i = 0; i < 6; i++) {
    OuiElement* cell = oui_element_create(doc, "div");
    oui_element_set_background_color(cell, colors[i]);
    oui_element_append_child(grid, cell);
  }

  std::string path = MakePath(output_dir, "grid_colors.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  grid_colors: %s\n", s == OUI_OK ? "OK" : "FAIL");

  oui_document_destroy(doc);
}

int main(int argc, char** argv) {
  if (argc < 2) {
    fprintf(stderr, "Usage: %s <output_dir>\n", argv[0]);
    return 1;
  }

  const char* output_dir = argv[1];

  OuiInitConfig config = {};
  OuiStatus init = oui_init(&config);
  if (init != OUI_OK) {
    fprintf(stderr, "oui_init failed: %d\n", init);
    return 1;
  }

  printf("Rendering pages to %s\n", output_dir);
  RenderRedBox(output_dir);
  RenderRGBFlex(output_dir);
  RenderBorderBox(output_dir);
  RenderNestedFlex(output_dir);
  RenderGridColors(output_dir);

  oui_shutdown();
  printf("Done.\n");
  return 0;
}
