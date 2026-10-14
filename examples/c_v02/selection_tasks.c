/* Native C/C++ selection consumer. Snapshot values come from public APIs.
 * ASCII scenarios share immutable Chromium observations v3062/v3084/v3111. */
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
  uint32_t mutation_type;
  int mutation_target_b;
  int detach;
  size_t start;
  size_t end;
  uint32_t direction;
  int fired;
  size_t roots;
  size_t connected_rows;
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
  add(output, capacity, length, ",\"start\":%zu,\"end\":%zu,\"direction\":\"%s\",\"connected\":%s}",
      start, end, name, connected(state, id) ? "true" : "false");
}
static void snapshot(State* state, char* output, size_t capacity, size_t* length) {
  control(state, state->a, "a", output, capacity, length);
  add(output, capacity, length, ",");
  control(state, state->b, "b", output, capacity, length);
  add(output, capacity, length, ",\"active\":\"%s\"", active(state));
}
static void selection(OuiElement* element, size_t start, size_t end, int backward) {
  CHECK(oui_element_set_selection_range_v1(
      element, start, end,
      backward ? OUI_SELECTION_DIRECTION_BACKWARD : OUI_SELECTION_DIRECTION_FORWARD));
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
static void prevent(OuiEvent* event, void* data) {
  (void)data;
  event->flags |= OUI_EVENT_DEFAULT_PREVENTED;
}
static void record(OuiEvent* event, void* data) {
  State* state = (State*)data;
  if (state->setup)
    return;
  if (event->flags & OUI_EVENT_DEFAULT_PREVENTED)
    abort();
  OuiEventPropertiesV1 properties = {sizeof(properties), OUI_ABI_VERSION, 0, 0};
  CHECK(oui_event_properties_v1(event, &properties));
  uint32_t target_a = 0, target_b = 0;
  CHECK(oui_element_is_same_node_v1(event->target, state->a, &target_a));
  CHECK(oui_element_is_same_node_v1(event->target, state->b, &target_b));
  const char* target = target_a ? "a" : target_b ? "b" : NULL;
  if (!target)
    abort();
  if (state->count++)
    add(state->rows, sizeof(state->rows), &state->length, ",");
  add(state->rows, sizeof(state->rows), &state->length, "{\"type\":\"%s\",\"target\":\"%s\",",
      event->event_type == OUI_EVENT_SELECT ? "select" : "selectionchange", target);
  snapshot(state, state->rows, sizeof(state->rows), &state->length);
  add(state->rows, sizeof(state->rows), &state->length, ",\"bubbles\":%s,\"cancelable\":%s}",
      properties.bubbles ? "true" : "false", properties.cancelable ? "true" : "false");
  state->connected_rows += (size_t)connected(state, target);
}
static void root_event(OuiEvent* event, void* data) {
  State* state = (State*)data;
  if (!state->setup) {
    if (event->phase != 3 || event->current_target != state->root)
      abort();
    ++state->roots;
  }
}
static void mutate(OuiEvent* event, void* data) {
  State* state = (State*)data;
  if (state->setup || state->fired || event->event_type != state->mutation_type)
    return;
  state->fired = 1;
  OuiElement* target = state->mutation_target_b ? state->b : state->a;
  if (state->detach)
    CHECK(oui_element_detach(target));
  else
    CHECK(oui_element_set_selection_range_v1(target, state->start, state->end, state->direction));
  /* Recursive pumping must not recursively deliver any new callback. */
  size_t before = state->count;
  uint32_t pending = 0;
  CHECK(oui_document_dispatch_pending_events_v1(state->document, &pending));
  if (state->count != before)
    abort();
}

int main(void) {
  const char* variants[] = {"cross-a-b",
                            "cross-b-a",
                            "reenter-selectionchange-self",
                            "reenter-select-self",
                            "reenter-selectionchange-other-pending",
                            "reenter-select-other",
                            "detach-on-selectionchange",
                            "direction-only",
                            "select-other-pending",
                            "select-self-two-pending",
                            "selectionchange-other-delivered",
                            "detach-on-select-two-pending",
                            "value-changed-same-length-caret-end",
                            "value-changed-same-length-range",
                            "value-identical-forward-range",
                            "value-identical-backward-range",
                            "value-shorter-backward-range",
                            "range-identical-forward-preserve",
                            "range-identical-backward-preserve",
                            "range-changed-same-length-preserve",
                            "range-changed-caret-tail-preserve",
                            "range-shorter-backward-end",
                            "selection-none-from-backward",
                            "value-change-then-restore-selection"};
  printf("[");
  size_t scenarios = 0;
  for (int tag = 0; tag < 2; ++tag)
    for (int variant = 0; variant < 24; ++variant) {
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
      OuiListener* listeners[12];
      size_t listener_count = 0;
      const OuiEventType kinds[] = {OUI_EVENT_SELECT, OUI_EVENT_SELECTION_CHANGE};
      OuiElement* controls[] = {state.a, state.b};
      for (size_t node = 0; node < 2; ++node)
        for (size_t kind = 0; kind < 2; ++kind) {
          CHECK(oui_element_add_event_listener(controls[node], kinds[kind], 0, prevent, &state,
                                               &listeners[listener_count++]));
          CHECK(oui_element_add_event_listener(controls[node], kinds[kind], 0, record, &state,
                                               &listeners[listener_count++]));
        }
      for (size_t kind = 0; kind < 2; ++kind) {
        CHECK(oui_element_add_event_listener(state.root, kinds[kind], 0, root_event, &state,
                                             &listeners[listener_count++]));
        CHECK(oui_element_add_event_listener(state.a, kinds[kind], 0, mutate, &state,
                                             &listeners[listener_count++]));
      }
      CHECK(oui_element_set_control_value(state.a, text("abcdef")));
      CHECK(oui_element_set_control_value(state.b, text("abcdef")));
      CHECK(oui_element_focus(state.a));
      size_t start = (variant == 12 || variant == 20) ? 6 : variant == 21 ? 4 : 1;
      size_t end = (variant == 12 || variant == 20 || variant == 21) ? 6 : 4;
      int backward =
          variant == 15 || variant == 16 || variant == 18 || variant == 21 || variant == 22;
      selection(state.a, start, end, backward);
      selection(state.b, 1, 4, 0);
      barrier(&state);
      state.setup = 0;
      switch (variant) {
        case 2:
        case 4:
        case 6:
        case 10:
          state.mutation_type = OUI_EVENT_SELECTION_CHANGE;
          break;
        case 3:
        case 5:
        case 8:
        case 9:
        case 11:
          state.mutation_type = OUI_EVENT_SELECT;
          break;
        default:
          break;
      }
      state.mutation_target_b = variant == 4 || variant == 5 || variant == 8 || variant == 10;
      state.detach = variant == 6 || variant == 11;
      state.start = variant == 4 || variant == 8 || variant == 10 ? 2 : variant == 9 ? 1 : 0;
      state.end = variant == 4 || variant == 8 || variant == 9 || variant == 10 ? 5 : 3;
      state.direction =
          variant == 5 ? OUI_SELECTION_DIRECTION_BACKWARD : OUI_SELECTION_DIRECTION_FORWARD;
      switch (variant) {
        case 0:
          selection(state.a, 2, 5, 0);
          selection(state.b, 0, 3, 1);
          break;
        case 1:
          selection(state.b, 0, 3, 1);
          selection(state.a, 2, 5, 0);
          break;
        case 2:
        case 3:
        case 5:
        case 6:
          selection(state.a, 2, 5, 0);
          break;
        case 4:
        case 8:
          selection(state.a, 2, 5, 0);
          selection(state.b, 0, 3, 1);
          break;
        case 7:
          selection(state.a, 1, 4, 1);
          break;
        case 9:
        case 11:
          selection(state.a, 2, 5, 0);
          selection(state.a, 0, 3, 0);
          break;
        case 10:
          selection(state.b, 0, 3, 1);
          selection(state.a, 2, 5, 0);
          break;
        case 12:
        case 13:
          CHECK(oui_element_set_control_value(state.a, text("ghijkl")));
          break;
        case 14:
        case 15:
          CHECK(oui_element_set_control_value(state.a, text("abcdef")));
          break;
        case 16:
          CHECK(oui_element_set_control_value(state.a, text("xyz")));
          break;
        case 17:
        case 18:
          CHECK(oui_element_replace_control_range_v1(state.a, text("bcd"), 1, 4,
                                                     OUI_RANGE_SELECTION_PRESERVE));
          break;
        case 19:
        case 20:
          CHECK(oui_element_replace_control_range_v1(state.a, text("ZZ"), 1, 3,
                                                     OUI_RANGE_SELECTION_PRESERVE));
          break;
        case 21:
          CHECK(oui_element_replace_control_range_v1(state.a, text("x"), 0, 6,
                                                     OUI_RANGE_SELECTION_END));
          break;
        case 22:
          CHECK(oui_element_set_selection_range_v1(state.a, 1, 4, OUI_SELECTION_DIRECTION_NONE));
          break;
        case 23:
          CHECK(oui_element_set_control_value(state.a, text("ghijkl")));
          selection(state.a, 1, 4, 0);
          break;
        default:
          abort();
      }
      if (state.count)
        abort();
      char synchronous[1024] = {0};
      size_t length = 0;
      snapshot(&state, synchronous, sizeof(synchronous), &length);
      barrier(&state);
      /* All notifications on connected targets must reach the root. */
      if (state.roots != state.connected_rows)
        abort();
      char observed[1024] = {0};
      length = 0;
      snapshot(&state, observed, sizeof(observed), &length);
      if (scenarios++)
        printf(",");
      printf(
          "{\"scenario\":\"%s-%s\",\"control\":\"%s\",\"variant\":\"%s\","
          "\"synchronous\":{\"rows\":"
          "[],\"state\":{%s}},\"observed\":{\"rows\":[%s],\"state\":{%s}}}",
          tag ? "textarea" : "input", variants[variant], tag ? "textarea" : "input",
          variants[variant], synchronous, state.rows, observed);
      for (size_t i = 0; i < listener_count; ++i)
        CHECK(oui_listener_destroy(listeners[i]));
      CHECK(oui_element_destroy(state.a));
      CHECK(oui_element_destroy(state.b));
      CHECK(oui_element_destroy(state.root));
      CHECK(oui_document_destroy(state.document));
    }
  const char* modes[] = {"preserve", "select",  "start",  "end",     "readonly",
                         "disabled", "reverse", "beyond", "newline", "direction-backward"};
  for (int tag = 0; tag < 2; ++tag)
    for (int mode = 0; mode < 10; ++mode) {
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
      OuiListener* listeners[4];
      size_t listener_count = 0;
      const OuiEventType kinds[] = {OUI_EVENT_SELECT, OUI_EVENT_SELECTION_CHANGE};
      for (size_t kind = 0; kind < 2; ++kind) {
        CHECK(oui_element_add_event_listener(state.a, kinds[kind], 0, prevent, &state,
                                             &listeners[listener_count++]));
        CHECK(oui_element_add_event_listener(state.a, kinds[kind], 0, record, &state,
                                             &listeners[listener_count++]));
      }
      CHECK(oui_element_set_control_value(state.a, text("abcdef")));
      CHECK(oui_element_set_control_value(state.b, text("abcdef")));
      CHECK(oui_element_focus(state.a));
      selection(state.a, 2, 5, mode == 9);
      selection(state.b, 1, 4, 0);
      if (mode == 4)
        CHECK(oui_element_set_attribute(state.a, text("readonly"), text("")));
      if (mode == 5)
        CHECK(oui_element_set_attribute(state.a, text("disabled"), text("")));
      barrier(&state);
      state.setup = 0;
      size_t start = mode == 6 ? 5 : mode == 7 ? 90 : 1, end = mode == 6 ? 2 : mode == 7 ? 100 : 4;
      const char* replacement = mode == 8 ? "X\nY\rZ" : "XY";
      OuiStatus status = oui_element_replace_control_range_v1(state.a, text(replacement), start,
                                                              end, (uint32_t)(mode < 4 ? mode : 0));
      if (mode != 6 && status != OUI_OK)
        abort();
      if (mode == 6 && status == OUI_OK)
        abort();
      if (state.count)
        abort();
      barrier(&state);
      char observed[1024] = {0};
      size_t length = 0;
      snapshot(&state, observed, sizeof(observed), &length);
      if (scenarios++)
        printf(",");
      printf(
          "{\"scenario\":\"%s-%s\",\"native_status\":%d,\"observed\":{"
          "\"rows\":[%s],\"state\":{%s}}"
          "}",
          tag ? "textarea" : "input", modes[mode], (int)status, state.rows, observed);
      for (size_t i = 0; i < listener_count; ++i)
        CHECK(oui_listener_destroy(listeners[i]));
      CHECK(oui_element_destroy(state.a));
      CHECK(oui_element_destroy(state.b));
      CHECK(oui_element_destroy(state.root));
      CHECK(oui_document_destroy(state.document));
    }
  const char* scalar_modes[] = {"preserve", "select", "start", "end"};
  for (int tag = 0; tag < 2; ++tag)
    for (int mode = 0; mode < 4; ++mode) {
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
      CHECK(oui_element_append_child(state.root, state.a));
      CHECK(oui_element_append_child(state.root, state.b));
      OuiListener* listeners[4];
      size_t listener_count = 0;
      const OuiEventType kinds[] = {OUI_EVENT_SELECT, OUI_EVENT_SELECTION_CHANGE};
      for (size_t kind = 0; kind < 2; ++kind) {
        CHECK(oui_element_add_event_listener(state.a, kinds[kind], 0, prevent, &state,
                                             &listeners[listener_count++]));
        CHECK(oui_element_add_event_listener(state.a, kinds[kind], 0, record, &state,
                                             &listeners[listener_count++]));
      }
      CHECK(oui_element_set_control_value(state.a, text("A🙂éZ")));
      CHECK(oui_element_set_control_value(state.b, text("abcdef")));
      CHECK(oui_element_focus(state.a));
      CHECK(oui_element_set_selection(state.a, 1, 7));
      barrier(&state);
      state.setup = 0;
      CHECK(oui_element_replace_control_range_v1(state.a, text("界"), 1, 5, (uint32_t)mode));
      if (state.count)
        abort();
      barrier(&state);
      char observed[1024] = {0};
      size_t length = 0;
      snapshot(&state, observed, sizeof(observed), &length);
      if (scenarios++)
        printf(",");
      printf(
          "{\"scenario\":\"%s-unicode-scalar-%s\",\"native_status\":0,"
          "\"observed\":{\"rows\":[%s],\"state\":{%s}}}",
          tag ? "textarea" : "input", scalar_modes[mode], state.rows, observed);
      for (size_t i = 0; i < listener_count; ++i)
        CHECK(oui_listener_destroy(listeners[i]));
      CHECK(oui_element_destroy(state.a));
      CHECK(oui_element_destroy(state.b));
      CHECK(oui_element_destroy(state.root));
      CHECK(oui_document_destroy(state.document));
    }
  printf("]\n");
  return 0;
}
