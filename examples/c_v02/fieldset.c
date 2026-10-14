/* Real C/C++ native consumers: group disabling, focus callbacks, and retained moves. */
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "openui.h"
#define CHECK(call)                                            \
  do {                                                         \
    OuiStatus status_ = (call);                                \
    if (status_ != OUI_OK) {                                   \
      fprintf(stderr, "%s failed: %d\n", #call, (int)status_); \
      exit(1);                                                 \
    }                                                          \
  } while (0)
static const char* ids[] = {"f",      "l1", "l2",         "normal", "nf", "first",
                            "second", "a",  "innerfirst", "inner",  "b",  "nl"};
static const int fields[] = {5, 6, 7, 8, 9, 10};
typedef struct State {
  OuiDocument* document;
  OuiElement* root;
  OuiElement* elements[12];
  char rows[131072];
  size_t length, count;
  int setup, mutation;
} State;
typedef struct Variant {
  const char* name;
  int focus;
} Variant;
static const Variant variants[] = {
    {"group-normal", 7},          {"group-first-legend", 5},  {"group-second-legend", 6},
    {"group-nested-first", 8},    {"group-nested-normal", 9}, {"inner-first-legend", 8},
    {"inner-normal", 9},          {"disable-enable", 7},      {"disable-twice", 7},
    {"false-attribute", 7},       {"ordinary-container", 7},  {"blur-reenable", 7},
    {"blur-redirect", 7},         {"edited-disable", 7},      {"legend-reorder", 5},
    {"move-out-before-fixup", 7}, {"remove-fieldset", 7},     {"both-groups", 8}};
static const Variant neighbors[] = {
    {"append-control-same-parent", 7},  {"append-focused-subtree", 7},
    {"append-fieldset-same-parent", 7}, {"append-control-out", 7},
    {"insert-self-before", 7},          {"append-unrelated", 7},
    {"insert-legend-before-first", 5},  {"remove-first-legend", 5},
    {"remove-other-legend", 5},         {"first-hidden-legend", 5},
    {"second-hidden-first", 6},         {"readonly-group", 7},
    {"detached-group-state", 7},        {"move-disabled-legend-out", 5}};
static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}
static const char* boolean(int value) {
  return value ? "true" : "false";
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
static void quoted(char* out, size_t capacity, size_t* length, const char* data, size_t count) {
  add(out, capacity, length, "\"");
  for (size_t i = 0; i < count; ++i) {
    unsigned char ch = (unsigned char)data[i];
    if (ch == '"' || ch == '\\')
      add(out, capacity, length, "\\%c", ch);
    else if (ch < 0x20)
      add(out, capacity, length, "\\u%04x", ch);
    else
      add(out, capacity, length, "%c", ch);
  }
  add(out, capacity, length, "\"");
}
static int has_attribute(OuiElement* element, const char* name) {
  OuiBuffer* value = NULL;
  CHECK(oui_element_get_attribute_v1(element, text(name), &value));
  if (!value)
    return 0;
  CHECK(oui_buffer_destroy(value));
  return 1;
}
static const char* identify(State* state, OuiElement* element) {
  if (!element)
    return NULL;
  for (int i = 0; i < 12; ++i) {
    uint32_t same = 0;
    CHECK(oui_element_is_same_node_v1(element, state->elements[i], &same));
    if (same)
      return ids[i];
  }
  uint32_t same = 0;
  CHECK(oui_element_is_same_node_v1(element, state->root, &same));
  if (same)
    return "body";
  abort();
}
static const char* active(State* state) {
  OuiElement* element = NULL;
  CHECK(oui_document_focused_element_v1(state->document, &element));
  if (!element)
    return "body";
  const char* name = identify(state, element);
  CHECK(oui_element_destroy(element));
  return name;
}
static void control(State* state, int index, char* out, size_t capacity, size_t* length) {
  OuiElement* e = state->elements[index];
  char value[128];
  size_t bytes = 0, start = 0, end = 0;
  uint32_t direction = 99, connected = 0, effective = 0, flags = 0;
  CHECK(oui_element_copy_control_value(e, (uint8_t*)value, sizeof(value), &bytes));
  CHECK(oui_element_get_selection(e, &start, &end));
  CHECK(oui_element_get_selection_direction_v1(e, &direction));
  CHECK(oui_element_is_connected_v1(e, &connected));
  CHECK(oui_element_is_effectively_disabled_v1(e, &effective));
  CHECK(oui_element_get_control_flags(e, &flags));
  const char* name = direction == OUI_SELECTION_DIRECTION_NONE       ? "none"
                     : direction == OUI_SELECTION_DIRECTION_FORWARD  ? "forward"
                     : direction == OUI_SELECTION_DIRECTION_BACKWARD ? "backward"
                                                                     : NULL;
  if (!name)
    abort();
  add(out, capacity, length, "\"%s\":{\"value\":", ids[index]);
  quoted(out, capacity, length, value, bytes);
  add(out, capacity, length,
      ",\"start\":%zu,\"end\":%zu,\"direction\":\"%s\",\"connected\":%s,\"ownDisabled\":%s,"
      "\"disabledProperty\":%s,\"effectivelyDisabled\":%s,\"readonly\":%s}",
      start, end, name, boolean(connected), boolean(has_attribute(e, "disabled")),
      boolean(flags & OUI_CONTROL_DISABLED), boolean(effective),
      boolean(has_attribute(e, "readonly")));
}
static void snapshot(State* state, char* out, size_t capacity, size_t* length) {
  add(out, capacity, length, "\"controls\":{");
  for (size_t i = 0; i < 6; ++i) {
    if (i)
      add(out, capacity, length, ",");
    control(state, fields[i], out, capacity, length);
  }
  add(out, capacity, length, "},\"active\":\"%s\",\"groupDisabled\":%s,\"innerGroupDisabled\":%s",
      active(state), boolean(has_attribute(state->elements[0], "disabled")),
      boolean(has_attribute(state->elements[4], "disabled")));
}
static void barrier(State* state) {
  for (int i = 0; i < 64; ++i) {
    uint32_t pending = 0;
    CHECK(oui_document_dispatch_pending_events_v1(state->document, &pending));
    if (!pending)
      return;
  }
  abort();
}
static void record(OuiEvent* event, void* data) {
  State* state = (State*)data;
  if (state->setup)
    return;
  const char* type = NULL;
  switch (event->event_type) {
    case OUI_EVENT_CHANGE:
      type = "change";
      break;
    case OUI_EVENT_BLUR:
      type = "blur";
      break;
    case OUI_EVENT_FOCUS_OUT:
      type = "focusout";
      break;
    case OUI_EVENT_FOCUS:
      type = "focus";
      break;
    case OUI_EVENT_FOCUS_IN:
      type = "focusin";
      break;
    default:
      abort();
  }
  OuiEventPropertiesV1 properties = {sizeof(properties), OUI_ABI_VERSION, 0, 0};
  CHECK(oui_event_properties_v1(event, &properties));
  OuiFocusEventInfoV1 focus = {sizeof(focus), OUI_ABI_VERSION, 0, 0, NULL};
  if (event->event_type != OUI_EVENT_CHANGE)
    CHECK(oui_event_focus_info_v1(event, &focus));
  const char* relation = identify(state, focus.related_target);
  if (state->count++)
    add(state->rows, sizeof(state->rows), &state->length, ",");
  add(state->rows, sizeof(state->rows), &state->length,
      "{\"type\":\"%s\",\"target\":\"%s\",\"related\":", type, identify(state, event->target));
  if (relation)
    quoted(state->rows, sizeof(state->rows), &state->length, relation, strlen(relation));
  else
    add(state->rows, sizeof(state->rows), &state->length, "null");
  add(state->rows, sizeof(state->rows), &state->length, ",\"bubbles\":%s,\"cancelable\":%s,",
      boolean(properties.bubbles), boolean(properties.cancelable));
  snapshot(state, state->rows, sizeof(state->rows), &state->length);
  add(state->rows, sizeof(state->rows), &state->length, "}");
  if (focus.related_target)
    CHECK(oui_element_destroy(focus.related_target));
}
static void mutate(OuiEvent* event, void* data) {
  State* s = (State*)data;
  (void)event;
  if (s->mutation == 11)
    CHECK(oui_element_remove_attribute(s->elements[0], text("disabled")));
  else if (s->mutation == 12)
    CHECK(oui_element_focus(s->elements[10]));
  else
    abort();
}
static void disable(State* s, int id) {
  CHECK(oui_element_set_attribute(s->elements[id], text("disabled"), text("")));
}
static void make(State* s, int tag) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {800, 600, 800, 600, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
  CHECK(oui_document_create(&config, &s->document));
  CHECK(oui_document_root(s->document, &s->root));
  for (int i = 0; i < 12; ++i) {
    OuiElementTag kind = i == 0 || i == 4              ? OUI_ELEMENT_FIELDSET
                         : i == 1 || i == 2 || i == 11 ? OUI_ELEMENT_LEGEND
                         : i == 3                      ? OUI_ELEMENT_DIV
                         : tag                         ? OUI_ELEMENT_TEXTAREA
                                                       : OUI_ELEMENT_INPUT;
    CHECK(oui_element_create(s->document, kind, &s->elements[i]));
    if (i != 11)
      CHECK(oui_element_set_attribute(s->elements[i], text("id"), text(ids[i])));
  }
  CHECK(oui_element_append_child(s->root, s->elements[0]));
  CHECK(oui_element_append_child(s->root, s->elements[10]));
  const int pairs[][2] = {{0, 1}, {1, 5}, {0, 2},  {2, 6},  {0, 3},
                          {3, 7}, {0, 4}, {4, 11}, {11, 8}, {4, 9}};
  for (size_t i = 0; i < 10; ++i)
    CHECK(oui_element_append_child(s->elements[pairs[i][0]], s->elements[pairs[i][1]]));
  for (size_t i = 0; i < 6; ++i) {
    OuiElement* e = s->elements[fields[i]];
    CHECK(oui_element_set_control_value(e, text("abcdef")));
    CHECK(oui_element_set_selection_range_v1(e, 2, 5, OUI_SELECTION_DIRECTION_FORWARD));
  }
}
int main(int argc, char** argv) {
  int adjacent = argc > 1 && strcmp(argv[1], "neighbors") == 0;
  if (argc > 1 && strcmp(argv[1], "focus") && strcmp(argv[1], "neighbors"))
    abort();
  const Variant* cases = adjacent ? neighbors : variants;
  size_t count = adjacent ? 14 : 18, scenarios = 0;
  printf("[");
  for (int tag = 0; tag < 2; ++tag)
    for (size_t variant = 0; variant < count; ++variant) {
      State s;
      memset(&s, 0, sizeof(s));
      s.setup = 1;
      make(&s, tag);
      OuiListener* listeners[31];
      size_t listeners_count = 0;
      const OuiEventType types[] = {OUI_EVENT_CHANGE, OUI_EVENT_BLUR, OUI_EVENT_FOCUS_OUT,
                                    OUI_EVENT_FOCUS, OUI_EVENT_FOCUS_IN};
      for (size_t i = 0; i < 6; ++i)
        for (size_t k = 0; k < 5; ++k)
          CHECK(oui_element_add_event_listener(s.elements[fields[i]], types[k], 0, record, &s,
                                               &listeners[listeners_count++]));
      barrier(&s);
      if (!adjacent && variant == 14)
        disable(&s, 0);
      if (!adjacent && (variant == 11 || variant == 12)) {
        s.mutation = (int)variant;
        CHECK(oui_element_add_event_listener(s.elements[7], OUI_EVENT_BLUR, 0, mutate, &s,
                                             &listeners[listeners_count++]));
      }
      if (adjacent && (variant == 6 || variant == 7 || variant == 8 || variant == 13))
        disable(&s, 0);
      if (adjacent && (variant == 9 || variant == 10)) {
        OuiStyleValue display;
        CHECK(oui_style_value_parse(OUI_STYLE_PROPERTY_DISPLAY, text("none"), &display));
        CHECK(oui_element_set_property(s.elements[1], OUI_STYLE_PROPERTY_DISPLAY, &display));
      }
      if (adjacent && variant == 11)
        CHECK(oui_element_set_attribute(s.elements[7], text("readonly"), text("")));
      OuiStatus focused = oui_element_focus(s.elements[cases[variant].focus]);
      if (adjacent && variant == 9) {
        if (focused != OUI_ERROR_INVALID_STATE)
          abort();
      } else
        CHECK(focused);
      barrier(&s);
      if (!adjacent && variant == 13)
        CHECK(oui_document_dispatch_text_input_v1(s.document, text("X")));
      char initial[16384] = {0};
      size_t length = 0;
      snapshot(&s, initial, sizeof(initial), &length);
      s.setup = 0;
      s.count = s.length = 0;
      s.rows[0] = 0;
      if (!adjacent) {
        switch (variant) {
          case 5:
          case 6:
            disable(&s, 4);
            break;
          case 7:
            disable(&s, 0);
            CHECK(oui_element_remove_attribute(s.elements[0], text("disabled")));
            break;
          case 8:
            disable(&s, 0);
            disable(&s, 0);
            break;
          case 9:
            CHECK(oui_element_set_attribute(s.elements[0], text("disabled"), text("false")));
            break;
          case 10:
            disable(&s, 3);
            break;
          case 14:
            CHECK(oui_element_append_child(s.elements[0], s.elements[1]));
            break;
          case 15:
            disable(&s, 0);
            CHECK(oui_element_append_child(s.root, s.elements[7]));
            break;
          case 16:
            CHECK(oui_element_detach(s.elements[0]));
            break;
          case 17:
            disable(&s, 4);
            disable(&s, 0);
            break;
          default:
            disable(&s, 0);
            break;
        }
      } else {
        switch (variant) {
          case 0:
            CHECK(oui_element_append_child(s.elements[3], s.elements[7]));
            break;
          case 1:
            CHECK(oui_element_append_child(s.elements[0], s.elements[3]));
            break;
          case 2:
            CHECK(oui_element_append_child(s.root, s.elements[0]));
            break;
          case 3:
            CHECK(oui_element_append_child(s.root, s.elements[7]));
            break;
          case 4:
            CHECK(oui_element_insert_before(s.elements[3], s.elements[7], s.elements[7]));
            break;
          case 5:
            CHECK(oui_element_append_child(s.elements[0], s.elements[2]));
            break;
          case 6:
            CHECK(oui_element_insert_before(s.elements[0], s.elements[2], s.elements[1]));
            break;
          case 7:
            CHECK(oui_element_detach(s.elements[1]));
            break;
          case 8:
            CHECK(oui_element_detach(s.elements[2]));
            break;
          case 12:
            CHECK(oui_element_detach(s.elements[0]));
            disable(&s, 0);
            break;
          case 13:
            CHECK(oui_element_append_child(s.root, s.elements[1]));
            break;
          default:
            disable(&s, 0);
            break;
        }
      }
      char synchronous[65536] = {0};
      length = 0;
      add(synchronous, sizeof(synchronous), &length, "\"rows\":[%s],\"state\":{", s.rows);
      snapshot(&s, synchronous, sizeof(synchronous), &length);
      add(synchronous, sizeof(synchronous), &length, "}");
      barrier(&s);
      char observed[16384] = {0};
      length = 0;
      snapshot(&s, observed, sizeof(observed), &length);
      if (scenarios++)
        printf(",");
      printf(
          "{\"scenario\":\"%s-%s\",\"control\":\"%s\",\"variant\":\"%s\",\"focus_id\":\"%s\","
          "\"initial\":{%s},\"synchronous\":{%s},\"observed\":{\"rows\":[%s],\"state\":{%s}}}",
          tag ? "textarea" : "input", cases[variant].name, tag ? "textarea" : "input",
          cases[variant].name, ids[cases[variant].focus], initial, synchronous, s.rows, observed);
      for (size_t i = 0; i < listeners_count; ++i)
        CHECK(oui_listener_destroy(listeners[i]));
      for (size_t i = 0; i < 12; ++i)
        CHECK(oui_element_destroy(s.elements[i]));
      CHECK(oui_element_destroy(s.root));
      CHECK(oui_document_destroy(s.document));
    }
  printf("]\n");
  return 0;
}
