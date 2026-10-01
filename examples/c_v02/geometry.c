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

static void vertical_geometry(const char* writing_mode, float final_parent_x, float hit_x) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {320.0, 240.0, 320, 240, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  OuiElement* columns = block(document, root);
  property(columns, OUI_STYLE_PROPERTY_HEIGHT, "300px");
  property(columns, OUI_STYLE_PROPERTY_WRITING_MODE, writing_mode);
  property(columns, OUI_STYLE_PROPERTY_COLUMN_COUNT, "3");
  property(columns, OUI_STYLE_PROPERTY_COLUMN_GAP, "24px");
  OuiElement* wrapper = block(document, columns);
  property(wrapper, OUI_STYLE_PROPERTY_WRITING_MODE, writing_mode);
  property(wrapper, OUI_STYLE_PROPERTY_MAX_WIDTH, "160px");
  OuiElement* target = block(document, wrapper);
  property(target, OUI_STYLE_PROPERTY_WIDTH, "200px");
  property(target, OUI_STYLE_PROPERTY_HEIGHT, "50px");
  property(target, OUI_STYLE_PROPERTY_BORDER, "3px solid black");
  OuiRect bounds;
  assert(oui_element_get_bounds(columns, &bounds) == OUI_OK && bounds.width == 68.671875f);
  size_t count = 0;
  OuiRect wrapper_rects[3];
  assert(oui_element_get_client_rects_v1(wrapper, wrapper_rects, 3, &count) == OUI_OK &&
         count == 3);
  assert(wrapper_rects[0].y == 0.0f && wrapper_rects[1].y == 108.0f &&
         wrapper_rects[2].y == 216.0f);
  assert(wrapper_rects[0].width == 68.671875f && wrapper_rects[1].width == 68.671875f);
  assert(wrapper_rects[2].width == 22.65625f && wrapper_rects[2].x == final_parent_x);
  assert(wrapper_rects[0].height == 84.0f && wrapper_rects[1].height == 84.0f &&
         wrapper_rects[2].height == 84.0f);
  OuiRect rects[3];
  assert(oui_element_get_client_rects_v1(target, rects, 3, &count) == OUI_OK && count == 3);
  assert(rects[0].y == 0.0f && rects[1].y == 108.0f && rects[2].y == 216.0f);
  assert(rects[0].width == 68.671875f && rects[1].width == 68.671875f &&
         rects[2].width == 68.65625f);
  assert(rects[0].height == 56.0f && rects[1].height == 56.0f && rects[2].height == 56.0f);
  OuiElement* hit = NULL;
  assert(oui_document_hit_test(document, hit_x, 230.0f, &hit) == OUI_OK && hit != NULL);
  assert(hit == target); /* Hit testing borrows the existing native handle. */
  OuiRect hit_rects[3];
  assert(oui_element_get_client_rects_v1(hit, hit_rects, 3, &count) == OUI_OK && count == 3);
  for (size_t i = 0; i < count; ++i) {
    assert(hit_rects[i].x == rects[i].x && hit_rects[i].y == rects[i].y &&
           hit_rects[i].width == rects[i].width && hit_rects[i].height == rects[i].height);
  }
  assert(oui_element_destroy(target) == OUI_OK);
  assert(oui_element_destroy(wrapper) == OUI_OK);
  assert(oui_element_destroy(columns) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
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
  /* An independent block formatting context still fragments ordinary children. */
  property(wrapper, OUI_STYLE_PROPERTY_DISPLAY, "flow-root");
  assert(oui_element_get_client_rects_v1(target, rects, 3, &count) == OUI_OK && count == 3);
  assert(rects[0].y == 0.0f && rects[1].y == 0.0f && rects[2].y == 0.0f);
  assert(rects[0].height == 68.671875f && rects[1].height == 68.671875f &&
         rects[2].height == 68.65625f);
  assert(oui_element_get_bounds(target, &bounds) == OUI_OK);
  assert(bounds.y == 0.0f && bounds.width == 272.0f && bounds.height == 68.671875f);
  property(wrapper, OUI_STYLE_PROPERTY_DISPLAY, "block");
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
  /* Native C mutations reach the same deferred atomic-child layout as Rust. */
  property(target, OUI_STYLE_PROPERTY_POINTER_EVENTS, "auto");
  property(target, OUI_STYLE_PROPERTY_VISIBILITY, "visible");
  property(columns, OUI_STYLE_PROPERTY_MAX_HEIGHT, "80px");
  OuiElement* clipped = block(document, target);
  property(clipped, OUI_STYLE_PROPERTY_WIDTH, "20px");
  property(clipped, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  const char* atomic_heights[] = {"90px", "200px", "300px"};
  const float child_heights[][3] = {
      {80.0f, 90.0f, 36.0f}, {80.0f, 126.0f, 0.0f}, {80.0f, 126.0f, 0.0f}};
  const size_t fragment_counts[] = {3, 2, 2};
  for (size_t state = 0; state < 3; ++state) {
    property(clipped, OUI_STYLE_PROPERTY_HEIGHT, atomic_heights[state]);
    assert(oui_element_get_client_rects_v1(target, rects, 3, &count) == OUI_OK);
    assert(count == fragment_counts[state]);
    for (size_t part = 0; part < count; ++part) {
      assert(rects[part].x == 108.0f * part && rects[part].y == 0.0f);
      assert(rects[part].height == child_heights[state][part]);
    }
    assert(oui_element_get_client_rects_v1(wrapper, wrapper_rects, 3, &count) == OUI_OK);
    assert(count == fragment_counts[state]);
    assert(wrapper_rects[0].height == 80.0f && wrapper_rects[1].height == 80.0f);
    if (count == 3)
      assert(wrapper_rects[2].height == 0.0f);
  }
  property(wrapper, OUI_STYLE_PROPERTY_PADDING_TOP, "10px");
  property(clipped, OUI_STYLE_PROPERTY_HEIGHT, "200px");
  assert(oui_element_get_client_rects_v1(target, rects, 3, &count) == OUI_OK && count == 2);
  assert(rects[0].y == 10.0f && rects[0].height == 70.0f);
  assert(rects[1].y == 0.0f && rects[1].height == 136.0f);
  assert(oui_element_get_client_rects_v1(wrapper, wrapper_rects, 3, &count) == OUI_OK);
  assert(count == 2 && wrapper_rects[0].height == 80.0f && wrapper_rects[1].height == 90.0f);
  assert(oui_element_destroy(clipped) == OUI_OK);
  assert(oui_element_remove(target) == OUI_OK);
  assert(oui_element_get_client_rects_v1(target, NULL, 0, &count) == OUI_ERROR_STALE_HANDLE);
  assert(oui_element_destroy(target) == OUI_OK);
  assert(oui_element_destroy(wrapper) == OUI_OK);
  assert(oui_element_destroy(columns) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  vertical_geometry("vertical-lr", 0.0f, 40.0f);
  vertical_geometry("vertical-rl", 46.015625f, 20.0f);
  puts("geometry: native column boxes, child overflow, owned copies, bounds and lifecycle passed");
  return 0;
}
