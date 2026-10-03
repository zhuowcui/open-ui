/* Keep application checks active in release SDK builds. */
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
    fprintf(stderr, "native property %d literal '%s': %d\n", id, literal, status);
  assert(status == OUI_OK);
  assert(oui_element_set_property(element, id, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}

typedef struct CallbackState {
  OuiElement* card;
  unsigned calls;
} CallbackState;

static void clicked(OuiEvent* event, void* data) {
  CallbackState* state = (CallbackState*)data;
  assert(event->event_type == OUI_EVENT_CLICK);
  property(state->card, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "blue");
  property(state->card, OUI_STYLE_PROPERTY_LEFT, "4em");
  property(state->card, OUI_STYLE_PROPERTY_FILTER_BLUR, "0");
  property(state->card, OUI_STYLE_PROPERTY_FILTER_GRAYSCALE, "0");
  property(state->card, OUI_STYLE_PROPERTY_COLUMN_WIDTH, "auto");
  property(state->card, OUI_STYLE_PROPERTY_SCROLLBAR_TRACK_COLOR, "green");
  ++state->calls;
}

static void native_styles(double scale) {
  OuiDocumentConfig config = {sizeof(config),
                              OUI_ABI_VERSION,
                              {320.0, 240.0, (uint32_t)(320.0 * scale), (uint32_t)(240.0 * scale),
                               scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* card = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &card) == OUI_OK);
  assert(oui_element_append_child(root, card) == OUI_OK);
  property(root, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  property(root, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "white");
  property(card, OUI_STYLE_PROPERTY_POSITION, "absolute");
  property(card, OUI_STYLE_PROPERTY_DISPLAY, "block");
  property(card, OUI_STYLE_PROPERTY_WIDTH, "80px");
  property(card, OUI_STYLE_PROPERTY_HEIGHT, "40px");
  property(card, OUI_STYLE_PROPERTY_FONT_SIZE, "20px");
  property(card, OUI_STYLE_PROPERTY_COLOR, "green");
  property(card, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "red");
  property(card, OUI_STYLE_PROPERTY_BORDER, "1px solid black");

  const struct {
    OuiStyleProperty id;
    const char* literal;
  } values[] = {
      {OUI_STYLE_PROPERTY_ALIGN_CONTENT, "safe end"},
      {OUI_STYLE_PROPERTY_ALIGN_SELF, "unsafe center"},
      {OUI_STYLE_PROPERTY_BORDER_BOTTOM_COLOR, "currentcolor"},
      {OUI_STYLE_PROPERTY_BORDER_BOTTOM_WIDTH, "1"},
      {OUI_STYLE_PROPERTY_BORDER_LEFT_COLOR, "#123456"},
      {OUI_STYLE_PROPERTY_BORDER_LEFT_WIDTH, "2"},
      {OUI_STYLE_PROPERTY_BORDER_RIGHT_COLOR, "blue"},
      {OUI_STYLE_PROPERTY_BORDER_RIGHT_WIDTH, "3"},
      {OUI_STYLE_PROPERTY_BORDER_TOP_COLOR, "red"},
      {OUI_STYLE_PROPERTY_BORDER_TOP_WIDTH, "4"},
      {OUI_STYLE_PROPERTY_BOTTOM, "auto"},
      {OUI_STYLE_PROPERTY_COLUMN_HEIGHT, "2em"},
      {OUI_STYLE_PROPERTY_COLUMN_RULE_COLOR, "currentcolor"},
      {OUI_STYLE_PROPERTY_COLUMN_RULE_WIDTH, "2"},
      {OUI_STYLE_PROPERTY_COLUMN_WIDTH, "96px"},
      {OUI_STYLE_PROPERTY_FILTER_BLUR, "1.25"},
      {OUI_STYLE_PROPERTY_FILTER_GRAYSCALE, "0.4"},
      {OUI_STYLE_PROPERTY_JUSTIFY_ITEMS, "self-end"},
      {OUI_STYLE_PROPERTY_JUSTIFY_SELF, "last baseline"},
      {OUI_STYLE_PROPERTY_LEFT, "2em"},
      {OUI_STYLE_PROPERTY_ORDER, "-7"},
      {OUI_STYLE_PROPERTY_ORPHANS, "4294967295"},
      {OUI_STYLE_PROPERTY_OUTLINE_COLOR, "currentcolor"},
      {OUI_STYLE_PROPERTY_OUTLINE_OFFSET, "-2"},
      {OUI_STYLE_PROPERTY_OVERFLOW_CLIP_MARGIN, "2.5"},
      {OUI_STYLE_PROPERTY_OVERFLOW_X, "clip"},
      {OUI_STYLE_PROPERTY_OVERFLOW_Y, "hidden"},
      {OUI_STYLE_PROPERTY_RIGHT, "auto"},
      {OUI_STYLE_PROPERTY_SCROLLBAR_TRACK_COLOR, "auto"},
      {OUI_STYLE_PROPERTY_TOP, "10vh"},
      {OUI_STYLE_PROPERTY_WIDOWS, "3"},
      {OUI_STYLE_PROPERTY_OUTLINE_WIDTH, "2"},
      {OUI_STYLE_PROPERTY_SCROLLBAR_THUMB_COLOR, "#123456"},
      {OUI_STYLE_PROPERTY_SHAPE_IMAGE_THRESHOLD, "0.25"},
      {OUI_STYLE_PROPERTY_SHAPE_MARGIN, "1.5rem"},
  };
  for (size_t i = 0; i < sizeof(values) / sizeof(values[0]); ++i)
    property(card, values[i].id, values[i].literal);

  OuiRect original;
  assert(oui_element_get_bounds(card, &original) == OUI_OK);
  assert(original.x == 40.0f && original.y == 24.0f);
  assert(original.width == 85.0f && original.height == 45.0f);
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
  OuiRect changed;
  assert(oui_element_get_bounds(card, &changed) == OUI_OK && changed.x == 80.0f);
  assert(changed.y == 24.0f && changed.width == 85.0f && changed.height == 45.0f);
  OuiBitmap after = {sizeof(after), OUI_ABI_VERSION, 0, 0, 0, NULL};
  assert(oui_document_render_rgba(document, &after) == OUI_OK);
  assert(before.width == after.width && before.height == after.height);
  assert(oui_buffer_length(before.pixels) == oui_buffer_length(after.pixels));
  assert(memcmp(oui_buffer_data(before.pixels), oui_buffer_data(after.pixels),
                oui_buffer_length(before.pixels)) != 0);
  size_t sample = (size_t)(40.0 * scale) * after.stride + (size_t)(100.0 * scale) * 4;
  const uint8_t* pixels = oui_buffer_data(after.pixels);
  if (pixels[sample] != 0 || pixels[sample + 1] != 0 || pixels[sample + 2] != 255 ||
      pixels[sample + 3] != 255)
    fprintf(stderr, "native blue pixel at scale=%g: %u,%u,%u,%u\n", scale, pixels[sample],
            pixels[sample + 1], pixels[sample + 2], pixels[sample + 3]);
  assert(pixels[sample] == 0 && pixels[sample + 1] == 0 && pixels[sample + 2] == 255 &&
         pixels[sample + 3] == 255);
  assert(original.x == 40.0f && original.width == 85.0f);
  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_element_destroy(card) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  /* Owned render buffers outlive the document. */
  assert(oui_buffer_data(after.pixels)[sample + 2] == 255);
  assert(oui_buffer_destroy(before.pixels) == OUI_OK);
  assert(oui_buffer_destroy(after.pixels) == OUI_OK);
  printf("native C primitive styles: scale=%g properties=35 callback=1 passed\n", scale);
}

int main(void) {
  const double scales[] = {1.0, 1.25, 1.5, 2.0, 3.0};
  for (size_t i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i)
    native_styles(scales[i]);
  return 0;
}
