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
  OuiElement* textarea = nullptr;
  OuiListener* input_listener = nullptr;
  OuiListener* focus_listener = nullptr;
  OuiListener* blur_listener = nullptr;
  unsigned input_events = 0;
  unsigned focus_events = 0;
  unsigned blur_events = 0;
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

extern "C" void on_input(OuiEvent* event, void* user_data) {
  auto& context = *static_cast<Context*>(user_data);
  const char* values[] = {"á👩‍💻", "á👩‍💻z", "á👩‍💻"};
  ++context.input_events;
  if (context.input_events > 3) {
    context.failed = true;
    return;
  }
  const char* expected = values[context.input_events - 1];
  uint8_t bytes[64];
  size_t length = 0;
  if (event->event_type != OUI_EVENT_INPUT || event->target != context.textarea ||
      event->current_target != context.textarea ||
      !check(context,
             oui_element_copy_control_value(context.textarea, bytes, sizeof(bytes), &length)) ||
      length != std::strlen(expected) || std::memcmp(bytes, expected, length) != 0)
    context.failed = true;
  const char label[] = "C++ input callback";
  check(context, oui_element_set_text(
                     context.card, {reinterpret_cast<const uint8_t*>(label), std::strlen(label)}));
}

extern "C" void on_focus(OuiEvent* event, void* user_data) {
  auto& context = *static_cast<Context*>(user_data);
  if (event->target != context.textarea || event->current_target != context.textarea)
    context.failed = true;
  if (event->event_type == OUI_EVENT_FOCUS)
    ++context.focus_events;
  else if (event->event_type == OUI_EVENT_BLUR)
    ++context.blur_events;
  else
    context.failed = true;
  const char key[] = "data-focus";
  const char value[] = "seen";
  check(context, oui_element_set_attribute(
                     context.card, {reinterpret_cast<const uint8_t*>(key), std::strlen(key)},
                     {reinterpret_cast<const uint8_t*>(value), std::strlen(value)}));
}

extern "C" void on_platform(OuiApp* app, const OuiPlatformEvent* event, void* user_data) {
  auto& context = *static_cast<Context*>(user_data);
  if (event->event_type == OUI_PLATFORM_BACKEND_CHANGED && event->backend != context.backend)
    context.failed = true;
  if (event->event_type == OUI_PLATFORM_PRESENTED) {
    context.frames = event->frame_number;
    if (context.frames == 1) {
      check(context, oui_element_focus(context.textarea));
      check(context, oui_element_focus(context.textarea));
      check(context, oui_element_blur(context.textarea));
      check(context, oui_element_blur(context.textarea));
      check(context, oui_element_focus(context.textarea));
      if (context.focus_events != 2 || context.blur_events != 1)
        context.failed = true;
      OuiEditCommandV1 edit{
          sizeof(edit), OUI_ABI_VERSION, OUI_EDIT_DELETE, OUI_TEXT_BACKWARD, OUI_TEXT_GRAPHEME, 0,
          {0, 0}};
      check(context, oui_element_edit_text_v1(context.textarea, &edit));
      edit.command = OUI_EDIT_UNDO;
      check(context, oui_element_edit_text_v1(context.textarea, &edit));
      edit.command = OUI_EDIT_REDO;
      check(context, oui_element_edit_text_v1(context.textarea, &edit));
      edit.command = OUI_EDIT_SELECT_ALL;
      check(context, oui_element_edit_text_v1(context.textarea, &edit));
      if (context.input_events != 3)
        context.failed = true;
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
  const char value[] = "á👩‍💻z";
  check(context, oui_element_create(context.document, OUI_ELEMENT_TEXTAREA, &context.textarea));
  check(context, oui_element_append_child(root, context.textarea));
  check(context,
        oui_element_set_control_value(
            context.textarea, {reinterpret_cast<const uint8_t*>(value), std::strlen(value)}));
  check(context,
        oui_element_set_selection(context.textarea, std::strlen(value), std::strlen(value)));
  check(context, oui_element_add_event_listener(context.textarea, OUI_EVENT_INPUT, 0, on_input,
                                                &context, &context.input_listener));
  check(context, oui_element_add_event_listener(context.textarea, OUI_EVENT_FOCUS, 0, on_focus,
                                                &context, &context.focus_listener));
  check(context, oui_element_add_event_listener(context.textarea, OUI_EVENT_BLUR, 0, on_focus,
                                                &context, &context.blur_listener));
  OuiAppRunConfig run{sizeof(run), OUI_ABI_VERSION, 0, 0, on_platform, &context};
  check(context, oui_app_run(app, &run));
  check(context, oui_listener_destroy(context.input_listener));
  check(context, oui_listener_destroy(context.focus_listener));
  check(context, oui_listener_destroy(context.blur_listener));
  check(context, oui_element_destroy(context.textarea));
  check(context, oui_element_destroy(context.card));
  check(context, oui_element_destroy(root));
  check(context, oui_document_destroy(context.document));
  check(context, oui_app_destroy(app));
  std::printf("native C++: backend=%u frames=%llu input=%u focus=%u blur=%u\n", context.backend,
              static_cast<unsigned long long>(context.frames), context.input_events,
              context.focus_events, context.blur_events);
  return context.failed || context.frames < 2 || context.input_events != 3 ||
         context.focus_events != 2 || context.blur_events != 1;
}
