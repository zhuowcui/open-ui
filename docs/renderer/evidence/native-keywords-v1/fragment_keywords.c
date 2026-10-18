/* Exercise owned native values and retained layout from a consuming C app. */
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
    fprintf(stderr, "native keyword %d '%s': %d\n", id, literal, status);
  assert(status == OUI_OK);
  assert(oui_element_set_property(element, id, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND) {
    OuiStyleProperty other = id == OUI_STYLE_PROPERTY_COLUMN_FILL
                                 ? OUI_STYLE_PROPERTY_BREAK_INSIDE
                                 : OUI_STYLE_PROPERTY_COLUMN_FILL;
    assert(oui_element_set_property(element, other, &value) == OUI_ERROR_WRONG_VALUE_TYPE);
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
    assert(oui_element_set_property(element, id, &value) == OUI_ERROR_INVALID_HANDLE);
  }
}

typedef struct CallbackState {
  OuiElement* card;
  unsigned calls;
} CallbackState;

static void clicked(OuiEvent* event, void* data) {
  CallbackState* state = (CallbackState*)data;
  assert(event->event_type == OUI_EVENT_CLICK);
  property(state->card, OUI_STYLE_PROPERTY_COLUMN_FILL, "balance");
  property(state->card, OUI_STYLE_PROPERTY_BREAK_INSIDE, "auto");
  property(state->card, OUI_STYLE_PROPERTY_BORDER_TOP_STYLE, "none");
  ++state->calls;
}

static void native_keywords(double scale) {
  OuiDocumentConfig config = {sizeof(config), OUI_ABI_VERSION,
      {64.0, 48.0, (uint32_t)(64.0 * scale), (uint32_t)(48.0 * scale),
       scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* card = NULL;
  OuiElement* probe = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &card) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &probe) == OUI_OK);
  const struct { OuiStyleProperty id; const char* literal; } cases[] = {
    {OUI_STYLE_PROPERTY_COLUMN_FILL, "auto"},
    {OUI_STYLE_PROPERTY_BREAK_INSIDE, "avoid"},
    {OUI_STYLE_PROPERTY_BREAK_BEFORE, "column"},
    {OUI_STYLE_PROPERTY_BREAK_AFTER, "avoid-page"},
    {OUI_STYLE_PROPERTY_COLUMN_SPAN, "all"},
    {OUI_STYLE_PROPERTY_COLUMN_WRAP, "nowrap"},
    {OUI_STYLE_PROPERTY_BOX_DECORATION_BREAK, "clone"},
    {OUI_STYLE_PROPERTY_BORDER_TOP_STYLE, "outset"},
    {OUI_STYLE_PROPERTY_BORDER_RIGHT_STYLE, "inset"},
    {OUI_STYLE_PROPERTY_BORDER_BOTTOM_STYLE, "ridge"},
    {OUI_STYLE_PROPERTY_BORDER_LEFT_STYLE, "groove"},
    {OUI_STYLE_PROPERTY_COLUMN_RULE_STYLE, "double"},
    {OUI_STYLE_PROPERTY_OUTLINE_STYLE, "dashed"}
  };
  for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); ++i) {
    property(probe, cases[i].id, cases[i].literal);
    OuiStyleValue invalid;
    memset(&invalid, 0, sizeof(invalid));
    invalid.tag = 77;
    invalid.reserved = 42;
    invalid.data.integer = 123;
    assert(oui_style_value_parse(cases[i].id, utf8("invalid-keyword"), &invalid) ==
           OUI_ERROR_INVALID_ARGUMENT);
    assert(invalid.tag == 77 && invalid.reserved == 42 && invalid.data.integer == 123);
  }
  const struct { const char* literal; OuiDisplay value; } displays[] = {
    {"inline-table", OUI_DISPLAY_INLINE_TABLE},
    {"table-row-group", OUI_DISPLAY_TABLE_ROW_GROUP},
    {"table-header-group", OUI_DISPLAY_TABLE_HEADER_GROUP},
    {"table-footer-group", OUI_DISPLAY_TABLE_FOOTER_GROUP},
    {"table-row", OUI_DISPLAY_TABLE_ROW}, {"table-cell", OUI_DISPLAY_TABLE_CELL},
    {"table-column-group", OUI_DISPLAY_TABLE_COLUMN_GROUP},
    {"table-column", OUI_DISPLAY_TABLE_COLUMN}, {"table-caption", OUI_DISPLAY_TABLE_CAPTION}
  };
  for (size_t i = 0; i < sizeof(displays) / sizeof(displays[0]); ++i) {
    OuiStyleValue value;
    assert(oui_style_value_parse(OUI_STYLE_PROPERTY_DISPLAY, utf8(displays[i].literal), &value) == OUI_OK);
    assert(value.tag == OUI_STYLE_VALUE_ENUM && value.data.enum_value == (int32_t)displays[i].value);
    assert(oui_element_set_property(probe, OUI_STYLE_PROPERTY_DISPLAY, &value) == OUI_OK);
  }
  property(card, OUI_STYLE_PROPERTY_DISPLAY, "block");
  property(card, OUI_STYLE_PROPERTY_WIDTH, "20px");
  property(card, OUI_STYLE_PROPERTY_HEIGHT, "20px");
  property(card, OUI_STYLE_PROPERTY_BORDER, "4px solid black");
  property(card, OUI_STYLE_PROPERTY_COLUMN_FILL, "auto");
  property(card, OUI_STYLE_PROPERTY_BREAK_INSIDE, "avoid");
  assert(oui_element_append_child(root, card) == OUI_OK);
  OuiRect before;
  assert(oui_element_get_bounds(card, &before) == OUI_OK);
  assert(before.x == 0.0f && before.y == 0.0f && before.width == 28.0f && before.height == 28.0f);
  CallbackState state = {card, 0};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(card, OUI_EVENT_CLICK, 0, clicked, &state, &listener) == OUI_OK);
  OuiEvent event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(event);
  event.abi_version = OUI_ABI_VERSION;
  event.event_type = OUI_EVENT_CLICK;
  assert(oui_document_dispatch_event(document, card, &event) == OUI_OK);
  assert(state.calls == 1);
  OuiRect after;
  assert(oui_element_get_bounds(card, &after) == OUI_OK);
  assert(after.x == 0.0f && after.y == 0.0f && after.width == 28.0f && after.height == 24.0f);
  size_t count = 0;
  OuiRect rect;
  assert(oui_element_get_client_rects_v1(card, &rect, 1, &count) == OUI_OK && count == 1);
  assert(rect.x == after.x && rect.y == after.y && rect.width == after.width && rect.height == after.height);
  assert(before.width == 28.0f && before.height == 28.0f);
  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_element_destroy(probe) == OUI_OK);
  assert(oui_element_destroy(card) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  printf("native fragment keywords: scale=%g values=22 callback=1 owned-bounds passed\n", scale);
}

int main(void) {
  const double scales[] = {1.0, 1.25, 1.5, 2.0, 3.0};
  for (size_t i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i)
    native_keywords(scales[i]);
  return 0;
}
