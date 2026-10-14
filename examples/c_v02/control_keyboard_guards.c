/* Checked active-state query, callback mutation and handle lifetime guards. */
#include "openui.h"
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#define OK(call) assert((call) == OUI_OK)
static OuiUtf8 text(const char* s) {
  OuiUtf8 result = {(const uint8_t*)s, strlen(s)};
  return result;
}
static void reject(OuiElement* node, OuiStatus status) {
  uint32_t value = 71;
  assert(oui_element_is_active_v1(node, &value) == status);
  assert(value == 71);
}
static void* foreign_thread(void* data) {
  reject((OuiElement*)data, OUI_ERROR_WRONG_THREAD);
  return NULL;
}
typedef struct State {
  OuiElement* node;
  unsigned calls;
} State;
static void callback(OuiEvent* event, void* data) {
  State* s = (State*)data;
  assert(event->event_type == OUI_EVENT_POINTER_DOWN);
  uint32_t active = 0;
  OK(oui_element_is_active_v1(s->node, &active));
  assert(active == 1);
  OK(oui_element_set_attribute(s->node, text("data-callback"), text("mutated")));
  OK(oui_element_is_active_v1(s->node, &active));
  assert(active == 1);
  ++s->calls;
}
static void property(OuiElement* node, OuiStyleProperty property_, const char* literal) {
  OuiStyleValue value;
  OK(oui_style_value_parse(property_, text(literal), &value));
  OK(oui_element_set_property(node, property_, &value));
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    OK(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound));
}
int main(void) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {100, 100, 100, 100, 1, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement *root = NULL, *button = NULL;
  OK(oui_document_create(&config, &document));
  OK(oui_document_root(document, &root));
  OK(oui_element_create(document, OUI_ELEMENT_BUTTON, &button));
  property(button, OUI_STYLE_PROPERTY_DISPLAY, "block");
  property(button, OUI_STYLE_PROPERTY_WIDTH, "40px");
  property(button, OUI_STYLE_PROPERTY_HEIGHT, "30px");
  OK(oui_element_append_child(root, button));
  OK(oui_document_update(document));
  uint32_t active = 99;
  OK(oui_element_is_active_v1(button, &active));
  assert(!active);
  State state = {button, 0};
  OuiListener* listener = NULL;
  OK(oui_element_add_event_listener(button, OUI_EVENT_POINTER_DOWN, 0, callback, &state,
                                    &listener));
  OuiEvent event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(event);
  event.abi_version = OUI_ABI_VERSION;
  event.event_type = OUI_EVENT_POINTER_DOWN;
  event.x = 10;
  event.y = 10;
  event.pointer_id = 7;
  OK(oui_document_dispatch_pointer_event(document, &event));
  assert(state.calls == 1);
  OK(oui_element_is_active_v1(button, &active));
  assert(active == 1);
  event.event_type = OUI_EVENT_POINTER_UP;
  event.flags = 0;
  OK(oui_document_dispatch_pointer_event(document, &event));
  OK(oui_element_is_active_v1(button, &active));
  assert(!active);
  OK(oui_listener_destroy(listener));
  reject(NULL, OUI_ERROR_INVALID_ARGUMENT);
  reject((OuiElement*)(uintptr_t)SIZE_MAX, OUI_ERROR_INVALID_HANDLE);
  reject((OuiElement*)document, OUI_ERROR_INVALID_HANDLE);
  assert(oui_element_is_active_v1(button, NULL) == OUI_ERROR_INVALID_ARGUMENT);
  pthread_t thread;
  assert(!pthread_create(&thread, NULL, foreign_thread, button));
  assert(!pthread_join(thread, NULL));
  OK(oui_element_remove(button));
  reject(button, OUI_ERROR_STALE_HANDLE);
  OK(oui_element_destroy(button));
  reject(button, OUI_ERROR_INVALID_HANDLE);
  OK(oui_document_destroy(document));
  reject(root, OUI_ERROR_INVALID_HANDLE);
  puts("native active-state callback and ownership guards passed");
  return 0;
}
