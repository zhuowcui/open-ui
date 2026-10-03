/* Consuming C app. Checks remain active in release SDK builds. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <math.h>
#include <string.h>

#include "openui.h"

static void property(OuiElement* e, OuiStyleProperty id, const char* literal) {
  OuiUtf8 text = {(const uint8_t*)literal, strlen(literal)};
  OuiStyleValue value;
  assert(oui_style_value_parse(id, text, &value) == OUI_OK);
  assert(oui_element_set_property(e, id, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}

static OuiElement* block(OuiDocument* d, OuiElement* parent, const char* w, const char* h) {
  OuiElement* e = NULL;
  assert(oui_element_create(d, OUI_ELEMENT_DIV, &e) == OUI_OK);
  property(e, OUI_STYLE_PROPERTY_WIDTH, w);
  property(e, OUI_STYLE_PROPERTY_HEIGHT, h);
  property(e, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
  assert(oui_element_append_child(parent, e) == OUI_OK);
  return e;
}

static void offset(OuiElement* e, double expected_x, double expected_y) {
  double x, y;
  assert(oui_element_get_scroll_offset(e, &x, &y) == OUI_OK);
  assert(x == expected_x && y == expected_y);
}

typedef struct Reveal {
  unsigned calls;
} Reveal;
static void clicked(OuiEvent* event, void* user_data) {
  Reveal* state = (Reveal*)user_data;
  assert(event->event_type == OUI_EVENT_CLICK);
  assert(oui_element_scroll_into_view_v1(event->target, OUI_SCROLL_NEAREST, OUI_SCROLL_NEAREST,
                                         OUI_SCROLL_CONTAINERS_ALL) == OUI_OK);
  ++state->calls;
}

static void run(double scale, const char* overflow, double final_y) {
  OuiDocumentConfig config = {sizeof(config),
                              OUI_ABI_VERSION,
                              {320.0, 240.0, (uint32_t)(320.0 * scale), (uint32_t)(240.0 * scale),
                               scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* d = NULL;
  OuiElement* root = NULL;
  assert(oui_document_create(&config, &d) == OUI_OK);
  assert(oui_document_root(d, &root) == OUI_OK);
  property(root, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
  OuiElement* outer = block(d, root, "100px", "80px");
  property(outer, OUI_STYLE_PROPERTY_POSITION, "absolute");
  property(outer, OUI_STYLE_PROPERTY_LEFT, "40px");
  property(outer, OUI_STYLE_PROPERTY_TOP, "30px");
  property(outer, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  OuiElement* spacer = block(d, outer, "200px", "200px");
  OuiElement* inner = block(d, outer, "200px", "100px");
  property(inner, OUI_STYLE_PROPERTY_OVERFLOW, overflow);
  OuiElement* target = block(d, inner, "20px", "20px");
  property(target, OUI_STYLE_PROPERTY_MARGIN_LEFT, "150px");
  property(target, OUI_STYLE_PROPERTY_MARGIN_TOP, "120px");
  assert(oui_element_scroll_into_view_v1(target, 99, OUI_SCROLL_NEAREST,
                                         OUI_SCROLL_CONTAINERS_ALL) == OUI_ERROR_INVALID_ARGUMENT);
  assert(oui_element_smooth_scroll_into_view_v1(target, OUI_SCROLL_NEAREST, OUI_SCROLL_NEAREST,
                                                OUI_SCROLL_CONTAINERS_ALL,
                                                NAN) == OUI_ERROR_INVALID_ARGUMENT);
  offset(outer, 0, 0);
  Reveal state = {0};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(target, OUI_EVENT_CLICK, 0, clicked, &state, &listener) ==
         OUI_OK);
  OuiEvent event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(event);
  event.abi_version = OUI_ABI_VERSION;
  event.event_type = OUI_EVENT_CLICK;
  assert(oui_document_dispatch_event(d, target, &event) == OUI_OK);
  assert(state.calls == 1);
  offset(outer, 70, final_y);
  offset(inner, 0, strcmp(overflow, "clip") == 0 ? 0 : 40);
  OuiRect bounds;
  assert(oui_element_get_bounds(target, &bounds) == OUI_OK);
  assert(bounds.x == 120 && bounds.y == 90 && bounds.width == 20 && bounds.height == 20);
  assert(oui_element_scroll_to(outer, 0, 0) == OUI_OK);
  assert(oui_element_scroll_to(inner, 0, 0) == OUI_OK);
  assert(oui_element_smooth_scroll_into_view_v1(target, OUI_SCROLL_NEAREST, OUI_SCROLL_NEAREST,
                                                OUI_SCROLL_CONTAINERS_ALL, 100) == OUI_OK);
  offset(outer, 0, 0);
  assert(oui_document_set_animation_time(d, 100) == OUI_OK);
  offset(outer, 70, final_y);
  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_element_destroy(target) == OUI_OK);
  assert(oui_element_destroy(inner) == OUI_OK);
  assert(oui_element_destroy(spacer) == OUI_OK);
  assert(oui_element_destroy(outer) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(d) == OUI_OK);
}

int main(void) {
  const double scales[] = {1, 1.25, 1.5, 2, 3};
  for (unsigned i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i) {
    run(scales[i], "hidden", 220);
    run(scales[i], "clip", 260);
  }
  return 0;
}
