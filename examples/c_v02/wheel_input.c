#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <math.h>
#include <stdbool.h>
#include <string.h>

#include "openui.h"

static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}
static void property(OuiElement* node, OuiStyleProperty id, const char* literal) {
  OuiStyleValue value = {0};
  assert(oui_style_value_parse(id, text(literal), &value) == OUI_OK);
  assert(oui_element_set_property(node, id, &value) == OUI_OK);
  if (value.tag == 6)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}
static OuiElement* child(OuiDocument* document,
                         OuiElement* parent,
                         const char* width,
                         const char* height) {
  OuiElement* node = NULL;
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &node) == OUI_OK);
  assert(oui_element_append_child(parent, node) == OUI_OK);
  property(node, OUI_STYLE_DISPLAY, "block");
  property(node, OUI_STYLE_WIDTH, width);
  property(node, OUI_STYLE_HEIGHT, height);
  return node;
}
typedef struct {
  OuiElement* source;
  bool cancel;
  unsigned capture_count, bubble_count, scroll_count;
  double before_y;
} Observation;
static void observe(OuiEvent* event, void* user_data) {
  Observation* data = (Observation*)user_data;
  if (event->event_type == OUI_EVENT_SCROLL) {
    ++data->scroll_count;
    return;
  }
  assert(event->event_type == OUI_EVENT_WHEEL);
  if (event->phase == 1)
    ++data->capture_count;
  else {
    assert(event->phase == 3);
    ++data->bubble_count;
  }
  double x = -1, y = -1;
  assert(oui_element_get_scroll_offset(data->source, &x, &y) == OUI_OK);
  data->before_y = y;
  assert(oui_element_set_attribute(data->source, text("data-wheel"), text("observed")) == OUI_OK);
  if (data->cancel)
    event->flags |= OUI_EVENT_FLAG_DEFAULT_PREVENTED;
}
int main(void) {
  const double scales[] = {1, 1.25, 1.5, 1.75, 2};
  for (size_t i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i) {
    OuiDocumentConfig config = {0};
    config.struct_size = sizeof(config);
    config.abi_version = OUI_ABI_VERSION;
    config.viewport.logical_width = 320;
    config.viewport.logical_height = 240;
    config.viewport.physical_width = (uint32_t)(320 * scales[i]);
    config.viewport.physical_height = (uint32_t)(240 * scales[i]);
    config.viewport.device_scale_factor = scales[i];
    config.viewport.authority = 1;
    OuiDocument* document = NULL;
    OuiElement* root = NULL;
    assert(oui_document_create(&config, &document) == OUI_OK);
    assert(oui_document_root(document, &root) == OUI_OK);
    OuiElement* source = child(document, root, "100px", "80px");
    property(source, OUI_STYLE_OVERFLOW, "auto");
    property(source, OUI_STYLE_SCROLLBAR_WIDTH, "none");
    OuiElement* content = child(document, source, "100px", "240px");
    property(content, OUI_STYLE_BACKGROUND_COLOR, "green");
    OuiElement* focus = child(document, root, "20px", "20px");
    property(focus, OUI_STYLE_POSITION, "absolute");
    property(focus, OUI_STYLE_LEFT, "200px");
    property(focus, OUI_STYLE_TOP, "0px");
    assert(oui_element_set_attribute(focus, text("tabindex"), text("0")) == OUI_OK);
    assert(oui_element_focus(focus) == OUI_OK);
    assert(oui_document_update(document) == OUI_OK);
    Observation data = {source, true, 0, 0, 0, -1};
    OuiListener *capture = NULL, *bubble = NULL, *scroll = NULL;
    assert(oui_element_add_event_listener(root, OUI_EVENT_WHEEL, 1, observe, &data, &capture) ==
           OUI_OK);
    assert(oui_element_add_event_listener(root, OUI_EVENT_WHEEL, 0, observe, &data, &bubble) ==
           OUI_OK);
    assert(oui_element_add_event_listener(source, OUI_EVENT_SCROLL, 0, observe, &data, &scroll) ==
           OUI_OK);
    OuiRect bounds = {0};
    assert(oui_element_get_bounds(source, &bounds) == OUI_OK);
    OuiEvent wheel = {0};
    wheel.struct_size = sizeof(wheel);
    wheel.abi_version = OUI_ABI_VERSION;
    wheel.event_type = OUI_EVENT_WHEEL;
    wheel.x = bounds.x + 2;
    wheel.y = bounds.y + 2;
    wheel.delta_y = 20;
    wheel.timestamp_ns = 123;
    assert(oui_document_dispatch_wheel_input_v1(document, &wheel) == OUI_OK);
    double x = -1, y = -1;
    assert(oui_element_get_scroll_offset(source, &x, &y) == OUI_OK);
    assert(x == 0 && y == 0 && data.capture_count == 1 && data.bubble_count == 1);
    assert(wheel.flags == 0 && wheel.timestamp_ns == 123);
    data.cancel = false;
    assert(oui_document_dispatch_wheel_input_v1(document, &wheel) == OUI_OK);
    assert(oui_element_get_scroll_offset(source, &x, &y) == OUI_OK);
    assert(y == 20 && data.before_y == 0);
    uint32_t pending = UINT32_MAX;
    assert(oui_document_dispatch_pending_events_v1(document, &pending) == OUI_OK);
    assert(data.scroll_count == 1);
    OuiElement* focused = NULL;
    assert(oui_document_focused_element_v1(document, &focused) == OUI_OK);
    uint32_t same = 0;
    assert(oui_element_is_same_node_v1(focused, focus, &same) == OUI_OK && same == 1);
    assert(oui_element_destroy(focused) == OUI_OK);
    OuiEvent raw = wheel;
    assert(oui_document_dispatch_event(document, content, &raw) == OUI_OK);
    assert(oui_element_get_scroll_offset(source, &x, &y) == OUI_OK && y == 20);
    wheel.delta_y = NAN;
    assert(oui_document_dispatch_wheel_input_v1(document, &wheel) == OUI_ERROR_INVALID_ARGUMENT);
    assert(oui_element_get_scroll_offset(source, &x, &y) == OUI_OK && y == 20);
    assert(oui_listener_destroy(capture) == OUI_OK);
    assert(oui_listener_destroy(bubble) == OUI_OK);
    assert(oui_listener_destroy(scroll) == OUI_OK);
    assert(oui_element_destroy(focus) == OUI_OK);
    assert(oui_element_destroy(content) == OUI_OK);
    assert(oui_element_destroy(source) == OUI_OK);
    assert(oui_element_destroy(root) == OUI_OK);
    assert(oui_document_destroy(document) == OUI_OK);
  }
  return 0;
}
