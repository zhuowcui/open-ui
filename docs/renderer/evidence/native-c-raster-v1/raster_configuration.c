/* Native C application: immutable Rust options, callbacks and owned frames. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "openui.h"

#ifdef __cplusplus
static_assert(sizeof(OuiRasterConfigurationV1) == 72, "raster v1 size");
static_assert(alignof(OuiRasterConfigurationV1) == 4, "raster v1 alignment");
static_assert(sizeof(OuiTextRasterConfigurationV1) == 16, "text raster v1 size");
static_assert(offsetof(OuiRasterConfigurationV1, author_text) == 24, "author prefix");
static_assert(offsetof(OuiRasterConfigurationV1, embedded_text) == 56, "embedded prefix");
#else
_Static_assert(sizeof(OuiRasterConfigurationV1) == 72, "raster v1 size");
_Static_assert(_Alignof(OuiRasterConfigurationV1) == 4, "raster v1 alignment");
_Static_assert(sizeof(OuiTextRasterConfigurationV1) == 16, "text raster v1 size");
_Static_assert(offsetof(OuiRasterConfigurationV1, author_text) == 24, "author prefix");
_Static_assert(offsetof(OuiRasterConfigurationV1, embedded_text) == 56, "embedded prefix");
#endif

static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}

static OuiRasterConfigurationV1 empty_raster(void) {
  OuiRasterConfigurationV1 result;
  memset(&result, 0, sizeof(result));
  result.struct_size = sizeof(result);
  result.abi_version = OUI_ABI_VERSION;
  return result;
}

static void same_policy(OuiDocument* document, const OuiRasterConfigurationV1* expected) {
  OuiRasterConfigurationV1 actual = empty_raster();
  assert(oui_document_get_raster_configuration_v1(document, &actual) == OUI_OK);
  /* Every v1 field is a 32-bit scalar; there is no padding or borrowed storage. */
  assert(memcmp(&actual, expected, sizeof(actual)) == 0);
}

static void length(OuiElement* element, OuiStyleProperty property, float pixels) {
  OuiStyleValue value;
  memset(&value, 0, sizeof(value));
  value.tag = OUI_STYLE_VALUE_LENGTH;
  value.data.length.value = pixels;
  value.data.length.unit = OUI_LENGTH_PX;
  assert(oui_element_set_property(element, property, &value) == OUI_OK);
}

static void color(OuiElement* element,
                  OuiStyleProperty property,
                  uint8_t red,
                  uint8_t green,
                  uint8_t blue) {
  OuiStyleValue value;
  memset(&value, 0, sizeof(value));
  value.tag = OUI_STYLE_VALUE_COLOR;
  value.data.color.red = red;
  value.data.color.green = green;
  value.data.color.blue = blue;
  value.data.color.alpha = 255;
  assert(oui_element_set_property(element, property, &value) == OUI_OK);
}

static void enumeration(OuiElement* element, OuiStyleProperty property, int32_t number) {
  OuiStyleValue value;
  memset(&value, 0, sizeof(value));
  value.tag = OUI_STYLE_VALUE_ENUM;
  value.data.enum_value = number;
  assert(oui_element_set_property(element, property, &value) == OUI_OK);
}

typedef struct Mutation {
  OuiDocument* document;
  OuiElement** elements;
  size_t count;
  unsigned calls;
  int glyphs;
  OuiRasterConfigurationV1 expected;
} Mutation;

static void clicked(OuiEvent* event, void* user_data) {
  Mutation* state = (Mutation*)user_data;
  assert(event->event_type == OUI_EVENT_CLICK);
  /* Query and mutation during a callback prove engine borrows were released. */
  same_policy(state->document, &state->expected);
  for (size_t index = 0; index < state->count; ++index) {
    if (state->glyphs) {
      assert(oui_element_set_text(state->elements[index], text("XX")) == OUI_OK);
      color(state->elements[index], OUI_STYLE_PROPERTY_COLOR, 0, 0, 255);
    } else {
      color(state->elements[index], OUI_STYLE_PROPERTY_BACKGROUND_COLOR, 0, 0, 255);
    }
  }
  ++state->calls;
}

static void click(OuiDocument* document, OuiElement* target) {
  OuiEvent event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(event);
  event.abi_version = OUI_ABI_VERSION;
  event.event_type = OUI_EVENT_CLICK;
  assert(oui_document_dispatch_event(document, target, &event) == OUI_OK);
}

static void core_consumer(uint32_t preset, double scale, int application) {
  OuiRasterConfigurationV1 raster = empty_raster();
  assert(oui_raster_configuration_init_v1(preset, &raster) == OUI_OK);
  const OuiRasterConfigurationV1 expected = raster;
  OuiDocumentConfig viewport = {
      sizeof(viewport), OUI_ABI_VERSION, {64, 48, 0, 0, scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiApp* app = NULL;
  OuiDocument* document = NULL;
  if (application) {
    OuiAppConfig config = {
        sizeof(config), OUI_ABI_VERSION, text("Raster C"), 64, 48, OUI_BACKEND_SOFTWARE, 0};
    assert(oui_app_create_with_raster_configuration_v1(&config, &raster, &app) == OUI_OK);
    assert(oui_app_document(app, &document) == OUI_OK);
    assert(oui_document_set_viewport(document, &viewport) == OUI_OK);
  } else {
    assert(oui_document_create_with_raster_configuration_v1(&viewport, &raster, &document) ==
           OUI_OK);
  }
  /* The engine owns its options, independent of the caller's reused storage. */
  memset(&raster, 0xff, sizeof(raster));
  same_policy(document, &expected);
  OuiElement *root = NULL, *card = NULL;
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &card) == OUI_OK);
  assert(oui_element_append_child(root, card) == OUI_OK);
  color(root, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, 255, 255, 255);
  length(card, OUI_STYLE_PROPERTY_WIDTH, 32);
  length(card, OUI_STYLE_PROPERTY_HEIGHT, 24);
  color(card, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, 255, 0, 0);
  OuiBitmap before = {sizeof(before), OUI_ABI_VERSION, 0, 0, 0, NULL};
  assert(oui_document_render_rgba(document, &before) == OUI_OK);
  OuiElement* targets[] = {card};
  Mutation mutation = {document, targets, 1, 0, 0, expected};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(card, OUI_EVENT_CLICK, 0, clicked, &mutation, &listener) ==
         OUI_OK);
  click(document, card);
  assert(mutation.calls == 1);
  OuiBitmap after = {sizeof(after), OUI_ABI_VERSION, 0, 0, 0, NULL};
  assert(oui_document_render_rgba(document, &after) == OUI_OK);
  same_policy(document, &expected);
  const size_t sample = (size_t)(8 * scale) * before.stride + (size_t)(8 * scale) * 4;
  assert(oui_buffer_data(before.pixels)[sample] == 255);
  assert(oui_buffer_data(before.pixels)[sample + 2] == 0);
  assert(oui_buffer_data(after.pixels)[sample] == 0);
  assert(oui_buffer_data(after.pixels)[sample + 2] == 255);
  assert(before.width == (uint32_t)(64 * scale) && before.height == (uint32_t)(48 * scale));
  viewport.viewport.logical_width = 80;
  viewport.viewport.logical_height = 56;
  assert(oui_document_set_viewport(document, &viewport) == OUI_OK);
  same_policy(document, &expected);
  OuiBitmap resized = {sizeof(resized), OUI_ABI_VERSION, 0, 0, 0, NULL};
  assert(oui_document_render_rgba(document, &resized) == OUI_OK);
  assert(resized.width == (uint32_t)(80 * scale) && resized.height == (uint32_t)(56 * scale));
  const size_t resized_sample = (size_t)(8 * scale) * resized.stride + (size_t)(8 * scale) * 4;
  assert(oui_buffer_data(resized.pixels)[resized_sample + 2] == 255);
  assert(oui_listener_destroy(listener) == OUI_OK);
  assert(oui_element_destroy(card) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  if (app)
    assert(oui_app_destroy(app) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  assert(oui_buffer_data(before.pixels)[sample] == 255);
  assert(oui_buffer_data(after.pixels)[sample + 2] == 255);
  assert(oui_buffer_data(resized.pixels)[resized_sample + 2] == 255);
  assert(oui_buffer_destroy(before.pixels) == OUI_OK);
  assert(oui_buffer_destroy(after.pixels) == OUI_OK);
  assert(oui_buffer_destroy(resized.pixels) == OUI_OK);
}

static OuiFontFace* register_font(OuiDocument* document, const char* family, const char* path) {
  FILE* input = fopen(path, "rb");
  assert(input != NULL && fseek(input, 0, SEEK_END) == 0);
  const long count = ftell(input);
  assert(count > 0 && fseek(input, 0, SEEK_SET) == 0);
  uint8_t* bytes = (uint8_t*)malloc((size_t)count);
  assert(bytes != NULL && fread(bytes, 1, (size_t)count, input) == (size_t)count);
  assert(fclose(input) == 0);
  OuiFontFaceDescriptor descriptor;
  memset(&descriptor, 0, sizeof(descriptor));
  descriptor.struct_size = sizeof(descriptor);
  descriptor.abi_version = OUI_ABI_VERSION;
  descriptor.family = text(family);
  descriptor.weight_min = descriptor.weight_max = 400;
  descriptor.stretch_min = descriptor.stretch_max = 100;
  OuiFontFace* face = NULL;
  assert(oui_document_register_font(document, bytes, (size_t)count, &descriptor, &face) == OUI_OK);
  free(bytes);
  return face;
}

static void save_png(OuiDocument* document, const char* folder, const char* state) {
  char path[4096];
  int count = snprintf(path, sizeof(path), "%s/%s.png", folder, state);
  assert(count > 0 && (size_t)count < sizeof(path));
  OuiBuffer* png = NULL;
  assert(oui_document_render_png(document, &png) == OUI_OK);
  FILE* output = fopen(path, "wb");
  assert(output != NULL);
  const size_t length = oui_buffer_length(png);
  assert(fwrite(oui_buffer_data(png), 1, length, output) == length);
  assert(fclose(output) == 0);
  assert(oui_buffer_destroy(png) == OUI_OK);
}

static void write_bounds(FILE* output, OuiElement** nodes) {
  fputc('[', output);
  for (size_t phase = 0; phase < 64; ++phase) {
    OuiRect rect;
    uint8_t present = 0;
    assert(oui_element_get_bounds(nodes[phase], &rect, &present) == OUI_OK && present == 1);
    fprintf(output, "%s{\"x\":%.17g,\"y\":%.17g,\"width\":%.17g,\"height\":%.17g}",
            phase ? "," : "", (double)rect.x, (double)rect.y, (double)rect.width,
            (double)rect.height);
  }
  fputc(']', output);
}

static void font_consumer(const char* folder,
                          double scale,
                          uint32_t preset,
                          const char* family,
                          float size,
                          const char* font_path) {
  assert(isfinite(scale) && scale > 0 && isfinite(size) && size > 0);
  assert(preset == OUI_RASTER_CHROMIUM_FREETYPE_LCD ||
         preset == OUI_RASTER_CHROMIUM_FONTATIONS_LCD);
  OuiRasterConfigurationV1 raster = empty_raster();
  assert(oui_raster_configuration_init_v1(preset, &raster) == OUI_OK);
  OuiDocumentConfig viewport = {
      sizeof(viewport), OUI_ABI_VERSION, {800, 600, 0, 0, scale, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  assert(oui_document_create_with_raster_configuration_v1(&viewport, &raster, &document) == OUI_OK);
  OuiFontFace* face = register_font(document, family, font_path);
  OuiStyleCompound* font_family = NULL;
  assert(oui_font_family_create(text(family), 0, &font_family) == OUI_OK);
  OuiStyleValue font;
  memset(&font, 0, sizeof(font));
  font.tag = OUI_STYLE_VALUE_COMPOUND;
  font.data.compound = font_family;
  OuiElement* root = NULL;
  assert(oui_document_root(document, &root) == OUI_OK);
  color(root, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, 255, 255, 255);
  enumeration(root, OUI_STYLE_PROPERTY_OVERFLOW, OUI_OVERFLOW_HIDDEN);
  OuiElement* nodes[64];
  for (size_t phase = 0; phase < 64; ++phase) {
    assert(oui_element_create(document, OUI_ELEMENT_DIV, &nodes[phase]) == OUI_OK);
    enumeration(nodes[phase], OUI_STYLE_PROPERTY_POSITION, OUI_POSITION_ABSOLUTE);
    length(nodes[phase], OUI_STYLE_PROPERTY_LEFT, 20 + (float)(phase % 8) * 92 + (float)phase / 64);
    length(nodes[phase], OUI_STYLE_PROPERTY_TOP, 20 + (float)(phase / 8) * 60);
    assert(oui_element_set_property(nodes[phase], OUI_STYLE_PROPERTY_FONT_FAMILY, &font) == OUI_OK);
    length(nodes[phase], OUI_STYLE_PROPERTY_FONT_SIZE, size);
    OuiStyleValue line_height;
    assert(oui_style_value_parse(OUI_STYLE_PROPERTY_LINE_HEIGHT, text("1"), &line_height) ==
           OUI_OK);
    assert(oui_element_set_property(nodes[phase], OUI_STYLE_PROPERTY_LINE_HEIGHT, &line_height) ==
           OUI_OK);
    if (line_height.tag == OUI_STYLE_VALUE_COMPOUND)
      assert(oui_style_compound_destroy((OuiStyleCompound*)line_height.data.compound) == OUI_OK);
    color(nodes[phase], OUI_STYLE_PROPERTY_COLOR, 0, 0, 0);
    assert(oui_element_set_text(nodes[phase], text("X")) == OUI_OK);
    assert(oui_element_append_child(root, nodes[phase]) == OUI_OK);
  }
  assert(oui_style_compound_destroy(font_family) == OUI_OK);
  char path[4096];
  int count = snprintf(path, sizeof(path), "%s/geometry.json", folder);
  assert(count > 0 && (size_t)count < sizeof(path));
  FILE* output = fopen(path, "wb");
  assert(output != NULL);
  fputs("{\"before\":", output);
  write_bounds(output, nodes);
  save_png(document, folder, "before");
  Mutation mutation = {document, nodes, 64, 0, 1, raster};
  OuiListener* listener = NULL;
  assert(oui_element_add_event_listener(root, OUI_EVENT_CLICK, 0, clicked, &mutation, &listener) ==
         OUI_OK);
  click(document, root);
  assert(mutation.calls == 1);
  fputs(",\"after\":", output);
  write_bounds(output, nodes);
  fprintf(output, ",\"callback_count\":%u}\n", mutation.calls);
  assert(fclose(output) == 0);
  save_png(document, folder, "after");
  same_policy(document, &raster);
  assert(oui_listener_destroy(listener) == OUI_OK);
  for (size_t phase = 0; phase < 64; ++phase)
    assert(oui_element_destroy(nodes[phase]) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_font_face_destroy(face) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
}

int main(int argc, char** argv) {
  if (argc == 7) {
    font_consumer(argv[1], strtod(argv[2], NULL), (uint32_t)strtoul(argv[3], NULL, 10), argv[4],
                  strtof(argv[5], NULL), argv[6]);
    return 0;
  }
  if (argc != 1)
    return 2;
  const double scales[] = {1, 1.25, 1.5, 2, 3};
  for (uint32_t preset = OUI_RASTER_DEFAULT; preset <= OUI_RASTER_CHROMIUM_FONTATIONS_LCD; ++preset)
    for (size_t index = 0; index < sizeof(scales) / sizeof(scales[0]); ++index)
      core_consumer(preset, scales[index], (int)((preset + index) % 2));
  puts("native raster options: 25 mutations, callbacks and owned frame pairs passed");
  return 0;
}
