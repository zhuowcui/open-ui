/* Retained inline image mutation over the same native Rust Engine. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include "openui.h"
#include "inline_green_image.h"

static OuiUtf8 inline_utf8(const char* text) {
  OuiUtf8 value = {(const uint8_t*)text, strlen(text)};
  return value;
}

static void inline_property(OuiElement* element, OuiStyleProperty property, const char* literal) {
  OuiStyleValue value;
  assert(oui_style_value_parse(property, inline_utf8(literal), &value) == OUI_OK);
  assert(oui_element_set_property(element, property, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}

typedef struct InlineCallbackState {
  OuiElement* image;
  unsigned calls;
} InlineCallbackState;

static void inline_clicked(OuiEvent* event, void* data) {
  InlineCallbackState* state = (InlineCallbackState*)data;
  assert(event->event_type == OUI_EVENT_CLICK);
  inline_property(state->image, OUI_STYLE_PROPERTY_WIDTH, "100px");
  OuiRect rect;
  assert(oui_element_get_bounds(state->image, &rect) == OUI_OK);
  assert(rect.x == 20 && rect.y == 20 && rect.width == 100 && rect.height == 150);
  ++state->calls;
}

static void inline_image(double scale) {
  OuiDocumentConfig config = {sizeof(config), OUI_ABI_VERSION,
      {320, 240, (uint32_t)(320 * scale), (uint32_t)(240 * scale),
       scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* parent = NULL;
  OuiElement* image = NULL;
  OuiResource* resource = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &parent) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_IMAGE, &image) == OUI_OK);
  inline_property(parent, OUI_STYLE_PROPERTY_DISPLAY, "block");
  inline_property(parent, OUI_STYLE_PROPERTY_POSITION, "absolute");
  inline_property(parent, OUI_STYLE_PROPERTY_LEFT, "20px");
  inline_property(parent, OUI_STYLE_PROPERTY_TOP, "20px");
  inline_property(parent, OUI_STYLE_PROPERTY_WIDTH, "150px");
  inline_property(parent, OUI_STYLE_PROPERTY_HEIGHT, "150px");
  assert(oui_element_append_child(root, parent) == OUI_OK);
  assert(oui_document_register_image(document, inline_utf8("memory:native-inline-green"),
      inline_utf8("image/png"),
      inline_utf8("d49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe"),
      inline_green_image, sizeof(inline_green_image), &resource) == OUI_OK);
  assert(oui_element_set_image(image, resource, 200, 200) == OUI_OK);
  assert(oui_resource_destroy(resource) == OUI_OK);
  inline_property(image, OUI_STYLE_PROPERTY_DISPLAY, "inline");
  inline_property(image, OUI_STYLE_PROPERTY_WIDTH, "150px");
  inline_property(image, OUI_STYLE_PROPERTY_HEIGHT, "150px");
  assert(oui_element_append_child(parent, image) == OUI_OK);
  OuiRect before;
  assert(oui_element_get_bounds(image, &before) == OUI_OK);
  assert(before.x == 20 && before.y == 20 && before.width == 150 && before.height == 150);
  InlineCallbackState state = {image, 0};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(image, OUI_EVENT_CLICK, 0,
      inline_clicked, &state, &listener) == OUI_OK);
  OuiEvent event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(event);
  event.abi_version = OUI_ABI_VERSION;
  event.event_type = OUI_EVENT_CLICK;
  assert(oui_document_dispatch_event(document, image, &event) == OUI_OK);
  assert(state.calls == 1 && before.width == 150);
  inline_property(image, OUI_STYLE_PROPERTY_DISPLAY, "none");
  size_t count = 1;
  assert(oui_element_get_client_rects_v1(image, NULL, 0, &count) == OUI_OK && count == 0);
  inline_property(image, OUI_STYLE_PROPERTY_DISPLAY, "inline");
  assert(oui_element_detach(image) == OUI_OK);
  count = 1;
  assert(oui_element_get_client_rects_v1(image, NULL, 0, &count) == OUI_OK && count == 0);
  assert(oui_element_append_child(parent, image) == OUI_OK);
  OuiRect after;
  assert(oui_element_get_bounds(image, &after) == OUI_OK);
  assert(after.x == 20 && after.y == 20 && after.width == 100 && after.height == 150);
  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_element_destroy(image) == OUI_OK);
  assert(oui_element_destroy(parent) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  printf("native inline image C: scale=%g callback=1 owned-bounds/detach/teardown passed\n", scale);
}

typedef struct InlineFallbackState {
  OuiElement* image;
  OuiResource* resource;
  unsigned calls;
} InlineFallbackState;

static void inline_fallback_clicked(OuiEvent* event, void* data) {
  InlineFallbackState* state = (InlineFallbackState*)data;
  assert(event->event_type == OUI_EVENT_CLICK);
  if (state->calls == 0)
    assert(oui_element_set_image(state->image, state->resource, 200, 200) == OUI_OK);
  else
    assert(oui_element_clear_image(state->image) == OUI_OK);
  ++state->calls;
}

static void inline_fallback(double scale) {
  OuiDocumentConfig config = {sizeof(config), OUI_ABI_VERSION,
      {320, 240, (uint32_t)(320 * scale), (uint32_t)(240 * scale),
       scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement *root = NULL, *image = NULL, *child = NULL;
  OuiResource* resource = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_IMAGE, &image) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &child) == OUI_OK);
  inline_property(image, OUI_STYLE_PROPERTY_DISPLAY, "inline");
  inline_property(image, OUI_STYLE_PROPERTY_WIDTH, "150px");
  inline_property(image, OUI_STYLE_PROPERTY_HEIGHT, "150px");
  inline_property(child, OUI_STYLE_PROPERTY_DISPLAY, "inline-block");
  inline_property(child, OUI_STYLE_PROPERTY_WIDTH, "60px");
  inline_property(child, OUI_STYLE_PROPERTY_HEIGHT, "40px");
  assert(oui_element_append_child(image, child) == OUI_OK);
  assert(oui_element_append_child(root, image) == OUI_OK);
  OuiRect before;
  assert(oui_element_get_bounds(image, &before) == OUI_OK && before.width == 60);
  assert(oui_document_register_image(document, inline_utf8("memory:native-inline-green"),
      inline_utf8("image/png"),
      inline_utf8("d49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe"),
      inline_green_image, sizeof(inline_green_image), &resource) == OUI_OK);
  InlineFallbackState state = {image, resource, 0};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(image, OUI_EVENT_CLICK, 0,
      inline_fallback_clicked, &state, &listener) == OUI_OK);
  OuiEvent event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(event);
  event.abi_version = OUI_ABI_VERSION;
  event.event_type = OUI_EVENT_CLICK;
  assert(oui_document_dispatch_event(document, image, &event) == OUI_OK && state.calls == 1);
  OuiRect loaded;
  assert(oui_element_get_bounds(image, &loaded) == OUI_OK);
  assert(loaded.width == 150 && loaded.height == 150 && before.width == 60);
  assert(oui_document_dispatch_event(document, image, &event) == OUI_OK && state.calls == 2);
  OuiRect after;
  assert(oui_element_get_bounds(image, &after) == OUI_OK);
  assert(after.x == before.x && after.y == before.y &&
      after.width == before.width && after.height == before.height);
  assert(oui_element_clear_image(image) == OUI_OK);
  assert(oui_element_detach(image) == OUI_OK);
  assert(oui_element_append_child(root, image) == OUI_OK);
  assert(oui_element_get_bounds(image, &after) == OUI_OK && after.width == 60);
  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_resource_destroy(resource) == OUI_OK);
  assert(oui_element_destroy(child) == OUI_OK);
  OuiElement* stale = image;
  assert(oui_element_destroy(image) == OUI_OK);
  assert(oui_element_clear_image(stale) != OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  assert(oui_element_clear_image(NULL) != OUI_OK);
  printf("native inline fallback C: scale=%g callback=2 resource-clear/owned-bounds/teardown passed\n", scale);
}

int main(void) {
  const double scales[] = {1, 1.25, 1.5, 2, 3};
  for (size_t index = 0; index < sizeof(scales) / sizeof(scales[0]); ++index)
    {
      inline_image(scales[index]);
      inline_fallback(scales[index]);
    }
  return 0;
}
