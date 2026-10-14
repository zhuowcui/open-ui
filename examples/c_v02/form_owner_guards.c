/* Independently owned C/C++ relation handles and invalid-input guards. */
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
typedef OuiStatus (*Query)(OuiElement*, OuiElement**);
static const Query queries[] = {oui_element_associated_form_v1, oui_element_parent_v1};
static OuiUtf8 text(const char* s) {
  OuiUtf8 out = {(const uint8_t*)s, strlen(s)};
  return out;
}
static void reject(Query query, OuiElement* element, OuiStatus expected) {
  OuiElement* output = (OuiElement*)(uintptr_t)1;
  assert(query(element, &output) == expected);
  assert(output == (OuiElement*)(uintptr_t)1);
}
static void* foreign_thread(void* node) {
  for (size_t i = 0; i < 2; ++i)
    reject(queries[i], (OuiElement*)node, OUI_ERROR_WRONG_THREAD);
  return NULL;
}
static void callback(OuiEvent* event, void* data) {
  OuiElement *input = (OuiElement*)data, *form = NULL, *parent = NULL;
  (void)event;
  OK(oui_element_associated_form_v1(input, &form));
  OK(oui_element_parent_v1(input, &parent));
  assert(form && parent);
  uint32_t same = 0;
  OK(oui_element_is_same_node_v1(form, parent, &same));
  assert(same);
  OK(oui_element_set_attribute(form, text("id"), text("callback")));
  OK(oui_element_destroy(form));
  OK(oui_element_destroy(parent));
}
int main(void) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {320, 200, 320, 200, 1, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement *body = NULL, *form = NULL, *input = NULL;
  OK(oui_document_create(&config, &document));
  OK(oui_document_root(document, &body));
  OK(oui_element_create(document, OUI_ELEMENT_FORM, &form));
  OK(oui_element_create(document, OUI_ELEMENT_INPUT, &input));
  OK(oui_element_append_child(body, form));
  OK(oui_element_append_child(form, input));
  OuiElement *owner = NULL, *parent = NULL;
  OK(oui_element_associated_form_v1(input, &owner));
  OK(oui_element_parent_v1(input, &parent));
  assert(owner && parent && owner != form && parent != form && owner != parent);
  uint32_t same = 0;
  OK(oui_element_is_same_node_v1(owner, form, &same));
  assert(same);
  OK(oui_element_is_same_node_v1(parent, form, &same));
  assert(same);
  OK(oui_element_destroy(owner));
  OK(oui_element_set_attribute(parent, text("id"), text("retained")));
  OuiListener* listener = NULL;
  OK(oui_element_add_event_listener(input, OUI_EVENT_CLICK, 0, callback, input, &listener));
  OK(oui_element_perform_accessibility_action(input, OUI_ACCESSIBILITY_CLICK, text(""), 0, 0));
  OK(oui_listener_destroy(listener));
  for (size_t i = 0; i < 2; ++i) {
    reject(queries[i], NULL, OUI_ERROR_INVALID_ARGUMENT);
    reject(queries[i], (OuiElement*)(uintptr_t)SIZE_MAX, OUI_ERROR_INVALID_HANDLE);
    assert(queries[i](input, NULL) == OUI_ERROR_INVALID_ARGUMENT);
    reject(queries[i], (OuiElement*)document, OUI_ERROR_INVALID_HANDLE);
  }
  pthread_t thread;
  assert(pthread_create(&thread, NULL, foreign_thread, input) == 0);
  assert(pthread_join(thread, NULL) == 0);
  OuiElement* absent = (OuiElement*)(uintptr_t)1;
  OK(oui_element_associated_form_v1(body, &absent));
  assert(!absent);
  OK(oui_element_detach(form));
  OK(oui_element_associated_form_v1(input, &owner));
  OK(oui_element_destroy(owner));
  OK(oui_element_destroy(form)); /* Other aliases still refer to the live form. */
  OK(oui_element_set_attribute(parent, text("id"), text("alias")));
  OK(oui_element_remove(input));
  for (size_t i = 0; i < 2; ++i)
    reject(queries[i], input, OUI_ERROR_STALE_HANDLE);
  OK(oui_element_destroy(input));
  for (size_t i = 0; i < 2; ++i)
    reject(queries[i], input, OUI_ERROR_INVALID_HANDLE);
  OK(oui_document_destroy(document));
  for (size_t i = 0; i < 2; ++i)
    reject(queries[i], parent, OUI_ERROR_INVALID_HANDLE);
  puts("native form/parent owned relation guards passed");
  return 0;
}
