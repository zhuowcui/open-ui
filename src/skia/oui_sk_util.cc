// src/skia/oui_sk_util.cc — Utility functions

#include "include/openui/openui_skia.h"

extern "C" {

OuiSkColor oui_sk_color_make(uint8_t r, uint8_t g, uint8_t b, uint8_t a) {
    OuiSkColor c = {r, g, b, a};
    return c;
}

OuiSkColor4f oui_sk_color4f_make(float r, float g, float b, float a) {
    OuiSkColor4f c = {r, g, b, a};
    return c;
}

OuiSkRect oui_sk_rect_make(float l, float t, float r, float b) {
    OuiSkRect rect = {l, t, r, b};
    return rect;
}

OuiSkRect oui_sk_rect_make_xywh(float x, float y, float w, float h) {
    OuiSkRect rect = {x, y, x + w, y + h};
    return rect;
}

OuiSkRRect oui_sk_rrect_make(OuiSkRect rect, float rx, float ry) {
    OuiSkRRect rrect;
    rrect.rect = rect;
    rrect.rx = rx;
    rrect.ry = ry;
    return rrect;
}

OuiSkMatrix oui_sk_matrix_identity(void) {
    OuiSkMatrix m;
    m.values[0] = 1; m.values[1] = 0; m.values[2] = 0;
    m.values[3] = 0; m.values[4] = 1; m.values[5] = 0;
    m.values[6] = 0; m.values[7] = 0; m.values[8] = 1;
    return m;
}

const char* oui_sk_status_string(OuiSkStatus status) {
    switch (status) {
        case OUI_SK_OK: return "OK";
        case OUI_SK_ERROR_INVALID_ARGUMENT: return "Invalid argument";
        case OUI_SK_ERROR_NULL_POINTER: return "Null pointer";
        case OUI_SK_ERROR_OUT_OF_MEMORY: return "Out of memory";
        case OUI_SK_ERROR_GPU_INIT_FAILED: return "GPU initialization failed";
        case OUI_SK_ERROR_SURFACE_CREATION_FAILED: return "Surface creation failed";
        case OUI_SK_ERROR_ENCODE_FAILED: return "Encode failed";
        case OUI_SK_ERROR_DECODE_FAILED: return "Decode failed";
        case OUI_SK_ERROR_FILE_NOT_FOUND: return "File not found";
        case OUI_SK_ERROR_FONT_NOT_FOUND: return "Font not found";
        case OUI_SK_ERROR_BACKEND_NOT_AVAILABLE: return "Backend not available";
        case OUI_SK_ERROR_WINDOW_CREATION_FAILED: return "Window creation failed";
        case OUI_SK_ERROR_UNKNOWN: return "Unknown error";
        default: return "Unknown status";
    }
}

}  // extern "C"
