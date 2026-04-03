// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui.h — Stable C API for Chromium's rendering pipeline.
// Any language with C FFI can create element trees, set CSS properties,
// trigger layout, and query geometry through this interface.

#ifndef OPENUI_OPENUI_H_
#define OPENUI_OPENUI_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ─── Export macro ───────────────────────────────────────────
#ifdef _WIN32
#ifdef OUI_BUILD
#define OUI_EXPORT __declspec(dllexport)
#else
#define OUI_EXPORT __declspec(dllimport)
#endif
#else
#define OUI_EXPORT __attribute__((visibility("default")))
#endif

// ─── Opaque handles ─────────────────────────────────────────
typedef struct OuiDocument OuiDocument;
typedef struct OuiElement OuiElement;

// ─── Status codes ───────────────────────────────────────────
typedef enum {
  OUI_OK = 0,
  OUI_ERROR_NOT_INITIALIZED = -1,
  OUI_ERROR_ALREADY_INITIALIZED = -2,
  OUI_ERROR_INVALID_ARGUMENT = -3,
  OUI_ERROR_UNKNOWN_TAG = -4,
  OUI_ERROR_UNKNOWN_PROPERTY = -5,
  OUI_ERROR_INVALID_VALUE = -6,
  OUI_ERROR_LAYOUT_REQUIRED = -7,
  OUI_ERROR_INTERNAL = -99,
} OuiStatus;

// ─── CSS length units ───────────────────────────────────────
typedef enum {
  OUI_UNIT_PX,
  OUI_UNIT_PERCENT,
  OUI_UNIT_EM,
  OUI_UNIT_REM,
  OUI_UNIT_VW,
  OUI_UNIT_VH,
  OUI_UNIT_AUTO,
  OUI_UNIT_NONE,
  OUI_UNIT_FR,
} OuiUnit;

typedef struct {
  float value;
  OuiUnit unit;
} OuiLength;

typedef struct {
  float x;
  float y;
  float width;
  float height;
} OuiRect;

// ─── Length helper constructors ──────────────────────────────
static inline OuiLength oui_px(float v) {
  OuiLength l = {v, OUI_UNIT_PX};
  return l;
}
static inline OuiLength oui_pct(float v) {
  OuiLength l = {v, OUI_UNIT_PERCENT};
  return l;
}
static inline OuiLength oui_em(float v) {
  OuiLength l = {v, OUI_UNIT_EM};
  return l;
}
static inline OuiLength oui_rem(float v) {
  OuiLength l = {v, OUI_UNIT_REM};
  return l;
}
static inline OuiLength oui_vw(float v) {
  OuiLength l = {v, OUI_UNIT_VW};
  return l;
}
static inline OuiLength oui_vh(float v) {
  OuiLength l = {v, OUI_UNIT_VH};
  return l;
}
static inline OuiLength oui_fr(float v) {
  OuiLength l = {v, OUI_UNIT_FR};
  return l;
}
static inline OuiLength oui_auto(void) {
  OuiLength l = {0, OUI_UNIT_AUTO};
  return l;
}
static inline OuiLength oui_none(void) {
  OuiLength l = {0, OUI_UNIT_NONE};
  return l;
}

// ─── CSS display values ─────────────────────────────────────
typedef enum {
  OUI_DISPLAY_BLOCK,
  OUI_DISPLAY_INLINE,
  OUI_DISPLAY_INLINE_BLOCK,
  OUI_DISPLAY_FLEX,
  OUI_DISPLAY_INLINE_FLEX,
  OUI_DISPLAY_GRID,
  OUI_DISPLAY_INLINE_GRID,
  OUI_DISPLAY_TABLE,
  OUI_DISPLAY_TABLE_ROW,
  OUI_DISPLAY_TABLE_CELL,
  OUI_DISPLAY_NONE,
  OUI_DISPLAY_CONTENTS,
} OuiDisplay;

// ─── CSS position values ────────────────────────────────────
typedef enum {
  OUI_POSITION_STATIC,
  OUI_POSITION_RELATIVE,
  OUI_POSITION_ABSOLUTE,
  OUI_POSITION_FIXED,
  OUI_POSITION_STICKY,
} OuiPosition;

// ─── CSS flex-direction values ──────────────────────────────
typedef enum {
  OUI_FLEX_ROW,
  OUI_FLEX_ROW_REVERSE,
  OUI_FLEX_COLUMN,
  OUI_FLEX_COLUMN_REVERSE,
} OuiFlexDirection;

// ─── CSS overflow values ────────────────────────────────────
typedef enum {
  OUI_OVERFLOW_VISIBLE,
  OUI_OVERFLOW_HIDDEN,
  OUI_OVERFLOW_SCROLL,
  OUI_OVERFLOW_AUTO,
} OuiOverflow;

// ─── CSS align-items values ─────────────────────────────────
typedef enum {
  OUI_ALIGN_STRETCH,
  OUI_ALIGN_FLEX_START,
  OUI_ALIGN_FLEX_END,
  OUI_ALIGN_CENTER,
  OUI_ALIGN_BASELINE,
} OuiAlignItems;

// ─── CSS justify-content values ─────────────────────────────
typedef enum {
  OUI_JUSTIFY_FLEX_START,
  OUI_JUSTIFY_FLEX_END,
  OUI_JUSTIFY_CENTER,
  OUI_JUSTIFY_SPACE_BETWEEN,
  OUI_JUSTIFY_SPACE_AROUND,
  OUI_JUSTIFY_SPACE_EVENLY,
} OuiJustifyContent;

// ─── CSS flex-wrap values ───────────────────────────────────
typedef enum {
  OUI_FLEX_WRAP_NOWRAP,
  OUI_FLEX_WRAP_WRAP,
  OUI_FLEX_WRAP_WRAP_REVERSE,
} OuiFlexWrap;

// ─── CSS text-align values ──────────────────────────────────
typedef enum {
  OUI_TEXT_ALIGN_LEFT,
  OUI_TEXT_ALIGN_RIGHT,
  OUI_TEXT_ALIGN_CENTER,
  OUI_TEXT_ALIGN_JUSTIFY,
} OuiTextAlign;

// ─── CSS font-style values ──────────────────────────────────
typedef enum {
  OUI_FONT_STYLE_NORMAL,
  OUI_FONT_STYLE_ITALIC,
  OUI_FONT_STYLE_OBLIQUE,
} OuiFontStyle;

// ═══════════════════════════════════════════════════════════
// Initialization
// ═══════════════════════════════════════════════════════════

typedef struct {
  const char* resource_pak_path;  // Path to content_shell.pak (NULL = auto-detect)
} OuiInitConfig;

OUI_EXPORT OuiStatus oui_init(const OuiInitConfig* config);
OUI_EXPORT void oui_shutdown(void);

// ═══════════════════════════════════════════════════════════
// Document
// ═══════════════════════════════════════════════════════════

OUI_EXPORT OuiDocument* oui_document_create(int viewport_width,
                                            int viewport_height);
OUI_EXPORT void oui_document_destroy(OuiDocument* doc);
OUI_EXPORT void oui_document_set_viewport(OuiDocument* doc,
                                          int width,
                                          int height);
OUI_EXPORT OuiStatus oui_document_layout(OuiDocument* doc);
OUI_EXPORT OuiStatus oui_document_update_all(OuiDocument* doc);

// Load HTML content into the document body.
// Sets the body's innerHTML to |html|, which may contain any HTML/CSS.
// Useful for loading pre-authored pages for rendering.
OUI_EXPORT OuiStatus oui_document_load_html(OuiDocument* doc, const char* html);

// ═══════════════════════════════════════════════════════════
// Element lifecycle
// ═══════════════════════════════════════════════════════════

OUI_EXPORT OuiElement* oui_element_create(OuiDocument* doc, const char* tag);
OUI_EXPORT void oui_element_destroy(OuiElement* elem);
OUI_EXPORT OuiElement* oui_document_body(OuiDocument* doc);

// ═══════════════════════════════════════════════════════════
// DOM tree manipulation
// ═══════════════════════════════════════════════════════════

OUI_EXPORT void oui_element_append_child(OuiElement* parent, OuiElement* child);
OUI_EXPORT void oui_element_remove_child(OuiElement* parent, OuiElement* child);
OUI_EXPORT void oui_element_insert_before(OuiElement* parent,
                                          OuiElement* child,
                                          OuiElement* before);
OUI_EXPORT OuiElement* oui_element_first_child(const OuiElement* parent);
OUI_EXPORT OuiElement* oui_element_next_sibling(const OuiElement* elem);
OUI_EXPORT OuiElement* oui_element_parent(const OuiElement* elem);

// ═══════════════════════════════════════════════════════════
// Generic style (any CSS property/value as strings)
// ═══════════════════════════════════════════════════════════

OUI_EXPORT OuiStatus oui_element_set_style(OuiElement* e,
                                           const char* property,
                                           const char* value);
OUI_EXPORT OuiStatus oui_element_remove_style(OuiElement* e,
                                              const char* property);
OUI_EXPORT void oui_element_clear_styles(OuiElement* e);

// ═══════════════════════════════════════════════════════════
// Typed convenience setters — layout dimensions
// ═══════════════════════════════════════════════════════════

OUI_EXPORT void oui_element_set_width(OuiElement* e, OuiLength len);
OUI_EXPORT void oui_element_set_height(OuiElement* e, OuiLength len);
OUI_EXPORT void oui_element_set_min_width(OuiElement* e, OuiLength len);
OUI_EXPORT void oui_element_set_min_height(OuiElement* e, OuiLength len);
OUI_EXPORT void oui_element_set_max_width(OuiElement* e, OuiLength len);
OUI_EXPORT void oui_element_set_max_height(OuiElement* e, OuiLength len);

// ═══════════════════════════════════════════════════════════
// Typed convenience setters — box model
// ═══════════════════════════════════════════════════════════

OUI_EXPORT void oui_element_set_margin(OuiElement* e,
                                       OuiLength top,
                                       OuiLength right,
                                       OuiLength bottom,
                                       OuiLength left);
OUI_EXPORT void oui_element_set_padding(OuiElement* e,
                                        OuiLength top,
                                        OuiLength right,
                                        OuiLength bottom,
                                        OuiLength left);

// ═══════════════════════════════════════════════════════════
// Typed convenience setters — display & positioning
// ═══════════════════════════════════════════════════════════

OUI_EXPORT void oui_element_set_display(OuiElement* e, OuiDisplay display);
OUI_EXPORT void oui_element_set_position(OuiElement* e, OuiPosition pos);
OUI_EXPORT void oui_element_set_overflow(OuiElement* e, OuiOverflow overflow);

// ═══════════════════════════════════════════════════════════
// Typed convenience setters — flexbox
// ═══════════════════════════════════════════════════════════

OUI_EXPORT void oui_element_set_flex_direction(OuiElement* e,
                                               OuiFlexDirection dir);
OUI_EXPORT void oui_element_set_flex_wrap(OuiElement* e, OuiFlexWrap wrap);
OUI_EXPORT void oui_element_set_flex_grow(OuiElement* e, float grow);
OUI_EXPORT void oui_element_set_flex_shrink(OuiElement* e, float shrink);
OUI_EXPORT void oui_element_set_flex_basis(OuiElement* e, OuiLength basis);
OUI_EXPORT void oui_element_set_align_items(OuiElement* e, OuiAlignItems align);
OUI_EXPORT void oui_element_set_justify_content(OuiElement* e,
                                                OuiJustifyContent jc);

// ═══════════════════════════════════════════════════════════
// Typed convenience setters — colors & visuals
// ═══════════════════════════════════════════════════════════

OUI_EXPORT void oui_element_set_color(OuiElement* e, uint32_t rgba);
OUI_EXPORT void oui_element_set_background_color(OuiElement* e, uint32_t rgba);
OUI_EXPORT void oui_element_set_opacity(OuiElement* e, float opacity);
OUI_EXPORT void oui_element_set_z_index(OuiElement* e, int z);

// ═══════════════════════════════════════════════════════════
// Text content
// ═══════════════════════════════════════════════════════════

OUI_EXPORT void oui_element_set_text_content(OuiElement* e, const char* text);

// ═══════════════════════════════════════════════════════════
// Font convenience setters
// ═══════════════════════════════════════════════════════════

OUI_EXPORT void oui_element_set_font_family(OuiElement* e, const char* family);
OUI_EXPORT void oui_element_set_font_size(OuiElement* e, OuiLength size);
OUI_EXPORT void oui_element_set_font_weight(OuiElement* e, int weight);
OUI_EXPORT void oui_element_set_font_style(OuiElement* e, OuiFontStyle style);
OUI_EXPORT void oui_element_set_line_height(OuiElement* e, OuiLength lh);
OUI_EXPORT void oui_element_set_text_align(OuiElement* e, OuiTextAlign align);

// ═══════════════════════════════════════════════════════════
// Geometry queries (valid after oui_document_layout)
// ═══════════════════════════════════════════════════════════

OUI_EXPORT float oui_element_get_offset_x(const OuiElement* e);
OUI_EXPORT float oui_element_get_offset_y(const OuiElement* e);
OUI_EXPORT float oui_element_get_width(const OuiElement* e);
OUI_EXPORT float oui_element_get_height(const OuiElement* e);
OUI_EXPORT OuiRect oui_element_get_bounding_rect(const OuiElement* e);

// ═══════════════════════════════════════════════════════════
// Computed style readback
// ═══════════════════════════════════════════════════════════

// Returns a string that must be freed by the caller with free().
OUI_EXPORT char* oui_element_get_computed_style(const OuiElement* e,
                                                const char* property);

// ═══════════════════════════════════════════════════════════
// Hit testing
// ═══════════════════════════════════════════════════════════

OUI_EXPORT OuiElement* oui_document_hit_test(OuiDocument* doc,
                                             float x,
                                             float y);

// ═══════════════════════════════════════════════════════════
// Scroll geometry
// ═══════════════════════════════════════════════════════════

OUI_EXPORT float oui_element_get_scroll_width(const OuiElement* e);
OUI_EXPORT float oui_element_get_scroll_height(const OuiElement* e);

// ═══════════════════════════════════════════════════════════
// Offscreen rendering (SP5)
// ═══════════════════════════════════════════════════════════

typedef struct {
  __attribute__((annotate("raw_ptr_exclusion")))
  uint8_t* pixels;  // RGBA pixel data (caller must free with oui_bitmap_free)
  int width;
  int height;
  int stride;  // Bytes per row (width * 4)
} OuiBitmap;

// Render the current element tree to an RGBA bitmap.
// Runs the full lifecycle (style → layout → paint → rasterize).
// On success, populates |out_bitmap| with heap-allocated pixel data.
OUI_EXPORT OuiStatus oui_document_render_to_bitmap(OuiDocument* doc,
                                                    OuiBitmap* out_bitmap);

// Free bitmap pixel data returned by oui_document_render_to_bitmap.
OUI_EXPORT void oui_bitmap_free(OuiBitmap* bitmap);

// Render the current element tree and write a PNG file to |file_path|.
OUI_EXPORT OuiStatus oui_document_render_to_png(OuiDocument* doc,
                                                 const char* file_path);

// Render the current element tree to a PNG in memory.
// On success, |*out_data| is heap-allocated (free with oui_free).
OUI_EXPORT OuiStatus oui_document_render_to_png_buffer(OuiDocument* doc,
                                                        uint8_t** out_data,
                                                        size_t* out_size);

// Free memory allocated by oui_document_render_to_png_buffer.
OUI_EXPORT void oui_free(void* ptr);

#ifdef __cplusplus
}
#endif

#endif  // OPENUI_OPENUI_H_
