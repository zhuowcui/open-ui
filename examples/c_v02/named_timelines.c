/* Native C11/C++17 consuming app: owned named descriptors and shared controls. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "openui.h"
#ifdef __cplusplus
#define ABI(t, n) static_assert(sizeof(t) == n, #t)
#else
#define ABI(t, n) _Static_assert(sizeof(t) == n, #t)
#endif
ABI(OuiAnimationTimeline, 40);
ABI(OuiTimelineLengthV1, 24);
ABI(OuiAnimationAttachmentRangeV1, 80);
ABI(OuiAnimationTimelineSourceV1, 80);
ABI(OuiAnimationTimelineSnapshotV1, 96);
ABI(OuiAnimationAttachmentSnapshotV1, 248);
static OuiUtf8 text(const char* s) {
  OuiUtf8 v = {(const uint8_t*)s, strlen(s)};
  return v;
}
static void style(OuiElement* n, OuiStyleProperty p, const char* s) {
  OuiStyleValue v;
  assert(oui_style_value_parse(p, text(s), &v) == OUI_OK);
  assert(oui_element_set_property(n, p, &v) == OUI_OK);
  if (v.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)v.data.compound) == OUI_OK);
}
static OuiElement* node(OuiDocument* d, OuiElement* p) {
  OuiElement* n = NULL;
  assert(oui_element_create(d, OUI_ELEMENT_DIV, &n) == OUI_OK);
  assert(oui_element_append_child(p, n) == OUI_OK);
  return n;
}
static OuiTimelineRangeOffsetV1 offset(uint32_t name, double percentage) {
  OuiTimelineRangeOffsetV1 v;
  memset(&v, 0, sizeof(v));
  v.name = name;
  v.offset.unit = OUI_LENGTH_PERCENT;
  v.offset.value = percentage;
  return v;
}
typedef struct SnapshotTail {
  OuiAnimationAttachmentSnapshotV1 known;
  uint64_t tail;
} SnapshotTail;
typedef struct Callback {
  OuiElement* port;
  unsigned calls;
} Callback;
static void started(OuiEvent* event, void* data) {
  Callback* c = (Callback*)data;
  double x, y;
  assert(event->event_type == OUI_EVENT_ANIMATION_START);
  assert(oui_element_get_scroll_offset(c->port, &x, &y) == OUI_OK);
  ++c->calls;
}
int main(void) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {320, 240, 320, 240, 1, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* d = NULL;
  OuiElement* root = NULL;
  assert(oui_document_create(&config, &d) == OUI_OK);
  assert(oui_document_root(d, &root) == OUI_OK);
  OuiElement* port = node(d, root);
  style(port, OUI_STYLE_PROPERTY_WIDTH, "100px");
  style(port, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  style(port, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  OuiElement* subject = node(d, port);
  style(subject, OUI_STYLE_PROPERTY_WIDTH, "100px");
  style(subject, OUI_STYLE_PROPERTY_HEIGHT, "240px");
  style(subject, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "green");
  OuiElement* target = node(d, root);
  style(target, OUI_STYLE_PROPERTY_WIDTH, "20px");
  style(target, OUI_STYLE_PROPERTY_HEIGHT, "20px");
  Callback callback = {port, 0};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(target, OUI_EVENT_ANIMATION_START, 0, started, &callback,
                                        &listener) == OUI_OK);
  OuiLinearStopV1 stops[] = {{0, 0}, {29, 0.25}, {29, 0.75}, {100, 1}};
  OuiEasingCurveV1 curve;
  memset(&curve, 0, sizeof(curve));
  curve.struct_size = sizeof(curve);
  curve.abi_version = OUI_ABI_VERSION;
  curve.kind = OUI_EASING_CURVE_LINEAR_STOPS_PERCENT;
  curve.stops = stops;
  curve.stop_count = 4;
  curve.stop_stride = sizeof(stops[0]);
  OuiAnimationOptionsV1 timing;
  memset(&timing, 0, sizeof(timing));
  timing.struct_size = sizeof(timing);
  timing.abi_version = OUI_ABI_VERSION;
  timing.duration_ms = 100;
  timing.iterations = 1;
  timing.playback_rate = 1;
  timing.fill = 3;
  timing.easing = &curve;
  OuiKeyframeV1 frames[2];
  memset(frames, 0, sizeof(frames));
  for (unsigned i = 0; i < 2; ++i) {
    frames[i].struct_size = sizeof(frames[i]);
    frames[i].abi_version = OUI_ABI_VERSION;
    frames[i].offset = i;
    frames[i].value.tag = OUI_STYLE_VALUE_NUMBER;
    frames[i].value.data.number = (float)i;
  }
  frames[0].easing = &curve;
  OuiAnimationTimelineSourceV1 source;
  memset(&source, 0, sizeof(source));
  source.struct_size = sizeof(source);
  source.abi_version = OUI_ABI_VERSION;
  source.kind = 2;
  source.source = subject;
  source.inset_present = 1;
  source.inset_start.unit = 13;
  source.inset_start.value = 10;
  source.inset_start.calc_offset = 2;
  OuiAnimationAttachmentRangeV1 range;
  memset(&range, 0, sizeof(range));
  range.struct_size = sizeof(range);
  range.abi_version = OUI_ABI_VERSION;
  range.present = 3;
  range.start = offset(2, 0);
  range.end = offset(4, 100);
  uint64_t id = UINT64_MAX;
  assert(oui_element_animate_with_attachment_range_v1(target, OUI_STYLE_PROPERTY_OPACITY, frames, 2,
                                                      sizeof(frames[0]), &timing, &source, &range,
                                                      &id) == OUI_OK);
  assert(id != UINT64_MAX);
  range.start.offset.value = 999;
  source.inset_start.value = 999;
  stops[1].output = 0;
  frames[1].value.data.number = 0;
  SnapshotTail saved;
  memset(&saved, 0, sizeof(saved));
  saved.known.struct_size = sizeof(saved);
  saved.known.abi_version = OUI_ABI_VERSION;
  saved.tail = UINT64_C(0xcafe);
  assert(oui_document_get_animation_attachment_snapshot_v1(d, id, &saved.known) == OUI_OK);
  assert(saved.tail == UINT64_C(0xcafe));
  assert(saved.known.present == 3);
  assert(saved.known.range.start.offset.value == 0);
  assert(saved.known.source != NULL);
  assert(oui_element_destroy(saved.known.source) == OUI_OK);
  saved.known.source = NULL;
  for (uint32_t name = 0; name < 7; ++name) {
    range.start = offset(name, 0);
    range.end = offset(name, 100);
    assert(oui_document_set_animation_attachment_range_v1(d, id, &range) == OUI_OK);
  }
  range.start = offset(0, 0);
  range.end = offset(0, 100);
  assert(oui_document_set_animation_attachment_range_v1(d, id, &range) == OUI_OK);
  assert(oui_document_seek_animation_progress_v1(d, id, 29) == OUI_OK);
  OuiAnimationTimelineSnapshotV1 snapshot;
  memset(&snapshot, 0, sizeof(snapshot));
  snapshot.struct_size = sizeof(snapshot);
  snapshot.abi_version = OUI_ABI_VERSION;
  assert(oui_document_get_animation_timeline_snapshot_v1(d, id, &snapshot) == OUI_OK);
  assert((snapshot.present & 3) == 3);
  assert(snapshot.active == 1);
  assert(snapshot.source != NULL);
  assert(oui_element_destroy(snapshot.source) == OUI_OK);
  uint32_t short_header[] = {8, OUI_ABI_VERSION};
  id = UINT64_MAX;
  assert(oui_element_animate_with_attachment_range_v1(
             target, OUI_STYLE_PROPERTY_OPACITY, frames, 2, sizeof(frames[0]), &timing,
             (const OuiAnimationTimelineSourceV1*)short_header, NULL,
             &id) == OUI_ERROR_INVALID_ARGUMENT);
  assert(id == UINT64_MAX);
  OuiBuffer* image = NULL;
  assert(oui_document_render_png(d, &image) == OUI_OK);
  size_t length = oui_buffer_length(image);
  assert(length > 0);
  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_element_destroy(target) == OUI_OK);
  assert(oui_element_destroy(subject) == OUI_OK);
  assert(oui_element_destroy(port) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(d) == OUI_OK);
  assert(saved.known.named[0].end > saved.known.named[0].start);
  assert(oui_buffer_length(image) == length);
  assert(oui_buffer_data(image) != NULL);
  assert(oui_buffer_destroy(image) == OUI_OK);
  return 0;
}
