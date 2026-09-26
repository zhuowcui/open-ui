#include "common.h"

int main(void) {
  OuiColor color = {33, 150, 243, 255};
  return oui_example_render("Hello from Open UI", color) ? 0 : 1;
}
