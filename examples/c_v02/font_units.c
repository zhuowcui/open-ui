/* Keep consumer checks active in packaged release builds. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "openui.h"

static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}

static void property(OuiElement* element, OuiStyleProperty id, const char* literal) {
  OuiStyleValue value;
  assert(oui_style_value_parse(id, text(literal), &value) == OUI_OK);
  assert(oui_element_set_property(element, id, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}

static OuiStyleValue length(float number, OuiLengthUnit unit) {
  OuiStyleValue value;
  memset(&value, 0, sizeof(value));
  value.tag = OUI_STYLE_VALUE_LENGTH;
  value.data.length.value = number;
  value.data.length.unit = unit;
  return value;
}

typedef struct CallbackState {
  OuiElement* parent;
  unsigned calls;
} CallbackState;

static void clicked(OuiEvent* event, void* data) {
  CallbackState* state = (CallbackState*)data;
  assert(event->event_type == OUI_EVENT_CLICK);
  property(state->parent, OUI_STYLE_PROPERTY_FONT_SIZE, "32px");
  ++state->calls;
}

static void native_font_units(double scale) {
  OuiDocumentConfig config = {
      sizeof(config),
      OUI_ABI_VERSION,
      {320, 240, (uint32_t)(320 * scale), (uint32_t)(240 * scale), scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement *root = NULL, *nodes[3] = {NULL, NULL, NULL};
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  FILE* input = fopen("bindings/rust/openui-text/fonts/Ahem.ttf", "rb");
  assert(input != NULL && fseek(input, 0, SEEK_END) == 0);
  const long count = ftell(input);
  assert(count > 0 && fseek(input, 0, SEEK_SET) == 0);
  uint8_t* bytes = (uint8_t*)malloc((size_t)count);
  assert(bytes != NULL && fread(bytes, 1, (size_t)count, input) == (size_t)count);
  assert(fclose(input) == 0);
  OuiFontFaceDescriptor descriptor;
  memset(&descriptor, 0, sizeof(descriptor));
  descriptor.struct_size = sizeof(descriptor);
  descriptor.abi_version = OUI_ABI_VERSION;
  descriptor.family = text("Native unit Ahem");
  descriptor.weight_min = descriptor.weight_max = 400;
  descriptor.stretch_min = descriptor.stretch_max = 100;
  OuiFontFace* face = NULL;
  assert(oui_document_register_font(document, bytes, (size_t)count, &descriptor, &face) == OUI_OK);
  free(bytes);
  property(root, OUI_STYLE_PROPERTY_FONT_FAMILY, "Native unit Ahem");
  property(root, OUI_STYLE_PROPERTY_FONT_SIZE, "16px");
  property(root, OUI_STYLE_PROPERTY_LINE_HEIGHT, "1.5");
  const OuiLengthUnit units[] = {OUI_LENGTH_CH, OUI_LENGTH_EX, OUI_LENGTH_LH};
  const float coefficients[] = {2.0f, 2.5f, 2.0f};
  const float before[] = {32.0f, 32.0f, 48.0f};
  const float after[] = {64.0f, 64.0f, 96.0f};
  OuiRect owned[3];
  for (size_t i = 0; i < 3; ++i) {
    assert(oui_element_create(document, OUI_ELEMENT_DIV, &nodes[i]) == OUI_OK);
    assert(oui_element_append_child(root, nodes[i]) == OUI_OK);
    OuiStyleValue width = length(coefficients[i], units[i]);
    assert(oui_element_set_property(nodes[i], OUI_STYLE_PROPERTY_WIDTH, &width) == OUI_OK);
    property(nodes[i], OUI_STYLE_PROPERTY_HEIGHT, "10px");
    assert(oui_element_get_bounds(nodes[i], &owned[i]) == OUI_OK);
    assert(owned[i].width == before[i]);
    width.data.length.value = NAN;
    assert(oui_element_set_property(nodes[i], OUI_STYLE_PROPERTY_WIDTH, &width) ==
           OUI_ERROR_INVALID_ARGUMENT);
  }
  CallbackState state = {root, 0};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(nodes[0], OUI_EVENT_CLICK, 0, clicked, &state, &listener) ==
         OUI_OK);
  OuiEvent event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(event);
  event.abi_version = OUI_ABI_VERSION;
  event.event_type = OUI_EVENT_CLICK;
  assert(oui_document_dispatch_event(document, nodes[0], &event) == OUI_OK);
  assert(state.calls == 1);
  for (size_t i = 0; i < 3; ++i) {
    OuiRect changed;
    assert(oui_element_get_bounds(nodes[i], &changed) == OUI_OK);
    assert(changed.width == after[i] && owned[i].width == before[i]);
    assert(oui_element_destroy(nodes[i]) == OUI_OK);
  }
  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_font_face_destroy(face) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  printf("native retained font units: scale=%g units=3 callback=1 passed\n", scale);
}

int main(void) {
  const double scales[] = {1.0, 1.25, 1.5, 2.0, 3.0};
  for (size_t i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i)
    native_font_units(scales[i]);
  return 0;
}
