// openui_render_pages.cc — Recreates HTML test pages using the Open UI C API
// and renders each to a PNG file. The output is compared against reference
// renders of the same HTML loaded through the same pipeline.
//
// Build: Add as a GN executable target depending on :openui_lib.
// Run:   ./openui_render_pages <output_dir>                   — C API renders
//        ./openui_render_pages --html <html_dir> <output_dir> — HTML renders

#include "openui/openui.h"

#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <dirent.h>
#include <string>
#include <vector>

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

// ─── Page 6: rounded_shadows — border-radius and box-shadow ──────────
static void RenderRoundedShadows(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xFFFFFFFF);

  // .card: 200x150 blue rounded with shadow
  OuiElement* card = oui_element_create(doc, "div");
  oui_element_set_width(card, oui_px(200));
  oui_element_set_height(card, oui_px(150));
  oui_element_set_margin(card, oui_px(30), oui_px(30), oui_px(30), oui_px(30));
  oui_element_set_background_color(card, 0x3498dbFF);
  oui_element_set_style(card, "border-radius", "20px");
  oui_element_set_style(card, "box-shadow", "8px 8px 16px rgba(0,0,0,0.3)");
  oui_element_append_child(body, card);

  // .pill: 160x50 red pill shape
  OuiElement* pill = oui_element_create(doc, "div");
  oui_element_set_width(pill, oui_px(160));
  oui_element_set_height(pill, oui_px(50));
  oui_element_set_margin(pill, oui_px(30), oui_px(30), oui_px(30), oui_px(30));
  oui_element_set_background_color(pill, 0xe74c3cFF);
  oui_element_set_style(pill, "border-radius", "25px");
  oui_element_append_child(body, pill);

  // .circle: 100x100 green circle
  OuiElement* circle = oui_element_create(doc, "div");
  oui_element_set_width(circle, oui_px(100));
  oui_element_set_height(circle, oui_px(100));
  oui_element_set_margin(circle, oui_px(30), oui_px(30), oui_px(30), oui_px(30));
  oui_element_set_background_color(circle, 0x2ecc71FF);
  oui_element_set_style(circle, "border-radius", "50%");
  oui_element_append_child(body, circle);

  std::string path = MakePath(output_dir, "rounded_shadows.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  rounded_shadows: %s\n", s == OUI_OK ? "OK" : "FAIL");
  oui_document_destroy(doc);
}

// ─── Page 7: transforms — CSS transforms ─────────────────────────────
static void RenderTransforms(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xFFFFFFFF);

  // .container: flex center, holds the rotated box
  OuiElement* container = oui_element_create(doc, "div");
  oui_element_set_width(container, oui_px(300));
  oui_element_set_height(container, oui_px(300));
  oui_element_set_margin(container, oui_px(50), oui_px(50), oui_px(50), oui_px(50));
  oui_element_set_display(container, OUI_DISPLAY_FLEX);
  oui_element_set_align_items(container, OUI_ALIGN_CENTER);
  oui_element_set_justify_content(container, OUI_JUSTIFY_CENTER);
  oui_element_append_child(body, container);

  OuiElement* rotated = oui_element_create(doc, "div");
  oui_element_set_width(rotated, oui_px(120));
  oui_element_set_height(rotated, oui_px(80));
  oui_element_set_background_color(rotated, 0x9b59b6FF);
  oui_element_set_style(rotated, "transform", "rotate(45deg)");
  oui_element_append_child(container, rotated);

  // .scaled: absolutely positioned, scaled 1.5x
  OuiElement* scaled = oui_element_create(doc, "div");
  oui_element_set_width(scaled, oui_px(100));
  oui_element_set_height(scaled, oui_px(100));
  oui_element_set_background_color(scaled, 0xe67e22FF);
  oui_element_set_style(scaled, "transform", "scale(1.5)");
  oui_element_set_position(scaled, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(scaled, "left", "400px");
  oui_element_set_style(scaled, "top", "50px");
  oui_element_append_child(body, scaled);

  // .skewed: skewX
  OuiElement* skewed = oui_element_create(doc, "div");
  oui_element_set_width(skewed, oui_px(150));
  oui_element_set_height(skewed, oui_px(60));
  oui_element_set_background_color(skewed, 0x1abc9cFF);
  oui_element_set_style(skewed, "transform", "skewX(-15deg)");
  oui_element_set_position(skewed, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(skewed, "left", "50px");
  oui_element_set_style(skewed, "top", "350px");
  oui_element_append_child(body, skewed);

  // .multi: compound transform
  OuiElement* multi = oui_element_create(doc, "div");
  oui_element_set_width(multi, oui_px(80));
  oui_element_set_height(multi, oui_px(80));
  oui_element_set_background_color(multi, 0xe74c3cFF);
  oui_element_set_style(multi, "transform",
                        "rotate(30deg) scale(1.2) translateX(20px)");
  oui_element_set_position(multi, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(multi, "left", "400px");
  oui_element_set_style(multi, "top", "300px");
  oui_element_append_child(body, multi);

  std::string path = MakePath(output_dir, "transforms.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  transforms: %s\n", s == OUI_OK ? "OK" : "FAIL");
  oui_document_destroy(doc);
}

// ─── Page 8: opacity_gradients — opacity blending and gradients ──────
static void RenderOpacityGradients(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xFFFFFFFF);

  // .base: dark blue-gray square
  OuiElement* base = oui_element_create(doc, "div");
  oui_element_set_width(base, oui_px(200));
  oui_element_set_height(base, oui_px(200));
  oui_element_set_background_color(base, 0x2c3e50FF);
  oui_element_set_position(base, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(base, "left", "50px");
  oui_element_set_style(base, "top", "50px");
  oui_element_append_child(body, base);

  // DOM order: overlay2 before overlay1
  OuiElement* overlay2 = oui_element_create(doc, "div");
  oui_element_set_width(overlay2, oui_px(200));
  oui_element_set_height(overlay2, oui_px(200));
  oui_element_set_background_color(overlay2, 0x3498dbFF);
  oui_element_set_opacity(overlay2, 0.5f);
  oui_element_set_position(overlay2, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(overlay2, "left", "150px");
  oui_element_set_style(overlay2, "top", "50px");
  oui_element_append_child(body, overlay2);

  OuiElement* overlay1 = oui_element_create(doc, "div");
  oui_element_set_width(overlay1, oui_px(200));
  oui_element_set_height(overlay1, oui_px(200));
  oui_element_set_background_color(overlay1, 0xe74c3cFF);
  oui_element_set_opacity(overlay1, 0.7f);
  oui_element_set_position(overlay1, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(overlay1, "left", "100px");
  oui_element_set_style(overlay1, "top", "100px");
  oui_element_append_child(body, overlay1);

  // linear gradient: red → green → blue
  OuiElement* gradBox = oui_element_create(doc, "div");
  oui_element_set_width(gradBox, oui_px(300));
  oui_element_set_height(gradBox, oui_px(100));
  oui_element_set_style(gradBox, "background",
                        "linear-gradient(90deg, #ff0000, #00ff00, #0000ff)");
  oui_element_set_position(gradBox, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(gradBox, "left", "50px");
  oui_element_set_style(gradBox, "top", "320px");
  oui_element_append_child(body, gradBox);

  // radial gradient: yellow → orange → dark red
  OuiElement* radialBox = oui_element_create(doc, "div");
  oui_element_set_width(radialBox, oui_px(200));
  oui_element_set_height(radialBox, oui_px(200));
  oui_element_set_style(
      radialBox, "background",
      "radial-gradient(circle, #ffff00, #ff6600, #cc0000)");
  oui_element_set_position(radialBox, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(radialBox, "left", "400px");
  oui_element_set_style(radialBox, "top", "320px");
  oui_element_append_child(body, radialBox);

  std::string path = MakePath(output_dir, "opacity_gradients.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  opacity_gradients: %s\n", s == OUI_OK ? "OK" : "FAIL");
  oui_document_destroy(doc);
}

// ─── Page 9: positioning_zindex — absolute/relative positioning ──────
static void RenderPositioningZindex(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xFFFFFFFF);

  // .container: relative, 400x400, margin:20px
  OuiElement* container = oui_element_create(doc, "div");
  oui_element_set_position(container, OUI_POSITION_RELATIVE);
  oui_element_set_width(container, oui_px(400));
  oui_element_set_height(container, oui_px(400));
  oui_element_set_margin(container, oui_px(20), oui_px(20), oui_px(20), oui_px(20));
  oui_element_append_child(body, container);

  // .abs-tl: red, z-index:1
  OuiElement* absTl = oui_element_create(doc, "div");
  oui_element_set_position(absTl, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(absTl, "top", "0");
  oui_element_set_style(absTl, "left", "0");
  oui_element_set_width(absTl, oui_px(120));
  oui_element_set_height(absTl, oui_px(120));
  oui_element_set_background_color(absTl, 0xe74c3cFF);
  oui_element_set_z_index(absTl, 1);
  oui_element_append_child(container, absTl);

  // .abs-br: green, z-index:2, overlaps
  OuiElement* absBr = oui_element_create(doc, "div");
  oui_element_set_position(absBr, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(absBr, "top", "30px");
  oui_element_set_style(absBr, "left", "30px");
  oui_element_set_width(absBr, oui_px(180));
  oui_element_set_height(absBr, oui_px(180));
  oui_element_set_background_color(absBr, 0x2ecc71FF);
  oui_element_set_z_index(absBr, 2);
  oui_element_append_child(container, absBr);

  // .abs-center: blue, z-index:3, on top
  OuiElement* absCenter = oui_element_create(doc, "div");
  oui_element_set_position(absCenter, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(absCenter, "top", "60px");
  oui_element_set_style(absCenter, "left", "60px");
  oui_element_set_width(absCenter, oui_px(120));
  oui_element_set_height(absCenter, oui_px(120));
  oui_element_set_background_color(absCenter, 0x3498dbFF);
  oui_element_set_z_index(absCenter, 3);
  oui_element_append_child(container, absCenter);

  // .rel-offset: orange, position:relative offset
  OuiElement* relOffset = oui_element_create(doc, "div");
  oui_element_set_position(relOffset, OUI_POSITION_RELATIVE);
  oui_element_set_style(relOffset, "top", "280px");
  oui_element_set_style(relOffset, "left", "200px");
  oui_element_set_width(relOffset, oui_px(100));
  oui_element_set_height(relOffset, oui_px(100));
  oui_element_set_background_color(relOffset, 0xf39c12FF);
  oui_element_set_z_index(relOffset, 1);
  oui_element_append_child(container, relOffset);

  // .fixed-bottom: dark bar at bottom, z-index:10
  OuiElement* fixedBottom = oui_element_create(doc, "div");
  oui_element_set_position(fixedBottom, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(fixedBottom, "bottom", "0");
  oui_element_set_style(fixedBottom, "left", "0");
  oui_element_set_style(fixedBottom, "right", "0");
  oui_element_set_height(fixedBottom, oui_px(60));
  oui_element_set_background_color(fixedBottom, 0x2c3e50FF);
  oui_element_set_z_index(fixedBottom, 10);
  oui_element_append_child(container, fixedBottom);

  std::string path = MakePath(output_dir, "positioning_zindex.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  positioning_zindex: %s\n", s == OUI_OK ? "OK" : "FAIL");
  oui_document_destroy(doc);
}

// ─── Page 10: overflow_clipping — overflow:hidden and nested clips ───
static void RenderOverflowClipping(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xFFFFFFFF);

  // .clip-box: 200x150, overflow:hidden, child overflows
  OuiElement* clipBox = oui_element_create(doc, "div");
  oui_element_set_width(clipBox, oui_px(200));
  oui_element_set_height(clipBox, oui_px(150));
  oui_element_set_overflow(clipBox, OUI_OVERFLOW_HIDDEN);
  oui_element_set_background_color(clipBox, 0xecf0f1FF);
  oui_element_set_margin(clipBox, oui_px(20), oui_px(20), oui_px(20), oui_px(20));
  oui_element_set_style(clipBox, "border", "2px solid #7f8c8d");
  oui_element_append_child(body, clipBox);

  OuiElement* inner = oui_element_create(doc, "div");
  oui_element_set_width(inner, oui_px(300));
  oui_element_set_height(inner, oui_px(300));
  oui_element_set_background_color(inner, 0xe74c3cFF);
  oui_element_set_style(inner, "margin-top", "-20px");
  oui_element_set_style(inner, "margin-left", "-30px");
  oui_element_append_child(clipBox, inner);

  // .scroll-box: stacked colored bars, only some visible
  OuiElement* scrollBox = oui_element_create(doc, "div");
  oui_element_set_width(scrollBox, oui_px(200));
  oui_element_set_height(scrollBox, oui_px(150));
  oui_element_set_overflow(scrollBox, OUI_OVERFLOW_HIDDEN);
  oui_element_set_background_color(scrollBox, 0xecf0f1FF);
  oui_element_set_margin(scrollBox, oui_px(20), oui_px(20), oui_px(20), oui_px(20));
  oui_element_set_style(scrollBox, "border", "2px solid #7f8c8d");
  oui_element_set_position(scrollBox, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(scrollBox, "left", "260px");
  oui_element_set_style(scrollBox, "top", "20px");
  oui_element_append_child(body, scrollBox);

  const uint32_t sColors[] = {0x3498dbFF, 0x2ecc71FF, 0xe67e22FF, 0x9b59b6FF};
  for (int i = 0; i < 4; i++) {
    OuiElement* item = oui_element_create(doc, "div");
    oui_element_set_width(item, oui_px(180));
    oui_element_set_height(item, oui_px(50));
    oui_element_set_margin(item, oui_px(10), oui_px(10), oui_px(10), oui_px(10));
    oui_element_set_background_color(item, sColors[i]);
    oui_element_append_child(scrollBox, item);
  }

  // .nested-clip: nested overflow with rounded child and gradient grandchild
  OuiElement* nestedClip = oui_element_create(doc, "div");
  oui_element_set_width(nestedClip, oui_px(250));
  oui_element_set_height(nestedClip, oui_px(250));
  oui_element_set_overflow(nestedClip, OUI_OVERFLOW_HIDDEN);
  oui_element_set_background_color(nestedClip, 0xbdc3c7FF);
  oui_element_set_margin(nestedClip, oui_px(20), oui_px(20), oui_px(20), oui_px(20));
  oui_element_set_position(nestedClip, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(nestedClip, "left", "20px");
  oui_element_set_style(nestedClip, "top", "220px");
  oui_element_set_style(nestedClip, "border", "3px solid #2c3e50");
  oui_element_append_child(body, nestedClip);

  OuiElement* child = oui_element_create(doc, "div");
  oui_element_set_width(child, oui_px(200));
  oui_element_set_height(child, oui_px(200));
  oui_element_set_overflow(child, OUI_OVERFLOW_HIDDEN);
  oui_element_set_background_color(child, 0x3498dbFF);
  oui_element_set_margin(child, oui_px(40), oui_px(40), oui_px(40), oui_px(40));
  oui_element_set_style(child, "border-radius", "20px");
  oui_element_append_child(nestedClip, child);

  OuiElement* grandchild = oui_element_create(doc, "div");
  oui_element_set_width(grandchild, oui_px(300));
  oui_element_set_height(grandchild, oui_px(300));
  oui_element_set_style(grandchild, "background",
                        "linear-gradient(135deg, #e74c3c, #f39c12)");
  oui_element_set_style(grandchild, "margin-top", "-50px");
  oui_element_append_child(child, grandchild);

  std::string path = MakePath(output_dir, "overflow_clipping.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  overflow_clipping: %s\n", s == OUI_OK ? "OK" : "FAIL");
  oui_document_destroy(doc);
}

// ─── Page 11: complex_ui — cards, badges, gradients, sidebar ─────────
static void RenderComplexUI(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xFFFFFFFF);

  // === Card 1 ===
  OuiElement* card1 = oui_element_create(doc, "div");
  oui_element_set_width(card1, oui_px(350));
  oui_element_set_margin(card1, oui_px(15), oui_px(20), oui_px(15), oui_px(20));
  oui_element_set_background_color(card1, 0xFFFFFFFF);
  oui_element_set_style(card1, "border", "1px solid #e0e0e0");
  oui_element_set_style(card1, "border-radius", "12px");
  oui_element_set_style(card1, "box-shadow", "0 2px 8px rgba(0,0,0,0.1)");
  oui_element_set_overflow(card1, OUI_OVERFLOW_HIDDEN);
  oui_element_append_child(body, card1);

  OuiElement* hdr1 = oui_element_create(doc, "div");
  oui_element_set_height(hdr1, oui_px(8));
  oui_element_set_style(hdr1, "background",
                        "linear-gradient(90deg, #667eea, #764ba2)");
  oui_element_append_child(card1, hdr1);

  OuiElement* cbody1 = oui_element_create(doc, "div");
  oui_element_set_padding(cbody1, oui_px(16), oui_px(20), oui_px(16), oui_px(20));
  oui_element_append_child(card1, cbody1);

  OuiElement* title1 = oui_element_create(doc, "div");
  oui_element_set_font_family(title1, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(title1, oui_px(18));
  oui_element_set_font_weight(title1, 700);
  oui_element_set_color(title1, 0x1a1a2eFF);
  oui_element_set_style(title1, "margin-bottom", "8px");
  oui_element_set_text_content(title1, "Dashboard Widget");
  oui_element_append_child(cbody1, title1);

  OuiElement* text1 = oui_element_create(doc, "div");
  oui_element_set_font_family(text1, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(text1, oui_px(13));
  oui_element_set_color(text1, 0x666666FF);
  oui_element_set_style(text1, "line-height", "1.5");
  oui_element_set_text_content(
      text1,
      "A complex UI component with gradients, rounded corners, shadows, "
      "typography, and nested flexbox layout.");
  oui_element_append_child(cbody1, text1);

  OuiElement* footer1 = oui_element_create(doc, "div");
  oui_element_set_display(footer1, OUI_DISPLAY_FLEX);
  oui_element_set_style(footer1, "border-top", "1px solid #f0f0f0");
  oui_element_set_padding(footer1, oui_px(12), oui_px(20), oui_px(12), oui_px(20));
  oui_element_set_justify_content(footer1, OUI_JUSTIFY_SPACE_BETWEEN);
  oui_element_set_align_items(footer1, OUI_ALIGN_CENTER);
  oui_element_append_child(card1, footer1);

  OuiElement* badge1 = oui_element_create(doc, "div");
  oui_element_set_display(badge1, OUI_DISPLAY_INLINE_BLOCK);
  oui_element_set_padding(badge1, oui_px(4), oui_px(12), oui_px(4), oui_px(12));
  oui_element_set_background_color(badge1, 0xe8f5e9FF);
  oui_element_set_color(badge1, 0x2e7d32FF);
  oui_element_set_style(badge1, "border-radius", "12px");
  oui_element_set_font_family(badge1, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(badge1, oui_px(11));
  oui_element_set_font_weight(badge1, 600);
  oui_element_set_text_content(badge1, "Active");
  oui_element_append_child(footer1, badge1);

  OuiElement* price1 = oui_element_create(doc, "div");
  oui_element_set_font_family(price1, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(price1, oui_px(20));
  oui_element_set_font_weight(price1, 700);
  oui_element_set_color(price1, 0x1a1a2eFF);
  oui_element_set_text_content(price1, "$49.99");
  oui_element_append_child(footer1, price1);

  // === Card 2 ===
  OuiElement* card2 = oui_element_create(doc, "div");
  oui_element_set_width(card2, oui_px(350));
  oui_element_set_margin(card2, oui_px(15), oui_px(20), oui_px(15), oui_px(20));
  oui_element_set_background_color(card2, 0xFFFFFFFF);
  oui_element_set_style(card2, "border", "1px solid #e0e0e0");
  oui_element_set_style(card2, "border-radius", "12px");
  oui_element_set_style(card2, "box-shadow", "0 2px 8px rgba(0,0,0,0.1)");
  oui_element_set_overflow(card2, OUI_OVERFLOW_HIDDEN);
  oui_element_append_child(body, card2);

  OuiElement* hdr2 = oui_element_create(doc, "div");
  oui_element_set_height(hdr2, oui_px(8));
  oui_element_set_style(hdr2, "background",
                        "linear-gradient(90deg, #f093fb, #f5576c)");
  oui_element_append_child(card2, hdr2);

  OuiElement* cbody2 = oui_element_create(doc, "div");
  oui_element_set_padding(cbody2, oui_px(16), oui_px(20), oui_px(16), oui_px(20));
  oui_element_append_child(card2, cbody2);

  OuiElement* title2 = oui_element_create(doc, "div");
  oui_element_set_font_family(title2, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(title2, oui_px(18));
  oui_element_set_font_weight(title2, 700);
  oui_element_set_color(title2, 0x1a1a2eFF);
  oui_element_set_style(title2, "margin-bottom", "8px");
  oui_element_set_text_content(title2, "Analytics Panel");
  oui_element_append_child(cbody2, title2);

  OuiElement* text2 = oui_element_create(doc, "div");
  oui_element_set_font_family(text2, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(text2, oui_px(13));
  oui_element_set_color(text2, 0x666666FF);
  oui_element_set_style(text2, "line-height", "1.5");
  oui_element_set_text_content(
      text2,
      "Testing multiple CSS properties: font-weight, line-height, "
      "border-radius, box-shadow, flex layout, and gradient backgrounds.");
  oui_element_append_child(cbody2, text2);

  OuiElement* footer2 = oui_element_create(doc, "div");
  oui_element_set_display(footer2, OUI_DISPLAY_FLEX);
  oui_element_set_style(footer2, "border-top", "1px solid #f0f0f0");
  oui_element_set_padding(footer2, oui_px(12), oui_px(20), oui_px(12), oui_px(20));
  oui_element_set_justify_content(footer2, OUI_JUSTIFY_SPACE_BETWEEN);
  oui_element_set_align_items(footer2, OUI_ALIGN_CENTER);
  oui_element_append_child(card2, footer2);

  OuiElement* badge2 = oui_element_create(doc, "div");
  oui_element_set_display(badge2, OUI_DISPLAY_INLINE_BLOCK);
  oui_element_set_padding(badge2, oui_px(4), oui_px(12), oui_px(4), oui_px(12));
  oui_element_set_background_color(badge2, 0xfce4ecFF);
  oui_element_set_color(badge2, 0xc62828FF);
  oui_element_set_style(badge2, "border-radius", "12px");
  oui_element_set_font_family(badge2, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(badge2, oui_px(11));
  oui_element_set_font_weight(badge2, 600);
  oui_element_set_text_content(badge2, "Premium");
  oui_element_append_child(footer2, badge2);

  OuiElement* price2 = oui_element_create(doc, "div");
  oui_element_set_font_family(price2, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(price2, oui_px(20));
  oui_element_set_font_weight(price2, 700);
  oui_element_set_color(price2, 0x1a1a2eFF);
  oui_element_set_text_content(price2, "$129.00");
  oui_element_append_child(footer2, price2);

  // === Sidebar ===
  OuiElement* sidebar = oui_element_create(doc, "div");
  oui_element_set_position(sidebar, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(sidebar, "right", "20px");
  oui_element_set_style(sidebar, "top", "15px");
  oui_element_set_width(sidebar, oui_px(180));
  oui_element_append_child(body, sidebar);

  struct { const char* text; uint32_t dotColor; bool active; } navItems[] = {
    {"Overview", 0x1e88e5FF, true},
    {"Alerts",   0xe53935FF, false},
    {"Reports",  0x43a047FF, false},
    {"Settings", 0xfb8c00FF, false},
  };
  for (int i = 0; i < 4; i++) {
    OuiElement* navItem = oui_element_create(doc, "div");
    oui_element_set_display(navItem, OUI_DISPLAY_FLEX);
    oui_element_set_align_items(navItem, OUI_ALIGN_CENTER);
    oui_element_set_padding(navItem, oui_px(10), oui_px(14), oui_px(10), oui_px(14));
    oui_element_set_style(navItem, "margin-bottom", "4px");
    oui_element_set_style(navItem, "border-radius", "8px");
    oui_element_set_font_family(navItem, "Arial, Helvetica, sans-serif");
    oui_element_set_font_size(navItem, oui_px(13));

    if (navItems[i].active) {
      oui_element_set_background_color(navItem, 0xe3f2fdFF);
      oui_element_set_color(navItem, 0x1565c0FF);
      oui_element_set_font_weight(navItem, 600);
    } else {
      oui_element_set_background_color(navItem, 0xf8f9faFF);
      oui_element_set_color(navItem, 0x444444FF);
    }

    OuiElement* dot = oui_element_create(doc, "div");
    oui_element_set_width(dot, oui_px(8));
    oui_element_set_height(dot, oui_px(8));
    oui_element_set_style(dot, "border-radius", "50%");
    oui_element_set_style(dot, "margin-right", "10px");
    oui_element_set_background_color(dot, navItems[i].dotColor);
    oui_element_append_child(navItem, dot);

    OuiElement* label = oui_element_create(doc, "span");
    oui_element_set_text_content(label, navItems[i].text);
    oui_element_append_child(navItem, label);

    oui_element_append_child(sidebar, navItem);
  }

  std::string path = MakePath(output_dir, "complex_ui.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  complex_ui: %s\n", s == OUI_OK ? "OK" : "FAIL");
  oui_document_destroy(doc);
}

// ─── Page 12: typography — font sizes, weights, styles, alignment ────
static void RenderTypography(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xf5f5f5FF);

  // heading-xl: 36px bold
  OuiElement* headXl = oui_element_create(doc, "div");
  oui_element_set_font_family(headXl, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(headXl, oui_px(36));
  oui_element_set_font_weight(headXl, 900);
  oui_element_set_color(headXl, 0x1a1a2eFF);
  oui_element_set_margin(headXl, oui_px(20), oui_px(20), oui_px(20), oui_px(20));
  oui_element_set_style(headXl, "letter-spacing", "-1px");
  oui_element_set_text_content(headXl, "Typography Test");
  oui_element_append_child(body, headXl);

  // heading-md: 20px uppercase with letter-spacing
  OuiElement* headMd = oui_element_create(doc, "div");
  oui_element_set_font_family(headMd, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(headMd, oui_px(20));
  oui_element_set_font_weight(headMd, 600);
  oui_element_set_color(headMd, 0x333333FF);
  oui_element_set_margin(headMd, oui_px(10), oui_px(20), oui_px(10), oui_px(20));
  oui_element_set_style(headMd, "text-transform", "uppercase");
  oui_element_set_style(headMd, "letter-spacing", "3px");
  oui_element_set_text_content(headMd, "Subheading Uppercase");
  oui_element_append_child(body, headMd);

  // body-text: 14px, line-height 1.6, max-width 400px
  OuiElement* bodyText = oui_element_create(doc, "div");
  oui_element_set_font_family(bodyText, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(bodyText, oui_px(14));
  oui_element_set_font_weight(bodyText, 400);
  oui_element_set_color(bodyText, 0x555555FF);
  oui_element_set_style(bodyText, "line-height", "1.6");
  oui_element_set_margin(bodyText, oui_px(10), oui_px(20), oui_px(10), oui_px(20));
  oui_element_set_max_width(bodyText, oui_px(400));
  oui_element_set_text_content(
      bodyText,
      "This is body text at 14px with 1.6 line height. It should wrap "
      "naturally within its container. The font is Arial with normal weight "
      "and a dark gray color.");
  oui_element_append_child(body, bodyText);

  // italic
  OuiElement* italic = oui_element_create(doc, "div");
  oui_element_set_font_family(italic, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(italic, oui_px(14));
  oui_element_set_font_style(italic, OUI_FONT_STYLE_ITALIC);
  oui_element_set_color(italic, 0x888888FF);
  oui_element_set_margin(italic, oui_px(5), oui_px(20), oui_px(5), oui_px(20));
  oui_element_set_text_content(italic,
                               "Italic text for emphasis and style variation.");
  oui_element_append_child(body, italic);

  // colored bold text
  struct { const char* t; uint32_t c; } colored[] = {
    {"Red bold text",   0xe74c3cFF},
    {"Blue bold text",  0x2980b9FF},
    {"Green bold text", 0x27ae60FF},
  };
  for (int i = 0; i < 3; i++) {
    OuiElement* ct = oui_element_create(doc, "div");
    oui_element_set_font_family(ct, "Arial, Helvetica, sans-serif");
    oui_element_set_font_size(ct, oui_px(16));
    oui_element_set_font_weight(ct, 700);
    oui_element_set_margin(ct, oui_px(10), oui_px(20), oui_px(10), oui_px(20));
    oui_element_set_color(ct, colored[i].c);
    oui_element_set_text_content(ct, colored[i].t);
    oui_element_append_child(body, ct);
  }

  // text-right
  OuiElement* tRight = oui_element_create(doc, "div");
  oui_element_set_font_family(tRight, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(tRight, oui_px(14));
  oui_element_set_text_align(tRight, OUI_TEXT_ALIGN_RIGHT);
  oui_element_set_color(tRight, 0x333333FF);
  oui_element_set_margin(tRight, oui_px(10), oui_px(20), oui_px(10), oui_px(20));
  oui_element_set_width(tRight, oui_px(300));
  oui_element_set_background_color(tRight, 0xFFFFFFFF);
  oui_element_set_padding(tRight, oui_px(8), oui_px(8), oui_px(8), oui_px(8));
  oui_element_set_text_content(tRight, "Right-aligned text block");
  oui_element_append_child(body, tRight);

  // text-center
  OuiElement* tCenter = oui_element_create(doc, "div");
  oui_element_set_font_family(tCenter, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(tCenter, oui_px(14));
  oui_element_set_text_align(tCenter, OUI_TEXT_ALIGN_CENTER);
  oui_element_set_color(tCenter, 0x333333FF);
  oui_element_set_margin(tCenter, oui_px(10), oui_px(20), oui_px(10), oui_px(20));
  oui_element_set_width(tCenter, oui_px(300));
  oui_element_set_background_color(tCenter, 0xFFFFFFFF);
  oui_element_set_padding(tCenter, oui_px(8), oui_px(8), oui_px(8), oui_px(8));
  oui_element_set_text_content(tCenter, "Center-aligned text block");
  oui_element_append_child(body, tCenter);

  // text-justify
  OuiElement* tJustify = oui_element_create(doc, "div");
  oui_element_set_font_family(tJustify, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(tJustify, oui_px(14));
  oui_element_set_text_align(tJustify, OUI_TEXT_ALIGN_JUSTIFY);
  oui_element_set_color(tJustify, 0x333333FF);
  oui_element_set_margin(tJustify, oui_px(10), oui_px(20), oui_px(10), oui_px(20));
  oui_element_set_width(tJustify, oui_px(300));
  oui_element_set_background_color(tJustify, 0xFFFFFFFF);
  oui_element_set_padding(tJustify, oui_px(8), oui_px(8), oui_px(8), oui_px(8));
  oui_element_set_text_content(
      tJustify,
      "Justified text should stretch to fill the full width of its container "
      "evenly on both sides.");
  oui_element_append_child(body, tJustify);

  // small text
  OuiElement* small = oui_element_create(doc, "div");
  oui_element_set_font_family(small, "Arial, Helvetica, sans-serif");
  oui_element_set_font_size(small, oui_px(10));
  oui_element_set_color(small, 0x999999FF);
  oui_element_set_margin(small, oui_px(5), oui_px(20), oui_px(5), oui_px(20));
  oui_element_set_text_content(small,
                               "Tiny 10px caption text for fine details.");
  oui_element_append_child(body, small);

  std::string path = MakePath(output_dir, "typography.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  typography: %s\n", s == OUI_OK ? "OK" : "FAIL");
  oui_document_destroy(doc);
}

// ─── Page 13: borders_shadows — multi-color borders, outlines, insets ─
static void RenderBordersShadows(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xFFFFFFFF);

  // .inset: neumorphism-style inset shadow
  OuiElement* inset = oui_element_create(doc, "div");
  oui_element_set_width(inset, oui_px(250));
  oui_element_set_height(inset, oui_px(80));
  oui_element_set_margin(inset, oui_px(25), oui_px(25), oui_px(25), oui_px(25));
  oui_element_set_background_color(inset, 0xecf0f1FF);
  oui_element_set_style(inset, "border", "3px solid #bdc3c7");
  oui_element_set_style(inset, "border-radius", "12px");
  oui_element_set_style(
      inset, "box-shadow",
      "inset 4px 4px 8px rgba(0,0,0,0.2), "
      "inset -4px -4px 8px rgba(255,255,255,0.7)");
  oui_element_append_child(body, inset);

  // .multi-border: different color per side
  OuiElement* multiBorder = oui_element_create(doc, "div");
  oui_element_set_width(multiBorder, oui_px(200));
  oui_element_set_height(multiBorder, oui_px(200));
  oui_element_set_margin(multiBorder, oui_px(25), oui_px(25), oui_px(25), oui_px(25));
  oui_element_set_background_color(multiBorder, 0xFFFFFFFF);
  oui_element_set_style(multiBorder, "border-top", "8px solid #e74c3c");
  oui_element_set_style(multiBorder, "border-right", "8px solid #3498db");
  oui_element_set_style(multiBorder, "border-bottom", "8px solid #2ecc71");
  oui_element_set_style(multiBorder, "border-left", "8px solid #f39c12");
  oui_element_set_position(multiBorder, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(multiBorder, "left", "320px");
  oui_element_set_style(multiBorder, "top", "20px");
  oui_element_append_child(body, multiBorder);

  // .outline-box: border + dashed outline with offset
  OuiElement* outlineBox = oui_element_create(doc, "div");
  oui_element_set_width(outlineBox, oui_px(200));
  oui_element_set_height(outlineBox, oui_px(100));
  oui_element_set_margin(outlineBox, oui_px(25), oui_px(25), oui_px(25), oui_px(25));
  oui_element_set_background_color(outlineBox, 0xFFFFFFFF);
  oui_element_set_style(outlineBox, "border", "2px solid #333");
  oui_element_set_style(outlineBox, "outline", "4px dashed #e74c3c");
  oui_element_set_style(outlineBox, "outline-offset", "6px");
  oui_element_set_position(outlineBox, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(outlineBox, "left", "20px");
  oui_element_set_style(outlineBox, "top", "170px");
  oui_element_append_child(body, outlineBox);

  // .double-border: double border style
  OuiElement* doubleBorder = oui_element_create(doc, "div");
  oui_element_set_width(doubleBorder, oui_px(200));
  oui_element_set_height(doubleBorder, oui_px(100));
  oui_element_set_margin(doubleBorder, oui_px(25), oui_px(25), oui_px(25), oui_px(25));
  oui_element_set_background_color(doubleBorder, 0xffeaa7FF);
  oui_element_set_style(doubleBorder, "border", "6px double #2c3e50");
  oui_element_set_style(doubleBorder, "border-radius", "8px");
  oui_element_set_position(doubleBorder, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(doubleBorder, "left", "320px");
  oui_element_set_style(doubleBorder, "top", "280px");
  oui_element_append_child(body, doubleBorder);

  // .gradient-border: gradient as border via background trick
  OuiElement* gradBorder = oui_element_create(doc, "div");
  oui_element_set_width(gradBorder, oui_px(250));
  oui_element_set_height(gradBorder, oui_px(120));
  oui_element_set_style(
      gradBorder, "background",
      "linear-gradient(white, white) padding-box, "
      "linear-gradient(135deg, #667eea, #764ba2) border-box");
  oui_element_set_style(gradBorder, "border", "4px solid transparent");
  oui_element_set_style(gradBorder, "border-radius", "16px");
  oui_element_set_position(gradBorder, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(gradBorder, "left", "20px");
  oui_element_set_style(gradBorder, "top", "350px");
  oui_element_append_child(body, gradBorder);

  // .complex-shadow: layered box-shadows for depth
  OuiElement* complexShadow = oui_element_create(doc, "div");
  oui_element_set_width(complexShadow, oui_px(200));
  oui_element_set_height(complexShadow, oui_px(120));
  oui_element_set_background_color(complexShadow, 0xFFFFFFFF);
  oui_element_set_style(complexShadow, "border-radius", "16px");
  oui_element_set_style(
      complexShadow, "box-shadow",
      "0 1px 3px rgba(0,0,0,0.12), "
      "0 4px 6px rgba(0,0,0,0.08), "
      "0 12px 24px rgba(0,0,0,0.06)");
  oui_element_set_position(complexShadow, OUI_POSITION_ABSOLUTE);
  oui_element_set_style(complexShadow, "left", "350px");
  oui_element_set_style(complexShadow, "top", "430px");
  oui_element_append_child(body, complexShadow);

  std::string path = MakePath(output_dir, "borders_shadows.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  borders_shadows: %s\n", s == OUI_OK ? "OK" : "FAIL");
  oui_document_destroy(doc);
}

// ─── Page 14: dashboard_layout — full holy-grail layout ──────────────
static void RenderDashboardLayout(const char* output_dir) {
  OuiDocument* doc = oui_document_create(800, 600);
  OuiElement* body = oui_document_body(doc);
  oui_element_set_style(body, "margin", "0");
  oui_element_set_style(body, "padding", "0");
  oui_element_set_background_color(body, 0xf0f2f5FF);

  // Holy grail wrapper (flex column, full viewport)
  OuiElement* hg = oui_element_create(doc, "div");
  oui_element_set_display(hg, OUI_DISPLAY_FLEX);
  oui_element_set_flex_direction(hg, OUI_FLEX_COLUMN);
  oui_element_set_height(hg, oui_px(600));
  oui_element_set_width(hg, oui_px(800));
  oui_element_set_style(hg, "box-sizing", "border-box");
  oui_element_append_child(body, hg);

  // ─── Header ───
  OuiElement* header = oui_element_create(doc, "div");
  oui_element_set_height(header, oui_px(60));
  oui_element_set_style(header, "background",
                        "linear-gradient(90deg, #2c3e50, #3498db)");
  oui_element_set_display(header, OUI_DISPLAY_FLEX);
  oui_element_set_align_items(header, OUI_ALIGN_CENTER);
  oui_element_set_padding(header, oui_px(0), oui_px(20), oui_px(0), oui_px(20));
  oui_element_set_style(header, "box-sizing", "border-box");
  oui_element_append_child(hg, header);

  OuiElement* logo = oui_element_create(doc, "div");
  oui_element_set_width(logo, oui_px(32));
  oui_element_set_height(logo, oui_px(32));
  oui_element_set_background_color(logo, 0xFFFFFFFF);
  oui_element_set_style(logo, "border-radius", "8px");
  oui_element_append_child(header, logo);

  OuiElement* nav = oui_element_create(doc, "div");
  oui_element_set_display(nav, OUI_DISPLAY_FLEX);
  oui_element_set_style(nav, "margin-left", "30px");
  oui_element_append_child(header, nav);

  const char* navLabels[] = {"Dashboard", "Analytics", "Reports", "Settings"};
  for (int i = 0; i < 4; i++) {
    OuiElement* ni = oui_element_create(doc, "div");
    oui_element_set_padding(ni, oui_px(8), oui_px(16), oui_px(8), oui_px(16));
    oui_element_set_style(ni, "margin-right", "4px");
    oui_element_set_style(ni, "border-radius", "6px");
    oui_element_set_font_family(ni, "Arial, sans-serif");
    oui_element_set_font_size(ni, oui_px(13));
    oui_element_set_style(ni, "box-sizing", "border-box");
    if (i == 0) {
      oui_element_set_style(ni, "background", "rgba(255,255,255,0.15)");
      oui_element_set_color(ni, 0xFFFFFFFF);
    } else {
      oui_element_set_style(ni, "color", "rgba(255,255,255,0.7)");
    }
    oui_element_set_text_content(ni, navLabels[i]);
    oui_element_append_child(nav, ni);
  }

  // ─── Body row (sidebar + main) ───
  OuiElement* hgBody = oui_element_create(doc, "div");
  oui_element_set_style(hgBody, "flex", "1");
  oui_element_set_display(hgBody, OUI_DISPLAY_FLEX);
  oui_element_set_style(hgBody, "box-sizing", "border-box");
  oui_element_append_child(hg, hgBody);

  // ─── Sidebar ───
  OuiElement* sb = oui_element_create(doc, "div");
  oui_element_set_width(sb, oui_px(200));
  oui_element_set_background_color(sb, 0xFFFFFFFF);
  oui_element_set_style(sb, "border-right", "1px solid #e0e0e0");
  oui_element_set_padding(sb, oui_px(16), oui_px(16), oui_px(16), oui_px(16));
  oui_element_set_style(sb, "box-sizing", "border-box");
  oui_element_append_child(hgBody, sb);

  struct { const char* label; const char* links[3]; int activeIdx; } sects[] = {
    {"Main",  {"Overview", "Metrics", "Events"},  0},
    {"Tools", {"Explorer", "Builder", "Export"}, -1},
  };
  for (int s = 0; s < 2; s++) {
    OuiElement* sec = oui_element_create(doc, "div");
    oui_element_set_style(sec, "margin-bottom", "20px");
    oui_element_append_child(sb, sec);

    OuiElement* slbl = oui_element_create(doc, "div");
    oui_element_set_font_family(slbl, "Arial, sans-serif");
    oui_element_set_font_size(slbl, oui_px(10));
    oui_element_set_font_weight(slbl, 600);
    oui_element_set_color(slbl, 0x999999FF);
    oui_element_set_style(slbl, "text-transform", "uppercase");
    oui_element_set_style(slbl, "letter-spacing", "1px");
    oui_element_set_style(slbl, "margin-bottom", "8px");
    oui_element_set_text_content(slbl, sects[s].label);
    oui_element_append_child(sec, slbl);

    for (int l = 0; l < 3; l++) {
      OuiElement* lnk = oui_element_create(doc, "div");
      oui_element_set_display(lnk, OUI_DISPLAY_BLOCK);
      oui_element_set_padding(lnk, oui_px(6), oui_px(10), oui_px(6), oui_px(10));
      oui_element_set_style(lnk, "border-radius", "6px");
      oui_element_set_font_family(lnk, "Arial, sans-serif");
      oui_element_set_font_size(lnk, oui_px(13));
      oui_element_set_style(lnk, "margin-bottom", "2px");
      oui_element_set_style(lnk, "box-sizing", "border-box");
      if (l == sects[s].activeIdx) {
        oui_element_set_background_color(lnk, 0xe3f2fdFF);
        oui_element_set_color(lnk, 0x1565c0FF);
        oui_element_set_font_weight(lnk, 600);
      } else {
        oui_element_set_color(lnk, 0x555555FF);
      }
      oui_element_set_text_content(lnk, sects[s].links[l]);
      oui_element_append_child(sec, lnk);
    }
  }

  // ─── Main content area ───
  OuiElement* main = oui_element_create(doc, "div");
  oui_element_set_style(main, "flex", "1");
  oui_element_set_padding(main, oui_px(20), oui_px(20), oui_px(20), oui_px(20));
  oui_element_set_overflow(main, OUI_OVERFLOW_HIDDEN);
  oui_element_set_style(main, "box-sizing", "border-box");
  oui_element_append_child(hgBody, main);

  // ─── Stats row ───
  OuiElement* statsRow = oui_element_create(doc, "div");
  oui_element_set_display(statsRow, OUI_DISPLAY_FLEX);
  oui_element_set_style(statsRow, "gap", "16px");
  oui_element_set_style(statsRow, "margin-bottom", "20px");
  oui_element_set_style(statsRow, "box-sizing", "border-box");
  oui_element_append_child(main, statsRow);

  struct { const char* val; const char* lbl; const char* fw; uint32_t fc; }
      stats[] = {
    {"2,847",  "Users",   "72%", 0x3498dbFF},
    {"94.2%",  "Uptime",  "85%", 0x2ecc71FF},
    {"$12.4k", "Revenue", "45%", 0xe67e22FF},
  };
  for (int i = 0; i < 3; i++) {
    OuiElement* sc = oui_element_create(doc, "div");
    oui_element_set_style(sc, "flex", "1");
    oui_element_set_background_color(sc, 0xFFFFFFFF);
    oui_element_set_style(sc, "border-radius", "12px");
    oui_element_set_padding(sc, oui_px(16), oui_px(16), oui_px(16), oui_px(16));
    oui_element_set_style(sc, "box-shadow", "0 1px 3px rgba(0,0,0,0.08)");
    oui_element_set_style(sc, "box-sizing", "border-box");
    oui_element_append_child(statsRow, sc);

    OuiElement* sv = oui_element_create(doc, "div");
    oui_element_set_font_family(sv, "Arial, sans-serif");
    oui_element_set_font_size(sv, oui_px(28));
    oui_element_set_font_weight(sv, 700);
    oui_element_set_color(sv, 0x1a1a2eFF);
    oui_element_set_text_content(sv, stats[i].val);
    oui_element_append_child(sc, sv);

    OuiElement* sl = oui_element_create(doc, "div");
    oui_element_set_font_family(sl, "Arial, sans-serif");
    oui_element_set_font_size(sl, oui_px(12));
    oui_element_set_color(sl, 0x888888FF);
    oui_element_set_style(sl, "margin-top", "4px");
    oui_element_set_text_content(sl, stats[i].lbl);
    oui_element_append_child(sc, sl);

    OuiElement* bar = oui_element_create(doc, "div");
    oui_element_set_height(bar, oui_px(4));
    oui_element_set_style(bar, "border-radius", "2px");
    oui_element_set_style(bar, "margin-top", "12px");
    oui_element_set_background_color(bar, 0xe0e0e0FF);
    oui_element_append_child(sc, bar);

    OuiElement* fill = oui_element_create(doc, "div");
    oui_element_set_style(fill, "height", "100%");
    oui_element_set_style(fill, "border-radius", "2px");
    oui_element_set_style(fill, "width", stats[i].fw);
    oui_element_set_background_color(fill, stats[i].fc);
    oui_element_append_child(bar, fill);
  }

  // ─── Content grid ───
  OuiElement* grid = oui_element_create(doc, "div");
  oui_element_set_display(grid, OUI_DISPLAY_GRID);
  oui_element_set_style(grid, "grid-template-columns", "1fr 1fr");
  oui_element_set_style(grid, "gap", "16px");
  oui_element_set_style(grid, "box-sizing", "border-box");
  oui_element_append_child(main, grid);

  struct { const char* grad; const char* title; const char* desc; } cards[] = {
    {"linear-gradient(90deg, #667eea, #764ba2)", "Project Alpha",
     "Advanced analytics pipeline with real-time data processing."},
    {"linear-gradient(90deg, #f093fb, #f5576c)", "Project Beta",
     "Machine learning model deployment and monitoring system."},
    {"linear-gradient(90deg, #4facfe, #00f2fe)", "Project Gamma",
     "Cloud infrastructure automation and orchestration."},
    {"linear-gradient(90deg, #f6d365, #fda085)", "Project Delta",
     "User experience optimization through A/B testing framework."},
  };
  for (int i = 0; i < 4; i++) {
    OuiElement* card = oui_element_create(doc, "div");
    oui_element_set_background_color(card, 0xFFFFFFFF);
    oui_element_set_style(card, "border-radius", "12px");
    oui_element_set_overflow(card, OUI_OVERFLOW_HIDDEN);
    oui_element_set_style(card, "box-shadow", "0 1px 3px rgba(0,0,0,0.08)");
    oui_element_set_style(card, "box-sizing", "border-box");
    oui_element_append_child(grid, card);

    OuiElement* cbar = oui_element_create(doc, "div");
    oui_element_set_height(cbar, oui_px(6));
    oui_element_set_style(cbar, "background", cards[i].grad);
    oui_element_append_child(card, cbar);

    OuiElement* cb = oui_element_create(doc, "div");
    oui_element_set_padding(cb, oui_px(14), oui_px(16), oui_px(14), oui_px(16));
    oui_element_set_style(cb, "box-sizing", "border-box");
    oui_element_append_child(card, cb);

    OuiElement* ct = oui_element_create(doc, "div");
    oui_element_set_font_family(ct, "Arial, sans-serif");
    oui_element_set_font_size(ct, oui_px(14));
    oui_element_set_font_weight(ct, 600);
    oui_element_set_color(ct, 0x333333FF);
    oui_element_set_style(ct, "margin-bottom", "6px");
    oui_element_set_text_content(ct, cards[i].title);
    oui_element_append_child(cb, ct);

    OuiElement* cd = oui_element_create(doc, "div");
    oui_element_set_font_family(cd, "Arial, sans-serif");
    oui_element_set_font_size(cd, oui_px(12));
    oui_element_set_color(cd, 0x888888FF);
    oui_element_set_style(cd, "line-height", "1.4");
    oui_element_set_text_content(cd, cards[i].desc);
    oui_element_append_child(cb, cd);
  }

  // ─── Footer ───
  OuiElement* footer = oui_element_create(doc, "div");
  oui_element_set_height(footer, oui_px(40));
  oui_element_set_background_color(footer, 0xFFFFFFFF);
  oui_element_set_style(footer, "border-top", "1px solid #e0e0e0");
  oui_element_set_display(footer, OUI_DISPLAY_FLEX);
  oui_element_set_align_items(footer, OUI_ALIGN_CENTER);
  oui_element_set_padding(footer, oui_px(0), oui_px(20), oui_px(0), oui_px(20));
  oui_element_set_style(footer, "box-sizing", "border-box");
  oui_element_append_child(hg, footer);

  OuiElement* ftxt = oui_element_create(doc, "div");
  oui_element_set_font_family(ftxt, "Arial, sans-serif");
  oui_element_set_font_size(ftxt, oui_px(11));
  oui_element_set_color(ftxt, 0x999999FF);
  oui_element_set_text_content(ftxt, "Open UI Dashboard v1.0");
  oui_element_append_child(footer, ftxt);

  std::string path = MakePath(output_dir, "dashboard_layout.png");
  OuiStatus s = oui_document_render_to_png(doc, path.c_str());
  printf("  dashboard_layout: %s\n", s == OUI_OK ? "OK" : "FAIL");
  oui_document_destroy(doc);
}

// ─── HTML file rendering: loads HTML and renders through the same pipeline ──
static bool ReadFile(const char* path, std::string* out) {
  FILE* f = fopen(path, "rb");
  if (!f) return false;
  fseek(f, 0, SEEK_END);
  long sz = ftell(f);
  fseek(f, 0, SEEK_SET);
  out->resize(sz);
  size_t read = fread(&(*out)[0], 1, sz, f);
  fclose(f);
  return read == static_cast<size_t>(sz);
}

static void RenderHTMLFile(const char* html_path, const char* output_dir) {
  std::string html;
  if (!ReadFile(html_path, &html)) {
    printf("  SKIP (cannot read): %s\n", html_path);
    return;
  }

  OuiDocument* doc = oui_document_create(800, 600);
  OuiStatus s = oui_document_load_html(doc, html.c_str());
  if (s != OUI_OK) {
    printf("  FAIL (load_html): %s\n", html_path);
    oui_document_destroy(doc);
    return;
  }

  // Extract filename: "foo.html" → "foo.png"
  std::string basename(html_path);
  size_t slash = basename.rfind('/');
  if (slash != std::string::npos) basename = basename.substr(slash + 1);
  size_t dot = basename.rfind('.');
  if (dot != std::string::npos) basename = basename.substr(0, dot);
  basename += ".png";

  std::string path = MakePath(output_dir, basename.c_str());
  s = oui_document_render_to_png(doc, path.c_str());
  printf("  %s: %s\n", basename.c_str(), s == OUI_OK ? "OK" : "FAIL");
  oui_document_destroy(doc);
}

static int RenderHTMLDir(const char* html_dir, const char* output_dir) {
  DIR* dir = opendir(html_dir);
  if (!dir) {
    fprintf(stderr, "Cannot open HTML directory: %s\n", html_dir);
    return 1;
  }

  std::vector<std::string> files;
  struct dirent* entry;
  while ((entry = readdir(dir)) != nullptr) {
    std::string name(entry->d_name);
    if (name.size() > 5 && name.substr(name.size() - 5) == ".html") {
      files.push_back(name);
    }
  }
  closedir(dir);
  std::sort(files.begin(), files.end());

  printf("Rendering %zu HTML pages from %s to %s\n",
         files.size(), html_dir, output_dir);
  for (const auto& f : files) {
    std::string full = std::string(html_dir) + "/" + f;
    RenderHTMLFile(full.c_str(), output_dir);
  }
  return 0;
}

int main(int argc, char** argv) {
  if (argc < 2) {
    fprintf(stderr,
            "Usage: %s <output_dir> [--html-dir <html_pages_dir>]\n"
            "       %s --html <html_dir> <output_dir>\n",
            argv[0], argv[0]);
    return 1;
  }

  OuiInitConfig config = {};
  OuiStatus init = oui_init(&config);
  if (init != OUI_OK) {
    fprintf(stderr, "oui_init failed: %d\n", init);
    return 1;
  }

  // --html mode: render HTML files through the same DummyPageHolder pipeline
  if (argc >= 4 && strcmp(argv[1], "--html") == 0) {
    int ret = RenderHTMLDir(argv[2], argv[3]);
    oui_shutdown();
    return ret;
  }

  const char* output_dir = argv[1];

  // Optional: --html-dir <path> for SP6 load_html() tests
  const char* html_dir = nullptr;
  for (int i = 1; i < argc - 1; i++) {
    if (strcmp(argv[i], "--html-dir") == 0) {
      html_dir = argv[i + 1];
    }
  }

  printf("Rendering pages to %s\n", output_dir);

  // ─── SP5 hand-crafted C API pages (14) ─────────────────────────────
  RenderRedBox(output_dir);
  RenderRGBFlex(output_dir);
  RenderBorderBox(output_dir);
  RenderNestedFlex(output_dir);
  RenderGridColors(output_dir);
  RenderRoundedShadows(output_dir);
  RenderTransforms(output_dir);
  RenderOpacityGradients(output_dir);
  RenderPositioningZindex(output_dir);
  RenderOverflowClipping(output_dir);
  RenderComplexUI(output_dir);
  RenderTypography(output_dir);
  RenderBordersShadows(output_dir);
  RenderDashboardLayout(output_dir);

  // ─── SP6 pages via oui_document_load_html() ────────────────────────
  // These test our HTML loading pipeline (element factory, UA styles,
  // CSS parsing) by loading the same HTML files used for reference.
  if (!html_dir) {
    printf("NOTE: Pass --html-dir <path> to also render 25 SP6 test pages.\n");
  } else {
    static const char* kSP6Pages[] = {
      "test_semantic_blocks",
      "test_inline_text",
      "test_headings_text",
      "test_lists",
      "test_tables",
      "test_forms",
      "test_flexbox",
      "test_grid",
      "test_positioning",
      "test_box_model",
      "test_colors_backgrounds",
      "test_transforms_filters",
      "test_advanced_css",
      "test_svg_shapes",
      "test_svg_advanced",
      "website_blog",
      "website_ecommerce",
      "website_dashboard",
      "website_landing",
      "website_portfolio",
      "website_news",
      "website_docs",
      "website_social",
      "website_email",
      "website_analytics",
    };
    for (const char* name : kSP6Pages) {
      std::string html_path =
          std::string(html_dir) + "/" + name + ".html";
      std::string html;
      if (!ReadFile(html_path.c_str(), &html)) {
        printf("  %s: SKIP (cannot read %s)\n", name, html_path.c_str());
        continue;
      }
      OuiDocument* doc = oui_document_create(800, 600);
      OuiStatus s = oui_document_load_html(doc, html.c_str());
      if (s != OUI_OK) {
        printf("  %s: FAIL (load_html)\n", name);
        oui_document_destroy(doc);
        continue;
      }
      std::string png_path = MakePath(output_dir, (std::string(name) + ".png").c_str());
      s = oui_document_render_to_png(doc, png_path.c_str());
      printf("  %s: %s\n", name, s == OUI_OK ? "OK" : "FAIL");
      oui_document_destroy(doc);
    }
  }

  oui_shutdown();
  printf("Done.\n");
  return 0;
}
