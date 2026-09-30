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
    fprintf(stderr, "geometry property %d literal '%s': %d\n", id, literal, status);
  assert(status == OUI_OK);
  assert(oui_element_set_property(element, id, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}

static OuiElement* block(OuiDocument* document, OuiElement* parent) {
  OuiElement* element = NULL;
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &element) == OUI_OK);
  property(element, OUI_STYLE_PROPERTY_DISPLAY, "block");
  assert(oui_element_append_child(parent, element) == OUI_OK);
  return element;
}

int main(void) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {320.0, 240.0, 320, 240, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  OuiStyleValue invalid_count;
  assert(oui_style_value_parse(OUI_STYLE_PROPERTY_COLUMN_COUNT, utf8("auto"), &invalid_count) ==
         OUI_OK);
  assert(invalid_count.tag == OUI_STYLE_VALUE_INTEGER && invalid_count.data.integer == 0);
  assert(oui_style_value_parse(OUI_STYLE_PROPERTY_COLUMN_COUNT, utf8("0"), &invalid_count) ==
         OUI_ERROR_INVALID_ARGUMENT);
  assert(oui_style_value_parse(OUI_STYLE_PROPERTY_COLUMN_COUNT, utf8("-1"), &invalid_count) ==
         OUI_ERROR_INVALID_ARGUMENT);
  assert(oui_style_value_parse(OUI_STYLE_PROPERTY_COLUMN_COUNT, utf8("4294967296"),
                               &invalid_count) == OUI_ERROR_INVALID_ARGUMENT);
  OuiElement* columns = block(document, root);
  property(columns, OUI_STYLE_PROPERTY_WIDTH, "300px");
  property(columns, OUI_STYLE_PROPERTY_COLUMN_COUNT, "3");
  property(columns, OUI_STYLE_PROPERTY_COLUMN_GAP, "24px");
  OuiElement* wrapper = block(document, columns);
  property(wrapper, OUI_STYLE_PROPERTY_MAX_HEIGHT, "160px");
  OuiElement* target = block(document, wrapper);
  property(target, OUI_STYLE_PROPERTY_WIDTH, "50px");
  property(target, OUI_STYLE_PROPERTY_HEIGHT, "200px");
  property(target, OUI_STYLE_PROPERTY_BORDER, "3px solid black");

  size_t count = 0;
  assert(oui_element_get_client_rects_v1(target, NULL, 0, &count) == OUI_OK);
  assert(count == 3);
  OuiRect rects[3] = {{-1.0f, -1.0f, -1.0f, -1.0f}};
  assert(oui_element_get_client_rects_v1(target, rects, 2, &count) == OUI_ERROR_BUFFER_TOO_SMALL);
  assert(count == 3 && rects[0].x == -1.0f);
  assert(oui_element_get_client_rects_v1(target, rects, 3, &count) == OUI_OK);
  assert(rects[0].x == 0.0f && rects[1].x == 108.0f && rects[2].x == 216.0f);
  assert(rects[0].height == 68.671875f && rects[2].height == 68.65625f);
  OuiRect bounds;
  assert(oui_element_get_bounds(target, &bounds) == OUI_OK);
  assert(bounds.x == 0.0f && bounds.y == 0.0f && bounds.width == 272.0f);
  assert(bounds.height == 68.671875f);
  OuiRect wrapper_rects[3];
  assert(oui_element_get_client_rects_v1(wrapper, wrapper_rects, 3, &count) == OUI_OK);
  assert(count == 3 && wrapper_rects[2].width == 84.0f);
  assert(wrapper_rects[2].height == 22.65625f);
  property(wrapper, OUI_STYLE_PROPERTY_MAX_HEIGHT, "120px");
  assert(oui_element_get_client_rects_v1(wrapper, rects, 3, &count) == OUI_OK);
  assert(rects[1].height == 51.328125f && rects[2].height == 0.0f);
  assert(oui_element_get_bounds(wrapper, &bounds) == OUI_OK);
  assert(bounds.width == 192.0f && bounds.height == 68.671875f);
  assert(wrapper_rects[2].height == 22.65625f); /* Earlier copies remain owned. */
  assert(oui_element_get_client_rects_v1(target, rects, 3, &count) == OUI_OK);
  assert(rects[2].height == 68.65625f); /* The child still owns visible overflow. */
  property(wrapper, OUI_STYLE_PROPERTY_MAX_HEIGHT, "160px");
  property(target, OUI_STYLE_PROPERTY_POINTER_EVENTS, "none");
  property(target, OUI_STYLE_PROPERTY_VISIBILITY, "hidden");
  assert(oui_element_get_bounds(target, &bounds) == OUI_OK && bounds.width == 272.0f);
  assert(oui_element_get_client_rects_v1(target, NULL, 0, &count) == OUI_OK && count == 3);
  assert(oui_element_get_client_rects_v1(target, rects, 3, NULL) == OUI_ERROR_INVALID_ARGUMENT);

  assert(oui_element_detach(target) == OUI_OK);
  assert(oui_element_get_client_rects_v1(target, NULL, 0, &count) == OUI_OK && count == 0);
  assert(rects[2].x == 216.0f); /* Copied values remain owned by this consumer. */
  assert(oui_element_append_child(wrapper, target) == OUI_OK);
  assert(oui_element_get_client_rects_v1(target, NULL, 0, &count) == OUI_OK && count == 3);
  assert(oui_element_remove(target) == OUI_OK);
  assert(oui_element_get_client_rects_v1(target, NULL, 0, &count) == OUI_ERROR_STALE_HANDLE);
  assert(oui_element_destroy(target) == OUI_OK);
  assert(oui_element_destroy(wrapper) == OUI_OK);
  assert(oui_element_destroy(columns) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  puts("geometry: native column boxes, child overflow, owned copies, bounds and lifecycle passed");
  return 0;
}
