#include "openui.h"
static_assert(sizeof(OuiStyleValue) == 16, "frozen OuiStyleValue size");
static_assert(sizeof(OuiEvent) == 88, "frozen OuiEvent size");

int main() {
  return oui_abi_version() == OUI_ABI_VERSION ? 0 : 1;
}
