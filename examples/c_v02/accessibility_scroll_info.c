/* Keep public consuming checks active in packaged release builds. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "openui.h"
#include "portable_thread.h"

#if defined(__cplusplus)
static_assert(sizeof(OuiAccessibilityScrollInfoV1) == 64, "append-only scroll info layout");
#else
_Static_assert(sizeof(OuiAccessibilityScrollInfoV1) == 64, "append-only scroll info layout");
#endif

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
static OuiAccessibilityScrollInfoV1 info(const OuiAccessibilitySnapshot* snapshot, uint64_t id) {
  OuiAccessibilityScrollInfoV1 result;
  memset(&result, 0, sizeof(result));
  result.struct_size = sizeof(result);
  result.abi_version = OUI_ABI_VERSION;
  assert(oui_accessibility_snapshot_get_scroll_info_v1(snapshot, id, &result) == OUI_OK);
  return result;
}
static OuiAccessibilitySnapshot* capture(OuiDocument* document,
                                         const OuiAccessibilitySnapshot* previous) {
  OuiAccessibilitySnapshot* result = NULL;
  assert(oui_document_accessibility_snapshot(document, previous, &result) == OUI_OK);
  return result;
}
static uint32_t all_flags(void) {
  return OUI_ACCESSIBILITY_SCROLL_HAS_X | OUI_ACCESSIBILITY_SCROLL_HAS_Y |
         OUI_ACCESSIBILITY_SCROLL_HAS_X_MIN | OUI_ACCESSIBILITY_SCROLL_HAS_X_MAX |
         OUI_ACCESSIBILITY_SCROLL_HAS_Y_MIN | OUI_ACCESSIBILITY_SCROLL_HAS_Y_MAX;
}
typedef struct ForeignQuery {
  const OuiAccessibilitySnapshot* snapshot;
  uint64_t id;
} ForeignQuery;
static void* foreign_query(void* pointer) {
  ForeignQuery* query = (ForeignQuery*)pointer;
  OuiAccessibilityScrollInfoV1 result;
  memset(&result, 0xa5, sizeof(result));
  result.struct_size = sizeof(result);
  result.abi_version = OUI_ABI_VERSION;
  const OuiAccessibilityScrollInfoV1 before = result;
  assert(oui_accessibility_snapshot_get_scroll_info_v1(query->snapshot, query->id, &result) ==
         OUI_ERROR_WRONG_THREAD);
  assert(memcmp(&result, &before, sizeof(result)) == 0);
  return NULL;
}
static void exercise(double scale) {
  OuiDocumentConfig config = {sizeof(config),
                              OUI_ABI_VERSION,
                              {320.0, 240.0, (uint32_t)(320.0 * scale), (uint32_t)(240.0 * scale),
                               scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement *root = NULL, *scroll = NULL, *child = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &scroll) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &child) == OUI_OK);
  assert(oui_element_append_child(root, scroll) == OUI_OK);
  assert(oui_element_append_child(scroll, child) == OUI_OK);
  property(scroll, OUI_STYLE_PROPERTY_WIDTH, "100px");
  property(scroll, OUI_STYLE_PROPERTY_HEIGHT, "80px");
  property(scroll, OUI_STYLE_PROPERTY_OVERFLOW, "auto");
  property(scroll, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
  property(child, OUI_STYLE_PROPERTY_WIDTH, "160px");
  property(child, OUI_STYLE_PROPERTY_HEIGHT, "240px");
  uint64_t id = 0;
  assert(oui_element_get_accessibility_id(scroll, &id) == OUI_OK);
  OuiAccessibilitySnapshot* initial = capture(document, NULL);
  OuiAccessibilityScrollInfoV1 result = info(initial, id);
  assert(result.flags == all_flags());
  assert(result.actions == OUI_ACCESSIBILITY_SCROLL_DIRECTION_ACTION_MASK);
  assert(result.scroll_x == 0.0 && result.scroll_y == 0.0);
  assert(result.scroll_x_min == 0.0 && result.scroll_x_max == 60.0);
  assert(result.scroll_y_min == 0.0 && result.scroll_y_max == 160.0);
  assert(oui_element_scroll_to(scroll, 20.0, 40.0) == OUI_OK);
  property(scroll, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  OuiAccessibilitySnapshot* hidden = capture(document, initial);
  result = info(hidden, id);
  assert(result.flags == all_flags() && result.actions == 0);
  assert(result.scroll_x == 20.0 && result.scroll_y == 40.0);
  assert(result.scroll_x_min == 0.0 && result.scroll_x_max == 60.0);
  assert(result.scroll_y_min == 0.0 && result.scroll_y_max == 160.0);
  property(scroll, OUI_STYLE_PROPERTY_OVERFLOW, "clip");
  OuiAccessibilitySnapshot* clipped = capture(document, hidden);
  result = info(clipped, id);
  assert(result.flags == 0 && result.actions == 0);
  assert(result.scroll_x == 0.0 && result.scroll_y == 0.0);
  assert(result.scroll_x_min == 0.0 && result.scroll_x_max == 0.0);
  assert(result.scroll_y_min == 0.0 && result.scroll_y_max == 0.0);
  property(scroll, OUI_STYLE_PROPERTY_DIRECTION, "rtl");
  property(scroll, OUI_STYLE_PROPERTY_OVERFLOW, "auto");
  assert(oui_element_scroll_to(scroll, -20.0, 40.0) == OUI_OK);
  OuiAccessibilitySnapshot* rtl = capture(document, clipped);
  result = info(rtl, id);
  assert(result.flags == all_flags());
  assert(result.actions == OUI_ACCESSIBILITY_SCROLL_DIRECTION_ACTION_MASK);
  assert(result.scroll_x == -20.0 && result.scroll_y == 40.0);
  assert(result.scroll_x_min == -60.0 && result.scroll_x_max == 0.0);

  struct FutureInfo {
    OuiAccessibilityScrollInfoV1 info;
    uint64_t tail[2];
  } future;
  memset(&future, 0, sizeof(future));
  future.info.struct_size = sizeof(future);
  future.info.abi_version = OUI_ABI_VERSION;
  future.tail[0] = UINT64_C(0x0123456789abcdef);
  future.tail[1] = UINT64_C(0xfedcba9876543210);
  assert(oui_accessibility_snapshot_get_scroll_info_v1(rtl, id, &future.info) == OUI_OK);
  assert(future.info.struct_size == sizeof(future));
  assert(future.tail[0] == UINT64_C(0x0123456789abcdef));
  assert(future.tail[1] == UINT64_C(0xfedcba9876543210));
  uint32_t short_output = sizeof(short_output);
  assert(oui_accessibility_snapshot_get_scroll_info_v1(
             rtl, id, (OuiAccessibilityScrollInfoV1*)&short_output) == OUI_ERROR_INVALID_ARGUMENT);
  assert(short_output == sizeof(short_output));
  uint32_t header[2] = {sizeof(header), OUI_ABI_VERSION};
  assert(oui_accessibility_snapshot_get_scroll_info_v1(
             rtl, id, (OuiAccessibilityScrollInfoV1*)header) == OUI_ERROR_INVALID_ARGUMENT);
  assert(header[0] == sizeof(header) && header[1] == OUI_ABI_VERSION);
  memset(&result, 0xa5, sizeof(result));
  result.struct_size = sizeof(result);
  result.abi_version = OUI_ABI_VERSION;
  const OuiAccessibilityScrollInfoV1 before = result;
  assert(oui_accessibility_snapshot_get_scroll_info_v1(rtl, UINT64_MAX, &result) ==
         OUI_ERROR_INVALID_ARGUMENT);
  assert(memcmp(&result, &before, sizeof(result)) == 0);
  ForeignQuery query = {rtl, id};
  pthread_t thread;
  assert(pthread_create(&thread, NULL, foreign_query, &query) == 0);
  assert(pthread_join(thread, NULL) == 0);

  assert(oui_element_destroy(child) == OUI_OK);
  assert(oui_element_destroy(scroll) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  /* Every value below comes from the owned tree after the live document died. */
  assert(info(initial, id).scroll_x == 0.0);
  assert(info(hidden, id).scroll_y == 40.0);
  assert(info(clipped, id).flags == 0);
  assert(info(rtl, id).scroll_x_min == -60.0);
  assert(oui_accessibility_snapshot_destroy(initial) == OUI_OK);
  assert(oui_accessibility_snapshot_destroy(hidden) == OUI_OK);
  assert(oui_accessibility_snapshot_destroy(clipped) == OUI_OK);
  assert(oui_accessibility_snapshot_destroy(rtl) == OUI_OK);
  assert(oui_accessibility_snapshot_get_scroll_info_v1(rtl, id, &result) ==
         OUI_ERROR_INVALID_HANDLE);
  assert(memcmp(&result, &before, sizeof(result)) == 0);
  printf("owned accessibility scroll info: scale=%g positions/ranges/actions/lifetimes passed\n",
         scale);
}
int main(void) {
  const double scales[] = {1.0, 1.25, 1.5, 1.75, 2.0};
  for (unsigned i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i)
    exercise(scales[i]);
  return 0;
}
