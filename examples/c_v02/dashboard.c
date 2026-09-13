#include "common.h"

int main(void) {
  OuiColor color = {103, 58, 183, 255};
  return oui_example_render("Open UI dashboard", color) ? 0 : 1;
}
