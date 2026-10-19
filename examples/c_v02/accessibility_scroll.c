/* Keep consuming-app checks active in packaged release builds. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "openui.h"
#include "portable_thread.h"

static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}

static void property(OuiElement* element, OuiStyleProperty id, const char* literal) {
  OuiStyleValue value;
  assert(oui_style_value_parse(id, text(literal), &value) == OUI_OK);
  assert(oui_element_set_property(element, id, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}

typedef struct ScrollObserver {
  OuiElement* element;
  unsigned calls;
} ScrollObserver;

static void scrolled(OuiEvent* event, void* data) {
  ScrollObserver* observer = (ScrollObserver*)data;
  assert(event->event_type == OUI_EVENT_SCROLL);
  double x = -1.0, y = -1.0;
  /* The same document can be borrowed again during the application callback. */
  assert(oui_element_get_scroll_offset(observer->element, &x, &y) == OUI_OK);
  assert(x >= 0.0 && y >= 0.0);
  ++observer->calls;
}

static void drain(OuiDocument* document) {
  uint32_t pending = 1;
  for (unsigned attempt = 0; pending && attempt < 16; ++attempt)
    assert(oui_document_dispatch_pending_events_v1(document, &pending) == OUI_OK);
  assert(pending == 0);
}

static void offset(OuiElement* element, double expected_x, double expected_y) {
  double x = -1.0, y = -1.0;
  assert(oui_element_get_scroll_offset(element, &x, &y) == OUI_OK);
  assert(x == expected_x && y == expected_y);
}

static void* foreign_action(void* pointer) {
  OuiElement* element = (OuiElement*)pointer;
  assert(oui_element_perform_accessibility_action(element, OUI_ACCESSIBILITY_SCROLL_DOWN, text(""),
                                                  0, 0) == OUI_ERROR_WRONG_THREAD);
  return NULL;
}

static void exercise(double scale) {
  OuiDocumentConfig config = {sizeof(config),
                              OUI_ABI_VERSION,
                              {320.0, 240.0, (uint32_t)(320.0 * scale), (uint32_t)(240.0 * scale),
                               scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* viewport = NULL;
  OuiElement* content = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &viewport) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &content) == OUI_OK);
  assert(oui_element_append_child(root, viewport) == OUI_OK);
  assert(oui_element_append_child(viewport, content) == OUI_OK);
  property(viewport, OUI_STYLE_PROPERTY_WIDTH, "100px");
  property(viewport, OUI_STYLE_PROPERTY_HEIGHT, "100px");
  property(viewport, OUI_STYLE_PROPERTY_OVERFLOW, "auto");
  property(viewport, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
  property(content, OUI_STYLE_PROPERTY_WIDTH, "300px");
  property(content, OUI_STYLE_PROPERTY_HEIGHT, "300px");
  offset(viewport, 0.0, 0.0);

  uint64_t id = 0;
  assert(oui_element_get_accessibility_id(viewport, &id) == OUI_OK);
  OuiAccessibilitySnapshot* first = NULL;
  assert(oui_document_accessibility_snapshot(document, NULL, &first) == OUI_OK);
  OuiAccessibilitySnapshotInfo info;
  memset(&info, 0, sizeof(info));
  info.struct_size = sizeof(info);
  info.abi_version = OUI_ABI_VERSION;
  assert(oui_accessibility_snapshot_get_info(first, &info) == OUI_OK);
  unsigned found = 0;
  for (size_t index = 0; index < info.node_count; ++index) {
    OuiAccessibilityNodeInfo node;
    memset(&node, 0, sizeof(node));
    node.struct_size = sizeof(node);
    node.abi_version = OUI_ABI_VERSION;
    assert(oui_accessibility_snapshot_get_node(first, index, &node) == OUI_OK);
    if (node.id == id) {
      const uint32_t directions =
          OUI_ACCESSIBILITY_ACTION_SCROLL_DOWN_BIT | OUI_ACCESSIBILITY_ACTION_SCROLL_UP_BIT |
          OUI_ACCESSIBILITY_ACTION_SCROLL_LEFT_BIT | OUI_ACCESSIBILITY_ACTION_SCROLL_RIGHT_BIT;
      assert((node.actions & directions) == directions);
      ++found;
    }
  }
  assert(found == 1);

  ScrollObserver observer = {viewport, 0};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(viewport, OUI_EVENT_SCROLL, 0, scrolled, &observer,
                                        &listener) == OUI_OK);
  pthread_t foreign;
  assert(pthread_create(&foreign, NULL, foreign_action, viewport) == 0);
  assert(pthread_join(foreign, NULL) == 0);
  assert(oui_element_perform_accessibility_action(viewport, (OuiAccessibilityAction)99, text(""), 0,
                                                  0) == OUI_ERROR_INVALID_ARGUMENT);
  offset(viewport, 0.0, 0.0);
  drain(document);
  assert(observer.calls == 0);

  const OuiAccessibilityAction actions[] = {
      OUI_ACCESSIBILITY_SCROLL_DOWN, OUI_ACCESSIBILITY_SCROLL_RIGHT, OUI_ACCESSIBILITY_SCROLL_UP,
      OUI_ACCESSIBILITY_SCROLL_LEFT};
  const double positions[][2] = {{0.0, 80.0}, {80.0, 80.0}, {80.0, 0.0}, {0.0, 0.0}};
  for (unsigned index = 0; index < 4; ++index) {
    assert(oui_element_perform_accessibility_action(viewport, actions[index], text(""), 0, 0) ==
           OUI_OK);
    offset(viewport, positions[index][0], positions[index][1]);
    assert(observer.calls == index);
    drain(document);
    assert(observer.calls == index + 1);
  }
  assert(oui_element_perform_accessibility_action(viewport, OUI_ACCESSIBILITY_SCROLL_UP, text(""),
                                                  0, 0) == OUI_OK);
  assert(oui_element_perform_accessibility_action(viewport, OUI_ACCESSIBILITY_SCROLL_LEFT, text(""),
                                                  0, 0) == OUI_OK);
  offset(viewport, 0.0, 0.0);
  drain(document);
  assert(observer.calls == 4);

  assert(oui_element_scroll_to(viewport, 190.0, 190.0) == OUI_OK);
  drain(document);
  assert(oui_element_perform_accessibility_action(viewport, OUI_ACCESSIBILITY_SCROLL_DOWN, text(""),
                                                  0, 0) == OUI_OK);
  assert(oui_element_perform_accessibility_action(viewport, OUI_ACCESSIBILITY_SCROLL_RIGHT,
                                                  text(""), 0, 0) == OUI_OK);
  offset(viewport, 200.0, 200.0);
  drain(document);
  unsigned before_clamped = observer.calls;
  assert(oui_element_perform_accessibility_action(viewport, OUI_ACCESSIBILITY_SCROLL_DOWN, text(""),
                                                  0, 0) == OUI_OK);
  assert(oui_element_perform_accessibility_action(viewport, OUI_ACCESSIBILITY_SCROLL_RIGHT,
                                                  text(""), 0, 0) == OUI_OK);
  drain(document);
  assert(observer.calls == before_clamped);

  OuiAccessibilitySnapshot* second = NULL;
  assert(oui_document_accessibility_snapshot(document, first, &second) == OUI_OK);
  assert(oui_accessibility_snapshot_get_info(second, &info) == OUI_OK);
  assert(!info.full_tree && info.changed_count > 0);
  OuiAccessibilityNodeState state;
  memset(&state, 0, sizeof(state));
  state.struct_size = sizeof(state);
  state.abi_version = OUI_ABI_VERSION;
  assert(oui_accessibility_snapshot_get_node_state(first, id, &state) == OUI_OK);
  assert(state.scroll_x == 0.0 && state.scroll_y == 0.0);
  assert(oui_accessibility_snapshot_get_node_state(second, id, &state) == OUI_OK);
  assert(state.scroll_x == 200.0 && state.scroll_y == 200.0);

  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_element_remove(viewport) == OUI_OK);
  assert(oui_element_destroy(content) == OUI_OK);
  assert(oui_element_destroy(viewport) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  assert(oui_accessibility_snapshot_get_node_state(second, id, &state) == OUI_OK);
  assert(state.scroll_x == 200.0 && state.scroll_y == 200.0);
  assert(oui_accessibility_snapshot_destroy(second) == OUI_OK);
  assert(oui_accessibility_snapshot_destroy(first) == OUI_OK);
  printf("native accessibility scroll: scale=%g actions=4 owned snapshots and callbacks passed\n",
         scale);
}

int main(void) {
  const double scales[] = {1.0, 1.25, 1.5, 1.75, 2.0};
  for (unsigned index = 0; index < sizeof(scales) / sizeof(scales[0]); ++index)
    exercise(scales[index]);
  return 0;
}
