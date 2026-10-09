/* C/C++ consuming app: native callback properties and lifetime guards. */
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
#ifdef __cplusplus
static_assert(sizeof(OuiEventPropertiesV1) == 16, "event properties layout");
#else
_Static_assert(sizeof(OuiEventPropertiesV1) == 16, "event properties layout");
#endif

typedef struct Context {
  OuiElement* a;
  OuiElement* b;
  const OuiEvent* outer;
  const OuiEvent* last;
  unsigned rows;
  unsigned nested_rows;
  unsigned nesting;
} Context;

static OuiEventPropertiesV1 properties(void) {
  OuiEventPropertiesV1 result = {sizeof(result), OUI_ABI_VERSION, 99, 99};
  return result;
}

static void reject(const OuiEvent* event, OuiEventPropertiesV1 value, OuiStatus expected) {
  OuiEventPropertiesV1 original = value;
  assert(oui_event_properties_v1(event, &value) == expected);
  assert(memcmp(&value, &original, sizeof(value)) == 0);
}

static void observe(const OuiEvent* event, uint32_t bubbles) {
  struct Extended {
    OuiEventPropertiesV1 value;
    unsigned char trailing[16];
  } extended;
  extended.value = properties();
  memset(extended.trailing, 0x6d, sizeof(extended.trailing));
  extended.value.struct_size = sizeof(extended);
  OK(oui_event_properties_v1(event, &extended.value));
  assert(extended.value.struct_size == sizeof(extended.value));
  assert(extended.value.abi_version == OUI_ABI_VERSION);
  assert(extended.value.bubbles == bubbles && extended.value.cancelable == 0);
  for (size_t i = 0; i < sizeof(extended.trailing); ++i)
    assert(extended.trailing[i] == 0x6d);
}

static void* wrong_thread(void* address) {
  reject((const OuiEvent*)address, properties(), OUI_ERROR_INVALID_STATE);
  return NULL;
}

static void callback(OuiEvent* event, void* data) {
  Context* context = (Context*)data;
  uint32_t bubbles =
      event->event_type == OUI_EVENT_FOCUS_IN || event->event_type == OUI_EVENT_FOCUS_OUT;
  observe(event, bubbles);
  context->last = event;
  ++context->rows;
  if (context->nesting) {
    observe(context->outer, 0);
    ++context->nested_rows;
  }
  uint32_t target_a = 0;
  OK(oui_element_is_same_node_v1(event->target, context->a, &target_a));
  if (target_a && event->event_type == OUI_EVENT_FOCUS) {
    OuiEventPropertiesV1 bad = properties();
    bad.struct_size = sizeof(uint32_t);
    reject(event, bad, OUI_ERROR_INVALID_ARGUMENT);
    bad = properties();
    bad.abi_version++;
    reject(event, bad, OUI_ERROR_ABI_MISMATCH);
    bad = properties();
    bad.struct_size = sizeof(bad) - 1;
    reject(event, bad, OUI_ERROR_INVALID_ARGUMENT);
    assert(oui_event_properties_v1(event, NULL) == OUI_ERROR_INVALID_ARGUMENT);
    reject(NULL, properties(), OUI_ERROR_INVALID_STATE);
    reject((const OuiEvent*)(uintptr_t)1, properties(), OUI_ERROR_INVALID_STATE);
    uint32_t short_header = sizeof(uint32_t);
    assert(oui_event_properties_v1(event, (OuiEventPropertiesV1*)&short_header) ==
           OUI_ERROR_INVALID_ARGUMENT);
    assert(short_header == sizeof(uint32_t));
    pthread_t thread;
    assert(pthread_create(&thread, NULL, wrong_thread, event) == 0);
    assert(pthread_join(thread, NULL) == 0);
    context->outer = event;
    context->nesting = 1;
    OK(oui_element_focus(context->b));
    context->nesting = 0;
    observe(event, 0);
  }
}

int main(void) {
  Context context;
  memset(&context, 0, sizeof(context));
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {320, 200, 320, 200, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document;
  OuiElement* root;
  OuiListener* listeners[6];
  OuiElement *alias_a, *alias_b;
  const uint8_t id_name[] = "id";
  const uint8_t id_a[] = "a", id_b[] = "b";
  OuiUtf8 name = {id_name, 2}, a_id = {id_a, 1}, b_id = {id_b, 1};
  OK(oui_document_create(&config, &document));
  OK(oui_document_root(document, &root));
  OK(oui_element_create(document, OUI_ELEMENT_INPUT, &context.a));
  OK(oui_element_create(document, OUI_ELEMENT_INPUT, &context.b));
  OK(oui_element_append_child(root, context.a));
  OK(oui_element_append_child(root, context.b));
  unsigned count = 0;
  const uint32_t kinds[] = {OUI_EVENT_FOCUS, OUI_EVENT_FOCUS_IN, OUI_EVENT_BLUR,
                            OUI_EVENT_FOCUS_OUT};
  for (unsigned i = 0; i < 4; ++i)
    OK(oui_element_add_event_listener(context.a, kinds[i], 0, callback, &context,
                                      &listeners[count++]));
  for (unsigned i = 0; i < 2; ++i)
    OK(oui_element_add_event_listener(context.b, kinds[i], 0, callback, &context,
                                      &listeners[count++]));
  /* Independent owned aliases become the callback target handles. Native node
   * identity must still resolve them to the listener's retained element. */
  OK(oui_element_set_attribute(context.a, name, a_id));
  OK(oui_element_set_attribute(context.b, name, b_id));
  OK(oui_document_element_by_id(document, a_id, &alias_a));
  OK(oui_document_element_by_id(document, b_id, &alias_b));
  assert(alias_a != context.a && alias_b != context.b);
  reject(NULL, properties(), OUI_ERROR_INVALID_STATE);
  OK(oui_element_focus(context.a));
  assert(context.rows == 5 && context.nested_rows == 4 && context.nesting == 0);
  reject(context.outer, properties(), OUI_ERROR_INVALID_STATE);
  reject(context.last, properties(), OUI_ERROR_INVALID_STATE);
  for (unsigned i = 0; i < count; ++i)
    OK(oui_listener_destroy(listeners[i]));
  OK(oui_element_destroy(alias_a));
  OK(oui_element_destroy(alias_b));
  OK(oui_element_destroy(context.a));
  OK(oui_element_destroy(context.b));
  OK(oui_element_destroy(root));
  OK(oui_document_destroy(document));
  puts(
      "native event properties nested/thread/header/stale/output guards "
      "passed");
  return 0;
}
