/* Exercise own and inherited native state through the public C ABI. */
#define main fieldset_focus_observations_main
#include "fieldset.c"
#undef main
static const char* state_variants[] = {"all-enabled",   "outer-disabled",      "inner-disabled",
                                       "both-disabled", "optgroup-disabled",   "select-disabled",
                                       "option-own",    "option-move-out",     "option-wrapper",
                                       "legend-hidden", "legend-second-first", "ordinary-disabled"};
static void state_snapshot(State* state,
                           OuiElement* const extra[5],
                           char* out,
                           size_t capacity,
                           size_t* length) {
  const int indexes[] = {0, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 3};
  const char* names[] = {"f", "nf", "first", "second", "a",  "innerfirst", "inner",
                         "b", "s",  "og",    "o",      "o2", "normal"};
  for (size_t i = 0; i < 13; ++i) {
    int index = indexes[i];
    OuiElement* e = index < 12 ? state->elements[index] : extra[index - 12];
    uint32_t connected = 0, effective = 0, own = 0;
    CHECK(oui_element_is_connected_v1(e, &connected));
    CHECK(oui_element_is_effectively_disabled_v1(e, &effective));
    CHECK(oui_element_is_own_disabled_v1(e, &own));
    if (i)
      add(out, capacity, length, ",");
    add(out, capacity, length, "\"%s\":{\"own\":%s,\"property\":", names[i],
        boolean(has_attribute(e, "disabled")));
    if (index == 3)
      add(out, capacity, length, "null");
    else
      add(out, capacity, length, "%s", boolean(own));
    add(out, capacity, length, ",\"effective\":%s,\"enabled\":%s,\"connected\":%s}",
        boolean(effective), boolean(index != 3 && !effective), boolean(connected));
  }
}
int main(void) {
  size_t scenarios = 0;
  printf("[");
  for (int tag = 0; tag < 2; ++tag)
    for (size_t variant = 0; variant < 12; ++variant) {
      State s;
      memset(&s, 0, sizeof(s));
      s.setup = 1;
      make(&s, tag);
      OuiElement* extra[5];
      const char* names[] = {"s", "og", "o", "o2", "wrap"};
      const OuiElementTag kinds[] = {OUI_ELEMENT_SELECT, OUI_ELEMENT_OPTGROUP, OUI_ELEMENT_OPTION,
                                     OUI_ELEMENT_OPTION, OUI_ELEMENT_SPAN};
      for (size_t i = 0; i < 5; ++i) {
        CHECK(oui_element_create(s.document, kinds[i], &extra[i]));
        CHECK(oui_element_set_attribute(extra[i], text("id"), text(names[i])));
      }
      CHECK(oui_element_append_child(s.elements[0], extra[0]));
      CHECK(oui_element_append_child(extra[0], extra[1]));
      CHECK(oui_element_append_child(extra[1], extra[2]));
      CHECK(oui_element_append_child(extra[0], extra[3]));
      CHECK(oui_element_append_child(extra[0], extra[4]));
      barrier(&s);
      if (variant == 7 || variant == 8)
        CHECK(oui_element_set_attribute(extra[1], text("disabled"), text("")));
      char initial[16384] = {0};
      size_t length = 0;
      state_snapshot(&s, extra, initial, sizeof(initial), &length);
      switch (variant) {
        case 0:
          break;
        case 1:
          disable(&s, 0);
          break;
        case 2:
          disable(&s, 4);
          break;
        case 3:
          disable(&s, 0);
          disable(&s, 4);
          break;
        case 4:
          CHECK(oui_element_set_attribute(extra[1], text("disabled"), text("")));
          break;
        case 5:
          CHECK(oui_element_set_attribute(extra[0], text("disabled"), text("")));
          break;
        case 6:
          CHECK(oui_element_set_attribute(extra[2], text("disabled"), text("")));
          break;
        case 7:
          CHECK(oui_element_append_child(extra[0], extra[2]));
          break;
        case 8:
          CHECK(oui_element_append_child(extra[4], extra[2]));
          break;
        case 9: {
          OuiStyleValue display;
          CHECK(oui_style_value_parse(OUI_STYLE_PROPERTY_DISPLAY, text("none"), &display));
          CHECK(oui_element_set_property(s.elements[1], OUI_STYLE_PROPERTY_DISPLAY, &display));
          disable(&s, 0);
          break;
        }
        case 10:
          disable(&s, 0);
          CHECK(oui_element_insert_before(s.elements[0], s.elements[2], s.elements[1]));
          break;
        case 11:
          disable(&s, 3);
          break;
        default:
          abort();
      }
      char synchronous[16384] = {0};
      length = 0;
      state_snapshot(&s, extra, synchronous, sizeof(synchronous), &length);
      barrier(&s);
      char observed[16384] = {0};
      length = 0;
      state_snapshot(&s, extra, observed, sizeof(observed), &length);
      if (scenarios++)
        printf(",");
      printf(
          "{\"scenario\":\"%s-%s\",\"control\":\"%s\",\"variant\":\"%s\",\"initial\":{%s},"
          "\"synchronous\":{%s},\"observed\":{%s}}",
          tag ? "textarea" : "input", state_variants[variant], tag ? "textarea" : "input",
          state_variants[variant], initial, synchronous, observed);
      for (size_t i = 0; i < 5; ++i)
        CHECK(oui_element_destroy(extra[i]));
      for (size_t i = 0; i < 12; ++i)
        CHECK(oui_element_destroy(s.elements[i]));
      CHECK(oui_element_destroy(s.root));
      CHECK(oui_document_destroy(s.document));
    }
  printf("]\n");
  return 0;
}
