/* Native C/C++ consumer: callback cancellation and completed control state. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "openui.h"

#define CHECK(call)                          \
  do {                                       \
    if ((call) != OUI_OK) {                  \
      fprintf(stderr, "%s failed\n", #call); \
      exit(1);                               \
    }                                        \
  } while (0)
typedef struct CallbackData {
  const char* target;
  int modifier;
} CallbackData;
static size_t rows;
static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}
static const char* kind(uint32_t type) {
  switch (type) {
    case OUI_EVENT_BEFORE_INPUT:
      return "beforeinput";
    case OUI_EVENT_INPUT:
      return "input";
    case OUI_EVENT_CHANGE:
      return "change";
    default:
      exit(1);
  }
}
static void callback(OuiEvent* event, void* user_data) {
  CallbackData* data = (CallbackData*)user_data;
  if (data->modifier) {
    if (event->event_type != OUI_EVENT_BEFORE_INPUT ||
        (event->text.length == 7 && memcmp(event->text.data, "blocked", 7) == 0)) {
      event->flags |= OUI_EVENT_DEFAULT_PREVENTED;
    }
    return;
  }
  if (rows++)
    printf(",");
  printf("{\"type\":\"%s\",\"target\":\"%s\",\"after\":%s}", kind(event->event_type), data->target,
         event->flags & OUI_EVENT_DEFAULT_PREVENTED ? "true" : "false");
}
int main(void) {
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* input = NULL;
  OuiElement* box = NULL;
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {320, 200, 320, 200, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
  CHECK(oui_document_create(&config, &document));
  CHECK(oui_document_root(document, &root));
  CHECK(oui_element_create(document, OUI_ELEMENT_INPUT, &input));
  CHECK(oui_element_create(document, OUI_ELEMENT_INPUT, &box));
  CHECK(oui_element_set_attribute(box, text("type"), text("checkbox")));
  CHECK(oui_element_append_child(root, input));
  CHECK(oui_element_append_child(root, box));
  const OuiEventType types[] = {OUI_EVENT_BEFORE_INPUT, OUI_EVENT_INPUT, OUI_EVENT_CHANGE};
  OuiListener* listeners[12];
  CallbackData data[12];
  size_t count = 0;
  for (size_t node = 0; node < 2; ++node) {
    for (size_t type = 0; type < 3; ++type) {
      for (int modifier = 1; modifier >= 0; --modifier) {
        data[count].target = node ? "box" : "text";
        data[count].modifier = modifier;
        CHECK(oui_element_add_event_listener(node ? box : input, types[type], 0, callback,
                                             &data[count], &listeners[count]));
        ++count;
      }
    }
  }
  printf("{\"rows\":[");
  CHECK(oui_element_focus(input));
  CHECK(oui_document_dispatch_text_input_v1(document, text("A")));
  CHECK(oui_document_dispatch_text_input_v1(document, text("blocked")));
  OuiEvent click;
  memset(&click, 0, sizeof(click));
  click.struct_size = sizeof(click);
  click.abi_version = OUI_ABI_VERSION;
  click.event_type = OUI_EVENT_CLICK;
  CHECK(oui_document_dispatch_event(document, box, &click));
  uint8_t value[16];
  size_t length = 0;
  CHECK(oui_element_copy_control_value(input, value, sizeof(value), &length));
  uint32_t flags = 0;
  CHECK(oui_element_get_control_flags(box, &flags));
  printf("],\"value\":\"%.*s\",\"checked\":%s}\n", (int)length, value,
         flags & OUI_CONTROL_CHECKED ? "true" : "false");
  for (size_t i = 0; i < count; ++i)
    CHECK(oui_listener_destroy(listeners[i]));
  CHECK(oui_element_destroy(box));
  CHECK(oui_element_destroy(input));
  CHECK(oui_element_destroy(root));
  CHECK(oui_document_destroy(document));
  return 0;
}
