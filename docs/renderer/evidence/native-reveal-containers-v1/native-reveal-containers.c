#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "openui.h"

static void property(OuiElement* element, OuiStyleProperty id, const char* literal) {
    OuiUtf8 text = {(const uint8_t*)literal, strlen(literal)};
    OuiStyleValue value;
    assert(oui_style_value_parse(id, text, &value) == OUI_OK);
    assert(oui_element_set_property(element, id, &value) == OUI_OK);
    if (value.tag == OUI_STYLE_VALUE_COMPOUND)
        assert(oui_style_compound_destroy((OuiStyleCompound*)value.data.compound) == OUI_OK);
}

static OuiElement* block(OuiDocument* document, OuiElement* parent, const char* width, const char* height) {
    OuiElement* element = NULL;
    assert(oui_element_create(document, OUI_ELEMENT_DIV, &element) == OUI_OK);
    property(element, OUI_STYLE_PROPERTY_WIDTH, width);
    property(element, OUI_STYLE_PROPERTY_HEIGHT, height);
    property(element, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
    assert(oui_element_append_child(parent, element) == OUI_OK);
    return element;
}

static void clicked(OuiEvent* event, void* user_data) {
    assert(event->event_type == OUI_EVENT_CLICK);
    assert(oui_element_scroll_into_view_v1(event->target, OUI_SCROLL_NEAREST, OUI_SCROLL_NEAREST,
                                          OUI_SCROLL_CONTAINERS_ALL) == OUI_OK);
    ++*(unsigned*)user_data;
}

static void snapshot(OuiDocument* document, OuiElement* elements[4], const char* directory,
                     const char* state, unsigned calls, FILE* geometry) {
    char path[4096];
    assert(snprintf(path, sizeof(path), "%s/%s.png", directory, state) < (int)sizeof(path));
    OuiBuffer* image = NULL;
    assert(oui_document_render_png(document, &image) == OUI_OK);
    FILE* output = fopen(path, "wb");
    assert(output);
    const size_t size = oui_buffer_length(image);
    assert(fwrite(oui_buffer_data(image), 1, size, output) == size);
    assert(fclose(output) == 0);
    assert(oui_buffer_destroy(image) == OUI_OK);
    fprintf(geometry, "{\"state\":\"%s\",\"callback_count\":%u,\"nodes\":{", state, calls);
    const char* names[4] = {"root", "outer", "inner", "target"};
    for (unsigned index = 0; index != 4; ++index) {
        OuiElement* element = elements[index];
        OuiScrollMetricsV1 metrics = {sizeof(metrics), OUI_ABI_VERSION, 0, 0, 0, 0};
        uint8_t has_metrics = 0;
        assert(oui_element_get_scroll_metrics_v1(element, &metrics, &has_metrics) == OUI_OK);
        assert(has_metrics);
        double x, y;
        assert(oui_element_get_scroll_offset(element, &x, &y) == OUI_OK);
        fprintf(geometry, "%s\"%s\":{", index ? "," : "", names[index]);
        if (index) {
            OuiRect bounds;
            assert(oui_element_get_bounds(element, &bounds) == OUI_OK);
            fprintf(geometry, "\"bounds\":{\"x\":%.9g,\"y\":%.9g,\"width\":%.9g,\"height\":%.9g},",
                    bounds.x, bounds.y, bounds.width, bounds.height);
        }
        fprintf(geometry, "\"scroll\":{\"x\":%.9g,\"y\":%.9g},\"client\":{\"width\":%.9g,\"height\":%.9g},\"extent\":{\"width\":%.9g,\"height\":%.9g}}",
                x, y, metrics.client_width, metrics.client_height, metrics.scroll_width, metrics.scroll_height);
    }
    fprintf(geometry, "}}");
}

int main(int argc, char** argv) {
    assert(argc == 4);
    const char* directory = argv[1];
    const double scale = atof(argv[2]);
    const char* mode = argv[3];
    assert(!strcmp(mode, "flow") || !strcmp(mode, "absolute-inner") || !strcmp(mode, "absolute-outer")
           || !strcmp(mode, "absolute-root") || !strcmp(mode, "fixed-root") || !strcmp(mode, "fixed-transform"));
    OuiDocumentConfig config = {sizeof(config), OUI_ABI_VERSION,
        {320, 240, (uint32_t)(320 * scale), (uint32_t)(240 * scale), scale, OUI_VIEWPORT_LOGICAL, 0}};
    OuiDocument* document = NULL;
    OuiElement* root = NULL;
    assert(oui_document_create(&config, &document) == OUI_OK);
    assert(oui_document_root(document, &root) == OUI_OK);
    property(root, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "white");
    property(root, OUI_STYLE_PROPERTY_SCROLLBAR_WIDTH, "none");
    OuiElement* outer = block(document, root, "100px", "80px");
    if (!strcmp(mode, "absolute-root")) {
        property(outer, OUI_STYLE_PROPERTY_MARGIN_LEFT, "40px");
        property(outer, OUI_STYLE_PROPERTY_MARGIN_TOP, "30px");
    } else {
        property(outer, OUI_STYLE_PROPERTY_POSITION, "absolute");
        property(outer, OUI_STYLE_PROPERTY_LEFT, "40px");
        property(outer, OUI_STYLE_PROPERTY_TOP, "30px");
    }
    if (!strcmp(mode, "fixed-transform")) {
        OuiTransformOperation operation = {0};
        operation.kind = OUI_TRANSFORM_TRANSLATE;
        operation.x = (OuiLength){10, OUI_LENGTH_PX};
        operation.y = (OuiLength){6, OUI_LENGTH_PX};
        OuiStyleCompound* compound = NULL;
        assert(oui_transform_create(&operation, 1, &compound) == OUI_OK);
        OuiStyleValue value = {0};
        value.tag = OUI_STYLE_VALUE_COMPOUND;
        value.data.compound = compound;
        assert(oui_element_set_property(outer, OUI_STYLE_PROPERTY_TRANSFORM, &value) == OUI_OK);
        assert(oui_style_compound_destroy(compound) == OUI_OK);
    }
    property(outer, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
    OuiElement* spacer = block(document, outer, "200px", "200px");
    OuiElement* inner = block(document, outer, "200px", "100px");
    property(inner, OUI_STYLE_PROPERTY_OVERFLOW, "hidden");
    if (!strcmp(mode, "absolute-inner")) property(inner, OUI_STYLE_PROPERTY_POSITION, "relative");
    OuiElement* filler = strcmp(mode, "flow") ? block(document, inner, "400px", "400px") : NULL;
    OuiElement* target = block(document, inner, "20px", "20px");
    property(target, OUI_STYLE_PROPERTY_BACKGROUND_COLOR, "green");
    if (!strcmp(mode, "flow")) {
        property(target, OUI_STYLE_PROPERTY_MARGIN_LEFT, "150px");
        property(target, OUI_STYLE_PROPERTY_MARGIN_TOP, "120px");
    } else {
        property(target, OUI_STYLE_PROPERTY_POSITION, !strncmp(mode, "fixed-", 6) ? "fixed" : "absolute");
        const int inner_container = !strcmp(mode, "absolute-inner");
        const int viewport_container = !strcmp(mode, "absolute-root") || !strcmp(mode, "fixed-root");
        property(target, OUI_STYLE_PROPERTY_LEFT, inner_container ? "150px" : viewport_container ? "280px" : "250px");
        property(target, OUI_STYLE_PROPERTY_TOP, inner_container ? "120px" : viewport_container ? "200px" : "180px");
    }
    unsigned calls = 0;
    OuiListener* listener = NULL;
    assert(oui_element_add_event_listener(target, OUI_EVENT_CLICK, 0, clicked, &calls, &listener) == OUI_OK);
    char path[4096];
    assert(snprintf(path, sizeof(path), "%s/geometry.json", directory) < (int)sizeof(path));
    FILE* geometry = fopen(path, "w");
    assert(geometry);
    OuiElement* elements[4] = {root, outer, inner, target};
    fprintf(geometry, "[");
    snapshot(document, elements, directory, "initial", calls, geometry);
    OuiEvent event = {0};
    event.struct_size = sizeof(event);
    event.abi_version = OUI_ABI_VERSION;
    event.event_type = OUI_EVENT_CLICK;
    assert(oui_document_dispatch_event(document, target, &event) == OUI_OK);
    assert(calls == 1);
    fprintf(geometry, ",");
    snapshot(document, elements, directory, "reveal", calls, geometry);
    fprintf(geometry, "]\n");
    assert(fclose(geometry) == 0);
    assert(oui_listener_destroy(listener) == OUI_OK);
    assert(oui_element_destroy(target) == OUI_OK);
    if (filler) assert(oui_element_destroy(filler) == OUI_OK);
    assert(oui_element_destroy(inner) == OUI_OK);
    assert(oui_element_destroy(spacer) == OUI_OK);
    assert(oui_element_destroy(outer) == OUI_OK);
    assert(oui_element_destroy(root) == OUI_OK);
    assert(oui_document_destroy(document) == OUI_OK);
    return 0;
}
