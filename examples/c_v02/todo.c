#include "common.h"

int main(void) {
  OuiColor color = {255, 152, 0, 255};
  return oui_example_render("Todo: ship Open UI v0.2", color) ? 0 : 1;
}
