#include "common.h"

int main(void) {
  OuiColor color = {76, 175, 80, 255};
  return oui_example_render("Counter: 0", color) ? 0 : 1;
}
