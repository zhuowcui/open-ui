/* Native C/C++ consumer of disabled attributes and real focus transitions.
 * Values, attributes, identity and event properties come from public APIs. */
#include <stdarg.h>
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
typedef struct State {
  OuiDocument* document;
  OuiElement* root;
  OuiElement* a;
  OuiElement* b;
  char rows[65536];
  size_t length;
  size_t count;
  int setup;
  int mutation;
} State;

static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}
static void add(char* output, size_t capacity, size_t* length, const char* format, ...) {
  va_list args;
  va_start(args, format);
  int count = vsnprintf(output + *length, capacity - *length, format, args);
  va_end(args);
  if (count < 0 || (size_t)count >= capacity - *length)
    abort();
  *length += (size_t)count;
}
static void quoted(char* output, size_t capacity, size_t* length, const char* data, size_t count) {
  add(output, capacity, length, "\"");
  for (size_t i = 0; i < count; ++i) {
    unsigned char ch = (unsigned char)data[i];
    if (ch == '"' || ch == '\\')
      add(output, capacity, length, "\\%c", ch);
    else if (ch < 0x20)
      add(output, capacity, length, "\\u%04x", ch);
    else
      add(output, capacity, length, "%c", ch);
  }
  add(output, capacity, length, "\"");
}
static int connected(State* state, const char* id) {
  uint32_t result = 0;
  CHECK(oui_element_is_connected_v1(strcmp(id, "a") == 0 ? state->a : state->b, &result));
  return result != 0;
}
static const char* active(State* state) {
  OuiElement* focused = NULL;
  uint32_t same_a = 0, same_b = 0;
  CHECK(oui_document_focused_element_v1(state->document, &focused));
  if (!focused)
    return "body";
  CHECK(oui_element_is_same_node_v1(focused, state->a, &same_a));
  CHECK(oui_element_is_same_node_v1(focused, state->b, &same_b));
  CHECK(oui_element_destroy(focused));
  if (same_a)
    return "a";
  if (same_b)
    return "b";
  abort();
}
static int has_attribute(OuiElement* element, const char* name) {
  OuiBuffer* value = NULL;
  CHECK(oui_element_get_attribute_v1(element, text(name), &value));
  if (!value)
    return 0;
  CHECK(oui_buffer_destroy(value));
  return 1;
}
static void control(State* state,
                    OuiElement* element,
                    const char* id,
                    char* output,
                    size_t capacity,
                    size_t* length) {
  char value[128];
  size_t bytes = 0, start = 0, end = 0;
  uint32_t direction = 99;
  CHECK(oui_element_copy_control_value(element, (uint8_t*)value, sizeof(value) - 1, &bytes));
  value[bytes] = 0;
  CHECK(oui_element_get_selection(element, &start, &end));
  CHECK(oui_element_get_selection_direction_v1(element, &direction));
  const char* name = direction == OUI_SELECTION_DIRECTION_NONE       ? "none"
                     : direction == OUI_SELECTION_DIRECTION_FORWARD  ? "forward"
                     : direction == OUI_SELECTION_DIRECTION_BACKWARD ? "backward"
                                                                     : NULL;
  if (!name)
    abort();
  add(output, capacity, length, "\"%s\":{\"value\":", id);
  quoted(output, capacity, length, value, bytes);
  add(output, capacity, length,
      ",\"start\":%zu,\"end\":%zu,\"direction\":\"%s\",\"connected\":%s,"
      "\"disabled\":%s,\"readonly\":%s}",
      start, end, name, connected(state, id) ? "true" : "false",
      has_attribute(element, "disabled") ? "true" : "false",
      has_attribute(element, "readonly") ? "true" : "false");
}
static void snapshot(State* state, char* output, size_t capacity, size_t* length) {
  control(state, state->a, "a", output, capacity, length);
  add(output, capacity, length, ",");
  control(state, state->b, "b", output, capacity, length);
  add(output, capacity, length, ",\"active\":\"%s\"", active(state));
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
static const char* related(State* state, OuiElement* element) {
  if (!element)
    return NULL;
  uint32_t a = 0, b = 0;
  CHECK(oui_element_is_same_node_v1(element, state->a, &a));
  CHECK(oui_element_is_same_node_v1(element, state->b, &b));
  if (a)
    return "a";
  if (b)
    return "b";
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
  const char* relation = related(state, focus.related_target);
  uint32_t target_a = 0, target_b = 0;
  CHECK(oui_element_is_same_node_v1(event->target, state->a, &target_a));
  CHECK(oui_element_is_same_node_v1(event->target, state->b, &target_b));
  const char* target = target_a ? "a" : target_b ? "b" : NULL;
  if (!target)
    abort();
  if (state->count++)
    add(state->rows, sizeof(state->rows), &state->length, ",");
  add(state->rows, sizeof(state->rows), &state->length,
      "{\"type\":\"%s\",\"target\":\"%s\",\"related\":", type, target);
  if (relation)
    quoted(state->rows, sizeof(state->rows), &state->length, relation, strlen(relation));
  else
    add(state->rows, sizeof(state->rows), &state->length, "null");
  add(state->rows, sizeof(state->rows), &state->length, ",\"bubbles\":%s,\"cancelable\":%s,",
      properties.bubbles ? "true" : "false", properties.cancelable ? "true" : "false");
  snapshot(state, state->rows, sizeof(state->rows), &state->length);
  add(state->rows, sizeof(state->rows), &state->length, "}");
  if (focus.related_target)
    CHECK(oui_element_destroy(focus.related_target));
}
static void mutate(OuiEvent* event, void* data) {
  State* state = (State*)data;
  (void)event;
  if (state->mutation == 7)
    CHECK(oui_element_remove_attribute(state->a, text("disabled")));
  else if (state->mutation == 8)
    CHECK(oui_element_focus(state->b));
  else if (state->mutation == 9)
    CHECK(oui_element_set_attribute(state->a, text("disabled"), text("")));
  else
    abort();
}
int main(void) {
  const char* variants[] = {"disable-focused",   "disable-unfocused",    "readonly-focused",
                            "disable-uppercase", "disable-false-string", "disable-enable",
                            "disable-twice",     "blur-reenable",        "blur-redirect",
                            "focus-disables",    "edited-disable"};
  size_t scenarios = 0;
  printf("[");
  for (int tag = 0; tag < 2; ++tag)
    for (int variant = 0; variant < 11; ++variant) {
      State state;
      memset(&state, 0, sizeof(state));
      state.setup = 1;
      OuiDocumentConfig config = {
          sizeof(config), OUI_ABI_VERSION, {320, 200, 320, 200, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
      CHECK(oui_document_create(&config, &state.document));
      CHECK(oui_document_root(state.document, &state.root));
      CHECK(oui_element_create(state.document, tag ? OUI_ELEMENT_TEXTAREA : OUI_ELEMENT_INPUT,
                               &state.a));
      CHECK(oui_element_create(state.document, tag ? OUI_ELEMENT_TEXTAREA : OUI_ELEMENT_INPUT,
                               &state.b));
      CHECK(oui_element_set_attribute(state.a, text("id"), text("a")));
      CHECK(oui_element_set_attribute(state.b, text("id"), text("b")));
      CHECK(oui_element_append_child(state.root, state.a));
      CHECK(oui_element_append_child(state.root, state.b));
      OuiListener* listeners[11];
      size_t count = 0;
      const uint32_t kinds[] = {OUI_EVENT_CHANGE, OUI_EVENT_BLUR, OUI_EVENT_FOCUS_OUT,
                                OUI_EVENT_FOCUS, OUI_EVENT_FOCUS_IN};
      for (int element = 0; element < 2; ++element)
        for (size_t i = 0; i < 5; ++i)
          CHECK(oui_element_add_event_listener(element ? state.b : state.a, kinds[i], 0, record,
                                               &state, &listeners[count++]));
      CHECK(oui_element_set_control_value(state.a, text("abcdef")));
      CHECK(oui_element_set_control_value(state.b, text("abcdef")));
      CHECK(oui_element_focus(state.a));
      CHECK(oui_element_set_selection_range_v1(state.a, 2, 5, OUI_SELECTION_DIRECTION_FORWARD));
      CHECK(oui_element_set_selection_range_v1(state.b, 1, 4, OUI_SELECTION_DIRECTION_FORWARD));
      barrier(&state);
      state.mutation = variant;
      if (variant == 7 || variant == 8 || variant == 9) {
        if (variant == 9)
          CHECK(oui_element_focus(state.b));
        CHECK(oui_element_add_event_listener(state.a,
                                             variant == 9 ? OUI_EVENT_FOCUS : OUI_EVENT_BLUR, 0,
                                             mutate, &state, &listeners[count++]));
        barrier(&state);
      }
      state.setup = 0;
      if (variant == 10)
        CHECK(oui_document_dispatch_text_input_v1(state.document, text("X")));
      switch (variant) {
        case 1:
          CHECK(oui_element_set_attribute(state.b, text("disabled"), text("")));
          break;
        case 2:
          CHECK(oui_element_set_attribute(state.a, text("readonly"), text("")));
          break;
        case 3:
          CHECK(oui_element_set_attribute(state.a, text("DiSaBlEd"), text("")));
          break;
        case 4:
          CHECK(oui_element_set_attribute(state.a, text("disabled"), text("false")));
          break;
        case 5:
          CHECK(oui_element_set_attribute(state.a, text("disabled"), text("")));
          CHECK(oui_element_remove_attribute(state.a, text("disabled")));
          break;
        case 6:
          CHECK(oui_element_set_attribute(state.a, text("disabled"), text("")));
          CHECK(oui_element_set_attribute(state.a, text("disabled"), text("")));
          break;
        case 9:
          CHECK(oui_element_focus(state.a));
          break;
        default:
          CHECK(oui_element_set_attribute(state.a, text("disabled"), text("")));
          break;
      }
      char synchronous[16384] = {0};
      size_t length = 0;
      add(synchronous, sizeof(synchronous), &length, "\"rows\":[%s],\"state\":{", state.rows);
      snapshot(&state, synchronous, sizeof(synchronous), &length);
      add(synchronous, sizeof(synchronous), &length, "}");
      barrier(&state);
      char observed[4096] = {0};
      length = 0;
      snapshot(&state, observed, sizeof(observed), &length);
      if (scenarios++)
        printf(",");
      printf(
          "{\"scenario\":\"%s-%s\",\"control\":\"%s\",\"variant\":\"%s\","
          "\"synchronous\":{%s},\"observed\":{\"rows\":[%s],\"state\":{%s}}}",
          tag ? "textarea" : "input", variants[variant], tag ? "textarea" : "input",
          variants[variant], synchronous, state.rows, observed);
      for (size_t i = 0; i < count; ++i)
        CHECK(oui_listener_destroy(listeners[i]));
      CHECK(oui_element_destroy(state.a));
      CHECK(oui_element_destroy(state.b));
      CHECK(oui_element_destroy(state.root));
      CHECK(oui_document_destroy(state.document));
    }
  printf("]\n");
  return 0;
}
