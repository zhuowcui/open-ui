/* Keep consumer checks active in optimized SDK builds. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "openui.h"

static OuiUtf8 utf8(const char* text) {
  OuiUtf8 value = {(const uint8_t*)text, strlen(text)};
  return value;
}
static void property(OuiElement* element, OuiStyleProperty id, const char* text) {
  OuiStyleValue value;
  assert(oui_style_value_parse(id, utf8(text), &value) == OUI_OK);
  assert(oui_element_set_property(element, id, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}
static void offset(OuiElement* element, double expected_x, double expected_y) {
  double x = NAN, y = NAN;
  assert(oui_element_get_scroll_offset(element, &x, &y) == OUI_OK);
  assert(x == expected_x && y == expected_y);
}
static void scenario(double scale) {
  OuiDocumentConfig config = {sizeof(config),
                              OUI_ABI_VERSION,
                              {320.0, 240.0, (uint32_t)(320.0 * scale), (uint32_t)(240.0 * scale),
                               scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* source = NULL;
  OuiElement* content = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &source) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &content) == OUI_OK);
  assert(oui_element_append_child(root, source) == OUI_OK);
  assert(oui_element_append_child(source, content) == OUI_OK);
  property(source, OUI_STYLE_PROPERTY_DISPLAY, "block");
  property(source, OUI_STYLE_PROPERTY_WIDTH, "100px");
  property(source, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  property(source, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  property(source, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
  property(content, OUI_STYLE_PROPERTY_DISPLAY, "block");
  property(content, OUI_STYLE_PROPERTY_WIDTH, "300px");
  property(content, OUI_STYLE_PROPERTY_HEIGHT, "280px");
  assert(oui_element_scroll_to(source, 40.0, 40.0) == OUI_OK);
  const double points[] = {0.0, 100.0};
  OuiEasingCurveV1 curve;
  memset(&curve, 0, sizeof(curve));
  curve.struct_size = sizeof(curve);
  curve.abi_version = OUI_ABI_VERSION;
  curve.kind = 2;          /* steps */
  curve.step_position = 1; /* jump-end */
  curve.step_count = 2;
  uint64_t id = UINT64_MAX;
  uint8_t found = 255;
  assert(oui_element_settle_scroll_snap_with_easing_v1(source, points, 2, NULL, 0, 100.0, &curve,
                                                       &id, &found) == OUI_OK);
  assert(found == 1 && id != UINT64_MAX && id != 0);
  assert(oui_document_set_animation_time(document, 25.0) == OUI_OK);
  offset(source, 40.0, 40.0);
  assert(oui_document_set_animation_time(document, 50.0) == OUI_OK);
  offset(source, 20.0, 40.0);
  assert(oui_document_cancel_smooth_scroll_v1(document, id) == OUI_OK);
  offset(source, 20.0, 40.0);
  assert(oui_document_cancel_smooth_scroll_v1(document, id) == OUI_ERROR_INVALID_STATE);
  assert(oui_document_set_animation_time(document, 100.0) == OUI_OK);
  offset(source, 20.0, 40.0);
  id = UINT64_MAX;
  found = 255;
  assert(oui_element_settle_scroll_snap_with_easing_v1(source, NULL, 0, NULL, 0, 100.0, NULL, &id,
                                                       &found) == OUI_OK);
  assert(id == 0 && found == 0);
  id = UINT64_MAX;
  found = 255;
  assert(oui_element_settle_scroll_snap_with_easing_v1(source, NULL, 0, NULL, 0, -1.0, NULL, &id,
                                                       &found) == OUI_ERROR_INVALID_ARGUMENT);
  assert(id == UINT64_MAX && found == 255);
  offset(source, 20.0, 40.0);
  assert(oui_element_destroy(content) == OUI_OK);
  assert(oui_element_destroy(source) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
}
int main(void) {
  const double scales[] = {1.0, 1.25, 1.5, 1.75, 2.0};
  for (size_t index = 0; index < sizeof(scales) / sizeof(scales[0]); ++index)
    scenario(scales[index]);
  puts("native explicit snap/cancel C ABI: passed");
  return 0;
}
