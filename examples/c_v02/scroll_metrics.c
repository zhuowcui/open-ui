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

int main(void) {
  const double scales[] = {1.0, 1.25, 1.5, 2.0, 3.0};
  for (size_t i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i)
    viewport(scales[i]);
  return 0;
}
