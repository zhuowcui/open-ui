/* Native SVG viewport mutation through the public C API and event pipeline. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "openui.h"

static OuiUtf8 svg_utf8(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}

static void svg_property(OuiElement* element, OuiStyleProperty property, const char* literal) {
  OuiStyleValue value;
  OuiStatus status = oui_style_value_parse(property, svg_utf8(literal), &value);
  if (status != OUI_OK)
    fprintf(stderr, "native SVG property %d literal '%s': %d\n", property, literal, status);
  assert(status == OUI_OK);
  assert(oui_element_set_property(element, property, &value) == OUI_OK);
  if (value.tag == OUI_STYLE_VALUE_COMPOUND)
    assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}

typedef struct SvgCallbackState {
  OuiElement* viewport;
  OuiElement* ordinary;
  unsigned calls;
} SvgCallbackState;

static void svg_clicked(OuiEvent* event, void* data) {
  SvgCallbackState* state = (SvgCallbackState*)data;
  assert(event->event_type == OUI_EVENT_CLICK);
  svg_property(state->viewport, OUI_STYLE_PROPERTY_BORDER_LEFT, "3px double black");
  svg_property(state->viewport, OUI_STYLE_PROPERTY_BORDER_RADIUS, "7px");
  svg_property(state->viewport, OUI_STYLE_PROPERTY_PADDING, "2px");
  svg_property(state->viewport, OUI_STYLE_PROPERTY_BOX_SIZING, "border-box");
  svg_property(state->ordinary, OUI_STYLE_PROPERTY_BOX_SIZING, "border-box");
  ++state->calls;
}

static void svg_viewport(double scale,
                         const char* width,
                         const char* height,
                         float expected_width,
                         float expected_height) {
  OuiDocumentConfig config = {sizeof(config),
                              OUI_ABI_VERSION,
                              {320.0, 240.0, (uint32_t)(320.0 * scale), (uint32_t)(240.0 * scale),
                               scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* svg = NULL;
  OuiElement* viewport = NULL;
  OuiElement* ordinary = NULL;
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_SVG, &svg) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_SVG_FOREIGN_OBJECT, &viewport) == OUI_OK);
  assert(oui_element_append_child(root, svg) == OUI_OK);
  assert(oui_element_append_child(svg, viewport) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &ordinary) == OUI_OK);
  assert(oui_element_append_child(root, ordinary) == OUI_OK);
  svg_property(root, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "white");
  svg_property(svg, OUI_STYLE_PROPERTY_POSITION, "absolute");
  svg_property(svg, OUI_STYLE_PROPERTY_LEFT, "20px");
  svg_property(svg, OUI_STYLE_PROPERTY_TOP, "20px");
  svg_property(svg, OUI_STYLE_PROPERTY_WIDTH, "240px");
  svg_property(svg, OUI_STYLE_PROPERTY_HEIGHT, "180px");
  svg_property(svg, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
  svg_property(viewport, OUI_STYLE_PROPERTY_WIDTH, width);
  svg_property(viewport, OUI_STYLE_PROPERTY_HEIGHT, height);
  svg_property(ordinary, OUI_STYLE_PROPERTY_POSITION, "absolute");
  svg_property(ordinary, OUI_STYLE_PROPERTY_LEFT, "120px");
  svg_property(ordinary, OUI_STYLE_PROPERTY_TOP, "20px");
  svg_property(ordinary, OUI_STYLE_PROPERTY_WIDTH, "24px");
  svg_property(ordinary, OUI_STYLE_PROPERTY_HEIGHT, "18px");
  svg_property(ordinary, OUI_STYLE_PROPERTY_BORDER_LEFT, "3px solid black");
  svg_property(ordinary, OUI_STYLE_PROPERTY_PADDING, "2px");
  svg_property(ordinary, OUI_STYLE_PROPERTY_BOX_SIZING, "content-box");
  OuiRect before;
  assert(oui_element_get_bounds(viewport, &before) == OUI_OK);
  assert(before.x == 20.0f && before.y == 20.0f);
  assert(before.width == expected_width && before.height == expected_height);
  OuiRect ordinary_before;
  assert(oui_element_get_bounds(ordinary, &ordinary_before) == OUI_OK);
  assert(ordinary_before.width == 31.0f && ordinary_before.height == 22.0f);

  SvgCallbackState state = {viewport, ordinary, 0};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(viewport, OUI_EVENT_CLICK, 0, svg_clicked, &state,
                                        &listener) == OUI_OK);
  OuiEvent event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(event);
  event.abi_version = OUI_ABI_VERSION;
  event.event_type = OUI_EVENT_CLICK;
  assert(oui_document_dispatch_event(document, viewport, &event) == OUI_OK);
  assert(state.calls == 1);
  OuiRect after;
  assert(oui_element_get_bounds(viewport, &after) == OUI_OK);
  assert(after.x == before.x && after.y == before.y);
  assert(after.width == expected_width && after.height == expected_height);
  assert(before.width == expected_width && before.height == expected_height);
  OuiRect ordinary_after;
  assert(oui_element_get_bounds(ordinary, &ordinary_after) == OUI_OK);
  assert(ordinary_after.x == ordinary_before.x && ordinary_after.y == ordinary_before.y);
  assert(ordinary_after.width == 24.0f && ordinary_after.height == 18.0f);
  assert(ordinary_before.width == 31.0f && ordinary_before.height == 22.0f);
  OuiBuffer* png = NULL;
  assert(oui_document_render_png(document, &png) == OUI_OK);
  assert(oui_buffer_length(png) > 0);
  assert(oui_buffer_destroy(png) == OUI_OK);

  assert(oui_element_detach(viewport) == OUI_OK);
  size_t count = 1;
  assert(oui_element_get_client_rects_v1(viewport, NULL, 0, &count) == OUI_OK && count == 0);
  assert(oui_element_append_child(svg, viewport) == OUI_OK);
  assert(oui_element_get_bounds(viewport, &after) == OUI_OK);
  assert(after.width == expected_width && after.height == expected_height);
  /* Callback data remains live until the owned listener is destroyed. */
  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_element_destroy(viewport) == OUI_OK);
  assert(oui_element_destroy(ordinary) == OUI_OK);
  assert(oui_element_destroy(svg) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
}

int main(void) {
  const double scales[] = {1.0, 1.25, 1.5, 2.0, 3.0};
  for (size_t i = 0; i < sizeof(scales) / sizeof(scales[0]); ++i) {
    svg_viewport(scales[i], "1px", "1px", 1.0f, 1.0f);
    svg_viewport(scales[i], "24px", "18px", 24.0f, 18.0f);
  }
  puts("native SVG viewport: bounds, mutation callbacks, detach/reattach and teardown passed");
  return 0;
}
