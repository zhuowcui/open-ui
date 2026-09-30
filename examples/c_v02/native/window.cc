#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "openui.h"

static_assert(sizeof(OuiAppConfig) == 40, "existing app configuration");
static_assert(sizeof(OuiEvent) == 88, "existing event layout");
static_assert(sizeof(OuiAppRunConfig) == 32, "versioned run configuration");
static_assert(sizeof(OuiPlatformEvent) == 152, "versioned platform event");

struct Context {
  OuiDocument* document = nullptr;
  OuiElement* card = nullptr;
  unsigned backend = OUI_BACKEND_SOFTWARE;
  uint64_t frames = 0;
  bool failed = false;
};

static bool check(Context& context, OuiStatus status) {
  if (status == OUI_OK)
    return true;
  context.failed = true;
  std::fprintf(stderr, "native C++ status: %d\n", status);
  return false;
}

extern "C" void on_platform(OuiApp* app, const OuiPlatformEvent* event, void* user_data) {
  auto& context = *static_cast<Context*>(user_data);
  if (event->event_type == OUI_PLATFORM_BACKEND_CHANGED && event->backend != context.backend)
    context.failed = true;
  if (event->event_type == OUI_PLATFORM_PRESENTED) {
    context.frames = event->frame_number;
    if (context.frames == 1) {
      check(context, oui_element_set_text(context.card, {(const uint8_t*)"C++ update", 10}));
    } else {
      check(context, oui_app_request_exit(app));
    }
  }
  if (context.failed)
    oui_app_request_exit(app);
}

int main(int argc, char** argv) {
  Context context;
  if (argc > 1)
    context.backend = static_cast<unsigned>(std::atoi(argv[1]));
  const char title[] = "Open UI native C++";
  OuiAppConfig config{sizeof(config),
                      OUI_ABI_VERSION,
                      {reinterpret_cast<const uint8_t*>(title), std::strlen(title)},
                      240,
                      120,
                      context.backend,
                      0};
  OuiApp* app = nullptr;
  OuiElement* root = nullptr;
  if (!check(context, oui_app_create(&config, &app)) ||
      !check(context, oui_app_document(app, &context.document)) ||
      !check(context, oui_document_root(context.document, &root)) ||
      !check(context, oui_element_create(context.document, OUI_ELEMENT_DIV, &context.card)) ||
      !check(context, oui_element_append_child(root, context.card)))
    return 1;
  OuiAppRunConfig run{sizeof(run), OUI_ABI_VERSION, 0, 0, on_platform, &context};
  check(context, oui_app_run(app, &run));
  check(context, oui_element_destroy(context.card));
  check(context, oui_element_destroy(root));
  check(context, oui_document_destroy(context.document));
  check(context, oui_app_destroy(app));
  std::printf("native C++: backend=%u frames=%llu\n", context.backend,
              static_cast<unsigned long long>(context.frames));
  return context.failed || context.frames < 2;
}
