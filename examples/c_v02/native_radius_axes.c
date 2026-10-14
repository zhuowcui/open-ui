/* Native owned radius-axis inputs. Uses the same Engine declarations/effects. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <string.h>

#include "openui.h"

static OuiUtf8 utf8(const char* text) {
  OuiUtf8 value = {(const uint8_t*)text, strlen(text)};
  return value;
}
static void property(OuiElement* node, OuiStyleProperty id, const char* text) {
  OuiStyleValue value;
  assert(oui_style_value_parse(id, utf8(text), &value) == OUI_OK);
  assert(oui_element_set_property(node, id, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}
static OuiRadiusAxisLengthV1 length(float value, float offset, uint32_t unit) {
  OuiRadiusAxisLengthV1 result = {sizeof(result), OUI_ABI_VERSION, value, offset, unit, 0};
  return result;
}
static OuiStyleValue carrier(OuiStyleProperty axis, OuiRadiusAxisLengthV1 input) {
  OuiStyleValue value;
  assert(oui_style_value_radius_axis_length_v1(axis, &input, &value) == OUI_OK);
  assert(value.tag == OUI_STYLE_VALUE_COMPOUND && value.reserved == 0);
  return value;
}
static void set(OuiElement* node, OuiStyleProperty axis, OuiRadiusAxisLengthV1 input) {
  OuiStyleValue value = carrier(axis, input);
  assert(oui_element_set_property(node, axis, &value) == OUI_OK);
  assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}
static OuiRadiusAxisLengthV1 get(OuiElement* node, OuiStyleProperty axis) {
  OuiRadiusAxisLengthV1 value = length(0, 0, 0);
  assert(oui_element_radius_axis_length_v1(node, axis, &value) == OUI_OK);
  return value;
}
static void equals(OuiElement* node,
                   OuiStyleProperty axis,
                   float value,
                   float offset,
                   uint32_t unit) {
  OuiRadiusAxisLengthV1 actual = get(node, axis);
  assert(actual.value == value && actual.calc_offset == offset && actual.unit == unit);
}
static void same_pixels(const OuiBitmap* a, const OuiBitmap* b) {
  assert(a->width == b->width && a->height == b->height);
  for (uint32_t y = 0; y < a->height; ++y)
    assert(memcmp(oui_buffer_data(a->pixels) + y * a->stride,
                  oui_buffer_data(b->pixels) + y * b->stride, a->width * 4) == 0);
}
static void native_axes(double scale) {
  assert(sizeof(OuiRadiusAxisLengthV1) == 24);
  OuiDocumentConfig config = {
      sizeof(config),
      OUI_ABI_VERSION,
      {160, 140, (uint32_t)(160 * scale), (uint32_t)(140 * scale), scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* doc = NULL;
  OuiElement *root = NULL, *node = NULL, *child = NULL;
  assert(oui_document_create(&config, &doc) == OUI_OK);
  assert(oui_document_root(doc, &root) == OUI_OK);
  assert(oui_element_create(doc, OUI_ELEMENT_DIV, &node) == OUI_OK);
  assert(oui_element_create(doc, OUI_ELEMENT_DIV, &child) == OUI_OK);
  assert(oui_element_append_child(root, node) == OUI_OK);
  assert(oui_element_append_child(node, child) == OUI_OK);
  property(root, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "white");
  property(node, OUI_STYLE_PROPERTY_DISPLAY, "block");
  property(node, OUI_STYLE_PROPERTY_WIDTH, "100px");
  property(node, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  property(node, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  property(child, OUI_STYLE_PROPERTY_WIDTH, "100px");
  property(child, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  property(child, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "green");
  const OuiStyleProperty axes[] = {OUI_STYLE_PROPERTY_BORDER_TOP_LEFT_RADIUS_X,
                                   OUI_STYLE_PROPERTY_BORDER_TOP_LEFT_RADIUS_Y,
                                   OUI_STYLE_PROPERTY_BORDER_TOP_RIGHT_RADIUS_X,
                                   OUI_STYLE_PROPERTY_BORDER_TOP_RIGHT_RADIUS_Y,
                                   OUI_STYLE_PROPERTY_BORDER_BOTTOM_RIGHT_RADIUS_X,
                                   OUI_STYLE_PROPERTY_BORDER_BOTTOM_RIGHT_RADIUS_Y,
                                   OUI_STYLE_PROPERTY_BORDER_BOTTOM_LEFT_RADIUS_X,
                                   OUI_STYLE_PROPERTY_BORDER_BOTTOM_LEFT_RADIUS_Y};
  for (size_t i = 0; i < 8; ++i) {
    assert((int)axes[i] == 251 + (int)i);
    set(node, axes[i], length((float)i + 1, 0, OUI_LENGTH_PX));
    equals(node, axes[i], (float)i + 1, 0, OUI_LENGTH_PX);
  }
  set(node, axes[0], length(25, 5, OUI_RADIUS_AXIS_LENGTH_CALCULATED));
  set(node, axes[1], length(25, 0, OUI_LENGTH_PERCENT));
  set(node, axes[3], length(1, 0, OUI_LENGTH_EM));
  equals(node, axes[0], 25, 5, OUI_RADIUS_AXIS_LENGTH_CALCULATED);
  equals(node, axes[3], 16, 0, OUI_LENGTH_PX);
  OuiBitmap source = {sizeof(source), OUI_ABI_VERSION, 0, 0, 0, NULL};
  assert(oui_document_render_rgba(doc, &source) == OUI_OK);
  set(node, axes[0], length(30, 0, OUI_LENGTH_PX));
  set(node, axes[1], length(20, 0, OUI_LENGTH_PX));
  set(node, axes[3], length(16, 0, OUI_LENGTH_PX));
  OuiBitmap fixed = {sizeof(fixed), OUI_ABI_VERSION, 0, 0, 0, NULL};
  assert(oui_document_render_rgba(doc, &fixed) == OUI_OK);
  same_pixels(&source, &fixed);
  set(node, axes[0], length(25, 0, OUI_LENGTH_PERCENT));
  OuiKeyframeV1 frames[2];
  memset(frames, 0, sizeof(frames));
  for (size_t i = 0; i < 2; ++i) {
    frames[i].struct_size = sizeof(frames[i]);
    frames[i].abi_version = OUI_ABI_VERSION;
    frames[i].offset = (double)i;
    frames[i].value = carrier(axes[0], length(5, 0, OUI_LENGTH_PX));
  }
  OuiAnimationOptionsV1 options;
  memset(&options, 0, sizeof(options));
  options.struct_size = sizeof(options);
  options.abi_version = OUI_ABI_VERSION;
  options.duration_ms = 100;
  options.iterations = 1;
  options.playback_rate = 1;
  options.fill = 3;
  options.composite = 1;
  OuiAnimationTimeline timeline;
  memset(&timeline, 0, sizeof(timeline));
  timeline.struct_size = sizeof(timeline);
  timeline.abi_version = OUI_ABI_VERSION;
  uint64_t animation = 0;
  assert(oui_element_animate_v1(node, axes[0], frames, 2, sizeof(frames[0]), &options, &timeline,
                                &animation) == OUI_OK);
  for (size_t i = 0; i < 2; ++i)
    assert(oui_style_compound_destroy((OuiStyleCompound*)frames[i].value.data.compound) == OUI_OK);
  equals(node, axes[0], 25, 5, OUI_RADIUS_AXIS_LENGTH_CALCULATED);
  set(node, axes[0], length(40, 0, OUI_LENGTH_PERCENT));
  equals(node, axes[0], 40, 5, OUI_RADIUS_AXIS_LENGTH_CALCULATED);
  assert(oui_document_animation_cancel(doc, animation) == OUI_OK);
  equals(node, axes[0], 40, 0, OUI_LENGTH_PERCENT);
  uint8_t removed = 255;
  assert(oui_element_remove_property(node, axes[0], &removed) == OUI_OK && removed == 1);
  equals(node, axes[0], 0, 0, OUI_LENGTH_PX);
  equals(node, axes[1], 20, 0, OUI_LENGTH_PX);
  assert(oui_element_remove_property(node, axes[0], &removed) == OUI_OK && removed == 0);
  struct Future {
    OuiRadiusAxisLengthV1 prefix;
    uint32_t tail[2];
  } future;
  memset(&future, 0, sizeof(future));
  future.prefix = length(0, 0, 0);
  future.prefix.struct_size = sizeof(future);
  future.tail[0] = 123;
  future.tail[1] = 456;
  assert(oui_element_radius_axis_length_v1(node, axes[1], &future.prefix) == OUI_OK);
  assert(future.prefix.value == 20 && future.tail[0] == 123 && future.tail[1] == 456);
  OuiRadiusAxisLengthV1 bad = length(10, 0, OUI_LENGTH_AUTO);
  OuiStyleValue output;
  memset(&output, 0x5a, sizeof(output));
  const OuiStyleValue before = output;
  assert(oui_style_value_radius_axis_length_v1(axes[0], &bad, &output) ==
         OUI_ERROR_INVALID_ARGUMENT);
  assert(memcmp(&output, &before, sizeof(output)) == 0);
  assert(oui_element_destroy(child) == OUI_OK);
  assert(oui_element_destroy(node) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(doc) == OUI_OK);
  assert(oui_buffer_data(source.pixels) != NULL);
  assert(oui_buffer_data(fixed.pixels) != NULL);
  same_pixels(&source, &fixed);
  assert(oui_buffer_destroy(source.pixels) == OUI_OK);
  assert(oui_buffer_destroy(fixed.pixels) == OUI_OK);
}
int main(void) {
  const double scales[] = {1, 1.25, 1.5, 1.75, 2};
  for (size_t i = 0; i < 5; ++i)
    native_axes(scales[i]);
  return 0;
}
