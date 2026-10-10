/* Native C/C++ keyboard consuming app. Data describes operations only. */
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "control_keyboard_cases.h"
#include "openui.h"
#define CHECK(call)                                            \
  do {                                                         \
    OuiStatus status_ = (call);                                \
    if (status_ != OUI_OK) {                                   \
      fprintf(stderr, "%s failed: %d\n", #call, (int)status_); \
      exit(1);                                                 \
    }                                                          \
  } while (0)
typedef struct State State;
typedef struct Registration {
  State* state;
  int label;
} Registration;
struct State {
  OuiDocument* document;
  OuiElement* nodes[6];
  OuiListener* listeners[36];
  Registration registrations[36];
  size_t listener_count, row_count, row_length;
  char rows[131072];
  const KeyboardCase* case_;
  int handled;
};
static const char* labels[] = {"body", "fa", "fb", "a", "b", "c"};
static const char* boolean(int x) {
  return x ? "true" : "false";
}
static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}
static void add(char* output, size_t capacity, size_t* length, const char* format, ...) {
  va_list args;
  va_start(args, format);
  int n = vsnprintf(output + *length, capacity - *length, format, args);
  va_end(args);
  if (n < 0 || (size_t)n >= capacity - *length)
    abort();
  *length += (size_t)n;
}
static void quoted(char* output, size_t capacity, size_t* length, const char* value) {
  if (!value) {
    add(output, capacity, length, "null");
    return;
  }
  add(output, capacity, length, "\"");
  for (const unsigned char* p = (const unsigned char*)value; *p; ++p) {
    if (*p == '"' || *p == '\\')
      add(output, capacity, length, "\\%c", *p);
    else if (*p < 32)
      add(output, capacity, length, "\\u%04x", *p);
    else
      add(output, capacity, length, "%c", *p);
  }
  add(output, capacity, length, "\"");
}
static int identify(State* s, OuiElement* node) {
  if (!node)
    return -1;
  for (int i = 0; i < 6; ++i) {
    uint32_t same = 0;
    CHECK(oui_element_is_same_node_v1(node, s->nodes[i], &same));
    if (same)
      return i;
  }
  abort();
}
static const char* active(State* s) {
  OuiElement* node = NULL;
  CHECK(oui_document_focused_element_v1(s->document, &node));
  int index = node ? identify(s, node) : 0;
  if (node)
    CHECK(oui_element_destroy(node));
  return labels[index];
}
static void snapshot(State* s, char* output, size_t capacity, size_t* length) {
  add(output, capacity, length, "{\"active\":");
  quoted(output, capacity, length, active(s));
  add(output, capacity, length, ",\"controls\":{");
  for (int i = 3; i < 6; ++i) {
    uint32_t flags = 0, connected = 0, pressed = 0;
    CHECK(oui_element_get_control_flags(s->nodes[i], &flags));
    CHECK(oui_element_is_connected_v1(s->nodes[i], &connected));
    CHECK(oui_element_is_active_v1(s->nodes[i], &pressed));
    if (i != 3)
      add(output, capacity, length, ",");
    add(output, capacity, length,
        "\"%s\":{\"checked\":%s,\"indeterminate\":%s,\"activePseudo\":%s,\"connected\":%s}",
        labels[i], boolean(flags & OUI_CONTROL_CHECKED), boolean(flags & OUI_CONTROL_INDETERMINATE),
        boolean(pressed), boolean(connected));
  }
  add(output, capacity, length, "}}");
}
static const char* event_name(unsigned kind) {
  switch (kind) {
    case OUI_EVENT_KEY_DOWN:
      return "keydown";
    case OUI_EVENT_KEY_UP:
      return "keyup";
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
static unsigned native_modifiers(int bits) {
  return ((bits & 1) ? OUI_MODIFIER_ALT : 0) | ((bits & 2) ? OUI_MODIFIER_CONTROL : 0) |
         ((bits & 4) ? OUI_MODIFIER_META : 0) | ((bits & 8) ? OUI_MODIFIER_SHIFT : 0);
}
static unsigned cdp_modifiers(unsigned bits) {
  return ((bits & OUI_MODIFIER_ALT) ? 1 : 0) | ((bits & OUI_MODIFIER_CONTROL) ? 2 : 0) |
         ((bits & OUI_MODIFIER_META) ? 4 : 0) | ((bits & OUI_MODIFIER_SHIFT) ? 8 : 0);
}
static void apply(State* s, const KeyboardOperation* op) {
  OuiElement* node = op->target >= 0 ? s->nodes[op->target] : NULL;
  switch (op->kind) {
    case KEY_ATTRIBUTE:
      CHECK(oui_element_set_attribute(node, text(op->name), text(op->value)));
      break;
    case KEY_CHECKED:
      CHECK(oui_element_set_checked(node, (uint8_t)op->checked));
      break;
    case KEY_FOCUS:
      CHECK(oui_element_focus(node));
      break;
    case KEY_DETACH:
      CHECK(oui_element_detach(node));
      break;
    case KEY_STYLE: {
      OuiStyleValue value;
      OuiStyleProperty property =
          !strcmp(op->name, "display") ? OUI_STYLE_PROPERTY_DISPLAY : OUI_STYLE_PROPERTY_VISIBILITY;
      CHECK(oui_style_value_parse(property, text(op->value), &value));
      CHECK(oui_element_set_property(node, property, &value));
      if (value.tag == OUI_STYLE_VALUE_COMPOUND)
        CHECK(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound));
      break;
    }
    case KEY_DISPATCH: {
      OuiEvent event;
      memset(&event, 0, sizeof(event));
      event.struct_size = sizeof(event);
      event.abi_version = OUI_ABI_VERSION;
      event.event_type = op->down ? OUI_EVENT_KEY_DOWN : OUI_EVENT_KEY_UP;
      event.key_code = op->code;
      event.text = text(op->name);
      event.modifiers = native_modifiers(op->modifiers);
      const char* commit = op->down && op->code == 32   ? " "
                           : op->down && op->code == 13 ? "\r"
                                                        : "";
      CHECK(oui_document_dispatch_key_input_v1(s->document, &event, text(commit)));
      break;
    }
    default:
      abort();
  }
}
static void record(OuiEvent* event, void* data) {
  Registration* registration = (Registration*)data;
  State* s = registration->state;
  int target = identify(s, event->target);
  if (target != registration->label)
    return;
  OuiEventPropertiesV1 props = {sizeof(props), OUI_ABI_VERSION, 0, 0};
  CHECK(oui_event_properties_v1(event, &props));
  OuiElement* related = NULL;
  if (event->event_type == OUI_EVENT_FOCUS || event->event_type == OUI_EVENT_BLUR ||
      event->event_type == OUI_EVENT_FOCUS_IN || event->event_type == OUI_EVENT_FOCUS_OUT) {
    OuiFocusEventInfoV1 info = {sizeof(info), OUI_ABI_VERSION, 0, 0, NULL};
    CHECK(oui_event_focus_info_v1(event, &info));
    related = info.related_target;
  }
  int relation = identify(s, related);
  char state[4096] = {0};
  size_t length = 0;
  snapshot(s, state, sizeof(state), &length);
  if (s->row_count++)
    add(s->rows, sizeof(s->rows), &s->row_length, ",");
  add(s->rows, sizeof(s->rows), &s->row_length,
      "{\"type\":\"%s\",\"target\":\"%s\",\"related\":", event_name(event->event_type),
      labels[target]);
  quoted(s->rows, sizeof(s->rows), &s->row_length, relation < 0 ? NULL : labels[relation]);
  add(s->rows, sizeof(s->rows), &s->row_length,
      ",\"bubbles\":%s,\"cancelable\":%s,\"key\":", boolean(props.bubbles),
      boolean(props.cancelable));
  int keyboard = event->event_type == OUI_EVENT_KEY_DOWN || event->event_type == OUI_EVENT_KEY_UP;
  if (keyboard) {
    char key[128];
    if (event->text.length >= sizeof(key))
      abort();
    memcpy(key, event->text.data, event->text.length);
    key[event->text.length] = 0;
    quoted(s->rows, sizeof(s->rows), &s->row_length, key);
  } else
    quoted(s->rows, sizeof(s->rows), &s->row_length, NULL);
  add(s->rows, sizeof(s->rows), &s->row_length, ",\"keyCode\":%d,\"modifiers\":%u,%.*s",
      keyboard ? event->key_code : 0, cdp_modifiers(event->modifiers), (int)length - 1, state + 1);
  if (related)
    CHECK(oui_element_destroy(related));
  const KeyboardCase* case_ = s->case_;
  if (!s->handled && case_->handler_event == event->event_type && case_->handler_target == target) {
    s->handled = 1;
    if (case_->prevent)
      event->flags |= OUI_EVENT_DEFAULT_PREVENTED;
    for (size_t i = 0; i < case_->handler_count; ++i)
      apply(s, &case_->handler_ops[i]);
  }
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
static void observation(State* s, char* output, size_t capacity, size_t* length) {
  char state[4096] = {0};
  size_t count = 0;
  snapshot(s, state, sizeof(state), &count);
  add(output, capacity, length, "{\"rows\":[%s],\"state\":%s,\"errors\":[]}", s->rows, state);
}
static void make(State* s, const KeyboardCase* case_) {
  s->case_ = case_;
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {800, 600, 800, 600, 1, OUI_VIEWPORT_LOGICAL, 0}};
  CHECK(oui_document_create(&config, &s->document));
  CHECK(oui_document_root(s->document, &s->nodes[0]));
  for (int i = 1; i < 6; ++i) {
    CHECK(oui_element_create(s->document,
                             i < 3                               ? OUI_ELEMENT_FORM
                             : !strcmp(case_->control, "button") ? OUI_ELEMENT_BUTTON
                                                                 : OUI_ELEMENT_INPUT,
                             &s->nodes[i]));
    CHECK(oui_element_set_attribute(s->nodes[i], text("id"), text(labels[i])));
    if (i >= 3) {
      CHECK(oui_element_set_attribute(s->nodes[i], text("type"), text(case_->control)));
      if (strcmp(case_->control, "button"))
        CHECK(oui_element_set_attribute(s->nodes[i], text("name"), text("g")));
    }
    CHECK(oui_element_append_child(s->nodes[0], s->nodes[i]));
  }
  const OuiEventType types[] = {OUI_EVENT_KEY_DOWN, OUI_EVENT_KEY_UP, OUI_EVENT_CLICK,
                                OUI_EVENT_INPUT,    OUI_EVENT_CHANGE, OUI_EVENT_FOCUS,
                                OUI_EVENT_FOCUS_IN, OUI_EVENT_BLUR,   OUI_EVENT_FOCUS_OUT};
  for (int i = 0; i < 6; ++i) {
    if (i == 1 || i == 2)
      continue;
    for (size_t j = 0; j < sizeof(types) / sizeof(types[0]); ++j) {
      size_t n = s->listener_count++;
      s->registrations[n].state = s;
      s->registrations[n].label = i;
      CHECK(oui_element_add_event_listener(s->nodes[i], types[j], 0, record, &s->registrations[n],
                                           &s->listeners[n]));
    }
  }
  if (strcmp(case_->control, "button"))
    CHECK(oui_element_set_checked(s->nodes[3], 1));
  CHECK(oui_element_focus(s->nodes[3]));
  for (size_t i = 0; i < case_->pre_count; ++i)
    apply(s, &case_->pre[i]);
  barrier(s);
  s->row_count = s->row_length = 0;
  s->rows[0] = 0;
  s->handled = 0;
}
int main(void) {
  puts("[");
  for (size_t n = 0; n < sizeof(keyboard_cases) / sizeof(keyboard_cases[0]); ++n) {
    State s;
    memset(&s, 0, sizeof(s));
    const KeyboardCase* case_ = &keyboard_cases[n];
    make(&s, case_);
    if (n)
      puts(",");
    printf("{\"scenario\":\"%s\",\"kind\":\"%s\",\"initial\":", case_->name, case_->control);
    char initial[4096] = {0};
    size_t length = 0;
    snapshot(&s, initial, sizeof(initial), &length);
    printf("%s,\"steps\":[", initial);
    for (size_t i = 0; i < case_->step_count; ++i) {
      apply(&s, &case_->steps[i]);
      char synchronous[135168] = {0};
      length = 0;
      observation(&s, synchronous, sizeof(synchronous), &length);
      barrier(&s);
      char observed[135168] = {0};
      length = 0;
      observation(&s, observed, sizeof(observed), &length);
      printf("%s{\"operation\":%s,\"synchronous\":%s,\"observed\":%s}", i ? "," : "",
             case_->steps[i].json, synchronous, observed);
    }
    printf("]}");
    for (size_t i = 0; i < s.listener_count; ++i)
      CHECK(oui_listener_destroy(s.listeners[i]));
    for (int i = 5; i >= 0; --i)
      CHECK(oui_element_destroy(s.nodes[i]));
    CHECK(oui_document_destroy(s.document));
  }
  puts("\n]");
  return 0;
}
