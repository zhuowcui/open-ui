#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "openui.h"

typedef struct NativeContext {
  OuiDocument* document;
  OuiElement* card;
  uint32_t backend;
  uint64_t frames;
  uint64_t first_hash;
  int failed;
  int resized;
} NativeContext;

static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}

static int check(NativeContext* context, OuiStatus status) {
  if (status != OUI_OK) {
    fprintf(stderr, "native C status: %d\n", status);
    context->failed = 1;
    return 0;
  }
  return 1;
}

static void platform_event(OuiApp* app, const OuiPlatformEvent* event, void* user_data) {
  NativeContext* context = (NativeContext*)user_data;
  if (event->struct_size < sizeof(*event) || event->abi_version != OUI_ABI_VERSION) {
    context->failed = 1;
  }
  if (event->event_type == OUI_PLATFORM_BACKEND_CHANGED) {
    if (event->backend != context->backend)
      context->failed = 1;
    OuiAppRunConfig nested = {sizeof(nested), OUI_ABI_VERSION, 0, 0, NULL, NULL};
    if (oui_app_run(app, &nested) != OUI_ERROR_REENTRANT ||
        oui_app_destroy(app) != OUI_ERROR_INVALID_STATE)
      context->failed = 1;
  }
  if (event->event_type == OUI_PLATFORM_RESIZED) {
    OuiViewportMetrics viewport;
    if (check(context, oui_document_get_viewport(context->document, &viewport)) &&
        viewport.physical_width == event->viewport.physical_width &&
        viewport.physical_height == event->viewport.physical_height &&
        viewport.device_scale_factor == event->viewport.device_scale_factor)
      context->resized = 1;
  }
  if (event->event_type == OUI_PLATFORM_PRESENTED) {
    context->frames = event->frame_number;
    OuiBitmap bitmap = {sizeof(bitmap), OUI_ABI_VERSION, 0, 0, 0, NULL};
    if (check(context, oui_document_render_rgba(context->document, &bitmap))) {
      uint64_t hash = UINT64_C(14695981039346656037);
      const uint8_t* bytes = oui_buffer_data(bitmap.pixels);
      for (size_t index = 0; index < oui_buffer_length(bitmap.pixels); ++index)
        hash = (hash ^ bytes[index]) * UINT64_C(1099511628211);
      check(context, oui_buffer_destroy(bitmap.pixels));
      if (event->frame_number == 1) {
        context->first_hash = hash;
        OuiStyleValue color = {0};
        color.tag = OUI_STYLE_VALUE_COLOR;
        color.data.color = (OuiColor){0, 0, 255, 255};
        check(context,
              oui_element_set_property(context->card, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, &color));
      } else {
        if (hash == context->first_hash)
          context->failed = 1;
        check(context, oui_app_request_exit(app));
      }
    }
  }
  if (context->failed)
    oui_app_request_exit(app);
}

int main(int argc, char** argv) {
  NativeContext context = {0};
  context.backend = argc > 1 ? (uint32_t)atoi(argv[1]) : OUI_BACKEND_SOFTWARE;
  OuiAppConfig config = {
      sizeof(config), OUI_ABI_VERSION, text("Open UI native C"), 240, 120, context.backend, 0};
  OuiApp* app = NULL;
  OuiElement* root = NULL;
  if (!check(&context, oui_app_create(&config, &app)) ||
      !check(&context, oui_app_document(app, &context.document)) ||
      !check(&context, oui_document_root(context.document, &root)) ||
      !check(&context, oui_element_create(context.document, OUI_ELEMENT_DIV, &context.card)) ||
      !check(&context, oui_element_append_child(root, context.card)))
    return 1;
  OuiStyleValue length = {0};
  OuiStyleValue display = {0};
  display.tag = OUI_STYLE_VALUE_ENUM;
  display.data.enum_value = OUI_DISPLAY_BLOCK;
  check(&context, oui_element_set_property(context.card, OUI_STYLE_PROPERTY_DISPLAY, &display));
  length.tag = OUI_STYLE_VALUE_LENGTH;
  length.data.length = (OuiLength){100.0f, OUI_LENGTH_PX};
  check(&context, oui_element_set_property(context.card, OUI_STYLE_PROPERTY_WIDTH, &length));
  length.data.length.value = 60.0f;
  check(&context, oui_element_set_property(context.card, OUI_STYLE_PROPERTY_HEIGHT, &length));
  OuiStyleValue color = {0};
  color.tag = OUI_STYLE_VALUE_COLOR;
  color.data.color = (OuiColor){255, 0, 0, 255};
  check(&context,
        oui_element_set_property(context.card, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, &color));
  OuiAppRunConfig run = {sizeof(run), OUI_ABI_VERSION, 0, 0, platform_event, &context};
  check(&context, oui_app_run(app, &run));
  if (oui_app_run(app, &run) != OUI_ERROR_INVALID_STATE)
    context.failed = 1;
  check(&context, oui_app_destroy(app));
  check(&context, oui_document_update(context.document));
  check(&context, oui_element_destroy(context.card));
  check(&context, oui_element_destroy(root));
  check(&context, oui_document_destroy(context.document));
  printf("native C: backend=%u frames=%llu resized=%d\n", context.backend,
         (unsigned long long)context.frames, context.resized);
  return context.failed || context.frames < 2 || !context.resized;
}
