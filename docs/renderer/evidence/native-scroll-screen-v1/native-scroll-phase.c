#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <math.h>
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
static void px(OuiElement* e, OuiStyleProperty id, double value) {
    char text[64];
    snprintf(text, sizeof(text), "%.9gpx", value);
    property(e, id, text);
}
static OuiElement* block(OuiDocument* d, OuiElement* parent, double width, double height) {
    OuiElement* e = NULL;
    assert(oui_element_create(d, OUI_ELEMENT_DIV, &e) == OUI_OK);
    px(e, OUI_STYLE_PROPERTY_WIDTH, width);
    px(e, OUI_STYLE_PROPERTY_HEIGHT, height);
    property(e, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
    assert(oui_element_append_child(parent, e) == OUI_OK);
    return e;
}
int main(int argc, char** argv) {
    assert(argc == 6);
    const double scale = atof(argv[2]), phase = atof(argv[4]);
    const char* mode = argv[3];
    OuiDocumentConfig config = {sizeof(config), OUI_ABI_VERSION,
        {320, 240, (uint32_t)(320 * scale), (uint32_t)(240 * scale), scale, OUI_VIEWPORT_LOGICAL, 0}};
    OuiDocument* d = NULL;
    OuiElement *root = NULL, *outer = NULL, *inner = NULL, *spacer = NULL, *target;
    assert(oui_document_create(&config, &d) == OUI_OK);
    assert(oui_document_root(d, &root) == OUI_OK);
    property(root, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "white");
    property(root, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
    if (strcmp(mode, "flat") == 0) {
        target = block(d, root, 20, 20);
        property(target, OUI_STYLE_PROPERTY_POSITION, "absolute");
        px(target, OUI_STYLE_PROPERTY_LEFT, 120 + phase);
        px(target, OUI_STYLE_PROPERTY_TOP, 90 + phase);
    } else {
        outer = block(d, root, 100, 80);
        property(outer, OUI_STYLE_PROPERTY_POSITION, "absolute");
        px(outer, OUI_STYLE_PROPERTY_LEFT, 40 + phase);
        px(outer, OUI_STYLE_PROPERTY_TOP, 30 + phase);
        property(outer, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
        if (strcmp(mode, "static") == 0) {
            target = block(d, outer, 20, 20);
            px(target, OUI_STYLE_PROPERTY_MARGIN_LEFT, 80);
            px(target, OUI_STYLE_PROPERTY_MARGIN_TOP, 60);
        } else if (strcmp(mode, "clipped") == 0) {
            inner = block(d, outer, 200, 100);
            property(inner, OUI_STYLE_PROPERTY_POSITION, "relative");
            px(inner, OUI_STYLE_PROPERTY_LEFT, -70);
            px(inner, OUI_STYLE_PROPERTY_TOP, -20);
            property(inner, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
            target = block(d, inner, 20, 20);
            px(target, OUI_STYLE_PROPERTY_MARGIN_LEFT, 150);
            px(target, OUI_STYLE_PROPERTY_MARGIN_TOP, 80);
        } else {
            assert(strcmp(mode, "scrolled") == 0 || strcmp(mode, "clip") == 0);
            spacer = block(d, outer, 200, 200);
            inner = block(d, outer, 200, 100);
            property(inner, OUI_STYLE_PROPERTY_OVERFLOW, strcmp(mode, "clip") == 0 ? "clip" : "hidden");
            target = block(d, inner, 20, 20);
            px(target, OUI_STYLE_PROPERTY_MARGIN_LEFT, 150);
            px(target, OUI_STYLE_PROPERTY_MARGIN_TOP, 120);
            if (strcmp(mode, "scrolled") == 0)
                assert(oui_element_scroll_to(inner, 0, 40) == OUI_OK);
            assert(oui_element_scroll_to(outer, 70, strcmp(mode, "clip") == 0 ? 260 : 220) == OUI_OK);
        }
    }
    property(target, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "green");
    OuiRect bounds;
    assert(oui_element_get_bounds(target, &bounds) == OUI_OK);
    OuiBuffer* buffer = NULL;
    assert(oui_document_render_png(d, &buffer) == OUI_OK);
    FILE* file = fopen(argv[1], "wb");
    assert(file);
    size_t size = oui_buffer_length(buffer);
    assert(fwrite(oui_buffer_data(buffer), 1, size, file) == size);
    assert(fclose(file) == 0);
    assert(oui_buffer_destroy(buffer) == OUI_OK);
    file = fopen(argv[5], "w");
    assert(file);
    fprintf(file, "{\"x\":%.9g,\"y\":%.9g,\"width\":%.9g,\"height\":%.9g}\n", bounds.x, bounds.y, bounds.width, bounds.height);
    assert(fclose(file) == 0);
    assert(oui_element_destroy(target) == OUI_OK);
    if (inner) assert(oui_element_destroy(inner) == OUI_OK);
    if (spacer) assert(oui_element_destroy(spacer) == OUI_OK);
    if (outer) assert(oui_element_destroy(outer) == OUI_OK);
    assert(oui_element_destroy(root) == OUI_OK);
    assert(oui_document_destroy(d) == OUI_OK);
    return 0;
}
