/* Public C consumer for native focus events and versioned owned metadata. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "openui.h"

#define CHECK(call) do { if ((call) != OUI_OK) { fprintf(stderr, "%s failed\n", #call); exit(1); } } while (0)
#ifdef __cplusplus
#define FOCUS_STATIC_ASSERT static_assert
#define FOCUS_ZERO_INIT {}
#else
#define FOCUS_STATIC_ASSERT _Static_assert
#define FOCUS_ZERO_INIT {0}
#endif

typedef struct Fixture Fixture;
typedef struct Callback {
  Fixture* fixture;
  int capture;
  int modifier;
} Callback;

struct Fixture {
  OuiDocument* document;
  OuiElement* nodes[5];
  OuiListener* listeners[48];
  Callback callbacks[48];
  size_t count;
  size_t rows;
  int scenario;
  OuiElement* retained_related;
};

static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}

static const char* name(Fixture* fixture, OuiElement* node, char storage[2]) {
  if (!node || node == fixture->nodes[0]) return "body";
  if (node == fixture->nodes[1]) return "host";
  size_t length = 0;
  CHECK(oui_element_copy_control_value(node, (uint8_t*)storage, 1, &length));
  if (length != 1 || storage[0] < 'a' || storage[0] > 'c') exit(1);
  storage[1] = '\0';
  return storage;
}

static const char* event_name(uint32_t type) {
  switch (type) {
    case OUI_EVENT_FOCUS: return "focus";
    case OUI_EVENT_BLUR: return "blur";
    case OUI_EVENT_FOCUS_IN: return "focusin";
    case OUI_EVENT_FOCUS_OUT: return "focusout";
    default: exit(1);
  }
}

static void callback(OuiEvent* event, void* user_data) {
  Callback* registration = (Callback*)user_data;
  Fixture* fixture = registration->fixture;
  if (registration->modifier) {
    switch (fixture->scenario) {
      case 1: case 3: CHECK(oui_element_focus(fixture->nodes[4])); break;
      case 2: case 4: CHECK(oui_element_focus(fixture->nodes[3])); break;
      case 5: CHECK(oui_element_detach(fixture->nodes[3])); break;
      case 6: CHECK(oui_element_set_attribute(fixture->nodes[3], text("disabled"), text(""))); break;
      case 8: case 9: event->flags |= OUI_EVENT_PROPAGATION_STOPPED; break;
      case 10: event->flags |= OUI_EVENT_IMMEDIATE_PROPAGATION_STOPPED; break;
      case 11: event->flags |= OUI_EVENT_DEFAULT_PREVENTED; break;
      default: exit(1);
    }
    return;
  }
  OuiFocusEventInfoV1 info = {sizeof(info), OUI_ABI_VERSION, 0, 0, NULL};
  CHECK(oui_event_focus_info_v1(event, &info));
  OuiElement* active = NULL;
  CHECK(oui_document_focused_element_v1(fixture->document, &active));
  char target_storage[2], current_storage[2], active_storage[2], related_storage[2];
  const char* target = name(fixture, event->target, target_storage);
  const char* current = name(fixture, event->current_target, current_storage);
  const char* focused = name(fixture, active, active_storage);
  if (fixture->rows++) printf(",\n");
  printf("{\"type\":\"%s\",\"target\":\"%s\",\"current\":\"%s\",\"phase\":%u,\"capture\":%s,\"related\":",
         event_name(event->event_type), target, current, event->phase,
         registration->capture ? "true" : "false");
  if (info.related_target) printf("\"%s\"", name(fixture, info.related_target, related_storage));
  else printf("null");
  printf(",\"active\":\"%s\",\"bubbles\":%s,\"cancelable\":%s,\"prevented\":%s}",
         focused, info.bubbles ? "true" : "false", info.cancelable ? "true" : "false",
         event->flags & OUI_EVENT_DEFAULT_PREVENTED ? "true" : "false");
  if (active) CHECK(oui_element_destroy(active));
  if (info.related_target) {
    if (!fixture->retained_related) fixture->retained_related = info.related_target;
    else CHECK(oui_element_destroy(info.related_target));
  }
}

static void listen(Fixture* fixture, OuiElement* node, int type, int capture, int modifier) {
  if (fixture->count == 48) exit(1);
  const size_t index = fixture->count++;
  fixture->callbacks[index].fixture = fixture;
  fixture->callbacks[index].capture = capture;
  fixture->callbacks[index].modifier = modifier;
  CHECK(oui_element_add_event_listener(node, (OuiEventType)type, (uint8_t)capture, callback,
                                      &fixture->callbacks[index], &fixture->listeners[index]));
}

int main(void) {
  /* The legacy event layout stays binary compatible. */
  FOCUS_STATIC_ASSERT(sizeof(OuiEvent) == 88, "legacy event layout");
  FOCUS_STATIC_ASSERT(sizeof(OuiFocusEventInfoV1) == 24, "versioned focus metadata layout");
  static const char* scenarios[] = {
      "ordinary", "blur-redirect", "focus-redirect", "focusout-redirect", "focusin-redirect",
      "blur-remove-pending", "blur-disable-pending", "target-capture-order", "capture-stop",
      "target-capture-stop", "target-capture-immediate-stop", "prevent-default"};
  static const int scenario_numbers[] = {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11};
  const uint32_t types[] = {OUI_EVENT_FOCUS, OUI_EVENT_FOCUS_IN, OUI_EVENT_BLUR, OUI_EVENT_FOCUS_OUT};
  printf("[\n");
  for (size_t scenario = 0; scenario < sizeof(scenarios) / sizeof(scenarios[0]); ++scenario) {
    Fixture fixture = FOCUS_ZERO_INIT;
    fixture.scenario = scenario_numbers[scenario];
    OuiDocumentConfig config = {sizeof(config), OUI_ABI_VERSION,
                               {800, 600, 800, 600, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
    CHECK(oui_document_create(&config, &fixture.document));
    CHECK(oui_document_root(fixture.document, &fixture.nodes[0]));
    CHECK(oui_element_create(fixture.document, OUI_ELEMENT_DIV, &fixture.nodes[1]));
    CHECK(oui_element_append_child(fixture.nodes[0], fixture.nodes[1]));
    for (int i = 0; i < 3; ++i) {
      char id[2] = {(char)('a' + i), '\0'};
      CHECK(oui_element_create(fixture.document, OUI_ELEMENT_INPUT, &fixture.nodes[i + 2]));
      CHECK(oui_element_set_control_value(fixture.nodes[i + 2], text(id)));
      CHECK(oui_element_append_child(fixture.nodes[1], fixture.nodes[i + 2]));
    }
    for (int node = 0; node < 5; ++node) {
      for (size_t type = 0; type < 4; ++type) {
        listen(&fixture, fixture.nodes[node], (int)types[type], 0, 0);
        listen(&fixture, fixture.nodes[node], (int)types[type], 1, 0);
      }
    }
    const int modifier_types[] = {0, OUI_EVENT_BLUR, OUI_EVENT_FOCUS, OUI_EVENT_FOCUS_OUT,
                                 OUI_EVENT_FOCUS_IN, OUI_EVENT_BLUR, OUI_EVENT_BLUR};
    if (fixture.scenario >= 1 && fixture.scenario <= 6)
      listen(&fixture, fixture.nodes[2], modifier_types[fixture.scenario], 0, 1);
    if (fixture.scenario >= 8 && fixture.scenario <= 10)
      listen(&fixture, fixture.nodes[fixture.scenario == 8 ? 1 : 2], OUI_EVENT_FOCUS, 1, 1);
    if (fixture.scenario == 11)
      for (size_t type = 0; type < 4; ++type) listen(&fixture, fixture.nodes[2], (int)types[type], 1, 1);
    printf("%s{\"scenario\":\"%s\",\"rows\":[\n", scenario ? ",\n" : "", scenarios[scenario]);
    CHECK(oui_element_focus(fixture.nodes[2]));
    if (fixture.scenario != 2 && fixture.scenario != 4) CHECK(oui_element_focus(fixture.nodes[3]));
    if (fixture.scenario == 0) {
      CHECK(oui_element_focus(fixture.nodes[3]));
      CHECK(oui_element_blur(fixture.nodes[2]));
      CHECK(oui_element_blur(fixture.nodes[3]));
      CHECK(oui_element_blur(fixture.nodes[3]));
    }
    OuiElement* active = NULL;
    CHECK(oui_document_focused_element_v1(fixture.document, &active));
    char active_storage[2];
    printf("\n],\"final\":\"%s\"}", name(&fixture, active, active_storage));
    if (active) CHECK(oui_element_destroy(active));
    for (size_t i = 0; i < fixture.count; ++i) CHECK(oui_listener_destroy(fixture.listeners[i]));
    for (int i = 0; i < 5; ++i) CHECK(oui_element_destroy(fixture.nodes[i]));
    CHECK(oui_document_destroy(fixture.document));
    if (fixture.retained_related) {
      size_t length = 0;
      if (oui_element_copy_control_value(fixture.retained_related, NULL, 0, &length) == OUI_OK) exit(1);
      CHECK(oui_element_destroy(fixture.retained_related));
    }
    OuiFocusEventInfoV1 expired = {sizeof(expired), OUI_ABI_VERSION, 0, 0, NULL};
    if (oui_event_focus_info_v1(NULL, &expired) != OUI_ERROR_INVALID_STATE) exit(1);
  }
  printf("\n]\n");
  return 0;
}
