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
static void px(OuiElement* e, OuiStyleProperty id, double value) {
    char text[64];
    snprintf(text, sizeof(text), "%.9gpx", value);
    property(e, id, text);
}
int main(int argc, char** argv) {
    assert(argc == 6);
    const double scale = atof(argv[2]), phase = atof(argv[4]);
    const int bordered = strcmp(argv[3], "white-bordered") == 0;
    assert(bordered || strcmp(argv[3], "green-bare") == 0);
    OuiDocumentConfig config = {sizeof(config), OUI_ABI_VERSION,
        {640, 480, (uint32_t)(640 * scale), (uint32_t)(480 * scale), scale, OUI_VIEWPORT_LOGICAL, 0}};
    OuiDocument* d = NULL;
    OuiElement *root = NULL, *scroller = NULL, *child = NULL;
    assert(oui_document_create(&config, &d) == OUI_OK);
    assert(oui_document_root(d, &root) == OUI_OK);
    property(root, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "white");
    property(root, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
    assert(oui_element_create(d, OUI_ELEMENT_DIV, &scroller) == OUI_OK);
    property(scroller, OUI_STYLE_PROPERTY_POSITION, "absolute");
    px(scroller, OUI_STYLE_PROPERTY_LEFT, 20 + phase);
    px(scroller, OUI_STYLE_PROPERTY_TOP, (bordered ? 31 : 46) + phase);
    px(scroller, OUI_STYLE_PROPERTY_WIDTH, bordered ? 160 : 600);
    px(scroller, OUI_STYLE_PROPERTY_HEIGHT, bordered ? 50 : 300);
    property(scroller, OUI_STYLE_PROPERTY_OVERFLOW, "scroll");
    property(scroller, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
    if (bordered) {
        property(scroller, OUI_STYLE_PROPERTY_BORDER, "5px solid black");
    }
    assert(oui_element_append_child(root, scroller) == OUI_OK);
    assert(oui_element_create(d, OUI_ELEMENT_DIV, &child) == OUI_OK);
    px(child, OUI_STYLE_PROPERTY_WIDTH, bordered ? 160 : 600);
    px(child, OUI_STYLE_PROPERTY_HEIGHT, bordered ? 5000 : 300);
    property(child, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, bordered ? "white" : "green");
    if (!bordered) px(child, OUI_STYLE_PROPERTY_MARGIN_BOTTOM, 20);
    assert(oui_element_append_child(scroller, child) == OUI_OK);
    OuiRect bounds;
    assert(oui_element_get_bounds(child, &bounds) == OUI_OK);
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
    fprintf(file, "{\"x\":%.9g,\"y\":%.9g,\"width\":%.9g,\"height\":%.9g}\n",
        bounds.x, bounds.y, bounds.width, bounds.height);
    assert(fclose(file) == 0);
    assert(oui_element_destroy(child) == OUI_OK);
    assert(oui_element_destroy(scroller) == OUI_OK);
    assert(oui_element_destroy(root) == OUI_OK);
    assert(oui_document_destroy(d) == OUI_OK);
    return 0;
}
