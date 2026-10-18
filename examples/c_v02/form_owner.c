/* Native C/C++ consuming app: operations are typed data, never scripts.
 * State and callback observations come from public native APIs. */
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "form_owner_cases.h"
#include "openui.h"
#define CHECK(call)                                          \
  do {                                                       \
    OuiStatus code_ = (call);                                \
    if (code_ != OUI_OK) {                                   \
      fprintf(stderr, "%s failed: %d\n", #call, (int)code_); \
      exit(1);                                               \
    }                                                        \
  } while (0)
typedef struct State {
  OuiDocument* document;
  OuiElement* nodes[12];
  OuiListener* listeners[35];
  size_t listener_count, row_count, row_length;
  char rows[65536];
} State;
static const char* labels[] = {"body",    "wrap", "fa", "fb", "parking", "blocker",
                               "newform", "a",    "b",  "c",  "d",       "e"};
static const char* boolean(int value) {
  return value ? "true" : "false";
}
static OuiUtf8 text(const char* value) {
  OuiUtf8 out = {(const uint8_t*)value, strlen(value)};
  return out;
}
static void add(char* out, size_t capacity, size_t* length, const char* format, ...) {
  va_list args;
  va_start(args, format);
  int n = vsnprintf(out + *length, capacity - *length, format, args);
  va_end(args);
  if (n < 0 || (size_t)n >= capacity - *length)
    abort();
  *length += (size_t)n;
}
static void quoted(char* out, size_t capacity, size_t* length, const char* value) {
  if (!value) {
    add(out, capacity, length, "null");
    return;
  }
  add(out, capacity, length, "\"");
  for (const unsigned char* p = (const unsigned char*)value; *p; ++p) {
    if (*p == '"' || *p == '\\')
      add(out, capacity, length, "\\%c", *p);
    else if (*p < 32)
      add(out, capacity, length, "\\u%04x", *p);
    else
      add(out, capacity, length, "%c", *p);
  }
  add(out, capacity, length, "\"");
}
static const char* identify(State* s, OuiElement* node) {
  if (!node)
    return NULL;
  for (int i = 0; i < 12; ++i) {
    uint32_t same = 0;
    CHECK(oui_element_is_same_node_v1(node, s->nodes[i], &same));
    if (same)
      return labels[i];
  }
  abort();
}
static const char* active(State* s) {
  OuiElement* node = NULL;
  CHECK(oui_document_focused_element_v1(s->document, &node));
  const char* name = node ? identify(s, node) : "body";
  if (node)
    CHECK(oui_element_destroy(node));
  return name;
}
static void snapshot(State* s, char* out, size_t capacity, size_t* length) {
  add(out, capacity, length, "{\"controls\":{");
  for (int i = 7; i < 12; ++i) {
    OuiElement *node = s->nodes[i], *form = NULL, *parent = NULL;
    uint32_t flags = 0, connected = 0;
    OuiBuffer *authored = NULL, *form_attribute = NULL;
    CHECK(oui_element_get_control_flags(node, &flags));
    CHECK(oui_element_is_connected_v1(node, &connected));
    CHECK(oui_element_get_attribute_v1(node, text("checked"), &authored));
    CHECK(oui_element_get_attribute_v1(node, text("form"), &form_attribute));
    CHECK(oui_element_associated_form_v1(node, &form));
    CHECK(oui_element_parent_v1(node, &parent));
    if (i != 7)
      add(out, capacity, length, ",");
    add(out, capacity, length,
        "\"%s\":{\"checked\":%s,\"checkedAttribute\":%s,\"connected\":%s,\"formAttribute\":",
        labels[i], boolean(flags & OUI_CONTROL_CHECKED), boolean(authored != NULL),
        boolean(connected));
    if (form_attribute) {
      size_t bytes = oui_buffer_length(form_attribute);
      char* value = (char*)malloc(bytes + 1);
      if (!value)
        abort();
      memcpy(value, oui_buffer_data(form_attribute), bytes);
      value[bytes] = 0;
      quoted(out, capacity, length, value);
      free(value);
    } else
      quoted(out, capacity, length, NULL);
    add(out, capacity, length, ",\"formOwner\":");
    quoted(out, capacity, length, identify(s, form));
    add(out, capacity, length, ",\"parent\":");
    quoted(out, capacity, length, identify(s, parent));
    add(out, capacity, length, "}");
    if (form)
      CHECK(oui_element_destroy(form));
    if (parent)
      CHECK(oui_element_destroy(parent));
    if (authored)
      CHECK(oui_buffer_destroy(authored));
    if (form_attribute)
      CHECK(oui_buffer_destroy(form_attribute));
  }
  add(out, capacity, length, "},\"active\":");
  quoted(out, capacity, length, active(s));
  add(out, capacity, length, "}");
}
static const char* event_name(uint32_t kind) {
  switch (kind) {
    case OUI_EVENT_CLICK:
      return "click";
    case OUI_EVENT_INPUT:
      return "input";
    case OUI_EVENT_CHANGE:
      return "change";
    case OUI_EVENT_FOCUS:
      return "focus";
    case OUI_EVENT_FOCUS_IN:
      return "focusin";
    case OUI_EVENT_BLUR:
      return "blur";
    case OUI_EVENT_FOCUS_OUT:
      return "focusout";
    default:
      abort();
  }
}
static void record(OuiEvent* event, void* data) {
  State* s = (State*)data;
  OuiEventPropertiesV1 props = {sizeof(props), OUI_ABI_VERSION, 0, 0};
  CHECK(oui_event_properties_v1(event, &props));
  char state[8192] = {0};
  size_t length = 0;
  snapshot(s, state, sizeof(state), &length);
  if (s->row_count++)
    add(s->rows, sizeof(s->rows), &s->row_length, ",");
  add(s->rows, sizeof(s->rows), &s->row_length,
      "{\"type\":\"%s\",\"target\":", event_name(event->event_type));
  quoted(s->rows, sizeof(s->rows), &s->row_length, identify(s, event->target));
  add(s->rows, sizeof(s->rows), &s->row_length, ",\"bubbles\":%s,\"cancelable\":%s,%.*s",
      boolean(props.bubbles), boolean(props.cancelable), (int)length - 1, state + 1);
}
static void barrier(State* s) {
  for (int i = 0; i < 64; ++i) {
    uint32_t pending = 0;
    CHECK(oui_document_dispatch_pending_events_v1(s->document, &pending));
    if (!pending)
      return;
  }
  abort();
}
static void apply(State* s, const FormOperation* op) {
  OuiElement* node = s->nodes[op->target];
  switch (op->kind) {
    case FORM_ATTRIBUTE:
      CHECK(oui_element_set_attribute(node, text(op->name), text(op->value)));
      break;
    case FORM_REMOVE_ATTRIBUTE:
      CHECK(oui_element_remove_attribute(node, text(op->name)));
      break;
    case FORM_DETACH:
      CHECK(oui_element_detach(node));
      break;
    case FORM_APPEND:
      CHECK(oui_element_append_child(s->nodes[op->parent], node));
      break;
    case FORM_INSERT:
      CHECK(oui_element_insert_before(s->nodes[op->parent], node, s->nodes[op->before]));
      break;
    case FORM_CHECKED:
      CHECK(oui_element_set_checked(node, (uint8_t)op->checked));
      break;
    default:
      abort();
  }
}
static void make(State* s, const char* control) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {800, 600, 800, 600, 1, OUI_VIEWPORT_LOGICAL, 0}};
  CHECK(oui_document_create(&config, &s->document));
  CHECK(oui_document_root(s->document, &s->nodes[0]));
  for (int i = 1; i <= 6; ++i) {
    CHECK(oui_element_create(s->document,
                             (i == 2 || i == 3 || i == 6) ? OUI_ELEMENT_FORM : OUI_ELEMENT_DIV,
                             &s->nodes[i]));
    if (i != 6) {
      CHECK(oui_element_append_child(s->nodes[0], s->nodes[i]));
      CHECK(oui_element_set_attribute(s->nodes[i], text("id"), text(labels[i])));
    }
  }
  const int parents[] = {1, 1, 2, 3, 0};
  for (int i = 7; i < 12; ++i) {
    CHECK(oui_element_create(s->document, OUI_ELEMENT_INPUT, &s->nodes[i]));
    CHECK(oui_element_set_attribute(s->nodes[i], text("id"), text(labels[i])));
    CHECK(oui_element_set_attribute(s->nodes[i], text("type"), text(control)));
    CHECK(oui_element_set_attribute(s->nodes[i], text("name"), text("g")));
    if (i == 7 || i == 8)
      CHECK(oui_element_set_attribute(s->nodes[i], text("form"), text(i == 7 ? "fa" : "fb")));
    CHECK(oui_element_append_child(s->nodes[parents[i - 7]], s->nodes[i]));
  }
  for (int i = 7; i <= 8; ++i)
    CHECK(oui_element_set_checked(s->nodes[i], 1));
  CHECK(oui_element_set_checked(s->nodes[11], 1));
  const OuiEventType events[] = {OUI_EVENT_CLICK,    OUI_EVENT_INPUT,    OUI_EVENT_CHANGE,
                                 OUI_EVENT_FOCUS,    OUI_EVENT_FOCUS_IN, OUI_EVENT_BLUR,
                                 OUI_EVENT_FOCUS_OUT};
  for (int i = 7; i < 12; ++i)
    for (size_t j = 0; j < sizeof(events) / sizeof(events[0]); ++j)
      CHECK(oui_element_add_event_listener(s->nodes[i], events[j], 0, record, s,
                                           &s->listeners[s->listener_count++]));
  barrier(s);
}
int main(void) {
  const char* controls[] = {"radio", "checkbox"};
  int comma = 0;
  puts("[");
  for (size_t t = 0; t < 2; ++t)
    for (size_t n = 0; n < sizeof(form_cases) / sizeof(form_cases[0]); ++n) {
      State s;
      memset(&s, 0, sizeof(s));
      make(&s, controls[t]);
      const FormCase* case_ = &form_cases[n];
      for (size_t i = 0; i < case_->pre_count; ++i)
        apply(&s, &case_->pre[i]);
      s.row_length = s.row_count = 0;
      s.rows[0] = 0;
      if (comma++)
        puts(",");
      printf("{\"scenario\":\"%s-%s\",\"control\":\"%s\",\"variant\":\"%s\",\"initial\":",
             controls[t], case_->name, controls[t], case_->name);
      char state[8192] = {0};
      size_t length = 0;
      snapshot(&s, state, sizeof(state), &length);
      printf("%s,\"steps\":[", state);
      for (size_t i = 0; i < case_->step_count; ++i) {
        if (i)
          printf(",");
        const FormOperation* op = &case_->steps[i];
        apply(&s, op);
        length = 0;
        snapshot(&s, state, sizeof(state), &length);
        printf("{\"operation\":%s,\"synchronous\":{\"rows\":[%s],\"state\":%s},", op->json, s.rows,
               state);
        barrier(&s);
        length = 0;
        snapshot(&s, state, sizeof(state), &length);
        printf("\"observed\":{\"rows\":[%s],\"state\":%s}}", s.rows, state);
      }
      printf("]}");
      for (size_t i = 0; i < s.listener_count; ++i)
        CHECK(oui_listener_destroy(s.listeners[i]));
      for (int i = 11; i >= 0; --i)
        CHECK(oui_element_destroy(s.nodes[i]));
      CHECK(oui_document_destroy(s.document));
    }
  puts("\n]");
  return 0;
}
