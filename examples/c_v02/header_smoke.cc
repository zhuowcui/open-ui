#include "openui.h"
static_assert(sizeof(OuiEditCommandV1) == 32, "editing command size");
static_assert(alignof(OuiEditCommandV1) == 4, "editing command alignment");
static_assert(offsetof(OuiEditCommandV1, command) == 8, "editing command header");
static_assert(offsetof(OuiEditCommandV1, reserved) == 24, "editing command reserved fields");
static_assert(sizeof(OuiStyleValue) == 16, "frozen OuiStyleValue size");
static_assert(sizeof(OuiEvent) == 88, "frozen OuiEvent size");
static_assert(sizeof(OuiScrollMetricsV1) == 40, "scroll metrics size");
static_assert(alignof(OuiScrollMetricsV1) == 8, "scroll metrics alignment");
static_assert(offsetof(OuiScrollMetricsV1, client_width) == 8, "scroll metrics prefix");
static_assert(offsetof(OuiScrollMetricsV1, scroll_height) == 32, "scroll metrics fields");
static_assert(sizeof(OuiAccessibilitySnapshotInfo) == 56, "snapshot info size");
static_assert(sizeof(OuiAccessibilityNodeInfo) == 120, "snapshot node size");
static_assert(sizeof(OuiAccessibilityNodeState) == 112, "snapshot node state size");

int main() {
  return oui_abi_version() == OUI_ABI_VERSION ? 0 : 1;
}
