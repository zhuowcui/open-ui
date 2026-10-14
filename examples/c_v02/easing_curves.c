/* Native C11/C++17 consumer of arbitrary owned easing inputs and shared clocks.
 * Run guards in packaged consumers, including optimized builds. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "openui.h"

#ifdef __cplusplus
#define OUI_ASSERT_LAYOUT(t, n) static_assert(sizeof(t) == (n), #t " ABI size")
#else
#define OUI_ASSERT_LAYOUT(t, n) _Static_assert(sizeof(t) == (n), #t " ABI size")
#endif
OUI_ASSERT_LAYOUT(OuiEasing, 48);
OUI_ASSERT_LAYOUT(OuiAnimationOptions, 104);
OUI_ASSERT_LAYOUT(OuiKeyframe, 80);
OUI_ASSERT_LAYOUT(OuiLinearStopV1, 16);
OUI_ASSERT_LAYOUT(OuiEasingCurveV1, 80);
OUI_ASSERT_LAYOUT(OuiAnimationOptionsV1, 64);
OUI_ASSERT_LAYOUT(OuiKeyframeV1, 40);

typedef struct PointTail {
  OuiLinearStopV1 point;
  uint64_t tail;
} PointTail;
typedef struct CurveTail {
  OuiEasingCurveV1 curve;
  uint64_t tail;
} CurveTail;
typedef struct OptionsTail {
  OuiAnimationOptionsV1 options;
  uint64_t tail;
} OptionsTail;
typedef struct FrameTail {
  OuiKeyframeV1 frame;
  uint64_t tail;
} FrameTail;

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
static OuiEasingCurveV1 curve(uint32_t kind,
                              const OuiLinearStopV1* points,
                              size_t count,
                              size_t stride) {
  OuiEasingCurveV1 value;
  memset(&value, 0, sizeof(value));
  value.struct_size = sizeof(value);
  value.abi_version = OUI_ABI_VERSION;
  value.kind = kind;
  value.stops = points;
  value.stop_count = count;
  value.stop_stride = stride;
  return value;
}
static OuiAnimationOptionsV1 options(const OuiEasingCurveV1* easing) {
  OuiAnimationOptionsV1 value;
  memset(&value, 0, sizeof(value));
  value.struct_size = sizeof(value);
  value.abi_version = OUI_ABI_VERSION;
  value.duration_ms = 100;
  value.iterations = 1;
  value.playback_rate = 1;
  value.fill = 3;
  value.easing = easing;
  return value;
}
static OuiKeyframeV1 frame(double offset, float width, const OuiEasingCurveV1* easing) {
  OuiKeyframeV1 value;
  memset(&value, 0, sizeof(value));
  value.struct_size = sizeof(value);
  value.abi_version = OUI_ABI_VERSION;
  value.offset = offset;
  value.value.tag = OUI_STYLE_VALUE_LENGTH;
  value.value.data.length.value = width;
  value.value.data.length.unit = OUI_LENGTH_PX;
  value.easing = easing;
  return value;
}
static OuiElement* node(OuiDocument* document, OuiElement* parent) {
  OuiElement* result = NULL;
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &result) == OUI_OK);
  assert(oui_element_append_child(parent, result) == OUI_OK);
  return result;
}
static float width(OuiElement* element) {
  OuiRect bounds;
  assert(oui_element_get_bounds(element, &bounds) == OUI_OK);
  return bounds.width;
}
static void scroll_offset(OuiElement* element, double x, double y) {
  double actual_x = -1, actual_y = -1;
  assert(oui_element_get_scroll_offset(element, &actual_x, &actual_y) == OUI_OK);
  assert(actual_x == x && actual_y == y);
}
typedef struct Callback {
  OuiDocument* document;
  unsigned starts;
} Callback;
static void started(OuiEvent* event, void* data) {
  Callback* callback = (Callback*)data;
  uint8_t active = 0;
  assert(event->event_type == OUI_EVENT_ANIMATION_START);
  assert(oui_document_is_animating(callback->document, &active) == OUI_OK);
  assert(active == 1);
  ++callback->starts;
}

int main(void) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {320, 240, 320, 240, 1, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  OuiElement* animated = node(document, root);
  OuiElement* linear_segment = node(document, root);
  property(animated, OUI_STYLE_PROPERTY_HEIGHT, "20px");
  property(linear_segment, OUI_STYLE_PROPERTY_HEIGHT, "20px");
  Callback callback = {document, 0};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(animated, OUI_EVENT_ANIMATION_START, 0, started, &callback,
                                        &listener) == OUI_OK);

  PointTail effect_points[] = {{{0, 0}, 11}, {{0.5, 0.25}, 12}, {{1, 1}, 13}};
  OuiLinearStopV1 segment_points[] = {{0, 0}, {0.25, 0.5}, {1, 1}};
  CurveTail effect;
  effect.curve =
      curve(OUI_EASING_CURVE_LINEAR_STOPS, &effect_points[0].point, 3, sizeof(PointTail));
  effect.curve.struct_size = sizeof(effect);
  effect.tail = 21;
  OuiEasingCurveV1 segment =
      curve(OUI_EASING_CURVE_LINEAR_STOPS, segment_points, 3, sizeof(segment_points[0]));
  OptionsTail timing;
  timing.options = options(&effect.curve);
  timing.options.struct_size = sizeof(timing);
  timing.tail = 22;
  FrameTail frames[] = {{frame(0, 20, &segment), 31}, {frame(1, 100, NULL), 32}};
  frames[0].frame.struct_size = frames[1].frame.struct_size = sizeof(FrameTail);
  uint64_t id = UINT64_MAX;
  assert(oui_element_animate_v1(animated, OUI_STYLE_PROPERTY_WIDTH, &frames[0].frame, 2,
                                sizeof(FrameTail), &timing.options, NULL, &id) == OUI_OK);
  assert(id != UINT64_MAX && callback.starts == 1);
  OuiKeyframeV1 defaults[] = {frame(0, 20, NULL), frame(1, 100, NULL)};
  assert(oui_element_animate_v1(linear_segment, OUI_STYLE_PROPERTY_WIDTH, defaults, 2,
                                sizeof(defaults[0]), &timing.options, NULL, &id) == OUI_OK);
  /* Mutating caller inputs cannot change either already-owned native effect. */
  effect_points[1].point.output = 0.9;
  segment_points[1].output = 0.1;
  frames[1].frame.value.data.length.value = 20;
  assert(oui_document_set_animation_time(document, 50) == OUI_OK);
  assert(width(animated) == 60); /* effect .5 -> .25; segment .25 -> .5 */
  assert(width(linear_segment) == 40);
  assert(effect_points[1].tail == 12 && effect.tail == 21 && timing.tail == 22);
  uint32_t short_header[] = {8, OUI_ABI_VERSION};
  id = UINT64_MAX;
  assert(oui_element_animate_v1(animated, OUI_STYLE_PROPERTY_WIDTH, defaults, 2,
                                sizeof(defaults[0]), (const OuiAnimationOptionsV1*)short_header,
                                NULL, &id) == OUI_ERROR_INVALID_ARGUMENT);
  assert(id == UINT64_MAX && width(animated) == 60);

  OuiElement* port = node(document, root);
  property(port, OUI_STYLE_PROPERTY_POSITION, "relative");
  property(port, OUI_STYLE_PROPERTY_WIDTH, "100px");
  property(port, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  property(port, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  property(port, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
  OuiElement* content = node(document, port);
  property(content, OUI_STYLE_PROPERTY_WIDTH, "300px");
  property(content, OUI_STYLE_PROPERTY_HEIGHT, "240px");
  OuiElement* target = node(document, port);
  property(target, OUI_STYLE_PROPERTY_POSITION, "absolute");
  property(target, OUI_STYLE_PROPERTY_LEFT, "150px");
  property(target, OUI_STYLE_PROPERTY_TOP, "120px");
  property(target, OUI_STYLE_PROPERTY_WIDTH, "20px");
  property(target, OUI_STYLE_PROPERTY_HEIGHT, "20px");
  OuiLinearStopV1 percent_points[] = {{0, 0}, {29, 0.25}, {29, 0.75}, {100, 1}};
  OuiEasingCurveV1 percent =
      curve(OUI_EASING_CURVE_LINEAR_STOPS_PERCENT, percent_points, 4, sizeof(percent_points[0]));
  assert(oui_element_smooth_scroll_to_v1(port, 100, 80, 100, &percent, &id) == OUI_OK);
  percent_points[1].output = 0;
  assert(oui_document_set_animation_time(document, 79) == OUI_OK);
  scroll_offset(port, 25, 20); /* 29ms preserves the authored 29% discontinuity */
  assert(oui_document_set_animation_time(document, 150) == OUI_OK);
  scroll_offset(port, 100, 80);
  assert(oui_element_scroll_to(port, 0, 0) == OUI_OK);
  assert(oui_element_smooth_scroll_into_view_v2(target, OUI_SCROLL_START, OUI_SCROLL_START,
                                                OUI_SCROLL_CONTAINER_NEAREST, 100, NULL) == OUI_OK);
  assert(oui_document_set_animation_time(document, 200) == OUI_OK);
  scroll_offset(port, 75, 60);
  assert(oui_document_set_animation_time(document, 250) == OUI_OK);
  scroll_offset(port, 150, 120);

  /* Authored percent endpoints outside 0..100 are valid native inputs. */
  OuiLinearStopV1 outer_points[] = {{-100, 0}, {200, 1}};
  OuiEasingCurveV1 outer =
      curve(OUI_EASING_CURVE_LINEAR_STOPS_PERCENT, outer_points, 2, sizeof(outer_points[0]));
  OuiAnimationOptionsV1 transition = options(&outer);
  transition.duration_ms = 100;
  OuiStyleValue destination = frame(1, 160, NULL).value;
  assert(oui_element_transition_v1(linear_segment, OUI_STYLE_PROPERTY_WIDTH, &destination,
                                   &transition, &id) == OUI_OK);
  assert(oui_document_set_animation_time(document, 300) == OUI_OK);
  assert(width(linear_segment) == 130);
  percent.stop_count = SIZE_MAX;
  id = UINT64_MAX;
  assert(oui_element_smooth_scroll_to_v1(port, 0, 0, 100, &percent, &id) ==
         OUI_ERROR_INVALID_ARGUMENT);
  assert(id == UINT64_MAX);
  scroll_offset(port, 150, 120);

  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_element_destroy(target) == OUI_OK);
  assert(oui_element_destroy(content) == OUI_OK);
  assert(oui_element_destroy(port) == OUI_OK);
  assert(oui_element_destroy(linear_segment) == OUI_OK);
  assert(oui_element_destroy(animated) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  return 0;
}
