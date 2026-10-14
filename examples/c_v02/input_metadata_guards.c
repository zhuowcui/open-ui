/* Native input metadata lifetime and ownership guards. */
#include "openui.h"
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <pthread.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#ifdef __cplusplus
static_assert(sizeof(OuiInputEventInfoV1) == 40, "input info layout");
static_assert(offsetof(OuiInputEventInfoV1, data) == 32, "input data offset");
#else
_Static_assert(sizeof(OuiInputEventInfoV1) == 40, "input info layout");
_Static_assert(offsetof(OuiInputEventInfoV1, data) == 32, "input data offset");
#endif
#define OK(call) assert((call) == OUI_OK)
static OuiUtf8 text(const char* s) {
  OuiUtf8 t = {(const uint8_t*)s, strlen(s)};
  return t;
}
static OuiInputEventInfoV1 empty_info(void) {
  OuiInputEventInfoV1 i;
  memset(&i, 0, sizeof(i));
  i.struct_size = sizeof(i);
  i.abi_version = OUI_ABI_VERSION;
  return i;
}
static void exact_data(OuiInputEventInfoV1 i, const char* s) {
  assert(i.has_data == 1 && i.data != NULL);
  assert(oui_buffer_length(i.data) == strlen(s));
  assert(memcmp(oui_buffer_data(i.data), s, strlen(s)) == 0);
}
typedef struct Context {
  OuiDocument *outer, *inner;
  const OuiEvent *outer_event, *last_event;
  OuiBuffer* saved;
  unsigned outer_rows, inner_rows, nesting;
} Context;
typedef struct ThreadCheck {
  const OuiEvent* event;
  OuiBuffer* buffer;
} ThreadCheck;
static void* other_thread(void* raw) {
  ThreadCheck* t = (ThreadCheck*)raw;
  OuiInputEventInfoV1 i = empty_info(), original = i;
  assert(oui_event_input_info_v1(t->event, &i) == OUI_ERROR_INVALID_STATE);
  assert(memcmp(&i, &original, sizeof(i)) == 0);
  assert(oui_buffer_destroy(t->buffer) == OUI_ERROR_WRONG_THREAD);
  return NULL;
}
static void reject(OuiEvent* event, OuiInputEventInfoV1 i, OuiStatus status) {
  OuiInputEventInfoV1 original = i;
  assert(oui_event_input_info_v1(event, &i) == status);
  assert(memcmp(&i, &original, sizeof(i)) == 0);
}
static void inner_callback(OuiEvent* event, void* raw) {
  Context* c = (Context*)raw;
  assert(c->nesting == 1);
  OuiInputEventInfoV1 inner = empty_info(), outer = empty_info();
  OK(oui_event_input_info_v1(event, &inner));
  exact_data(inner, "B");
  OK(oui_event_input_info_v1(c->outer_event, &outer));
  exact_data(outer, "A");
  assert(inner.input_type == OUI_INPUT_INSERT_TEXT && outer.input_type == OUI_INPUT_INSERT_TEXT);
  assert(inner.cancelable == (event->event_type == OUI_EVENT_BEFORE_INPUT));
  assert(outer.cancelable == 1);
  OK(oui_buffer_destroy(inner.data));
  OK(oui_buffer_destroy(outer.data));
  c->inner_rows++;
}
static void outer_callback(OuiEvent* event, void* raw) {
  Context* c = (Context*)raw;
  OuiInputEventInfoV1 i = empty_info();
  OK(oui_event_input_info_v1(event, &i));
  exact_data(i, "A");
  assert(i.input_type == OUI_INPUT_INSERT_TEXT && i.bubbles == 1 && i.is_composing == 0);
  assert(i.cancelable == (event->event_type == OUI_EVENT_BEFORE_INPUT));
  c->last_event = event;
  c->outer_rows++;
  if (event->event_type == OUI_EVENT_BEFORE_INPUT) {
    c->outer_event = event;
    c->nesting = 1;
    OuiInputEventInfoV1 bad = empty_info();
    bad.struct_size = 4;
    reject(event, bad, OUI_ERROR_INVALID_ARGUMENT);
    bad = empty_info();
    bad.abi_version++;
    reject(event, bad, OUI_ERROR_ABI_MISMATCH);
    bad = empty_info();
    bad.data = i.data;
    reject(event, bad, OUI_ERROR_INVALID_ARGUMENT);
    assert(oui_event_input_info_v1(event, NULL) == OUI_ERROR_INVALID_ARGUMENT);
    reject(NULL, empty_info(), OUI_ERROR_INVALID_STATE);
    ThreadCheck t = {event, i.data};
    pthread_t thread;
    assert(pthread_create(&thread, NULL, other_thread, &t) == 0);
    assert(pthread_join(thread, NULL) == 0);
    exact_data(i, "A");
    OK(oui_document_dispatch_text_input_v1(c->inner, text("B")));
    c->nesting = 0;
    OuiInputEventInfoV1 after = empty_info();
    OK(oui_event_input_info_v1(event, &after));
    exact_data(after, "A");
    OK(oui_buffer_destroy(after.data));
    c->saved = i.data;
  } else {
    OK(oui_buffer_destroy(i.data));
  }
}
static void setup(OuiDocument** document, OuiElement** root, OuiElement** field) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {320, 200, 320, 200, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
  OK(oui_document_create(&config, document));
  OK(oui_document_root(*document, root));
  OK(oui_element_create(*document, OUI_ELEMENT_INPUT, field));
  OK(oui_element_append_child(*root, *field));
  OK(oui_element_focus(*field));
}
int main(void) {
  Context c;
  memset(&c, 0, sizeof(c));
  OuiElement *outer_root, *outer_field, *inner_root, *inner_field;
  OuiListener* listeners[4];
  setup(&c.outer, &outer_root, &outer_field);
  setup(&c.inner, &inner_root, &inner_field);
  OK(oui_element_add_event_listener(outer_field, OUI_EVENT_BEFORE_INPUT, 0, outer_callback, &c,
                                    &listeners[0]));
  OK(oui_element_add_event_listener(outer_field, OUI_EVENT_INPUT, 0, outer_callback, &c,
                                    &listeners[1]));
  OK(oui_element_add_event_listener(inner_field, OUI_EVENT_BEFORE_INPUT, 0, inner_callback, &c,
                                    &listeners[2]));
  OK(oui_element_add_event_listener(inner_field, OUI_EVENT_INPUT, 0, inner_callback, &c,
                                    &listeners[3]));
  OK(oui_document_dispatch_text_input_v1(c.outer, text("A")));
  assert(c.outer_rows == 2 && c.inner_rows == 2 && c.nesting == 0 && c.saved != NULL);
  reject((OuiEvent*)c.last_event, empty_info(), OUI_ERROR_INVALID_STATE);
  for (unsigned n = 0; n < 4; ++n)
    OK(oui_listener_destroy(listeners[n]));
  OK(oui_element_destroy(outer_field));
  OK(oui_element_destroy(outer_root));
  OK(oui_document_destroy(c.outer));
  OK(oui_element_destroy(inner_field));
  OK(oui_element_destroy(inner_root));
  OK(oui_document_destroy(c.inner));
  OuiInputEventInfoV1 owned = empty_info();
  owned.has_data = 1;
  owned.data = c.saved;
  exact_data(owned, "A");
  OK(oui_buffer_destroy(c.saved));
  assert(oui_buffer_destroy(c.saved) == OUI_ERROR_INVALID_HANDLE);
  puts("native input metadata nested/thread/header/stale/owned guards passed");
  return 0;
}
