#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

#include "openui.h"

static void property(OuiElement* element, OuiStyleProperty id, const char* literal) {
  OuiUtf8 text = {(const uint8_t*)literal, strlen(literal)};
  OuiStyleValue value;
  assert(oui_style_value_parse(id, text, &value) == OUI_OK);
  assert(oui_element_set_property(element, id, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}

static void color(OuiElement* element,
                  OuiStyleProperty id,
                  float red,
                  float green,
                  float blue,
                  float alpha) {
  OuiStyleValue value;
  assert(oui_style_value_color_f32_v1(id, red, green, blue, alpha, &value) == OUI_OK);
  assert(value.tag == OUI_STYLE_VALUE_COMPOUND && value.reserved == 0);
  assert(oui_element_set_property(element, id, &value) == OUI_OK);
  assert(oui_element_set_property(element, OUI_STYLE_PROPERTY_WIDTH, &value) ==
         OUI_ERROR_WRONG_VALUE_TYPE);
  assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
  assert(oui_element_set_property(element, id, &value) == OUI_ERROR_INVALID_HANDLE);
}

typedef struct CallbackState {
  OuiElement* card;
  unsigned calls;
} CallbackState;

static void clicked(OuiEvent* event, void* data) {
  CallbackState* state = (CallbackState*)data;
  assert(event->event_type == OUI_EVENT_CLICK);
  color(state->card, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, 0.0f, 0.0f, 1.0f, 0.5f);
  ++state->calls;
}

static void native_colors(double scale) {
  OuiDocumentConfig config = {
      sizeof(config),
      OUI_ABI_VERSION,
      {64, 64, (uint32_t)(64 * scale), (uint32_t)(64 * scale), scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement *root = NULL, *card = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &card) == OUI_OK);
  assert(oui_element_append_child(root, card) == OUI_OK);
  property(root, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "black");
  property(root, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  property(card, OUI_STYLE_PROPERTY_WIDTH, "32px");
  property(card, OUI_STYLE_PROPERTY_HEIGHT, "32px");
  const OuiStyleProperty properties[] = {
      OUI_STYLE_PROPERTY_BACKGROUND_COLOR,      OUI_STYLE_PROPERTY_COLOR,
      OUI_STYLE_PROPERTY_BORDER_TOP_COLOR,      OUI_STYLE_PROPERTY_BORDER_RIGHT_COLOR,
      OUI_STYLE_PROPERTY_BORDER_BOTTOM_COLOR,   OUI_STYLE_PROPERTY_BORDER_LEFT_COLOR,
      OUI_STYLE_PROPERTY_COLUMN_RULE_COLOR,     OUI_STYLE_PROPERTY_OUTLINE_COLOR,
      OUI_STYLE_PROPERTY_SCROLLBAR_TRACK_COLOR, OUI_STYLE_PROPERTY_SCROLLBAR_THUMB_COLOR,
      OUI_STYLE_PROPERTY_TEXT_DECORATION_COLOR, OUI_STYLE_PROPERTY_TEXT_EMPHASIS_COLOR};
  for (size_t i = 0; i < sizeof(properties) / sizeof(properties[0]); ++i)
    color(card, properties[i], 0.123456f, 0.234567f, 0.345678f, 0.5f);
  OuiStyleValue output;
  memset(&output, 0, sizeof(output));
  output.tag = 77;
  output.reserved = 42;
  output.data.integer = 123;
  assert(oui_style_value_color_f32_v1(OUI_STYLE_PROPERTY_COLOR, NAN, 0, 0, 1, &output) ==
         OUI_ERROR_INVALID_ARGUMENT);
  assert(output.tag == 77 && output.reserved == 42 && output.data.integer == 123);
  color(card, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, 1.0f, 0.0f, 0.0f, 0.5f);
  OuiBitmap before = {sizeof(before), OUI_ABI_VERSION, 0, 0, 0, NULL};
  assert(oui_document_render_rgba(document, &before) == OUI_OK);
  CallbackState state = {card, 0};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(card, OUI_EVENT_CLICK, 0, clicked, &state, &listener) ==
         OUI_OK);
  OuiEvent event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(event);
  event.abi_version = OUI_ABI_VERSION;
  event.event_type = OUI_EVENT_CLICK;
  assert(oui_document_dispatch_event(document, card, &event) == OUI_OK);
  assert(state.calls == 1);
  OuiBitmap after = {sizeof(after), OUI_ABI_VERSION, 0, 0, 0, NULL};
  assert(oui_document_render_rgba(document, &after) == OUI_OK);
  const size_t sample = (size_t)(16 * scale) * before.stride + (size_t)(16 * scale) * 4;
  const uint8_t* initial = oui_buffer_data(before.pixels);
  const uint8_t* changed = oui_buffer_data(after.pixels);
  assert(initial[sample] == 128 && initial[sample + 1] == 0 && initial[sample + 2] == 0 &&
         initial[sample + 3] == 255);
  assert(changed[sample] == 0 && changed[sample + 1] == 0 && changed[sample + 2] == 128 &&
         changed[sample + 3] == 255);
  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_element_destroy(card) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  assert(oui_buffer_data(before.pixels)[sample] == 128);
  assert(oui_buffer_data(after.pixels)[sample + 2] == 128);
  assert(oui_buffer_destroy(before.pixels) == OUI_OK);
  assert(oui_buffer_destroy(after.pixels) == OUI_OK);
  printf("native float colors: scale=%g properties=12 callback=1 owned-frames=2 passed\n", scale);
}

int main(void) {
  const double scales[] = {1.0, 1.25, 1.5, 2.0, 3.0};
  for (size_t i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i)
    native_colors(scales[i]);
  return 0;
}
