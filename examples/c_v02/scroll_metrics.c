/* Keep consumer checks active when the SDK is built with NDEBUG. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "openui.h"

static OuiUtf8 utf8(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}

static void property(OuiElement* element, OuiStyleProperty id, const char* literal) {
  OuiStyleValue value;
  OuiStatus status = oui_style_value_parse(id, utf8(literal), &value);
  if (status != OUI_OK)
    fprintf(stderr, "scroll property %d literal '%s': %d\n", id, literal, status);
  assert(status == OUI_OK);
  assert(oui_element_set_property(element, id, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}

static OuiScrollMetricsV1 metrics(OuiElement* element) {
  OuiScrollMetricsV1 value = {sizeof(value), OUI_ABI_VERSION, 0.0, 0.0, 0.0, 0.0};
  uint8_t found = 0;
  assert(oui_element_get_scroll_metrics_v1(element, &value, &found) == OUI_OK);
  assert(found == 1);
  return value;
}

static OuiStyleValue overflow_value(int value) {
  OuiStyleValue result;
  memset(&result, 0, sizeof(result));
  result.tag = OUI_STYLE_VALUE_ENUM;
  result.data.enum_value = value;
  return result;
}

typedef struct NestedScrollCallback {
  OuiElement* viewport;
  unsigned calls;
} NestedScrollCallback;

static void nested_clicked(OuiEvent* event, void* data) {
  NestedScrollCallback* state = (NestedScrollCallback*)data;
  assert(event->event_type == OUI_EVENT_CLICK);
  assert(oui_element_scroll_to(state->viewport, 1000.0, 1000.0) == OUI_OK);
  ++state->calls;
}

static void nested(double scale) {
  OuiDocumentConfig config = {sizeof(config),
                              OUI_ABI_VERSION,
                              {320.0, 240.0, (uint32_t)(320.0 * scale), (uint32_t)(240.0 * scale),
                               scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* viewport = NULL;
  OuiElement* child = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &viewport) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &child) == OUI_OK);
  assert(oui_element_append_child(root, viewport) == OUI_OK);
  assert(oui_element_append_child(viewport, child) == OUI_OK);
  property(viewport, OUI_STYLE_PROPERTY_POSITION, "absolute");
  property(viewport, OUI_STYLE_PROPERTY_LEFT, "40px");
  property(viewport, OUI_STYLE_PROPERTY_TOP, "40px");
  property(viewport, OUI_STYLE_PROPERTY_WIDTH, "80px");
  property(viewport, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  property(viewport, OUI_STYLE_PROPERTY_PADDING, "10px");
  property(viewport, OUI_STYLE_PROPERTY_BORDER, "3px solid orange");
  property(viewport, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  property(viewport, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
  property(child, OUI_STYLE_PROPERTY_WIDTH, "130px");
  property(child, OUI_STYLE_PROPERTY_HEIGHT, "120px");
  OuiScrollMetricsV1 original = metrics(viewport);
  assert(original.client_width == 100.0 && original.client_height == 100.0);
  assert(original.scroll_width == 150.0 && original.scroll_height == 140.0);
  NestedScrollCallback state = {viewport, 0};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(viewport, OUI_EVENT_CLICK, 0, nested_clicked, &state,
                                        &listener) == OUI_OK);
  OuiEvent event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(event);
  event.abi_version = OUI_ABI_VERSION;
  event.event_type = OUI_EVENT_CLICK;
  assert(oui_document_dispatch_event(document, viewport, &event) == OUI_OK);
  assert(state.calls == 1);
  double x = 0.0, y = 0.0;
  assert(oui_element_get_scroll_offset(viewport, &x, &y) == OUI_OK && x == 50.0 && y == 40.0);
  OuiRect bounds;
  assert(oui_element_get_bounds(child, &bounds) == OUI_OK && bounds.x == 3.0f && bounds.y == 13.0f);
  property(viewport, OUI_STYLE_PROPERTY_DIRECTION, "rtl");
  assert(oui_element_scroll_to(viewport, -1000.0, 1000.0) == OUI_OK);
  assert(oui_element_get_scroll_offset(viewport, &x, &y) == OUI_OK && x == -50.0 && y == 40.0);
  property(child, OUI_STYLE_PROPERTY_WIDTH, "30px");
  property(child, OUI_STYLE_PROPERTY_HEIGHT, "20px");
  assert(oui_element_get_scroll_offset(viewport, &x, &y) == OUI_OK && x == 0.0 && y == 0.0);
  OuiScrollMetricsV1 small = metrics(viewport);
  assert(small.scroll_width == 100.0 && small.scroll_height == 100.0);
  property(viewport, OUI_STYLE_PROPERTY_WIDTH, "140px");
  property(viewport, OUI_STYLE_PROPERTY_HEIGHT, "140px");
  OuiScrollMetricsV1 resized = metrics(viewport);
  assert(resized.client_width == 160.0 && resized.client_height == 160.0);
  assert(resized.scroll_width == 160.0 && resized.scroll_height == 160.0);
  assert(original.scroll_width == 150.0 && original.scroll_height == 140.0);
  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_element_destroy(child) == OUI_OK);
  assert(oui_element_destroy(viewport) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  printf("native C nested scroll metrics: scale=%g passed\n", scale);
}

static void viewport(double scale) {
  OuiDocumentConfig config = {sizeof(config),
                              OUI_ABI_VERSION,
                              {320.0, 240.0, (uint32_t)(320.0 * scale), (uint32_t)(240.0 * scale),
                               scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* content = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &content) == OUI_OK);
  assert(oui_element_append_child(root, content) == OUI_OK);
  property(root, OUI_STYLE_PROPERTY_OVERFLOW_X, "auto");
  property(root, OUI_STYLE_PROPERTY_OVERFLOW_Y, "auto");
  property(content, OUI_STYLE_PROPERTY_POSITION, "absolute");
  property(content, OUI_STYLE_PROPERTY_LEFT, "27px");
  property(content, OUI_STYLE_PROPERTY_TOP, "22px");
  property(content, OUI_STYLE_PROPERTY_WIDTH, "300px");
  property(content, OUI_STYLE_PROPERTY_HEIGHT, "224px");

  /* Chromium's retained viewport has paired 15px gutters and these dimensions
   * at all five scales. No explicit document update is needed for the query. */
  OuiScrollMetricsV1 original = metrics(root);
  assert(original.client_width == 305.0 && original.client_height == 225.0);
  assert(original.scroll_width == 327.0 && original.scroll_height == 246.0);
  assert(oui_element_scroll_to(root, 1000.0, 1000.0) == OUI_OK);
  double x = -1.0;
  double y = -1.0;
  assert(oui_element_get_scroll_offset(root, &x, &y) == OUI_OK && x == 22.0 && y == 21.0);

  property(content, OUI_STYLE_PROPERTY_WIDTH, "80px");
  property(content, OUI_STYLE_PROPERTY_HEIGHT, "40px");
  assert(oui_element_get_scroll_offset(root, &x, &y) == OUI_OK && x == 0.0 && y == 0.0);
  OuiScrollMetricsV1 small = metrics(root);
  assert(small.client_width == 320.0 && small.client_height == 240.0);
  assert(small.scroll_width == 320.0 && small.scroll_height == 240.0);
  assert(original.client_width == 305.0 && original.scroll_width == 327.0);

  /* Direct typed values change one axis without changing the other. */
  OuiStyleValue horizontal = overflow_value(OUI_OVERFLOW_SCROLL);
  OuiStyleValue vertical = overflow_value(OUI_OVERFLOW_HIDDEN);
  assert(oui_element_set_property(root, OUI_STYLE_PROPERTY_OVERFLOW_X, &horizontal) == OUI_OK);
  assert(oui_element_set_property(root, OUI_STYLE_PROPERTY_OVERFLOW_Y, &vertical) == OUI_OK);
  OuiScrollMetricsV1 asymmetric = metrics(root);
  assert(asymmetric.client_width == 320.0 && asymmetric.client_height == 225.0);
  assert(asymmetric.scroll_width == 320.0 && asymmetric.scroll_height == 225.0);
  OuiStyleValue invalid = overflow_value(99);
  assert(oui_element_set_property(root, OUI_STYLE_PROPERTY_OVERFLOW_X, &invalid) ==
         OUI_ERROR_INVALID_ARGUMENT);
  asymmetric = metrics(root);
  assert(asymmetric.client_width == 320.0 && asymmetric.client_height == 225.0);

  property(root, OUI_STYLE_PROPERTY_OVERFLOW_Y, "scroll");
  OuiScrollMetricsV1 forced = metrics(root);
  assert(forced.client_width == 305.0 && forced.client_height == 225.0);
  assert(forced.scroll_width == 305.0 && forced.scroll_height == 225.0);

  property(root, OUI_STYLE_PROPERTY_OVERFLOW_X, "hidden");
  property(root, OUI_STYLE_PROPERTY_OVERFLOW_Y, "hidden");
  property(content, OUI_STYLE_PROPERTY_WIDTH, "300px");
  property(content, OUI_STYLE_PROPERTY_HEIGHT, "224px");
  OuiScrollMetricsV1 hidden = metrics(root);
  assert(hidden.client_width == 320.0 && hidden.client_height == 240.0);
  assert(hidden.scroll_width == 327.0 && hidden.scroll_height == 246.0);
  assert(oui_element_scroll_to(root, 1000.0, 1000.0) == OUI_OK);
  assert(oui_element_get_scroll_offset(root, &x, &y) == OUI_OK && x == 7.0 && y == 6.0);

  config.viewport.logical_width = 360.0;
  config.viewport.logical_height = 260.0;
  config.viewport.physical_width = (uint32_t)(360.0 * scale);
  config.viewport.physical_height = (uint32_t)(260.0 * scale);
  assert(oui_document_set_viewport(document, &config) == OUI_OK);
  assert(oui_element_get_scroll_offset(root, &x, &y) == OUI_OK && x == 0.0 && y == 0.0);
  OuiScrollMetricsV1 resized = metrics(root);
  assert(resized.client_width == 360.0 && resized.client_height == 260.0);
  assert(resized.scroll_width == 360.0 && resized.scroll_height == 260.0);

  assert(oui_element_detach(content) == OUI_OK);
  uint8_t found = 1;
  assert(oui_element_get_scroll_metrics_v1(content, &resized, &found) == OUI_OK);
  assert(found == 0 && resized.client_width == 0.0 && resized.client_height == 0.0);
  assert(resized.scroll_width == 0.0 && resized.scroll_height == 0.0);
  assert(oui_element_append_child(root, content) == OUI_OK);
  property(content, OUI_STYLE_PROPERTY_DISPLAY, "none");
  found = 1;
  assert(oui_element_get_scroll_metrics_v1(content, &resized, &found) == OUI_OK && found == 0);

  assert(oui_element_destroy(content) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  assert(original.client_width == 305.0 && original.scroll_height == 246.0);
  printf("native C scroll metrics: scale=%g passed\n", scale);
}

typedef OuiStatus (*DimensionQuery)(OuiElement*, int32_t*);

static void integer_dimensions(OuiElement* element, int32_t width, int32_t height,
                               int32_t scroll_width, int32_t scroll_height) {
  const DimensionQuery queries[] = {oui_element_get_client_width_v1,
                                   oui_element_get_client_height_v1,
                                   oui_element_get_scroll_width_v1,
                                   oui_element_get_scroll_height_v1};
  const int32_t expected[] = {width, height, scroll_width, scroll_height};
  for (size_t i = 0; i < sizeof(queries) / sizeof(queries[0]); ++i) {
    /* A scalar output has no version prefix. Only these four bytes may change. */
    struct Output { int32_t value; uint8_t tail[12]; } output;
    output.value = INT32_MIN;
    memset(output.tail, 0x6d, sizeof(output.tail));
    assert(queries[i](element, NULL) == OUI_ERROR_INVALID_ARGUMENT);
    assert(queries[i](element, &output.value) == OUI_OK);
    assert(output.value == expected[i]);
    for (size_t j = 0; j < sizeof(output.tail); ++j)
      assert(output.tail[j] == 0x6d);
  }
}

static void integer_api(double scale) {
  OuiDocumentConfig config = {sizeof(config), OUI_ABI_VERSION,
                              {320.0, 240.0, (uint32_t)(320.0 * scale),
                               (uint32_t)(240.0 * scale), scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiDocument* other_document = NULL;
  OuiElement* root = NULL;
  OuiElement* other_root = NULL;
  OuiElement* scroller = NULL;
  OuiElement* content = NULL;
  OuiElement* inline_element = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  integer_dimensions(root, 320, 240, 320, 240);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &scroller) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &content) == OUI_OK);
  assert(oui_element_append_child(root, scroller) == OUI_OK);
  assert(oui_element_append_child(scroller, content) == OUI_OK);
  property(scroller, OUI_STYLE_PROPERTY_WIDTH, "100.5px");
  property(scroller, OUI_STYLE_PROPERTY_HEIGHT, "80.5px");
  property(scroller, OUI_STYLE_PROPERTY_BORDER, "2px solid black");
  property(scroller, OUI_STYLE_PROPERTY_OVERFLOW, "scroll");
  property(scroller, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "auto");
  property(content, OUI_STYLE_PROPERTY_WIDTH, "200.5px");
  property(content, OUI_STYLE_PROPERTY_HEIGHT, "160.5px");
  integer_dimensions(scroller, 86, 66, 201, 161);
  OuiScrollMetricsV1 raw = metrics(scroller);
  assert(raw.client_width == 85.5 && raw.client_height == 65.5);
  assert(raw.scroll_width == 200.5 && raw.scroll_height == 160.5);
  property(scroller, OUI_STYLE_PROPERTY_TRANSFORM, "scale(2,3)");
  integer_dimensions(scroller, 86, 66, 201, 161);
  OuiScrollMetricsV1 transformed = metrics(scroller);
  assert(transformed.client_width == raw.client_width);
  assert(transformed.client_height == raw.client_height);
  assert(transformed.scroll_width == raw.scroll_width);
  assert(transformed.scroll_height == raw.scroll_height);
  OuiRect bounds;
  assert(oui_element_get_bounds(scroller, &bounds) == OUI_OK);
  assert(bounds.width == 209.0f && bounds.height == 253.5f);
  property(scroller, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  integer_dimensions(scroller, 101, 81, 201, 161);
  raw = metrics(scroller);
  assert(raw.client_width == 100.5 && raw.client_height == 80.5);
  assert(raw.scroll_width == 200.5 && raw.scroll_height == 160.5);

  assert(oui_element_create(document, OUI_ELEMENT_SPAN, &inline_element) == OUI_OK);
  assert(oui_element_append_child(root, inline_element) == OUI_OK);
  assert(oui_element_set_text(inline_element, utf8("native inline")) == OUI_OK);
  integer_dimensions(inline_element, 0, 0, 0, 0);
  property(inline_element, OUI_STYLE_PROPERTY_DISPLAY, "inline-block");
  assert(oui_element_set_text(inline_element, utf8("")) == OUI_OK);
  property(inline_element, OUI_STYLE_PROPERTY_WIDTH, "40.5px");
  property(inline_element, OUI_STYLE_PROPERTY_HEIGHT, "20.5px");
  integer_dimensions(inline_element, 41, 21, 41, 21);
  assert(oui_element_detach(inline_element) == OUI_OK);
  integer_dimensions(inline_element, 0, 0, 0, 0);

  config.viewport.logical_width = 96.0;
  config.viewport.logical_height = 48.0;
  config.viewport.physical_width = (uint32_t)(96.0 * scale);
  config.viewport.physical_height = (uint32_t)(48.0 * scale);
  assert(oui_document_create(&config, &other_document) == OUI_OK);
  assert(oui_document_root(other_document, &other_root) == OUI_OK);
  integer_dimensions(other_root, 96, 48, 96, 48);
  assert(oui_element_append_child(other_root, inline_element) == OUI_ERROR_WRONG_DOCUMENT);
  integer_dimensions(inline_element, 0, 0, 0, 0);
  integer_dimensions(other_root, 96, 48, 96, 48);
  assert(oui_element_append_child(root, inline_element) == OUI_OK);
  integer_dimensions(inline_element, 41, 21, 41, 21);
  property(inline_element, OUI_STYLE_PROPERTY_DISPLAY, "contents");
  integer_dimensions(inline_element, 0, 0, 0, 0);
  property(inline_element, OUI_STYLE_PROPERTY_DISPLAY, "none");
  integer_dimensions(inline_element, 0, 0, 0, 0);
  assert(oui_element_remove(inline_element) == OUI_OK);
  const DimensionQuery queries[] = {oui_element_get_client_width_v1,
                                   oui_element_get_client_height_v1,
                                   oui_element_get_scroll_width_v1,
                                   oui_element_get_scroll_height_v1};
  for (size_t i = 0; i < sizeof(queries) / sizeof(queries[0]); ++i) {
    int32_t unchanged = -113;
    assert(queries[i](inline_element, &unchanged) == OUI_ERROR_STALE_HANDLE);
    assert(unchanged == -113);
  }
  assert(oui_element_destroy(inline_element) == OUI_OK);
  assert(oui_element_destroy(content) == OUI_OK);
  assert(oui_element_destroy(scroller) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  assert(oui_element_destroy(other_root) == OUI_OK);
  assert(oui_document_destroy(other_document) == OUI_OK);
  printf("native C integer dimensions: scale=%g passed\n", scale);
}

static void integer_table_api(void) {
  OuiDocumentConfig config = {sizeof(config), OUI_ABI_VERSION,
                              {320.0, 240.0, 320, 240, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* table = NULL;
  OuiElement* caption = NULL;
  OuiElement* group = NULL;
  OuiElement* row = NULL;
  OuiElement* cell = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_TABLE, &table) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_TABLE_CAPTION, &caption) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_TABLE_BODY, &group) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_TABLE_ROW, &row) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_TABLE_CELL, &cell) == OUI_OK);
  assert(oui_element_append_child(root, table) == OUI_OK);
  assert(oui_element_append_child(table, caption) == OUI_OK);
  assert(oui_element_append_child(table, group) == OUI_OK);
  assert(oui_element_append_child(group, row) == OUI_OK);
  assert(oui_element_append_child(row, cell) == OUI_OK);
  property(table, OUI_STYLE_PROPERTY_WIDTH, "120px");
  property(table, OUI_STYLE_PROPERTY_BOX_SIZING, "border-box");
  property(table, OUI_STYLE_PROPERTY_BORDER_SPACING, "0px");
  property(table, OUI_STYLE_PROPERTY_BORDER, "2px solid black");
  property(caption, OUI_STYLE_PROPERTY_HEIGHT, "12px");
  property(cell, OUI_STYLE_PROPERTY_HEIGHT, "20px");
  integer_dimensions(table, 120, 36, 118, 32);
  integer_dimensions(caption, 120, 12, 120, 12);
  integer_dimensions(group, 116, 20, 116, 20);
  integer_dimensions(row, 116, 20, 116, 20);
  integer_dimensions(cell, 116, 20, 116, 20);
  property(table, OUI_STYLE_PROPERTY_TRANSFORM, "translate(20px,7px)");
  integer_dimensions(table, 120, 36, 118, 32);
  assert(oui_element_remove(caption) == OUI_OK);
  integer_dimensions(table, 120, 24, 116, 20);
  property(table, OUI_STYLE_PROPERTY_BORDER_COLLAPSE, "collapse");
  int32_t width = 0;
  int32_t height = 0;
  assert(oui_element_get_client_width_v1(table, &width) == OUI_OK && width == 120);
  assert(oui_element_get_client_height_v1(cell, &height) == OUI_OK && height == 20);
  assert(oui_element_destroy(cell) == OUI_OK);
  assert(oui_element_destroy(row) == OUI_OK);
  assert(oui_element_destroy(group) == OUI_OK);
  assert(oui_element_destroy(caption) == OUI_OK);
  assert(oui_element_destroy(table) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
}

int main(void) {
  const double scales[] = {1.0, 1.25, 1.5, 2.0, 3.0};
  for (size_t i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i) {
    viewport(scales[i]);
    nested(scales[i]);
    integer_api(scales[i]);
  }
  integer_table_api();
  return 0;
}
