/* Public C/C++ consumers of retained native scrollbar and overflow geometry.
 * Run after the shared layout implementation and runtime ABI gate are built.
 * Logical dimensions must be independent of raster scale. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include "openui.h"

static OuiUtf8 utf8(const char* text) {
  OuiUtf8 result = {(const uint8_t*)text, strlen(text)};
  return result;
}

static void property(OuiElement* element, OuiStyleProperty id, const char* literal) {
  OuiStyleValue value;
  OuiStatus status = oui_style_value_parse(id, utf8(literal), &value);
  if (status != OUI_OK)
    fprintf(stderr, "native gutter property %d '%s': %d\n", (int)id, literal, (int)status);
  assert(status == OUI_OK);
  assert(oui_element_set_property(element, id, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}

typedef OuiStatus (*DimensionQuery)(OuiElement*, int32_t*);

static void dimensions(OuiElement* element, int32_t width, int32_t height,
                       int32_t scroll_width, int32_t scroll_height) {
  const DimensionQuery queries[] = {oui_element_get_client_width_v1,
                                   oui_element_get_client_height_v1,
                                   oui_element_get_scroll_width_v1,
                                   oui_element_get_scroll_height_v1};
  const int32_t expected[] = {width, height, scroll_width, scroll_height};
  for (size_t i = 0; i < sizeof(queries) / sizeof(queries[0]); ++i) {
    struct Output { int32_t value; uint8_t tail[12]; } output;
    output.value = INT32_MIN;
    memset(output.tail, 0x6d, sizeof(output.tail));
    assert(queries[i](element, &output.value) == OUI_OK);
    if (output.value != expected[i])
      fprintf(stderr, "native gutter dimension %zu: got %d expected %d\n", i,
              (int)output.value, (int)expected[i]);
    assert(output.value == expected[i]);
    for (size_t j = 0; j < sizeof(output.tail); ++j) assert(output.tail[j] == 0x6d);
  }
}

static OuiRect bounds(OuiElement* element) {
  OuiRect value;
  assert(oui_element_get_bounds(element, &value) == OUI_OK);
  return value;
}

static void size(OuiElement* element, float width, float height) {
  OuiRect value = bounds(element);
  assert(value.width == width && value.height == height);
}

static OuiScrollMetricsV1 raw_metrics(OuiElement* element) {
  OuiScrollMetricsV1 value = {sizeof(value), OUI_ABI_VERSION, 0.0, 0.0, 0.0, 0.0};
  uint8_t found = 0;
  assert(oui_element_get_scroll_metrics_v1(element, &value, &found) == OUI_OK);
  assert(found == 1);
  return value;
}

static void offset(OuiElement* element, double expected_x, double expected_y) {
  double x = -999.0, y = -999.0;
  assert(oui_element_get_scroll_offset(element, &x, &y) == OUI_OK);
  assert(x == expected_x && y == expected_y);
}

static void consumer(const char* display, double scale) {
  OuiDocumentConfig config = {sizeof(config), OUI_ABI_VERSION,
    {320.0, 240.0, (uint32_t)(320.0 * scale), (uint32_t)(240.0 * scale),
     scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement *root = NULL, *container = NULL, *child = NULL, *absolute = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &container) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &child) == OUI_OK);
  assert(oui_element_append_child(root, container) == OUI_OK);
  assert(oui_element_append_child(container, child) == OUI_OK);
  property(container, OUI_STYLE_PROPERTY_DISPLAY, display);
  property(container, OUI_STYLE_PROPERTY_POSITION, "absolute");
  property(container, OUI_STYLE_PROPERTY_LEFT, "20px");
  property(container, OUI_STYLE_PROPERTY_TOP, "20px");
  property(container, OUI_STYLE_PROPERTY_WIDTH, "100px");
  property(container, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  property(container, OUI_STYLE_PROPERTY_BOX_SIZING, "content-box");
  property(container, OUI_STYLE_PROPERTY_OVERFLOW, "scroll");
  property(container, OUI_STYLE_PROPERTY_FLEX_DIRECTION, "column");
  property(container, OUI_STYLE_PROPERTY_ALIGN_ITEMS, "start");
  property(container, OUI_STYLE_PROPERTY_GRID_TEMPLATE_COLUMNS, "100%");
  property(container, OUI_STYLE_PROPERTY_GRID_TEMPLATE_ROWS, "100%");
  property(child, OUI_STYLE_PROPERTY_WIDTH, "100%");
  property(child, OUI_STYLE_PROPERTY_HEIGHT, "100%");
  property(child, OUI_STYLE_PROPERTY_MIN_WIDTH, "0px");
  property(child, OUI_STYLE_PROPERTY_MIN_HEIGHT, "0px");
  property(child, OUI_STYLE_PROPERTY_FLEX_SHRINK, "0");
  dimensions(container, 85, 65, 85, 65);
  size(container, 100.0f, 80.0f);
  size(child, 85.0f, 65.0f);

  property(container, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
  dimensions(container, 100, 80, 100, 80);
  size(child, 100.0f, 80.0f);
  property(container, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "thin");
  dimensions(container, 90, 70, 90, 70);
  size(child, 90.0f, 70.0f);
  property(container, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "auto");
  property(container, OUI_STYLE_PROPERTY_PADDING, "5px");
  property(container, OUI_STYLE_PROPERTY_BORDER, "2px solid black");
  dimensions(container, 95, 75, 95, 75);
  size(container, 114.0f, 94.0f);
  size(child, 85.0f, 65.0f);

  /* Retain fractional layout metrics separately from the integer DOM-style API. */
  property(container, OUI_STYLE_PROPERTY_PADDING, "0px");
  property(container, OUI_STYLE_PROPERTY_BORDER, "0px solid black");
  property(container, OUI_STYLE_PROPERTY_WIDTH, "100.25px");
  property(container, OUI_STYLE_PROPERTY_HEIGHT, "80.5px");
  dimensions(container, 85, 66, 85, 66);
  OuiScrollMetricsV1 fractional = raw_metrics(container);
  assert(fractional.client_width == 85.25 && fractional.client_height == 65.5);
  size(child, 85.25f, 65.5f);
  property(container, OUI_STYLE_PROPERTY_TRANSFORM, "translate(0.25px, 0.5px) scale(2)");
  dimensions(container, 85, 66, 85, 66);
  property(container, OUI_STYLE_PROPERTY_TRANSFORM, "none");
  property(container, OUI_STYLE_PROPERTY_WIDTH, "100px");
  property(container, OUI_STYLE_PROPERTY_HEIGHT, "80px");

  /* Absolute percentages resolve against the same gutter-reduced containing box. */
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &absolute) == OUI_OK);
  assert(oui_element_append_child(container, absolute) == OUI_OK);
  property(absolute, OUI_STYLE_PROPERTY_POSITION, "absolute");
  property(absolute, OUI_STYLE_PROPERTY_LEFT, "0px");
  property(absolute, OUI_STYLE_PROPERTY_TOP, "0px");
  property(absolute, OUI_STYLE_PROPERTY_WIDTH, "100%");
  property(absolute, OUI_STYLE_PROPERTY_HEIGHT, "100%");
  dimensions(container, 85, 65, 85, 65);
  size(absolute, 85.0f, 65.0f);
  assert(oui_element_detach(absolute) == OUI_OK);

  /* Auto overflow must relayout percentage descendants and clamp retained offsets. */
  property(container, OUI_STYLE_PROPERTY_OVERFLOW, "auto");
  property(child, OUI_STYLE_PROPERTY_WIDTH, "150px");
  property(child, OUI_STYLE_PROPERTY_HEIGHT, "120px");
  dimensions(container, 85, 65, 150, 120);
  assert(oui_element_scroll_to(container, 1000.0, 1000.0) == OUI_OK);
  offset(container, 65.0, 55.0);
  property(child, OUI_STYLE_PROPERTY_WIDTH, "20px");
  property(child, OUI_STYLE_PROPERTY_HEIGHT, "20px");
  dimensions(container, 100, 80, 100, 80);
  offset(container, 0.0, 0.0);
  property(child, OUI_STYLE_PROPERTY_WIDTH, "100%");
  property(child, OUI_STYLE_PROPERTY_HEIGHT, "100%");
  dimensions(container, 100, 80, 100, 80);
  size(child, 100.0f, 80.0f);

  /* Stable gutters survive absent overflow and follow writing direction. */
  property(container, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  property(container, OUI_STYLE_PROPERTY_DIRECTION, "rtl");
  property(container, OUI_STYLE_PROPERTY_SCROLLBAR_GUTTER, "stable");
  dimensions(container, 85, 80, 85, 80);
  size(child, 85.0f, 80.0f);
  assert(bounds(child).x == bounds(container).x + 15.0f);
  property(container, OUI_STYLE_PROPERTY_SCROLLBAR_GUTTER, "stable both-edges");
  dimensions(container, 70, 80, 70, 80);
  size(child, 70.0f, 80.0f);
  assert(bounds(child).x == bounds(container).x + 15.0f);
  property(container, OUI_STYLE_PROPERTY_DIRECTION, "ltr");
  property(container, OUI_STYLE_PROPERTY_WRITING_MODE, "vertical-rl");
  property(container, OUI_STYLE_PROPERTY_SCROLLBAR_GUTTER, "stable");
  dimensions(container, 100, 65, 100, 65);
  size(child, 100.0f, 65.0f);
  assert(bounds(child).y == bounds(container).y);
  property(container, OUI_STYLE_PROPERTY_SCROLLBAR_GUTTER, "stable both-edges");
  dimensions(container, 100, 50, 100, 50);
  size(child, 100.0f, 50.0f);
  assert(bounds(child).y == bounds(container).y + 15.0f);

  /* No-box transitions use the API's zero dimensions, then recover live geometry. */
  property(container, OUI_STYLE_PROPERTY_DISPLAY, "none");
  dimensions(container, 0, 0, 0, 0);
  property(container, OUI_STYLE_PROPERTY_DISPLAY, display);
  dimensions(container, 100, 50, 100, 50);
  assert(oui_element_detach(container) == OUI_OK);
  dimensions(container, 0, 0, 0, 0);
  assert(oui_element_append_child(root, container) == OUI_OK);
  dimensions(container, 100, 50, 100, 50);
  assert(fractional.client_width == 85.25 && fractional.client_height == 65.5);

  assert(oui_element_destroy(absolute) == OUI_OK);
  assert(oui_element_destroy(child) == OUI_OK);
  assert(oui_element_destroy(container) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  printf("native C flex/grid scroll gutters: display=%s scale=%g passed\n", display, scale);
}

static OuiDocument* document_for_scale(double scale, OuiElement** root) {
  OuiDocumentConfig config = {sizeof(config), OUI_ABI_VERSION,
    {320.0, 240.0, (uint32_t)(320.0 * scale), (uint32_t)(240.0 * scale),
     scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, root) == OUI_OK);
  return document;
}

static void authored_size(OuiElement* element) {
  property(element, OUI_STYLE_PROPERTY_WIDTH, "100px");
  property(element, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  property(element, OUI_STYLE_PROPERTY_BOX_SIZING, "content-box");
}

static void table_consumer(double scale) {
  OuiElement *root = NULL, *table = NULL, *row = NULL, *cell = NULL;
  OuiDocument* document = document_for_scale(scale, &root);
  assert(oui_element_create(document, OUI_ELEMENT_TABLE, &table) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_TABLE_ROW, &row) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_TABLE_CELL, &cell) == OUI_OK);
  assert(oui_element_append_child(root, table) == OUI_OK);
  assert(oui_element_append_child(table, row) == OUI_OK);
  assert(oui_element_append_child(row, cell) == OUI_OK);
  authored_size(table);
  property(table, OUI_STYLE_PROPERTY_BORDER_SPACING, "0px");
  property(cell, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  property(table, OUI_STYLE_PROPERTY_SCROLLBAR_GUTTER, "stable");
  const char* overflow[] = {"auto", "scroll"};
  for (size_t i = 0; i < sizeof(overflow) / sizeof(overflow[0]); ++i) {
    property(table, OUI_STYLE_PROPERTY_OVERFLOW, overflow[i]);
    dimensions(table, 100, 80, 100, 80);
    assert(oui_element_scroll_to(table, 1000.0, 1000.0) == OUI_OK);
    offset(table, 0.0, 0.0);
  }
  /* Hidden/hidden keeps table scrolling; columns retain authored grid width. */
  property(table, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  dimensions(table, 85, 80, 100, 80);
  dimensions(cell, 100, 80, 100, 80);
  size(table, 100.0f, 80.0f);
  assert(oui_element_scroll_to(table, 1000.0, 1000.0) == OUI_OK);
  offset(table, 15.0, 0.0);
  /* A mixed hidden/auto table normalizes both axes to visible. */
  property(table, OUI_STYLE_PROPERTY_OVERFLOW_Y, "auto");
  dimensions(table, 100, 80, 100, 80);
  offset(table, 0.0, 0.0);
  assert(oui_element_destroy(cell) == OUI_OK);
  assert(oui_element_destroy(row) == OUI_OK);
  assert(oui_element_destroy(table) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  printf("native C table scroll dimensions: scale=%g passed\n", scale);
}

static void replaced_consumer(double scale) {
  OuiElement *root = NULL, *image = NULL;
  OuiResource* resource = NULL;
  OuiDocument* document = document_for_scale(scale, &root);
  assert(oui_element_create(document, OUI_ELEMENT_IMAGE, &image) == OUI_OK);
  /* A missing image uses block fallback layout. Install actual primary content. */
  const char svg[] = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\">"
                     "<rect width=\"1\" height=\"1\" fill=\"black\"/></svg>";
  assert(oui_document_register_image(document, utf8("memory:dimensions-pixel.svg"),
      utf8("image/svg+xml"),
      utf8("949c974a51bfed3daa2be2aed981026336857d4cfd494dd120dbb6da66c891f7"),
      (const uint8_t*)svg, sizeof(svg) - 1, &resource) == OUI_OK);
  assert(oui_element_set_image(image, resource, 1.0f, 1.0f) == OUI_OK);
  assert(oui_element_append_child(root, image) == OUI_OK);
  authored_size(image);
  property(image, OUI_STYLE_PROPERTY_OVERFLOW, "scroll");
  dimensions(image, 100, 80, 100, 80);
  property(image, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  property(image, OUI_STYLE_PROPERTY_SCROLLBAR_GUTTER, "stable");
  dimensions(image, 85, 80, 85, 80);
  size(image, 100.0f, 80.0f);
  property(image, OUI_STYLE_PROPERTY_DIRECTION, "rtl");
  dimensions(image, 85, 80, 85, 80);
  property(image, OUI_STYLE_PROPERTY_SCROLLBAR_GUTTER, "stable both-edges");
  dimensions(image, 70, 80, 70, 80);
  assert(oui_element_scroll_to(image, -1000.0, 1000.0) == OUI_OK);
  offset(image, 0.0, 0.0);
  property(image, OUI_STYLE_PROPERTY_DIRECTION, "ltr");
  property(image, OUI_STYLE_PROPERTY_WRITING_MODE, "vertical-rl");
  property(image, OUI_STYLE_PROPERTY_SCROLLBAR_GUTTER, "stable");
  dimensions(image, 100, 65, 100, 65);
  property(image, OUI_STYLE_PROPERTY_SCROLLBAR_GUTTER, "stable both-edges");
  /* The leading gutter is not additional scroll height: content is 50px high. */
  dimensions(image, 100, 50, 100, 50);
  assert(oui_element_scroll_to(image, 1000.0, 1000.0) == OUI_OK);
  offset(image, 0.0, 0.0);
  property(image, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
  dimensions(image, 100, 80, 100, 80);
  assert(oui_element_detach(image) == OUI_OK);
  dimensions(image, 0, 0, 0, 0);
  assert(oui_element_append_child(root, image) == OUI_OK);
  dimensions(image, 100, 80, 100, 80);
  assert(oui_element_destroy(image) == OUI_OK);
  assert(oui_resource_destroy(resource) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  printf("native C replaced scroll dimensions: scale=%g passed\n", scale);
}

static void body_consumer(double scale) {
  OuiElement *root = NULL, *html = NULL, *body = NULL, *content = NULL;
  OuiDocument* document = document_for_scale(scale, &root);
  assert(oui_element_create(document, OUI_ELEMENT_HTML, &html) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_BODY, &body) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &content) == OUI_OK);
  assert(oui_element_append_child(root, html) == OUI_OK);
  assert(oui_element_append_child(html, body) == OUI_OK);
  assert(oui_element_append_child(body, content) == OUI_OK);
  authored_size(body);
  property(content, OUI_STYLE_PROPERTY_WIDTH, "150px");
  property(content, OUI_STYLE_PROPERTY_HEIGHT, "120px");
  property(body, OUI_STYLE_PROPERTY_OVERFLOW, "scroll");
  dimensions(root, 305, 225, 305, 225);
  dimensions(html, 305, 225, 305, 225);
  dimensions(body, 100, 80, 150, 120);
  assert(oui_element_scroll_to(body, 1000.0, 1000.0) == OUI_OK);
  offset(body, 0.0, 0.0);
  /* Disabling propagation transfers actual scroll ownership to the body. */
  property(html, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  dimensions(body, 85, 65, 150, 120);
  assert(oui_element_scroll_to(body, 1000.0, 1000.0) == OUI_OK);
  offset(body, 65.0, 55.0);
  property(html, OUI_STYLE_PROPERTY_OVERFLOW, "visible");
  dimensions(body, 100, 80, 150, 120);
  offset(body, 0.0, 0.0);
  assert(oui_element_destroy(content) == OUI_OK);
  assert(oui_element_destroy(body) == OUI_OK);
  assert(oui_element_destroy(html) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  printf("native C body scroll dimensions: scale=%g passed\n", scale);
}

int main(void) {
  const double scales[] = {1.0, 1.25, 1.5, 1.75, 2.0};
  for (size_t i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i) {
    consumer("flex", scales[i]);
    consumer("grid", scales[i]);
    table_consumer(scales[i]);
    replaced_consumer(scales[i]);
    body_consumer(scales[i]);
  }
  return 0;
}
