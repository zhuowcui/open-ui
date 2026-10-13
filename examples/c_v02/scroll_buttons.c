/* Native C/C++ consumer of retained scroll-button ownership and interaction.
 * Keep all guards active in packaged release builds. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stddef.h>
#include <string.h>

#include "openui.h"

#ifdef __cplusplus
static_assert(sizeof(OuiScrollButtonStateV1) == 56, "scroll-button ABI size");
static_assert(alignof(OuiScrollButtonStateV1) == 8, "scroll-button ABI alignment");
static_assert(offsetof(OuiScrollButtonStateV1, current_x) == 24, "scroll-button ABI offset");
#else
_Static_assert(sizeof(OuiScrollButtonStateV1) == 56, "scroll-button ABI size");
_Static_assert(_Alignof(OuiScrollButtonStateV1) == 8, "scroll-button ABI alignment");
_Static_assert(offsetof(OuiScrollButtonStateV1, current_x) == 24, "scroll-button ABI offset");
#endif

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

static OuiScrollButtonStateV1 snapshot(OuiElement* button) {
  OuiScrollButtonStateV1 value;
  memset(&value, 0, sizeof(value));
  value.struct_size = sizeof(value);
  value.abi_version = OUI_ABI_VERSION;
  assert(oui_scroll_button_get_state_v1(button, &value) == OUI_OK);
  return value;
}

static void same_node(OuiElement* first, OuiElement* second) {
  uint32_t same = 0;
  assert(oui_element_is_same_node_v1(first, second, &same) == OUI_OK);
  assert(same == 1);
}

typedef struct Callback {
  OuiElement* origin;
  OuiElement* button;
  unsigned clicks;
  unsigned changes;
  unsigned key_down;
  unsigned key_up;
  int cancel;
  int cancel_key;
} Callback;

static void clicked(OuiEvent* event, void* data) {
  Callback* callback = (Callback*)data;
  same_node(event->target, callback->origin);
  same_node(event->current_target, callback->origin);
  if (event->event_type == OUI_EVENT_CLICK) {
    ++callback->clicks;
    /* The callback can read the same Engine after all dispatch borrows end. */
    OuiScrollButtonStateV1 value = snapshot(callback->button);
    assert(value.current_y >= 0);
    if (callback->cancel)
      event->flags |= OUI_EVENT_DEFAULT_PREVENTED;
  } else if (event->event_type == OUI_EVENT_KEY_DOWN) {
    ++callback->key_down;
    /* The same retained Engine is readable before the key's default action. */
    OuiScrollButtonStateV1 value = snapshot(callback->button);
    assert(value.current_y >= 0);
    if (callback->cancel_key)
      event->flags |= OUI_EVENT_DEFAULT_PREVENTED;
  } else if (event->event_type == OUI_EVENT_KEY_UP) {
    ++callback->key_up;
  } else {
    assert(event->event_type == OUI_EVENT_INPUT || event->event_type == OUI_EVENT_CHANGE);
    ++callback->changes;
  }
}

static void key(OuiDocument* document, uint32_t kind, uint32_t code, const char* name) {
  OuiEvent event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(event);
  event.abi_version = OUI_ABI_VERSION;
  event.event_type = kind;
  event.key_code = code;
  event.text = text(name);
  assert(oui_document_dispatch_key_input_v1(document, &event, text("")) == OUI_OK);
}

static void run(double scale) {
  OuiDocumentConfig config = {sizeof(config),
                              OUI_ABI_VERSION,
                              {320.0, 240.0, (uint32_t)(320.0 * scale), (uint32_t)(240.0 * scale),
                               scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* origin = NULL;
  OuiElement* content = NULL;
  OuiElement* button = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &origin) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &content) == OUI_OK);
  assert(oui_element_append_child(root, origin) == OUI_OK);
  assert(oui_element_append_child(origin, content) == OUI_OK);
  property(origin, OUI_STYLE_PROPERTY_WIDTH, "80px");
  property(origin, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  property(origin, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  property(origin, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
  property(content, OUI_STYLE_PROPERTY_WIDTH, "80px");
  property(content, OUI_STYLE_PROPERTY_HEIGHT, "500px");
  assert(oui_element_create_scroll_button_v1(origin, OUI_SCROLL_BUTTON_DOWN, &button) == OUI_OK);
  assert(oui_element_set_native_appearance_v1(button, 0) == OUI_OK);
  assert(oui_element_set_text(button, text("next")) == OUI_OK);
  property(button, OUI_STYLE_PROPERTY_WIDTH, "24px");
  property(button, OUI_STYLE_PROPERTY_HEIGHT, "24px");
  assert(oui_element_set_attribute(button, text("aria-label"), text("next page")) == OUI_OK);
  OuiElement* alias = NULL;
  assert(oui_element_get_scroll_button_v1(origin, OUI_SCROLL_BUTTON_DOWN, &alias) == OUI_OK);
  assert(alias != NULL && alias != button);
  same_node(button, alias);
  /* Aliases own only handles; the Engine owns the retained control. */
  assert(oui_element_destroy(button) == OUI_OK);
  button = alias;
  OuiElement* related = NULL;
  assert(oui_scroll_button_get_origin_v1(button, &related) == OUI_OK);
  same_node(origin, related);
  assert(oui_element_destroy(related) == OUI_OK);
  OuiScrollButtonStateV1 initial = snapshot(button);
  assert(initial.enabled == 1 && initial.has_target == 1);
  assert(initial.current_y == 0 && initial.target_y == 70);
  assert(initial.direction == OUI_SCROLL_BUTTON_DOWN);
  size_t reason_length = 1;
  assert(oui_scroll_button_copy_unavailable_reason_v1(button, NULL, 0, &reason_length) == OUI_OK);
  assert(reason_length == 0);
  reason_length = 37;
  assert(oui_scroll_button_copy_unavailable_reason_v1(button, NULL, 1, &reason_length) ==
         OUI_ERROR_INVALID_ARGUMENT);
  assert(reason_length == 37);
  related = (OuiElement*)(uintptr_t)1;
  assert(oui_scroll_button_get_snap_target_v1(button, 1, OUI_SCROLL_BUTTON_PREVIEW_SNAP,
                                              &related) == OUI_OK);
  assert(related == NULL);
  related = (OuiElement*)(uintptr_t)1;
  assert(oui_scroll_button_get_snap_target_v1(button, 2, OUI_SCROLL_BUTTON_CURRENT_SNAP,
                                              &related) == OUI_ERROR_INVALID_ARGUMENT);
  assert(related == (OuiElement*)(uintptr_t)1);

  Callback callback = {origin, button, 0, 0, 0, 0, 1, 0};
  OuiListener* listeners[5] = {NULL, NULL, NULL, NULL, NULL};
  const OuiEventType kinds[5] = {OUI_EVENT_CLICK, OUI_EVENT_INPUT, OUI_EVENT_CHANGE,
                                 OUI_EVENT_KEY_DOWN, OUI_EVENT_KEY_UP};
  for (unsigned i = 0; i < 5; ++i)
    assert(oui_element_add_event_listener(origin, kinds[i], 0, clicked, &callback, &listeners[i]) ==
           OUI_OK);
  assert(oui_scroll_button_activate_v1(button) == OUI_OK);
  assert(snapshot(button).current_y == 0);
  assert(callback.clicks == 1 && callback.changes == 0);
  assert(callback.key_down == 0 && callback.key_up == 0);
  callback.cancel = 0;
  assert(oui_scroll_button_activate_v1(button) == OUI_OK);
  assert(snapshot(button).current_y == 70);
  assert(callback.clicks == 2 && callback.changes == 0);
  OuiElement* focused = NULL;
  assert(oui_document_focused_element_v1(document, &focused) == OUI_OK);
  assert(focused != NULL);
  same_node(origin, focused);
  assert(oui_element_destroy(focused) == OUI_OK);
  /* Pinned Chromium scroll-button defaults activate on keydown directly;
   * they do not synthesize a click or arm ordinary Button's Space-keyup path. */
  key(document, OUI_EVENT_KEY_DOWN, 32, " ");
  assert(snapshot(button).current_y == 140);
  assert(callback.clicks == 2 && callback.changes == 0);
  assert(callback.key_down == 1 && callback.key_up == 0);
  key(document, OUI_EVENT_KEY_UP, 32, " ");
  assert(snapshot(button).current_y == 140);
  assert(callback.clicks == 2 && callback.changes == 0);
  assert(callback.key_down == 1 && callback.key_up == 1);
  key(document, OUI_EVENT_KEY_DOWN, 13, "Enter");
  assert(snapshot(button).current_y == 210);
  assert(callback.clicks == 2 && callback.changes == 0);
  assert(callback.key_down == 2 && callback.key_up == 1);
  key(document, OUI_EVENT_KEY_UP, 13, "Enter");
  assert(snapshot(button).current_y == 210);
  assert(callback.clicks == 2 && callback.changes == 0);
  assert(callback.key_down == 2 && callback.key_up == 2);
  callback.cancel_key = 1;
  key(document, OUI_EVENT_KEY_DOWN, 32, " ");
  assert(snapshot(button).current_y == 210);
  assert(callback.clicks == 2 && callback.changes == 0);
  assert(callback.key_down == 3 && callback.key_up == 2);
  key(document, OUI_EVENT_KEY_UP, 32, " ");
  assert(snapshot(button).current_y == 210);
  assert(callback.clicks == 2 && callback.changes == 0);
  assert(callback.key_down == 3 && callback.key_up == 3);
  callback.cancel_key = 0;
  assert(oui_element_set_attribute(button, text("disabled"), text("")) == OUI_OK);
  OuiScrollButtonStateV1 disabled = snapshot(button);
  assert(disabled.enabled == 0 && disabled.availability == OUI_SCROLL_BUTTON_AUTHORED_DISABLED);
  assert(oui_scroll_button_activate_v1(button) == OUI_OK);
  assert(snapshot(button).current_y == 210);
  assert(callback.clicks == 2 && callback.changes == 0);

  for (unsigned i = 0; i < 5; ++i)
    assert(oui_listener_destroy(listeners[i]) == OUI_OK);
  assert(oui_element_remove(origin) == OUI_OK);
  OuiScrollButtonStateV1 stale = initial;
  assert(oui_scroll_button_get_state_v1(button, &stale) == OUI_ERROR_STALE_HANDLE);
  assert(memcmp(&stale, &initial, sizeof(stale)) == 0);
  assert(oui_element_destroy(button) == OUI_OK);
  assert(oui_element_destroy(content) == OUI_OK);
  assert(oui_element_destroy(origin) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  assert(initial.current_y == 0 && disabled.current_y == 210);
}

static OuiElement* snap_area(OuiDocument* document, OuiElement* content, const char* x) {
  OuiElement* area = NULL;
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &area) == OUI_OK);
  property(area, OUI_STYLE_PROPERTY_WIDTH, "20px");
  property(area, OUI_STYLE_PROPERTY_HEIGHT, "20px");
  property(area, OUI_STYLE_PROPERTY_POSITION, "absolute");
  property(area, OUI_STYLE_PROPERTY_LEFT, x);
  property(area, OUI_STYLE_PROPERTY_TOP, "0px");
  /* One property declaration carries the block and inline alignments. */
  property(area, OUI_STYLE_PROPERTY_SCROLL_SNAP_ALIGN, "end start");
  assert(oui_element_append_child(content, area) == OUI_OK);
  return area;
}

static void snap_target(OuiElement* button, uint32_t phase, OuiElement* expected) {
  OuiElement* alias = NULL;
  assert(oui_scroll_button_get_snap_target_v1(button, 0, phase, &alias) == OUI_OK);
  assert(alias != NULL);
  same_node(alias, expected);
  assert(oui_element_destroy(alias) == OUI_OK);
}

static void run_snap(double scale) {
  OuiDocumentConfig config = {sizeof(config),
                              OUI_ABI_VERSION,
                              {320.0, 240.0, (uint32_t)(320.0 * scale), (uint32_t)(240.0 * scale),
                               scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* origin = NULL;
  OuiElement* content = NULL;
  OuiElement* button = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &origin) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &content) == OUI_OK);
  assert(oui_element_append_child(root, origin) == OUI_OK);
  assert(oui_element_append_child(origin, content) == OUI_OK);
  property(origin, OUI_STYLE_PROPERTY_WIDTH, "100px");
  property(origin, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  property(origin, OUI_STYLE_PROPERTY_OVERFLOW, "scroll");
  property(origin, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
  property(origin, OUI_STYLE_PROPERTY_SCROLL_SNAP_AXIS, "x");
  property(content, OUI_STYLE_PROPERTY_WIDTH, "400px");
  property(content, OUI_STYLE_PROPERTY_HEIGHT, "300px");
  property(content, OUI_STYLE_PROPERTY_POSITION, "relative");
  OuiElement* first = snap_area(document, content, "40px");
  OuiElement* second = snap_area(document, content, "200px");
  OuiElement* preferred = snap_area(document, content, "200px");
  assert(oui_element_create_scroll_button_v1(origin, OUI_SCROLL_BUTTON_RIGHT, &button) == OUI_OK);
  assert(oui_element_set_native_appearance_v1(button, 0) == OUI_OK);
  property(button, OUI_STYLE_PROPERTY_WIDTH, "24px");
  property(button, OUI_STYLE_PROPERTY_HEIGHT, "24px");
  assert(snapshot(button).current_x == 40);
  snap_target(button, OUI_SCROLL_BUTTON_CURRENT_SNAP, first);

  assert(oui_element_set_scroll_snap_target_v1(origin, preferred) == OUI_OK);
  snap_target(button, OUI_SCROLL_BUTTON_PREVIEW_SNAP, preferred);
  OuiDocument* foreign_document = NULL;
  OuiElement* foreign = NULL;
  assert(oui_document_create(&config, &foreign_document) == OUI_OK);
  assert(oui_element_create(foreign_document, OUI_ELEMENT_DIV, &foreign) == OUI_OK);
  assert(oui_element_set_scroll_snap_target_v1(origin, foreign) == OUI_ERROR_WRONG_DOCUMENT);
  snap_target(button, OUI_SCROLL_BUTTON_PREVIEW_SNAP, preferred);
  assert(oui_element_destroy(foreign) == OUI_OK);
  assert(oui_document_destroy(foreign_document) == OUI_OK);

  assert(oui_scroll_button_activate_v1(button) == OUI_OK);
  assert(snapshot(button).current_x == 200);
  snap_target(button, OUI_SCROLL_BUTTON_CURRENT_SNAP, preferred);
  property(preferred, OUI_STYLE_PROPERTY_LEFT, "250px");
  assert(snapshot(button).current_x == 250);
  snap_target(button, OUI_SCROLL_BUTTON_CURRENT_SNAP, preferred);
  property(preferred, OUI_STYLE_PROPERTY_DISPLAY, "none");
  assert(snapshot(button).current_x == 200);
  snap_target(button, OUI_SCROLL_BUTTON_CURRENT_SNAP, second);
  assert(oui_element_set_scroll_snap_target_v1(origin, NULL) == OUI_OK);
  uint8_t changed = 37;
  assert(oui_element_settle_scroll_snap_v1(origin, &changed) == OUI_OK);
  assert(changed == 0 && snapshot(button).current_x == 200);
  assert(oui_element_settle_scroll_snap_v1(origin, NULL) == OUI_ERROR_INVALID_ARGUMENT);
  assert(oui_element_remove(origin) == OUI_OK);
  changed = 37;
  assert(oui_element_settle_scroll_snap_v1(origin, &changed) == OUI_ERROR_STALE_HANDLE);
  assert(changed == 37);

  assert(oui_element_destroy(preferred) == OUI_OK);
  assert(oui_element_destroy(second) == OUI_OK);
  assert(oui_element_destroy(first) == OUI_OK);
  assert(oui_element_destroy(button) == OUI_OK);
  assert(oui_element_destroy(content) == OUI_OK);
  assert(oui_element_destroy(origin) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
}

int main(void) {
  const double scales[] = {1, 1.25, 1.5, 2, 3};
  for (unsigned i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i) {
    run(scales[i]);
    run_snap(scales[i]);
  }
  return 0;
}
