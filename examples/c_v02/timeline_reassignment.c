/* C11/C++17 native app: source reassignment uses the shared Engine controls. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdint.h>
#include <string.h>

#include "openui.h"
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
static OuiElement* port(OuiDocument* d, OuiElement* root, OuiElement** content) {
  OuiElement* p = node(d, root);
  style(p, OUI_STYLE_PROPERTY_WIDTH, "100px");
  style(p, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  style(p, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  *content = node(d, p);
  style(*content, OUI_STYLE_PROPERTY_WIDTH, "100px");
  style(*content, OUI_STYLE_PROPERTY_HEIGHT, "240px");
  return p;
}
static OuiAnimationTimelineSourceV1 source(OuiElement* n) {
  OuiAnimationTimelineSourceV1 v;
  memset(&v, 0, sizeof(v));
  v.struct_size = sizeof(v);
  v.abi_version = OUI_ABI_VERSION;
  v.kind = 1;
  v.source = n;
  return v;
}
static OuiAnimationState state(OuiDocument* d, uint64_t id) {
  OuiAnimationState v;
  memset(&v, 0, sizeof(v));
  v.struct_size = sizeof(v);
  v.abi_version = OUI_ABI_VERSION;
  assert(oui_document_animation_get_state(d, id, &v) == OUI_OK);
  return v;
}
static double percentage(OuiDocument* d, uint64_t id) {
  OuiAnimationTimelineSnapshotV1 v;
  memset(&v, 0, sizeof(v));
  v.struct_size = sizeof(v);
  v.abi_version = OUI_ABI_VERSION;
  assert(oui_document_get_animation_timeline_snapshot_v1(d, id, &v) == OUI_OK);
  assert(v.active == 1);
  assert(v.source != NULL);
  assert(oui_element_destroy(v.source) == OUI_OK);
  return v.current_time_percentage;
}
static uint64_t animate(OuiElement* target,
                        OuiElement* p,
                        const OuiAnimationAttachmentRangeV1* range) {
  OuiKeyframeV1 frames[2];
  memset(frames, 0, sizeof(frames));
  for (unsigned i = 0; i < 2; ++i) {
    frames[i].struct_size = sizeof(frames[i]);
    frames[i].abi_version = OUI_ABI_VERSION;
    frames[i].offset = i;
    frames[i].value.tag = OUI_STYLE_VALUE_NUMBER;
    frames[i].value.data.number = (float)i;
  }
  OuiAnimationOptionsV1 options;
  memset(&options, 0, sizeof(options));
  options.struct_size = sizeof(options);
  options.abi_version = OUI_ABI_VERSION;
  options.duration_ms = 1000;
  options.iterations = 1;
  options.playback_rate = 1;
  options.fill = 3;
  OuiAnimationTimelineSourceV1 s = source(p);
  uint64_t id = UINT64_MAX;
  assert(oui_element_animate_with_attachment_range_v1(target, OUI_STYLE_PROPERTY_OPACITY, frames, 2,
                                                      sizeof(frames[0]), &options, &s, range,
                                                      &id) == OUI_OK);
  return id;
}
typedef struct Reentry {
  OuiDocument* document;
  uint64_t id;
  unsigned calls;
  OuiStatus set_status;
  OuiStatus pause_status;
} Reentry;
static void started(OuiEvent* event, void* data) {
  Reentry* c = (Reentry*)data;
  assert(event->event_type == OUI_EVENT_ANIMATION_START);
  ++c->calls;
  c->set_status = oui_document_set_animation_timeline_source_v1(c->document, c->id, NULL);
  c->pause_status = oui_document_animation_pause(c->document, c->id);
}
int main(void) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {400, 400, 400, 400, 1, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* d = NULL;
  OuiElement* root = NULL;
  assert(oui_document_create(&config, &d) == OUI_OK);
  assert(oui_document_root(d, &root) == OUI_OK);
  OuiElement *ac = NULL, *bc = NULL;
  OuiElement *a = port(d, root, &ac), *b = port(d, root, &bc), *target = node(d, root);
  style(target, OUI_STYLE_PROPERTY_WIDTH, "20px");
  style(target, OUI_STYLE_PROPERTY_HEIGHT, "20px");
  style(target, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "blue");
  assert(oui_element_scroll_to(a, 0, 40) == OUI_OK);
  assert(oui_element_scroll_to(b, 0, 120) == OUI_OK);
  uint64_t id = animate(target, a, NULL);
  assert(oui_document_seek_animation_progress_v1(d, id, 50) == OUI_OK);
  OuiAnimationTimelineSourceV1 same = source(a);
  assert(oui_document_set_animation_timeline_source_v1(d, id, &same) == OUI_OK);
  assert(percentage(d, id) == 50);
  typedef struct Future {
    OuiAnimationTimelineSourceV1 known;
    uint64_t tail;
  } Future;
  Future next;
  memset(&next, 0, sizeof(next));
  next.known = source(b);
  next.known.struct_size = sizeof(next);
  next.tail = UINT64_C(0xcafe);
  Future before = next;
  assert(oui_document_set_animation_timeline_source_v1(d, id, &next.known) == OUI_OK);
  assert(memcmp(&next, &before, sizeof(next)) == 0);
  assert(percentage(d, id) == 75);
  next.known.kind = 99;
  assert(oui_document_set_animation_timeline_source_v1(d, id, &next.known) ==
         OUI_ERROR_INVALID_ARGUMENT);
  assert(percentage(d, id) == 75);
  assert(oui_document_animation_pause(d, id) == OUI_OK);
  assert(oui_document_set_animation_timeline_source_v1(d, id, NULL) == OUI_OK);
  OuiAnimationState held = state(d, id);
  assert(held.current_time_ms == 750);
  assert(held.play_state == 1);
  OuiAnimationTimelineSnapshotV1 timeline;
  memset(&timeline, 0, sizeof(timeline));
  timeline.struct_size = sizeof(timeline);
  timeline.abi_version = OUI_ABI_VERSION;
  assert(oui_document_get_animation_timeline_snapshot_v1(d, id, &timeline) == OUI_OK);
  assert(timeline.present == 0 && timeline.source == NULL);
  OuiAnimationAttachmentSnapshotV1 attachment;
  memset(&attachment, 0, sizeof(attachment));
  attachment.struct_size = sizeof(attachment);
  attachment.abi_version = OUI_ABI_VERSION;
  assert(oui_document_get_animation_attachment_snapshot_v1(d, id, &attachment) == OUI_OK);
  assert(attachment.present == 1 && attachment.source == NULL);
  assert(oui_document_animation_cancel(d, id) == OUI_OK);
  assert(oui_element_scroll_to(a, 0, 0) == OUI_OK);
  OuiAnimationAttachmentRangeV1 range;
  memset(&range, 0, sizeof(range));
  range.struct_size = sizeof(range);
  range.abi_version = OUI_ABI_VERSION;
  range.present = 1;
  range.start.name = 6;
  range.start.offset.unit = OUI_LENGTH_PERCENT;
  range.start.offset.value = 50;
  id = animate(target, a, &range);
  Reentry callback = {d, id, 0, OUI_ERROR_INTERNAL, OUI_ERROR_INTERNAL};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(target, OUI_EVENT_ANIMATION_START, 0, started, &callback,
                                        &listener) == OUI_OK);
  next.known = source(b);
  assert(oui_document_set_animation_timeline_source_v1(d, id, &next.known) == OUI_OK);
  assert(callback.calls == 1 && callback.set_status == OUI_OK && callback.pause_status == OUI_OK);
  held = state(d, id);
  /* Preserve source-clock subtraction before EffectEnd division and ms reflection. */
  const double duration = (160.0 * 16.0 / 1000.0) / 1000.0;
  const double current = (1920.0 / 1000000.0) - 0.5 * duration;
  const double expected_hold_ms = (current / (duration * 0.5)) * 1000.0;
  assert(held.current_time_ms == expected_hold_ms && held.play_state == 1);
  assert(oui_listener_destroy(listener) == OUI_OK);
  OuiBuffer* image = NULL;
  assert(oui_document_render_png(d, &image) == OUI_OK);
  size_t length = oui_buffer_length(image);
  assert(length > 0);
  assert(oui_element_destroy(target) == OUI_OK);
  assert(oui_element_destroy(ac) == OUI_OK);
  assert(oui_element_destroy(bc) == OUI_OK);
  assert(oui_element_destroy(a) == OUI_OK);
  assert(oui_element_destroy(b) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(d) == OUI_OK);
  assert(oui_buffer_length(image) == length);
  assert(oui_buffer_data(image) != NULL);
  assert(oui_buffer_destroy(image) == OUI_OK);
  return 0;
}
