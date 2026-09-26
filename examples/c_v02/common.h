#ifndef OPENUI_EXAMPLE_COMMON_H_
#define OPENUI_EXAMPLE_COMMON_H_

#include <stdio.h>
#include <string.h>

#include "openui.h"

static OuiUtf8 oui_example_utf8(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}

static int oui_example_check(OuiStatus status, const char* operation) {
  if (status == OUI_OK)
    return 1;
  OuiErrorInfo error = {sizeof(error), OUI_ABI_VERSION, 0, 0, 0};
  oui_error_get_last(&error);
  fprintf(stderr, "%s failed (%d, detail %u)\n", operation, error.status, error.detail);
  return 0;
}

static OuiStyleValue oui_example_px(float value) {
  OuiStyleValue result = {.tag = OUI_STYLE_VALUE_LENGTH, .reserved = 0, .data = {.integer = 0}};
  result.data.length.value = value;
  result.data.length.unit = OUI_LENGTH_PX;
  return result;
}

static int oui_example_render(const char* label, OuiColor background) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {480.0, 240.0, 480, 240, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* card = NULL;
  OuiElement* text_node = NULL;
  OuiBuffer* png = NULL;
  int result = 1;
  if (!oui_example_check(oui_document_create(&config, &document), "create document") ||
      !oui_example_check(oui_document_root(document, &root), "get root") ||
      !oui_example_check(oui_element_create(document, OUI_ELEMENT_DIV, &card), "create card") ||
      !oui_example_check(oui_text_create(document, oui_example_utf8(label), &text_node),
                         "create text") ||
      !oui_example_check(oui_element_append_child(root, card), "append card") ||
      !oui_example_check(oui_element_append_child(card, text_node), "append text")) {
    result = 0;
    goto cleanup;
  }
  OuiStyleValue width = oui_example_px(440.0f);
  OuiStyleValue height = oui_example_px(200.0f);
  OuiStyleValue color = {.tag = OUI_STYLE_VALUE_COLOR, .reserved = 0, .data = {.integer = 0}};
  color.data.color = background;
  if (!oui_example_check(oui_element_set_property(card, OUI_STYLE_PROPERTY_WIDTH, &width),
                         "set width") ||
      !oui_example_check(oui_element_set_property(card, OUI_STYLE_PROPERTY_HEIGHT, &height),
                         "set height") ||
      !oui_example_check(
          oui_element_set_property(card, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, &color),
          "set color") ||
      !oui_example_check(oui_document_render_png(document, &png), "render PNG")) {
    result = 0;
    goto cleanup;
  }
  printf("%s: %zu PNG bytes\n", label, oui_buffer_length(png));

cleanup:
  if (png)
    oui_buffer_destroy(png);
  if (text_node)
    oui_element_destroy(text_node);
  if (card)
    oui_element_destroy(card);
  if (root)
    oui_element_destroy(root);
  if (document)
    oui_document_destroy(document);
  return result;
}

#endif
