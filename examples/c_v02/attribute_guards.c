/* C/C++ native attribute lookup, owned data and handle lifetime guards. */
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
static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}
static void equal(OuiBuffer* value, const char* expected) {
  assert(value != NULL);
  assert(oui_buffer_length(value) == strlen(expected));
  if (*expected)
    assert(memcmp(oui_buffer_data(value), expected, strlen(expected)) == 0);
}
static void rejected(OuiElement* element, OuiUtf8 name, OuiStatus expected) {
  OuiBuffer* output = (OuiBuffer*)(uintptr_t)1;
  assert(oui_element_get_attribute_v1(element, name, &output) == expected);
  assert(output == (OuiBuffer*)(uintptr_t)1);
}
static void* wrong_thread(void* element) {
  rejected((OuiElement*)element, text("data-text"), OUI_ERROR_WRONG_THREAD);
  return NULL;
}
int main(void) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {320, 200, 320, 200, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document;
  OuiElement *root, *element;
  OK(oui_document_create(&config, &document));
  OK(oui_document_root(document, &root));
  OK(oui_element_create(document, OUI_ELEMENT_INPUT, &element));
  OK(oui_element_append_child(root, element));
  OuiBuffer* value = (OuiBuffer*)(uintptr_t)1;
  OK(oui_element_get_attribute_v1(element, text("absent"), &value));
  assert(value == NULL);
  OK(oui_element_get_attribute_v1(element, text(""), &value));
  assert(value == NULL);
  OK(oui_element_set_attribute(element, text("DiSaBlEd"), text("")));
  OK(oui_element_get_attribute_v1(element, text("DISABLED"), &value));
  equal(value, "");
  OK(oui_buffer_destroy(value));
  OK(oui_element_remove_attribute(element, text("disabled")));
  OK(oui_element_get_attribute_v1(element, text("disabled"), &value));
  assert(value == NULL);
  OK(oui_element_set_attribute(element, text("data-text"), text("界🙂é")));
  OuiBuffer* saved = NULL;
  OK(oui_element_get_attribute_v1(element, text("DATA-TEXT"), &saved));
  equal(saved, "界🙂é");
  OK(oui_element_set_attribute(element, text("data-text"), text("changed")));
  OK(oui_element_get_attribute_v1(element, text("data-text"), &value));
  equal(value, "changed");
  equal(saved, "界🙂é");
  OK(oui_buffer_destroy(value));
  const uint8_t bad_byte = 0xff;
  OuiUtf8 bad = {&bad_byte, 1};
  rejected(element, bad, OUI_ERROR_INVALID_ARGUMENT);
  bad.data = NULL;
  rejected(element, bad, OUI_ERROR_INVALID_ARGUMENT);
  rejected(NULL, text("data-text"), OUI_ERROR_INVALID_ARGUMENT);
  rejected((OuiElement*)(uintptr_t)SIZE_MAX, text("data-text"), OUI_ERROR_INVALID_HANDLE);
  assert(oui_element_get_attribute_v1(element, text("data-text"), NULL) ==
         OUI_ERROR_INVALID_ARGUMENT);
  pthread_t thread;
  assert(pthread_create(&thread, NULL, wrong_thread, element) == 0);
  assert(pthread_join(thread, NULL) == 0);
  OK(oui_element_detach(element));
  OK(oui_element_get_attribute_v1(element, text("data-text"), &value));
  equal(value, "changed");
  OK(oui_buffer_destroy(value));
  OK(oui_element_remove(element));
  rejected(element, text("data-text"), OUI_ERROR_STALE_HANDLE);
  OK(oui_element_destroy(element));
  rejected(element, text("data-text"), OUI_ERROR_INVALID_HANDLE);
  OK(oui_element_destroy(root));
  OK(oui_document_destroy(document));
  equal(saved, "界🙂é");
  OK(oui_buffer_destroy(saved));
  assert(oui_buffer_destroy(saved) == OUI_ERROR_INVALID_HANDLE);
  puts(
      "native attribute presence/unicode/mutation/thread/stale/owned guards "
      "passed");
  return 0;
}
