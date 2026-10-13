/* Public C11/C++17 property ownership and callback removal guards. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "openui.h"

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

static OuiElement* node(OuiDocument* document, OuiElement* parent) {
  OuiElement* result = NULL;
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &result) == OUI_OK);
  assert(oui_element_append_child(parent, result) == OUI_OK);
  return result;
}

static OuiRect bounds(OuiElement* element) {
  OuiRect value;
  assert(oui_element_get_bounds(element, &value) == OUI_OK);
  return value;
}

static void remove_property(OuiElement* element, OuiStyleProperty id, uint8_t expected) {
  uint8_t removed = 99;
  assert(oui_element_remove_property(element, id, &removed) == OUI_OK);
  assert(removed == expected);
}

typedef struct CallbackState {
  OuiElement* node;
  unsigned calls;
  OuiStatus first, second, bounds_status;
  uint8_t removed[2];
  float width;
} CallbackState;

static void clicked(OuiEvent* event, void* data) {
  CallbackState* state = (CallbackState*)data;
  OuiRect value;
  (void)event;
  ++state->calls;
  /* No assertions, exceptions or unchecked reads in the C callback. */
  state->first =
      oui_element_remove_property(state->node, OUI_STYLE_PROPERTY_WIDTH, &state->removed[0]);
  state->second =
      oui_element_remove_property(state->node, OUI_STYLE_PROPERTY_WIDTH, &state->removed[1]);
  state->bounds_status = oui_element_get_bounds(state->node, &value);
  if (state->bounds_status == OUI_OK)
    state->width = value.width;
}

static void check_removal(double scale) {
  OuiDocumentConfig config;
  memset(&config, 0, sizeof(config));
  config.struct_size = sizeof(config);
  config.abi_version = OUI_ABI_VERSION;
  config.viewport.logical_width = 320;
  config.viewport.logical_height = 240;
  config.viewport.physical_width = (uint32_t)(320 * scale);
  config.viewport.physical_height = (uint32_t)(240 * scale);
  config.viewport.device_scale_factor = scale;
  config.viewport.authority = OUI_VIEWPORT_LOGICAL;
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  OuiElement* owner = node(document, root);
  OuiElement* child = node(document, owner);
  property(root, OUI_STYLE_PROPERTY_FONT_SIZE, "10px");
  property(owner, OUI_STYLE_PROPERTY_BOX_SIZING, "border-box");
  property(owner, OUI_STYLE_PROPERTY_WIDTH, "120px");
  property(owner, OUI_STYLE_PROPERTY_HEIGHT, "60px");
  property(child, OUI_STYLE_PROPERTY_WIDTH, "10px");
  property(child, OUI_STYLE_PROPERTY_HEIGHT, "5px");
  property(owner, OUI_STYLE_PROPERTY_PADDING, "1em");
  property(owner, OUI_STYLE_PROPERTY_PADDING_LEFT, "7px");
  assert(bounds(child).x - bounds(owner).x == 7);
  remove_property(owner, OUI_STYLE_PROPERTY_PADDING_LEFT, 1);
  assert(bounds(child).x - bounds(owner).x == 0);
  assert(bounds(child).y - bounds(owner).y == 10);
  property(root, OUI_STYLE_PROPERTY_FONT_SIZE, "20px");
  assert(bounds(child).x - bounds(owner).x == 0);
  assert(bounds(child).y - bounds(owner).y == 20);
  remove_property(owner, OUI_STYLE_PROPERTY_PADDING_LEFT, 0);
  remove_property(owner, OUI_STYLE_PROPERTY_PADDING, 1);
  assert(bounds(child).y - bounds(owner).y == 0);
  property(owner, OUI_STYLE_PROPERTY_DISPLAY, "none");
  remove_property(owner, OUI_STYLE_PROPERTY_DISPLAY, 1);
  assert(bounds(owner).width == 120);

  uint8_t unchanged = 99;
  assert(oui_element_remove_property(owner, OUI_STYLE_PROPERTY_WIDTH, NULL) ==
         OUI_ERROR_INVALID_ARGUMENT);
  assert(oui_element_remove_property(owner, (OuiStyleProperty)INT32_MAX, &unchanged) ==
         OUI_ERROR_INVALID_ARGUMENT);
  assert(unchanged == 99 && bounds(owner).width == 120);
  assert(oui_element_remove_property((OuiElement*)document, OUI_STYLE_PROPERTY_WIDTH, &unchanged) ==
         OUI_ERROR_INVALID_HANDLE);
  assert(unchanged == 99 && bounds(owner).width == 120);

  /* The source reset stays masked by a real public width animation. */
  OuiAnimationOptionsV1 options;
  memset(&options, 0, sizeof(options));
  options.struct_size = sizeof(options);
  options.abi_version = OUI_ABI_VERSION;
  options.duration_ms = 100;
  options.iterations = 1;
  options.playback_rate = 1;
  options.fill = 3;
  OuiAnimationTimeline timeline;
  memset(&timeline, 0, sizeof(timeline));
  timeline.struct_size = sizeof(timeline);
  timeline.abi_version = OUI_ABI_VERSION;
  OuiKeyframeV1 frames[2];
  memset(frames, 0, sizeof(frames));
  for (size_t i = 0; i < 2; ++i) {
    frames[i].struct_size = sizeof(frames[i]);
    frames[i].abi_version = OUI_ABI_VERSION;
    frames[i].offset = (double)i;
    frames[i].value.tag = OUI_STYLE_VALUE_LENGTH;
    frames[i].value.data.length.value = 40;
    frames[i].value.data.length.unit = OUI_LENGTH_PX;
  }
  uint64_t animation = 0;
  assert(oui_element_animate_v1(owner, OUI_STYLE_PROPERTY_WIDTH, frames, 2, sizeof(frames[0]),
                                &options, &timeline, &animation) == OUI_OK);
  assert(animation != 0 && bounds(owner).width == 40);
  remove_property(owner, OUI_STYLE_PROPERTY_WIDTH, 1);
  assert(bounds(owner).width == 40);
  assert(oui_document_animation_cancel(document, animation) == OUI_OK);
  assert(bounds(owner).width == 320);
  remove_property(owner, OUI_STYLE_PROPERTY_WIDTH, 0);

  /* Raw em inheritance updates after source removal, without font idealization. */
  property(owner, OUI_STYLE_PROPERTY_FONT_SIZE, "30px");
  property(child, OUI_STYLE_PROPERTY_WIDTH, "2em");
  assert(bounds(child).width == 60);
  remove_property(owner, OUI_STYLE_PROPERTY_FONT_SIZE, 1);
  assert(bounds(child).width == 40);
  property(root, OUI_STYLE_PROPERTY_FONT_SIZE, "24px");
  assert(bounds(child).width == 48);

  property(owner, OUI_STYLE_PROPERTY_WIDTH, "77px");
  CallbackState state;
  memset(&state, 0, sizeof(state));
  state.node = owner;
  state.first = state.second = state.bounds_status = OUI_ERROR_INTERNAL;
  state.removed[0] = state.removed[1] = 99;
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(owner, OUI_EVENT_CLICK, 0, clicked, &state, &listener) ==
         OUI_OK);
  OuiEvent event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(event);
  event.abi_version = OUI_ABI_VERSION;
  event.event_type = OUI_EVENT_CLICK;
  assert(oui_document_dispatch_event(document, owner, &event) == OUI_OK);
  assert(state.calls == 1 && state.first == OUI_OK && state.second == OUI_OK &&
         state.bounds_status == OUI_OK && state.removed[0] == 1 && state.removed[1] == 0 &&
         state.width == 320);
  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_element_destroy(child) == OUI_OK);
  assert(oui_element_destroy(owner) == OUI_OK);
  assert(oui_element_remove_property(owner, OUI_STYLE_PROPERTY_WIDTH, &unchanged) ==
         OUI_ERROR_INVALID_HANDLE);
  assert(unchanged == 99);
  assert(oui_document_destroy(document) == OUI_OK);
  assert(oui_element_remove_property(root, OUI_STYLE_PROPERTY_DISPLAY, &unchanged) ==
         OUI_ERROR_INVALID_HANDLE);
  assert(unchanged == 99);
  assert(oui_element_destroy(root) == OUI_OK);
  printf("native property removal: scale=%g passed\n", scale);
}

int main(void) {
  const double scales[] = {1.0, 1.25, 1.5, 1.75, 2.0};
  for (size_t i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i)
    check_removal(scales[i]);
  return 0;
}
