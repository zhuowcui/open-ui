#include "openui.h"
static_assert(sizeof(OuiStyleValue) == 16, "frozen OuiStyleValue size");
static_assert(sizeof(OuiEvent) == 88, "frozen OuiEvent size");
static_assert(sizeof(OuiAccessibilitySnapshotInfo) == 56, "snapshot info size");
static_assert(sizeof(OuiAccessibilityNodeInfo) == 120, "snapshot node size");
static_assert(sizeof(OuiAccessibilityNodeState) == 112, "snapshot node state size");

int main() {
  return oui_abi_version() == OUI_ABI_VERSION ? 0 : 1;
}
