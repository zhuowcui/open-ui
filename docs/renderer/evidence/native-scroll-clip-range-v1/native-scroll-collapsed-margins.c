#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "openui.h"

static void property(OuiElement* e, OuiStyleProperty id, const char* text) {
    OuiUtf8 s = {(const uint8_t*)text, strlen(text)};
    OuiStyleValue v;
    assert(oui_style_value_parse(id, s, &v) == OUI_OK);
    assert(oui_element_set_property(e, id, &v) == OUI_OK);
    if (v.tag == OUI_STYLE_VALUE_COMPOUND)
        assert(oui_style_compound_destroy((OuiStyleCompound*)v.data.compound) == OUI_OK);
}
static OuiElement* child(OuiDocument* d, OuiElement* parent, const char* height) {
    OuiElement* e = NULL;
    assert(oui_element_create(d, OUI_ELEMENT_DIV, &e) == OUI_OK);
    property(e, OUI_STYLE_PROPERTY_WIDTH, "100px");
    if (height) property(e, OUI_STYLE_PROPERTY_HEIGHT, height);
    assert(oui_element_append_child(parent, e) == OUI_OK);
    return e;
}
static OuiScrollMetricsV1 snapshot(OuiElement* scroller, OuiElement* target,
                                   const char* state) {
    OuiScrollMetricsV1 m = {sizeof(m), OUI_ABI_VERSION, 0, 0, 0, 0};
    uint8_t has_metrics = 0;
    OuiRect bounds;
    double x = 0, y = 0;
    assert(oui_element_get_scroll_metrics_v1(scroller, &m, &has_metrics) == OUI_OK);
    assert(has_metrics == 1);
    assert(oui_element_get_scroll_offset(scroller, &x, &y) == OUI_OK);
    assert(oui_element_get_bounds(target, &bounds) == OUI_OK);
    printf("{\"state\":\"%s\",\"clientWidth\":%.9g,\"clientHeight\":%.9g,\"scrollWidth\":%.9g,\"scrollHeight\":%.9g,\"scrollLeft\":%.9g,\"scrollTop\":%.9g,\"bounds\":{\"x\":%.9g,\"y\":%.9g,\"width\":%.9g,\"height\":%.9g}}",
           state, m.client_width, m.client_height, m.scroll_width, m.scroll_height,
           x, y, bounds.x, bounds.y, bounds.width, bounds.height);
    return m;
}
int main(int argc, char** argv) {
    assert(argc == 3);
    const double scale = atof(argv[1]);
    const int nested = strcmp(argv[2], "nested-collapse") == 0;
    const int empty = strcmp(argv[2], "empty-collapse") == 0;
    assert(nested || empty || strcmp(argv[2], "leaf-margin") == 0);
    OuiDocumentConfig c = {sizeof(c), OUI_ABI_VERSION,
        {640, 480, (uint32_t)(640 * scale), (uint32_t)(480 * scale), scale, OUI_VIEWPORT_LOGICAL, 0}};
    OuiDocument* d = NULL;
    OuiElement *root = NULL, *scroller = NULL, *extra = NULL, *target = NULL;
    assert(oui_document_create(&c, &d) == OUI_OK);
    assert(oui_document_root(d, &root) == OUI_OK);
    property(root, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
    scroller = child(d, root, "100px");
    property(scroller, OUI_STYLE_PROPERTY_POSITION, "absolute");
    property(scroller, OUI_STYLE_PROPERTY_LEFT, "20px");
    property(scroller, OUI_STYLE_PROPERTY_TOP, "20px");
    property(scroller, OUI_STYLE_PROPERTY_OVERFLOW, "scroll");
    property(scroller, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
    if (nested) {
        extra = child(d, scroller, NULL);
        target = child(d, extra, "100px");
    } else if (empty) {
        extra = child(d, scroller, "100px");
        target = child(d, scroller, "0px");
        property(target, OUI_STYLE_PROPERTY_MARGIN_TOP, "20px");
    } else {
        target = child(d, scroller, "100px");
    }
    property(target, OUI_STYLE_PROPERTY_MARGIN_BOTTOM, empty ? "30px" : "20px");
    printf("{\"states\":[");
    const OuiScrollMetricsV1 original = snapshot(scroller, target, "initial");
    const double original_height = original.scroll_height;
    assert(oui_element_scroll_to(scroller, 0, 1000) == OUI_OK);
    printf(",");
    snapshot(scroller, target, "end");
    property(target, OUI_STYLE_PROPERTY_MARGIN_BOTTOM, "0px");
    printf(",");
    snapshot(scroller, target, "shrink");
    assert(original.scroll_height == original_height);
    assert(oui_element_destroy(target) == OUI_OK);
    if (extra) assert(oui_element_destroy(extra) == OUI_OK);
    assert(oui_element_destroy(scroller) == OUI_OK);
    assert(oui_element_destroy(root) == OUI_OK);
    assert(oui_document_destroy(d) == OUI_OK);
    assert(original.scroll_height == original_height);
    printf("],\"ownedSnapshotHeight\":%.9g}\n", original.scroll_height);
    return 0;
}
