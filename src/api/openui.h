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
typedef struct OuiTextNode OuiTextNode;

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

// Load HTML content into the document.
// Sets the <html> element's innerHTML to |html|, replacing both <head> and
// <body>. The HTML may include <head>, <style>, and <body> tags. Any
// previously obtained OuiElement* handles are invalidated by this call.
OUI_EXPORT OuiStatus oui_document_load_html(OuiDocument* doc, const char* html);

// ═══════════════════════════════════════════════════════════
// Element lifecycle
// ═══════════════════════════════════════════════════════════

OUI_EXPORT OuiElement* oui_element_create(OuiDocument* doc, const char* tag);
OUI_EXPORT void oui_element_append_text(OuiElement* elem, const char* text);
OUI_EXPORT void oui_element_destroy(OuiElement* elem);
OUI_EXPORT OuiElement* oui_document_body(OuiDocument* doc);

// ═══════════════════════════════════════════════════════════
// Text node lifecycle (for mutable text content)
// ═══════════════════════════════════════════════════════════

// Create a DOM Text node, append it to parent, and return a mutable handle.
// The handle can be used with oui_text_node_set_data() for reactive updates.
OUI_EXPORT OuiTextNode* oui_element_create_text_child(OuiElement* parent,
                                                       const char* text);
// Update the text content of a text node.
OUI_EXPORT void oui_text_node_set_data(OuiTextNode* node, const char* data);
// Remove from DOM and free. Safe to call on null.
OUI_EXPORT void oui_text_node_destroy(OuiTextNode* node);

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
// Remove ALL child nodes (elements, text nodes, etc.) from the DOM.
// Element wrappers must be cleaned up before calling this.
OUI_EXPORT void oui_element_remove_all_child_nodes(OuiElement* elem);

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
// HTML attributes (generic)
// ═══════════════════════════════════════════════════════════

// Set an HTML attribute on the element (e.g. "type", "value", "checked",
// "disabled", "placeholder", "src", "href", "alt", "width", "height",
// "colspan", "rowspan", "class", "id", "role", "aria-label", etc.)
// For boolean attributes like "checked" or "disabled", pass "" as value to set,
// or use oui_element_remove_attribute to unset.
OUI_EXPORT OuiStatus oui_element_set_attribute(OuiElement* e,
                                                const char* name,
                                                const char* value);

// Remove an HTML attribute from the element.
OUI_EXPORT OuiStatus oui_element_remove_attribute(OuiElement* e,
                                                   const char* name);

// Get the value of an HTML attribute. Returns NULL if not set.
// Caller must free the returned string with free().
OUI_EXPORT char* oui_element_get_attribute(const OuiElement* e,
                                            const char* name);

// Set the "id" attribute (shorthand).
OUI_EXPORT OuiStatus oui_element_set_id(OuiElement* e, const char* id);

// Set the "class" attribute (shorthand).
OUI_EXPORT OuiStatus oui_element_set_class(OuiElement* e, const char* classes);

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
// Scroll geometry & control (SP7)
// ═══════════════════════════════════════════════════════════

OUI_EXPORT float oui_element_get_scroll_width(const OuiElement* e);
OUI_EXPORT float oui_element_get_scroll_height(const OuiElement* e);

// Get current scroll position.
OUI_EXPORT double oui_element_get_scroll_left(const OuiElement* e);
OUI_EXPORT double oui_element_get_scroll_top(const OuiElement* e);

// Scroll to absolute position.
OUI_EXPORT OuiStatus oui_element_scroll_to(OuiElement* e, double x, double y);

// Scroll by a delta (relative to current position).
OUI_EXPORT OuiStatus oui_element_scroll_by(OuiElement* e, double dx, double dy);

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

// ═══════════════════════════════════════════════════════════
// Resource provider (SP6)
// ═══════════════════════════════════════════════════════════

// Callback to free resource response data when Blink is done with it.
typedef void (*OuiResourceFreeFunc)(uint8_t* data, void* user_data);

// Response data returned by the resource provider callback.
typedef struct {
  __attribute__((annotate("raw_ptr_exclusion")))
  uint8_t* data;
  size_t length;
  __attribute__((annotate("raw_ptr_exclusion")))
  const char* mime_type;  // NULL = auto-detect
  OuiResourceFreeFunc free_func;
  __attribute__((annotate("raw_ptr_exclusion")))
  void* free_user_data;
} OuiResourceResponse;

// Resource provider callback. Return 1 if resource found, 0 if not.
typedef int (*OuiResourceProviderFunc)(
    const char* url,
    OuiResourceResponse* response,
    void* user_data);

// Set the resource provider for a document.  Must be called before loading
// HTML that references external resources (images, etc.).
OUI_EXPORT OuiStatus oui_document_set_resource_provider(
    OuiDocument* doc,
    OuiResourceProviderFunc provider,
    void* user_data);

// ═══════════════════════════════════════════════════════════
// Direct image injection (SP6)
// ═══════════════════════════════════════════════════════════

// Set raw RGBA pixel data on an <img> element.  The pixels are copied.
OUI_EXPORT OuiStatus oui_element_set_image_data(
    OuiElement* elem,
    const uint8_t* rgba_pixels,
    int width,
    int height);

// Set encoded image data (PNG, JPEG, WebP, GIF, etc.) on an <img> element.
// Blink's image decoder will decode internally.
OUI_EXPORT OuiStatus oui_element_set_image_encoded(
    OuiElement* elem,
    const uint8_t* data,
    size_t length);

// ═══════════════════════════════════════════════════════════
// Frame & time management (SP7)
// ═══════════════════════════════════════════════════════════

// Advance the animation clock to an absolute time (milliseconds).
// Time 0.0 is the document's epoch (set on first call to any time function).
OUI_EXPORT OuiStatus oui_document_advance_time(OuiDocument* doc, double time_ms);

// Advance the animation clock by a delta from the current time.
OUI_EXPORT OuiStatus oui_document_advance_time_by(OuiDocument* doc,
                                                    double delta_ms);

// Get the current animation time (milliseconds from epoch).
OUI_EXPORT double oui_document_get_time(OuiDocument* doc);

// Full frame tick: advance time + service animations + full lifecycle update.
// This is the primary function callers use in their render loop.
OUI_EXPORT OuiStatus oui_document_begin_frame(OuiDocument* doc, double time_ms);

// ═══════════════════════════════════════════════════════════
// Input event dispatch (SP7)
// ═══════════════════════════════════════════════════════════

typedef enum {
  OUI_MOUSE_DOWN = 0,
  OUI_MOUSE_UP = 1,
  OUI_MOUSE_MOVE = 2,
} OuiMouseEventType;

typedef enum {
  OUI_BUTTON_LEFT = 0,
  OUI_BUTTON_MIDDLE = 1,
  OUI_BUTTON_RIGHT = 2,
} OuiMouseButton;

typedef enum {
  OUI_KEY_DOWN = 0,
  OUI_KEY_UP = 1,
  OUI_KEY_CHAR = 2,
} OuiKeyEventType;

typedef enum {
  OUI_MOD_SHIFT = 1 << 0,
  OUI_MOD_CTRL = 1 << 1,
  OUI_MOD_ALT = 1 << 2,
  OUI_MOD_META = 1 << 3,
} OuiModifiers;

// Dispatch a mouse event at viewport coordinates.
OUI_EXPORT OuiStatus oui_document_dispatch_mouse_event(
    OuiDocument* doc,
    OuiMouseEventType type,
    float x,
    float y,
    OuiMouseButton button,
    int modifiers);

// Dispatch a keyboard event.
// |key_code| is a platform-independent virtual key code.
// |key_text| is the character text for OUI_KEY_CHAR (UTF-8, NULL otherwise).
OUI_EXPORT OuiStatus oui_document_dispatch_key_event(
    OuiDocument* doc,
    OuiKeyEventType type,
    int key_code,
    const char* key_text,
    int modifiers);

// Dispatch a mouse wheel event at viewport coordinates.
OUI_EXPORT OuiStatus oui_document_dispatch_wheel_event(
    OuiDocument* doc,
    float x,
    float y,
    float delta_x,
    float delta_y,
    int modifiers);

// ═══════════════════════════════════════════════════════════
// Event callbacks (SP7)
// ═══════════════════════════════════════════════════════════

// Event info passed to callbacks.
typedef struct {
  __attribute__((annotate("raw_ptr_exclusion")))
  const char* type;       // Event type name (e.g. "click")
  __attribute__((annotate("raw_ptr_exclusion")))
  OuiElement* target;     // Element that received the event
  float mouse_x;          // Mouse position (mouse events only)
  float mouse_y;
  int mouse_button;       // OuiMouseButton (mouse events only)
  int key_code;           // Virtual key code (keyboard events only)
  __attribute__((annotate("raw_ptr_exclusion")))
  const char* key_text;   // Character text (keyboard events only)
  int modifiers;          // OuiModifiers bitmask
  int default_prevented;  // Set to 1 to call preventDefault()
} OuiEvent;

typedef void (*OuiEventCallback)(OuiEvent* event, void* user_data);

// Set a callback for an event type on an element (replaces any existing).
// Supported event types: "click", "mousedown", "mouseup", "mousemove",
// "mouseenter", "mouseleave", "keydown", "keyup", "input", "scroll",
// "focus", "blur", "transitionend", "animationend", "animationstart",
// "animationiteration".
OUI_EXPORT OuiStatus oui_element_set_event_callback(
    OuiElement* elem,
    const char* event_type,
    OuiEventCallback callback,
    void* user_data);

// Remove a previously set event callback.
OUI_EXPORT OuiStatus oui_element_remove_event_callback(OuiElement* elem,
                                                        const char* event_type);

// ═══════════════════════════════════════════════════════════
// Focus management (SP7)
// ═══════════════════════════════════════════════════════════

// Set keyboard focus to an element.
OUI_EXPORT OuiStatus oui_element_focus(OuiElement* elem);

// Remove keyboard focus from an element.
OUI_EXPORT OuiStatus oui_element_blur(OuiElement* elem);

// Get the currently focused element (NULL if none).
OUI_EXPORT OuiElement* oui_document_get_focused_element(OuiDocument* doc);

// Advance focus to next (direction=1) or previous (direction=-1) element.
OUI_EXPORT OuiStatus oui_document_advance_focus(OuiDocument* doc, int direction);

// Check if an element currently has focus. Returns 1 if focused, 0 otherwise.
OUI_EXPORT int oui_element_has_focus(const OuiElement* elem);

#ifdef __cplusplus
}
#endif

#endif  // OPENUI_OPENUI_H_
