/* Keep consumer checks active when the SDK is built with NDEBUG. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "openui.h"

static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}

static OuiEditCommandV1 command(uint32_t kind) {
  OuiEditCommandV1 result = {sizeof(result), OUI_ABI_VERSION, kind, 0, 0, 0, {0, 0}};
  return result;
}

static void value_is(OuiElement* element, const char* expected) {
  uint8_t bytes[64];
  size_t length = 0;
  assert(oui_element_copy_control_value(element, bytes, sizeof(bytes), &length) == OUI_OK);
  assert(length == strlen(expected) && memcmp(bytes, expected, length) == 0);
}

static void selection_is(OuiElement* element, size_t start, size_t end) {
  size_t actual_start = 0;
  size_t actual_end = 0;
  assert(oui_element_get_selection(element, &actual_start, &actual_end) == OUI_OK);
  assert(actual_start == start && actual_end == end);
}

typedef struct Context {
  OuiElement* input;
  OuiElement* sibling;
  unsigned input_events;
  unsigned click_events;
} Context;

static void on_input(OuiEvent* event, void* user_data) {
  Context* context = (Context*)user_data;
  assert(event->event_type == OUI_EVENT_INPUT);
  assert(event->target == context->input && event->current_target == context->input);
  ++context->input_events;
  value_is(context->input, context->input_events == 2 ? "á👩‍💻z" : "á👩‍💻");
  /* Reenter the retained document while an input callback is running. */
  assert(oui_element_set_text(context->sibling, text("native input callback")) == OUI_OK);
}

static void on_click(OuiEvent* event, void* user_data) {
  Context* context = (Context*)user_data;
  assert(event->event_type == OUI_EVENT_CLICK);
  ++context->click_events;
  OuiEditCommandV1 edit = command(OUI_EDIT_DELETE);
  assert(oui_element_edit_text_v1(context->input, &edit) == OUI_OK);
}

static void exercise(uint32_t kind) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {320.0, 200.0, 320, 200, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* button = NULL;
  OuiListener* input_listener = NULL;
  OuiListener* click_listener = NULL;
  Context context = {NULL, NULL, 0, 0};
  assert(oui_document_create(&config, &document) == OUI_OK);
  assert(oui_document_root(document, &root) == OUI_OK);
  assert(oui_element_create(document, (OuiElementTag)kind, &context.input) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_DIV, &context.sibling) == OUI_OK);
  assert(oui_element_create(document, OUI_ELEMENT_BUTTON, &button) == OUI_OK);
  assert(oui_element_append_child(root, context.input) == OUI_OK);
  assert(oui_element_append_child(root, context.sibling) == OUI_OK);
  assert(oui_element_append_child(root, button) == OUI_OK);
  assert(oui_element_set_control_value(context.input, text("á👩‍💻z")) == OUI_OK);
  assert(oui_element_set_selection(context.input, 15, 15) == OUI_OK);
  assert(oui_element_add_event_listener(context.input, OUI_EVENT_INPUT, 0, on_input, &context,
                                        &input_listener) == OUI_OK);
  assert(oui_element_add_event_listener(button, OUI_EVENT_CLICK, 0, on_click, &context,
                                        &click_listener) == OUI_OK);
  OuiEvent click = {sizeof(click),
                    OUI_ABI_VERSION,
                    OUI_EVENT_CLICK,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    {NULL, 0},
                    NULL,
                    NULL};
  assert(oui_document_dispatch_event(document, button, &click) == OUI_OK);
  OuiEditCommandV1 edit = command(OUI_EDIT_UNDO);
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_OK);
  edit = command(OUI_EDIT_REDO);
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_OK);
  assert(context.input_events == 3 && context.click_events == 1);
  value_is(context.input, "á👩‍💻");

  edit = command(OUI_EDIT_MOVE);
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_OK);
  selection_is(context.input, 3, 3);
  edit.extend_selection = 1;
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_OK);
  selection_is(context.input, 0, 3);
  edit = command(OUI_EDIT_SELECT_ALL);
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_OK);
  selection_is(context.input, 0, 14);
  edit = command(OUI_EDIT_MOVE);
  edit.direction = OUI_TEXT_FORWARD;
  edit.unit = OUI_TEXT_DOCUMENT;
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_OK);
  selection_is(context.input, 14, 14);

  assert(oui_element_set_control_value(context.input, text("one two three")) == OUI_OK);
  assert(oui_element_set_selection(context.input, 13, 13) == OUI_OK);
  edit.direction = OUI_TEXT_BACKWARD;
  edit.unit = OUI_TEXT_WORD;
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_OK);
  selection_is(context.input, 8, 8);
  const char* lines = kind == OUI_ELEMENT_TEXTAREA ? "one\ntwo" : "one two";
  assert(oui_element_set_control_value(context.input, text(lines)) == OUI_OK);
  assert(oui_element_set_selection(context.input, 7, 7) == OUI_OK);
  edit.unit = OUI_TEXT_LINE;
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_OK);
  size_t line_start = kind == OUI_ELEMENT_TEXTAREA ? 4 : 0;
  selection_is(context.input, line_start, line_start);
  assert(context.input_events == 3);
  assert(oui_element_set_attribute(context.input, text("readonly"), text("")) == OUI_OK);
  edit = command(OUI_EDIT_SELECT_ALL);
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_OK);
  edit = command(OUI_EDIT_DELETE);
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_ERROR_INVALID_STATE);
  edit.reserved[1] = 1;
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_ERROR_INVALID_ARGUMENT);
  edit = command(OUI_EDIT_SELECT_ALL);
  assert(oui_element_set_attribute(context.input, text("disabled"), text("")) == OUI_OK);
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_ERROR_INVALID_STATE);
  assert(oui_listener_destroy(input_listener) == OUI_OK);
  assert(oui_listener_destroy(click_listener) == OUI_OK);
  assert(oui_element_remove(context.input) == OUI_OK);
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_ERROR_STALE_HANDLE);
  assert(oui_element_destroy(context.input) == OUI_OK);
  assert(oui_element_edit_text_v1(context.input, &edit) == OUI_ERROR_INVALID_HANDLE);
  assert(oui_element_destroy(button) == OUI_OK);
  assert(oui_element_destroy(context.sibling) == OUI_OK);
  assert(oui_element_destroy(root) == OUI_OK);
  assert(oui_document_destroy(document) == OUI_OK);
  printf(
      "native editing: kind=%u input=%u click=%u unicode, history, selection and teardown passed\n",
      kind, context.input_events, context.click_events);
}

int main(void) {
  exercise(OUI_ELEMENT_INPUT);
  exercise(OUI_ELEMENT_TEXTAREA);
  return 0;
}
