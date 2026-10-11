/* Native C/C++ consumer of input event metadata. */
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
#define ASSERT(condition)                         \
  do {                                            \
    if (!(condition)) {                           \
      fprintf(stderr, "%s failed\n", #condition); \
      exit(1);                                    \
    }                                             \
  } while (0)
typedef struct Fixture {
  OuiDocument* document;
  OuiElement *root, *field;
  OuiListener* listeners[5];
  size_t rows, buffer_count;
  OuiBuffer* saved_buffers[64];
  const OuiEvent* last_event;
  int cancel;
} Fixture;
static OuiUtf8 text(const char* s) {
  OuiUtf8 t = {(const uint8_t*)s, strlen(s)};
  return t;
}
static void string(const uint8_t* s, size_t n) {
  putchar('"');
  for (size_t i = 0; i < n; ++i) {
    unsigned c = s[i];
    switch (c) {
      case '"':
        fputs("\\\"", stdout);
        break;
      case '\\':
        fputs("\\\\", stdout);
        break;
      case '\n':
        fputs("\\n", stdout);
        break;
      case '\r':
        fputs("\\r", stdout);
        break;
      case '\t':
        fputs("\\t", stdout);
        break;
      default:
        if (c < 32)
          printf("\\u%04x", c);
        else
          putchar((int)c);
    }
  }
  putchar('"');
}
static const char* kind(uint32_t t) {
  switch (t) {
    case OUI_EVENT_BEFORE_INPUT:
      return "beforeinput";
    case OUI_EVENT_INPUT:
      return "input";
    case OUI_EVENT_CHANGE:
      return "change";
    case OUI_EVENT_BLUR:
      return "blur";
    case OUI_EVENT_FOCUS_OUT:
      return "focusout";
    default:
      exit(1);
  }
}
static const char* intent(uint32_t t) {
  switch (t) {
    case OUI_INPUT_INSERT_TEXT:
      return "insertText";
    case OUI_INPUT_INSERT_LINE_BREAK:
      return "insertLineBreak";
    case OUI_INPUT_DELETE_CONTENT_BACKWARD:
      return "deleteContentBackward";
    case OUI_INPUT_DELETE_CONTENT_FORWARD:
      return "deleteContentForward";
    case OUI_INPUT_HISTORY_UNDO:
      return "historyUndo";
    case OUI_INPUT_HISTORY_REDO:
      return "historyRedo";
    case OUI_INPUT_DELETE_WORD_BACKWARD:
      return "deleteWordBackward";
    case OUI_INPUT_DELETE_WORD_FORWARD:
      return "deleteWordForward";
    case OUI_INPUT_DELETE_BY_CUT:
      return "deleteByCut";
    case OUI_INPUT_INSERT_FROM_PASTE:
      return "insertFromPaste";
    default:
      exit(1);
  }
}
static const char* active(Fixture* f) {
  OuiElement* e = NULL;
  CHECK(oui_document_focused_element_v1(f->document, &e));
  if (!e)
    return "body";
  uint32_t same = 0;
  CHECK(oui_element_is_same_node_v1(e, f->field, &same));
  CHECK(oui_element_destroy(e));
  ASSERT(same == 1);
  return "text";
}
static void value(Fixture* f) {
  uint8_t bytes[128];
  size_t n = 0;
  CHECK(oui_element_copy_control_value(f->field, bytes, sizeof(bytes), &n));
  string(bytes, n);
}
static OuiInputEventInfoV1 empty_info(void) {
  OuiInputEventInfoV1 i;
  memset(&i, 0, sizeof(i));
  i.struct_size = sizeof(i);
  i.abi_version = OUI_ABI_VERSION;
  return i;
}
static void callback(OuiEvent* event, void* context) {
  Fixture* f = (Fixture*)context;
  OuiInputEventInfoV1 info = empty_info();
  int editing = event->event_type == OUI_EVENT_INPUT || event->event_type == OUI_EVENT_BEFORE_INPUT;
  OuiStatus result = oui_event_input_info_v1(event, &info);
  if (editing)
    CHECK(result);
  else
    ASSERT(result == OUI_ERROR_INVALID_STATE);
  OuiEventPropertiesV1 properties = {sizeof(properties), OUI_ABI_VERSION, 0, 0};
  CHECK(oui_event_properties_v1(event, &properties));
  if (editing)
    ASSERT(info.bubbles == properties.bubbles && info.cancelable == properties.cancelable);
  f->last_event = event;
  if (f->rows++)
    putchar(',');
  printf("{\"type\":\"%s\",\"value\":", kind(event->event_type));
  value(f);
  printf(",\"active\":\"%s\",\"bubbles\":%s,\"cancelable\":%s,\"input_type\":", active(f),
         properties.bubbles ? "true" : "false", properties.cancelable ? "true" : "false");
  if (editing)
    printf("\"%s\"", intent(info.input_type));
  else
    fputs("null", stdout);
  fputs(",\"data\":", stdout);
  if (editing && info.has_data) {
    ASSERT(info.data != NULL);
    string(oui_buffer_data(info.data), oui_buffer_length(info.data));
    ASSERT(f->buffer_count < 64);
    f->saved_buffers[f->buffer_count++] = info.data;
  } else {
    ASSERT(info.data == NULL);
    fputs("null", stdout);
  }
  fputs(",\"is_composing\":", stdout);
  if (editing)
    fputs(info.is_composing ? "true" : "false", stdout);
  else
    fputs("null", stdout);
  fputs("}", stdout);
  if (f->cancel && event->event_type == OUI_EVENT_BEFORE_INPUT)
    event->flags |= OUI_EVENT_DEFAULT_PREVENTED;
}
static void key(Fixture* f, int code, const char* name, uint32_t modifiers) {
  OuiEvent e;
  memset(&e, 0, sizeof(e));
  e.struct_size = sizeof(e);
  e.abi_version = OUI_ABI_VERSION;
  e.key_code = code;
  e.text = text(name);
  e.modifiers = modifiers;
  e.event_type = OUI_EVENT_KEY_DOWN;
  CHECK(oui_document_dispatch_key_input_v1(f->document, &e, text("")));
  e.event_type = OUI_EVENT_KEY_UP;
  CHECK(oui_document_dispatch_key_input_v1(f->document, &e, text("")));
}
int main(void) {
  const char* names[] = {"insert-input",         "delete-backward",      "delete-forward",
                         "input-enter",          "textarea-enter",       "undo-redo",
                         "beforeinput-canceled", "delete-word-backward", "delete-word-forward",
                         "cut-keyboard",         "paste-keyboard"};
  OuiEventType types[] = {OUI_EVENT_BEFORE_INPUT, OUI_EVENT_INPUT, OUI_EVENT_CHANGE, OUI_EVENT_BLUR,
                          OUI_EVENT_FOCUS_OUT};
  putchar('[');
  for (int n = 0; n < 11; ++n) {
    Fixture f;
    memset(&f, 0, sizeof(f));
    OuiDocumentConfig config = {
        sizeof(config), OUI_ABI_VERSION, {320, 200, 320, 200, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
    CHECK(oui_document_create(&config, &f.document));
    CHECK(oui_document_root(f.document, &f.root));
    CHECK(oui_element_create(f.document, n == 4 ? OUI_ELEMENT_TEXTAREA : OUI_ELEMENT_INPUT,
                             &f.field));
    CHECK(oui_element_append_child(f.root, f.field));
    for (size_t i = 0; i < 5; ++i)
      CHECK(oui_element_add_event_listener(f.field, types[i], 0, callback, &f, &f.listeners[i]));
    if (n)
      putchar(',');
    printf("{\"scenario\":\"%s\",\"observed\":{\"rows\":[", names[n]);
    CHECK(oui_element_focus(f.field));
    f.cancel = n == 6;
    CHECK(oui_document_dispatch_text_input_v1(f.document, text(n >= 7 ? "one two" : "A")));
    switch (n) {
      case 1:
        key(&f, 8, "Backspace", 0);
        break;
      case 2:
        key(&f, 36, "Home", 0);
        key(&f, 46, "Delete", 0);
        break;
      case 3:
      case 4:
        key(&f, 13, "Enter", 0);
        break;
      case 5:
        key(&f, 90, "z", OUI_MODIFIER_CONTROL);
        key(&f, 89, "y", OUI_MODIFIER_CONTROL);
        break;
      case 7:
        key(&f, 8, "Backspace", OUI_MODIFIER_CONTROL);
        break;
      case 8:
        key(&f, 36, "Home", 0);
        key(&f, 46, "Delete", OUI_MODIFIER_CONTROL);
        break;
      case 9:
        key(&f, 65, "a", OUI_MODIFIER_CONTROL);
        key(&f, 88, "x", OUI_MODIFIER_CONTROL);
        break;
      case 10:
        key(&f, 65, "a", OUI_MODIFIER_CONTROL);
        key(&f, 67, "c", OUI_MODIFIER_CONTROL);
        key(&f, 36, "Home", 0);
        key(&f, 86, "v", OUI_MODIFIER_CONTROL);
        break;
    }
    CHECK(oui_element_blur(f.field));
    fputs("],\"value\":", stdout);
    value(&f);
    printf(",\"active\":\"%s\"}}", active(&f));
    OuiInputEventInfoV1 expired = empty_info(), unchanged = expired;
    ASSERT(oui_event_input_info_v1(f.last_event, &expired) == OUI_ERROR_INVALID_STATE);
    ASSERT(memcmp(&expired, &unchanged, sizeof(expired)) == 0);
    for (size_t i = 0; i < 5; ++i)
      CHECK(oui_listener_destroy(f.listeners[i]));
    CHECK(oui_element_destroy(f.field));
    CHECK(oui_element_destroy(f.root));
    CHECK(oui_document_destroy(f.document));
    for (size_t i = 0; i < f.buffer_count; ++i) {
      ASSERT(oui_buffer_data(f.saved_buffers[i]) != NULL);
      ASSERT(oui_buffer_length(f.saved_buffers[i]) > 0);
      CHECK(oui_buffer_destroy(f.saved_buffers[i]));
    }
  }
  puts("]");
  return 0;
}
