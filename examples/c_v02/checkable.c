/* Native consuming C/C++ app. Every state and callback field comes from the
 * public C ABI; expected browser values and scripts are absent. */
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "openui.h"
#define CHECK(call) do { OuiStatus code_ = (call); if (code_ != OUI_OK) { \
  fprintf(stderr, "%s failed: %d\n", #call, (int)code_); exit(1); } } while (0)
typedef struct State {
  OuiDocument* document;
  OuiElement* root;
  OuiElement* elements[10];
  OuiListener* listeners[80];
  size_t listener_count;
  char rows[131072];
  size_t row_length, row_count;
  int extended, mutation, setup, explicit_dispatch;
} State;
static const char* ids[] = {"fa", "fb", "f", "l1", "l2", "first", "second", "a", "b", "copy"};
static const char* programmatic[] = {"enabled-set", "own-disabled-set", "group-disabled-set", "first-legend-set", "second-legend-set", "disabled-neighbor-uncheck", "group-neighbor-uncheck", "own-disabled-false", "group-disabled-false", "disabled-click", "group-disabled-click", "first-legend-click", "second-legend-click", "independent-forms", "detached-group", "different-names", "empty-names"};
static const char* defaults[] = {"default-clean-set", "default-clean-remove", "assign-false-then-default", "assign-true-then-default-remove", "assign-false-then-default-remove", "same-true-after-attribute", "clone-live-true", "clone-dirty-false", "clone-clean", "click-cancel", "click-callback-false", "click-callback-disable", "click-callback-default", "neighbor-cancel", "neighbor-cancel-disable", "neighbor-cancel-assign", "click-default-remove", "programmatic-no-events"};
static const char* activation[] = {"neighbor-click", "already-checked-click", "neighbor-callback-check-old", "neighbor-callback-check-other", "neighbor-callback-disable", "neighbor-callback-rename", "neighbor-callback-form", "neighbor-callback-detach", "neighbor-callback-default", "neighbor-cancel-detach", "neighbor-cancel-check-other", "neighbor-cancel-rename", "neighbor-cancel-form", "neighbor-cancel-default", "indeterminate-click", "indeterminate-cancel", "indeterminate-callback", "disabled-neighbor-click", "fieldset-disabled-neighbor-click", "default-checked-neighbor-click"};
static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}
static const char* boolean(int value) { return value ? "true" : "false"; }
static void add(char* out, size_t capacity, size_t* length, const char* format, ...) {
  va_list args; va_start(args, format);
  int count = vsnprintf(out + *length, capacity - *length, format, args);
  va_end(args);
  if (count < 0 || (size_t)count >= capacity - *length) abort();
  *length += (size_t)count;
}
static void attr(State* s, int node, const char* name, const char* value) {
  CHECK(oui_element_set_attribute(s->elements[node], text(name), text(value)));
}
static void remove_attr(State* s, int node, const char* name) {
  CHECK(oui_element_remove_attribute(s->elements[node], text(name)));
}
static void checked(State* s, int node, int value) {
  CHECK(oui_element_set_checked(s->elements[node], (uint8_t)value));
}
static void indeterminate(State* s, int node, int value) {
  CHECK(oui_element_set_indeterminate(s->elements[node], (uint8_t)value));
}
static void move_to(State* s, int node, int parent) {
  CHECK(oui_element_append_child(s->elements[parent], s->elements[node]));
}
static void detach(State* s, int node) { CHECK(oui_element_detach(s->elements[node])); }
static int has_attribute(OuiElement* element, const char* name) {
  OuiBuffer* value = NULL;
  CHECK(oui_element_get_attribute_v1(element, text(name), &value));
  if (!value) return 0;
  CHECK(oui_buffer_destroy(value)); return 1;
}
static const char* identify(State* s, OuiElement* node) {
  if (!node) return "body";
  for (int i = 0; i < 10; ++i) {
    if (!s->elements[i]) continue;
    uint32_t same = 0; CHECK(oui_element_is_same_node_v1(node, s->elements[i], &same));
    if (same) return ids[i];
  }
  uint32_t same = 0; CHECK(oui_element_is_same_node_v1(node, s->root, &same));
  if (same) return "body";
  abort();
}
static const char* active(State* s) {
  OuiElement* node = NULL; CHECK(oui_document_focused_element_v1(s->document, &node));
  const char* result = identify(s, node);
  if (node) CHECK(oui_element_destroy(node));
  return result;
}
static void snapshot(State* s, char* out, size_t capacity, size_t* length) {
  add(out, capacity, length, "\"controls\":{");
  for (int i = 5; i < 10; ++i) {
    OuiElement* node = s->elements[i];
    if (!node) continue;
    uint32_t own = 0, effective = 0, connected = 0, flags = 0;
    CHECK(oui_element_is_own_disabled_v1(node, &own));
    CHECK(oui_element_is_effectively_disabled_v1(node, &effective));
    CHECK(oui_element_is_connected_v1(node, &connected));
    CHECK(oui_element_get_control_flags(node, &flags));
    int authored = has_attribute(node, "checked");
    if (i != 5) add(out, capacity, length, ",");
    add(out, capacity, length, "\"%s\":{\"checked\":%s,\"checkedAttribute\":%s,"
      "\"own\":%s,\"effective\":%s,\"connected\":%s", ids[i],
      boolean(flags & OUI_CONTROL_CHECKED), boolean(authored), boolean(own),
      boolean(effective), boolean(connected));
    if (s->extended) add(out, capacity, length, ",\"defaultChecked\":%s,\"indeterminate\":%s",
      boolean(authored), boolean(flags & OUI_CONTROL_INDETERMINATE));
    add(out, capacity, length, "}");
  }
  add(out, capacity, length, "},\"active\":\"%s\"", active(s));
}
static void barrier(State* s) {
  for (int i = 0; i < 64; ++i) {
    uint32_t pending = 0;
    CHECK(oui_document_dispatch_pending_events_v1(s->document, &pending));
    if (!pending) return;
  }
  abort();
}
static const char* kind(uint32_t type) {
  switch (type) {
    case OUI_EVENT_CLICK: return "click";
    case OUI_EVENT_INPUT: return "input";
    case OUI_EVENT_CHANGE: return "change";
    case OUI_EVENT_FOCUS: return "focus";
    case OUI_EVENT_FOCUS_IN: return "focusin";
    case OUI_EVENT_BLUR: return "blur";
    case OUI_EVENT_FOCUS_OUT: return "focusout";
    default: abort();
  }
}
static void record(OuiEvent* event, void* data) {
  State* s = (State*)data; if (s->setup) return;
  OuiEventPropertiesV1 props = {sizeof(props), OUI_ABI_VERSION, 0, 0};
  CHECK(oui_event_properties_v1(event, &props));
  if (s->row_count++) add(s->rows, sizeof(s->rows), &s->row_length, ",");
  add(s->rows, sizeof(s->rows), &s->row_length, "{");
  snapshot(s, s->rows, sizeof(s->rows), &s->row_length);
  add(s->rows, sizeof(s->rows), &s->row_length,
    ",\"type\":\"%s\",\"target\":\"%s\",\"bubbles\":%s,\"cancelable\":%s}",
    kind(event->event_type), identify(s, event->target), boolean(props.bubbles), boolean(props.cancelable));
}
static void mutate(OuiEvent* event, void* data) {
  State* s = (State*)data;
  switch (s->mutation) {
    case 1: event->flags |= OUI_EVENT_DEFAULT_PREVENTED; break;
    case 2: checked(s, 7, 0); break;
    case 3: attr(s, 7, "disabled", ""); break;
    case 4: attr(s, 7, "checked", ""); break;
    case 5: attr(s, 8, "disabled", ""); event->flags |= OUI_EVENT_DEFAULT_PREVENTED; break;
    case 6: checked(s, 8, 1); event->flags |= OUI_EVENT_DEFAULT_PREVENTED; break;
    case 7: checked(s, 7, 1); break;
    case 8: checked(s, 6, 1); break;
    case 9: attr(s, 8, "disabled", ""); break;
    case 10: attr(s, 8, "name", "other"); break;
    case 11: move_to(s, 8, 1); break;
    case 12: detach(s, 8); break;
    case 13: detach(s, 8); event->flags |= OUI_EVENT_DEFAULT_PREVENTED; break;
    case 14: checked(s, 6, 1); event->flags |= OUI_EVENT_DEFAULT_PREVENTED; break;
    case 15: attr(s, 8, "name", "other"); event->flags |= OUI_EVENT_DEFAULT_PREVENTED; break;
    case 16: move_to(s, 8, 1); event->flags |= OUI_EVENT_DEFAULT_PREVENTED; break;
    case 17: attr(s, 7, "checked", ""); event->flags |= OUI_EVENT_DEFAULT_PREVENTED; break;
    case 18: indeterminate(s, 8, 1); break;
    default: abort();
  }
}
static void callback(State* s, int node, int operation) {
  s->mutation = operation;
  CHECK(oui_element_add_event_listener(s->elements[node], OUI_EVENT_CLICK, 0, mutate,
    s, &s->listeners[s->listener_count++]));
}
static void click(State* s, int node) {
  if (!s->explicit_dispatch) {
    CHECK(oui_element_perform_accessibility_action(s->elements[node], OUI_ACCESSIBILITY_CLICK, text(""), 0, 0));
  } else {
    OuiEvent event;
    memset(&event, 0, sizeof(event));
    event.struct_size = sizeof(event); event.abi_version = OUI_ABI_VERSION;
    event.event_type = OUI_EVENT_CLICK;
    CHECK(oui_document_dispatch_event(s->document, s->elements[node], &event));
  }
}
static void make(State* s, const char* control) {
  OuiDocumentConfig config = {
    sizeof(config), OUI_ABI_VERSION, {800, 600, 800, 600, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
  CHECK(oui_document_create(&config, &s->document));
  CHECK(oui_document_root(s->document, &s->root));
  for (int i = 0; i < 9; ++i) {
    OuiElementTag tag = i < 2 ? OUI_ELEMENT_FORM : i == 2 ? OUI_ELEMENT_FIELDSET :
      i < 5 ? OUI_ELEMENT_LEGEND : OUI_ELEMENT_INPUT;
    CHECK(oui_element_create(s->document, tag, &s->elements[i]));
    attr(s, i, "id", ids[i]);
    if (i >= 5) { attr(s, i, "type", control); attr(s, i, "name", "g"); }
  }
  const int roots[] = {0, 1, 2, 8};
  for (size_t i = 0; i < 4; ++i) CHECK(oui_element_append_child(s->root, s->elements[roots[i]]));
  const int pairs[][2] = {{2, 3}, {3, 5}, {2, 4}, {4, 6}, {2, 7}};
  for (size_t i = 0; i < 5; ++i) move_to(s, pairs[i][1], pairs[i][0]);
  const OuiEventType types[] = {OUI_EVENT_CLICK, OUI_EVENT_INPUT, OUI_EVENT_CHANGE,
    OUI_EVENT_FOCUS, OUI_EVENT_FOCUS_IN, OUI_EVENT_BLUR, OUI_EVENT_FOCUS_OUT};
  for (int node = 5; node < 9; ++node) for (size_t j = 0; j < 7; ++j)
    CHECK(oui_element_add_event_listener(s->elements[node], types[j], 0, record, s,
      &s->listeners[s->listener_count++]));
  barrier(s);
}
static int same(const char* a, const char* b) { return strcmp(a, b) == 0; }
static void prepare(State* s, int suite, const char* v) {
  if (suite == 0) {
    if (same(v, "own-disabled-set") || same(v, "disabled-click")) attr(s, 7, "disabled", "");
    else if (strstr(v, "legend") || same(v, "group-disabled-set") || same(v, "group-disabled-click")) attr(s, 2, "disabled", "");
    else if (same(v, "disabled-neighbor-uncheck") || same(v, "own-disabled-false")) { checked(s, 7, 1); attr(s, 7, "disabled", ""); }
    else if (same(v, "group-neighbor-uncheck") || same(v, "group-disabled-false")) { checked(s, 7, 1); attr(s, 2, "disabled", ""); }
    else if (same(v, "independent-forms")) { checked(s, 7, 1); attr(s, 7, "form", "fa"); attr(s, 8, "form", "fb"); }
    else if (same(v, "detached-group")) { detach(s, 2); checked(s, 7, 1); }
    else if (same(v, "different-names")) { checked(s, 7, 1); attr(s, 8, "name", "other"); }
    else if (same(v, "empty-names")) { checked(s, 7, 1); remove_attr(s, 7, "name"); remove_attr(s, 8, "name"); }
  } else if (suite == 1) {
    if (same(v, "default-clean-remove") || same(v, "same-true-after-attribute") ||
        same(v, "clone-clean") || same(v, "click-default-remove")) attr(s, 7, "checked", "");
    else if (same(v, "assign-true-then-default-remove")) { attr(s, 7, "checked", ""); checked(s, 7, 1); }
    else if (same(v, "assign-false-then-default-remove") || same(v, "clone-dirty-false")) { attr(s, 7, "checked", ""); checked(s, 7, 0); }
    else if (same(v, "clone-live-true")) checked(s, 7, 1);
    else if (same(v, "click-cancel")) callback(s, 7, 1);
    else if (same(v, "click-callback-false")) callback(s, 7, 2);
    else if (same(v, "click-callback-disable")) callback(s, 7, 3);
    else if (same(v, "click-callback-default")) callback(s, 7, 4);
    else if (same(v, "neighbor-cancel") || same(v, "neighbor-cancel-disable") || same(v, "neighbor-cancel-assign")) {
      checked(s, 7, 1); callback(s, 8, same(v, "neighbor-cancel") ? 1 : same(v, "neighbor-cancel-disable") ? 5 : 6);
    }
  } else {
    if (same(v, "already-checked-click")) checked(s, 8, 1);
    else if (strstr(v, "indeterminate")) {
      indeterminate(s, 8, 1);
      if (same(v, "indeterminate-cancel")) callback(s, 8, 1);
      if (same(v, "indeterminate-callback")) callback(s, 8, 18);
    } else if (same(v, "default-checked-neighbor-click")) attr(s, 7, "checked", "");
    else {
      checked(s, 7, 1);
      const char* names[] = {"neighbor-callback-check-old", "neighbor-callback-check-other",
        "neighbor-callback-disable", "neighbor-callback-rename", "neighbor-callback-form",
        "neighbor-callback-detach", "neighbor-callback-default", "neighbor-cancel-detach",
        "neighbor-cancel-check-other", "neighbor-cancel-rename", "neighbor-cancel-form", "neighbor-cancel-default"};
      const int operations[] = {7, 8, 9, 10, 11, 12, 4, 13, 14, 15, 16, 17};
      for (size_t i = 0; i < 12; ++i) if (same(v, names[i])) callback(s, 8, operations[i]);
      if (same(v, "disabled-neighbor-click")) attr(s, 7, "disabled", "");
      if (same(v, "fieldset-disabled-neighbor-click")) attr(s, 2, "disabled", "");
    }
  }
}
static void action(State* s, int suite, const char* v) {
  if (suite == 0) {
    if (same(v, "first-legend-set")) checked(s, 5, 1);
    else if (same(v, "second-legend-set") || same(v, "detached-group")) checked(s, 6, 1);
    else if (same(v, "own-disabled-false") || same(v, "group-disabled-false")) checked(s, 7, 0);
    else if (same(v, "disabled-neighbor-uncheck") || same(v, "group-neighbor-uncheck") ||
        same(v, "independent-forms") || same(v, "different-names") || same(v, "empty-names")) checked(s, 8, 1);
    else if (same(v, "disabled-click") || same(v, "group-disabled-click")) click(s, 7);
    else if (same(v, "first-legend-click")) click(s, 5);
    else if (same(v, "second-legend-click")) click(s, 6);
    else checked(s, 7, 1);
  } else if (suite == 1) {
    if (same(v, "default-clean-set")) attr(s, 7, "checked", "");
    else if (same(v, "default-clean-remove") || same(v, "assign-true-then-default-remove") ||
        same(v, "assign-false-then-default-remove")) remove_attr(s, 7, "checked");
    else if (same(v, "assign-false-then-default")) { checked(s, 7, 0); attr(s, 7, "checked", ""); }
    else if (same(v, "same-true-after-attribute")) { checked(s, 7, 1); remove_attr(s, 7, "checked"); }
    else if (strstr(v, "clone-") == v) {
      CHECK(oui_element_clone_subtree_v1(s->elements[7], &s->elements[9]));
      attr(s, 9, "id", "copy");
      if (!same(v, "clone-live-true")) attr(s, 9, "name", "clone");
      CHECK(oui_element_append_child(s->root, s->elements[9]));
      if (same(v, "clone-live-true")) attr(s, 9, "name", "clone");
      if (same(v, "clone-dirty-false")) attr(s, 9, "checked", "");
      else remove_attr(s, 9, "checked");
    } else if (strstr(v, "neighbor-cancel") == v) click(s, 8);
    else if (same(v, "click-default-remove")) { click(s, 7); remove_attr(s, 7, "checked"); }
    else if (same(v, "programmatic-no-events")) { checked(s, 7, 1); checked(s, 7, 0); indeterminate(s, 7, 1); }
    else click(s, 7);
  } else click(s, 8);
}
int main(int argc, char** argv) {
  int explicit_dispatch = argc == 2 && strcmp(argv[1], "dispatch") == 0;
  if (argc > 2 || (argc == 2 && !explicit_dispatch)) return 2;
  const char* suite_names[] = {"programmatic", "default", "activation"};
  const char* const* variants[] = {programmatic, defaults, activation};
  const size_t sizes[] = {17, 18, 20};
  printf("{");
  for (int suite = 0; suite < 3; ++suite) {
    if (suite) printf(",");
    printf("\"%s\":[", suite_names[suite]);
    size_t count = 0;
    for (int type = 0; type < 2; ++type) for (size_t variant = 0; variant < sizes[suite]; ++variant) {
      const char* control = type ? "checkbox" : "radio"; const char* v = variants[suite][variant];
      State s; memset(&s, 0, sizeof(s)); s.setup = 1; s.extended = suite != 0; s.explicit_dispatch = explicit_dispatch;
      make(&s, control); prepare(&s, suite, v);
      s.row_length = s.row_count = 0; s.rows[0] = 0;
      char initial[16384] = {0}, synchronous[16384] = {0}, observed[16384] = {0};
      size_t length = 0; snapshot(&s, initial, sizeof(initial), &length); s.setup = 0;
      action(&s, suite, v); length = 0; snapshot(&s, synchronous, sizeof(synchronous), &length);
      char synchronous_rows[131072]; memcpy(synchronous_rows, s.rows, s.row_length + 1);
      barrier(&s); length = 0; snapshot(&s, observed, sizeof(observed), &length);
      if (count++) printf(",");
      printf("{\"scenario\":\"%s-%s\",\"control\":\"%s\",\"variant\":\"%s\",\"initial\":{%s},"
        "\"synchronous\":{\"rows\":[%s],\"state\":{%s}},\"observed\":{\"rows\":[%s],\"state\":{%s}},\"api_errors\":[]}",
        control, v, control, v, initial, synchronous_rows, synchronous, s.rows, observed);
      for (size_t i = 0; i < s.listener_count; ++i) CHECK(oui_listener_destroy(s.listeners[i]));
      for (int i = 0; i < 10; ++i) if (s.elements[i]) CHECK(oui_element_destroy(s.elements[i]));
      CHECK(oui_element_destroy(s.root)); CHECK(oui_document_destroy(s.document));
    }
    printf("]");
  }
  printf("}\n"); return 0;
}
