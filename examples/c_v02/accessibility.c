#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "openui.h"

#define CHECK(call)                                           \
  do {                                                        \
    OuiStatus status = (call);                                \
    if (status != OUI_OK) {                                   \
      fprintf(stderr, "%s failed: %d\n", #call, (int)status); \
      failed = 1;                                             \
      goto cleanup;                                           \
    }                                                         \
  } while (0)

static OuiUtf8 text(const char* value) {
  OuiUtf8 result = {(const uint8_t*)value, strlen(value)};
  return result;
}

int main(void) {
  OuiDocumentConfig config = {
      sizeof(config), OUI_ABI_VERSION, {100.0, 100.0, 100, 100, 1.0, OUI_VIEWPORT_LOGICAL, 0}};
  OuiDocument* document = NULL;
  OuiElement* root = NULL;
  OuiElement* button = NULL;
  OuiElement* text_control = NULL;
  OuiAccessibilitySnapshot* first = NULL;
  OuiAccessibilitySnapshot* second = NULL;
  OuiAccessibilitySnapshot* third = NULL;
  OuiAccessibilitySnapshot* read_only = NULL;
  int failed = 0;
  uint64_t root_id = 0;
  uint64_t button_id = 0;
  CHECK(oui_document_create(&config, &document));
  CHECK(oui_document_root(document, &root));
  CHECK(oui_element_create(document, OUI_ELEMENT_BUTTON, &button));
  CHECK(oui_element_append_child(root, button));
  CHECK(oui_element_set_accessibility_label(button, text("Run")));
  CHECK(oui_element_set_accessibility_live(button, OUI_ACCESSIBILITY_LIVE_POLITE));
  CHECK(oui_element_get_accessibility_id(root, &root_id));
  CHECK(oui_element_get_accessibility_id(button, &button_id));
  CHECK(oui_document_accessibility_snapshot(document, NULL, &first));

  OuiAccessibilitySnapshotInfo snapshot_info = {.struct_size = sizeof(snapshot_info),
                                                .abi_version = OUI_ABI_VERSION};
  CHECK(oui_accessibility_snapshot_get_info(first, &snapshot_info));
  if (!snapshot_info.full_tree || snapshot_info.node_count < 2 ||
      snapshot_info.changed_count != snapshot_info.node_count) {
    failed = 1;
    goto cleanup;
  }
  int found_button = 0;
  for (size_t index = 0; index < snapshot_info.node_count; ++index) {
    OuiAccessibilityNodeInfo node = {.struct_size = sizeof(node), .abi_version = OUI_ABI_VERSION};
    CHECK(oui_accessibility_snapshot_get_node(first, index, &node));
    if (node.id == button_id) {
      found_button = node.role == OUI_ACCESSIBILITY_ROLE_BUTTON && node.label_length == 3 &&
                     (node.actions & OUI_ACCESSIBILITY_ACTION_CLICK_BIT) != 0;
    }
  }
  if (!found_button) {
    failed = 1;
    goto cleanup;
  }
  OuiAccessibilityNodeState state = {.struct_size = sizeof(state), .abi_version = OUI_ABI_VERSION};
  CHECK(oui_accessibility_snapshot_get_node_state(first, button_id, &state));
  if (state.live != OUI_ACCESSIBILITY_LIVE_POLITE + 1) {
    failed = 1;
    goto cleanup;
  }
  uint64_t child_id = 0;
  size_t child_count = 0;
  CHECK(oui_accessibility_snapshot_copy_ids(first, root_id, OUI_ACCESSIBILITY_IDS_CHILDREN,
                                            &child_id, 1, &child_count));
  if (child_count != 1 || child_id != button_id) {
    failed = 1;
    goto cleanup;
  }
  uint8_t label[3];
  size_t length = 0;
  CHECK(oui_accessibility_snapshot_copy_text(first, button_id, OUI_ACCESSIBILITY_TEXT_LABEL, NULL,
                                             0, &length));
  if (length != 3 ||
      oui_accessibility_snapshot_copy_text(first, button_id, OUI_ACCESSIBILITY_TEXT_LABEL, label, 2,
                                           &length) != OUI_ERROR_BUFFER_TOO_SMALL) {
    failed = 1;
    goto cleanup;
  }
  CHECK(oui_accessibility_snapshot_copy_text(first, button_id, OUI_ACCESSIBILITY_TEXT_LABEL, label,
                                             sizeof(label), &length));
  if (length != 3 || memcmp(label, "Run", 3) != 0) {
    failed = 1;
    goto cleanup;
  }

  CHECK(oui_element_set_accessibility_label(button, text("Go")));
  CHECK(oui_document_accessibility_snapshot(document, first, &second));
  CHECK(oui_accessibility_snapshot_get_info(second, &snapshot_info));
  if (snapshot_info.full_tree || snapshot_info.changed_count != 1 ||
      snapshot_info.removed_count != 0) {
    failed = 1;
    goto cleanup;
  }
  uint64_t changed_id = 0;
  size_t changed_count = 0;
  CHECK(oui_accessibility_snapshot_copy_changes(second, 0, &changed_id, 1, &changed_count));
  if (changed_count != 1 || changed_id != button_id) {
    failed = 1;
    goto cleanup;
  }

  CHECK(oui_element_remove(button));
  CHECK(oui_document_accessibility_snapshot(document, second, &third));
  CHECK(oui_accessibility_snapshot_get_info(third, &snapshot_info));
  if (snapshot_info.removed_count != 1) {
    failed = 1;
    goto cleanup;
  }
  uint64_t removed_id = 0;
  size_t removed_count = 0;
  CHECK(oui_accessibility_snapshot_copy_changes(third, 1, &removed_id, 1, &removed_count));
  if (removed_count != 1 || removed_id != button_id) {
    failed = 1;
    goto cleanup;
  }

  const OuiElementTag text_kinds[] = {OUI_ELEMENT_INPUT, OUI_ELEMENT_TEXTAREA};
  for (size_t kind = 0; kind < sizeof(text_kinds) / sizeof(text_kinds[0]); ++kind) {
    CHECK(oui_element_create(document, text_kinds[kind], &text_control));
    CHECK(oui_element_append_child(root, text_control));
    CHECK(oui_element_set_control_value(text_control, text("original")));
    CHECK(oui_element_set_attribute(text_control, text("readonly"), text("")));
    CHECK(oui_element_focus(text_control));
    OuiEvent key = {.struct_size = sizeof(key),
                    .abi_version = OUI_ABI_VERSION,
                    .event_type = OUI_EVENT_KEY_DOWN,
                    .key_code = 13,
                    .text = text("Enter")};
    CHECK(oui_document_dispatch_key_input_v1(document, &key, text("\r")));
    CHECK(oui_document_dispatch_text_input_v1(document, text("blocked")));
    if (oui_element_perform_accessibility_action(text_control, OUI_ACCESSIBILITY_SET_VALUE,
                                                 text("blocked"), 0, 0) == OUI_OK ||
        oui_element_perform_accessibility_action(text_control,
                                                 OUI_ACCESSIBILITY_REPLACE_SELECTED_TEXT,
                                                 text("blocked"), 0, 0) == OUI_OK) {
      failed = 1;
      goto cleanup;
    }
    CHECK(oui_element_perform_accessibility_action(
        text_control, OUI_ACCESSIBILITY_SET_TEXT_SELECTION, text(""), 0, 8));
    uint8_t value[8];
    CHECK(oui_element_copy_control_value(text_control, value, sizeof(value), &length));
    if (length != sizeof(value) || memcmp(value, "original", sizeof(value)) != 0) {
      failed = 1;
      goto cleanup;
    }
    uint64_t control_id = 0;
    CHECK(oui_element_get_accessibility_id(text_control, &control_id));
    CHECK(oui_document_accessibility_snapshot(document, NULL, &read_only));
    CHECK(oui_accessibility_snapshot_get_info(read_only, &snapshot_info));
    int found_control = 0;
    for (size_t index = 0; index < snapshot_info.node_count; ++index) {
      OuiAccessibilityNodeInfo node = {.struct_size = sizeof(node), .abi_version = OUI_ABI_VERSION};
      CHECK(oui_accessibility_snapshot_get_node(read_only, index, &node));
      if (node.id == control_id) {
        found_control = (node.flags & OUI_ACCESSIBILITY_NODE_READ_ONLY) != 0 &&
                        (node.actions & OUI_ACCESSIBILITY_ACTION_SET_TEXT_SELECTION_BIT) != 0 &&
                        (node.actions & (OUI_ACCESSIBILITY_ACTION_SET_VALUE_BIT |
                                         OUI_ACCESSIBILITY_ACTION_REPLACE_SELECTED_TEXT_BIT)) == 0;
      }
    }
    if (!found_control) {
      failed = 1;
      goto cleanup;
    }
    CHECK(oui_element_set_control_value(text_control, text("native")));
    CHECK(oui_accessibility_snapshot_destroy(read_only));
    read_only = NULL;
    CHECK(oui_element_remove(text_control));
    CHECK(oui_element_destroy(text_control));
    text_control = NULL;
  }
  CHECK(oui_document_destroy(document));
  document = NULL;
  CHECK(oui_accessibility_snapshot_copy_text(first, button_id, OUI_ACCESSIBILITY_TEXT_LABEL, label,
                                             sizeof(label), &length));
  if (length != 3 || memcmp(label, "Run", 3) != 0) {
    failed = 1;
    goto cleanup;
  }
  puts("Owned accessibility snapshots and incremental changes: OK");

cleanup:
  if (read_only)
    oui_accessibility_snapshot_destroy(read_only);
  if (text_control)
    oui_element_destroy(text_control);
  if (third)
    oui_accessibility_snapshot_destroy(third);
  if (second)
    oui_accessibility_snapshot_destroy(second);
  if (first)
    oui_accessibility_snapshot_destroy(first);
  if (button)
    oui_element_destroy(button);
  if (root)
    oui_element_destroy(root);
  if (document)
    oui_document_destroy(document);
  return failed;
}
