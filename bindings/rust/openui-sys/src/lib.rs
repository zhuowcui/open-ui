//! Raw FFI bindings to the Open UI C API.
//!
//! This crate provides `unsafe extern "C"` declarations for every function,
//! type, and enum defined in `openui.h`.

#![allow(non_camel_case_types)]

use std::os::raw::{c_char, c_int, c_void};

// ─── Opaque handles ─────────────────────────────────────────

#[repr(C)]
pub struct OuiDocument {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OuiElement {
    _private: [u8; 0],
}

// ─── Status codes ───────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiStatus {
    OUI_OK = 0,
    OUI_ERROR_NOT_INITIALIZED = -1,
    OUI_ERROR_ALREADY_INITIALIZED = -2,
    OUI_ERROR_INVALID_ARGUMENT = -3,
    OUI_ERROR_UNKNOWN_TAG = -4,
    OUI_ERROR_UNKNOWN_PROPERTY = -5,
    OUI_ERROR_INVALID_VALUE = -6,
    OUI_ERROR_LAYOUT_REQUIRED = -7,
    OUI_ERROR_INTERNAL = -99,
}

// ─── CSS length units ───────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiUnit {
    OUI_UNIT_PX = 0,
    OUI_UNIT_PERCENT = 1,
    OUI_UNIT_EM = 2,
    OUI_UNIT_REM = 3,
    OUI_UNIT_VW = 4,
    OUI_UNIT_VH = 5,
    OUI_UNIT_AUTO = 6,
    OUI_UNIT_NONE = 7,
    OUI_UNIT_FR = 8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OuiLength {
    pub value: f32,
    pub unit: OuiUnit,
}

impl OuiLength {
    pub const fn px(value: f32) -> Self {
        Self { value, unit: OuiUnit::OUI_UNIT_PX }
    }
    pub const fn pct(value: f32) -> Self {
        Self { value, unit: OuiUnit::OUI_UNIT_PERCENT }
    }
    pub const fn em(value: f32) -> Self {
        Self { value, unit: OuiUnit::OUI_UNIT_EM }
    }
    pub const fn rem(value: f32) -> Self {
        Self { value, unit: OuiUnit::OUI_UNIT_REM }
    }
    pub const fn vw(value: f32) -> Self {
        Self { value, unit: OuiUnit::OUI_UNIT_VW }
    }
    pub const fn vh(value: f32) -> Self {
        Self { value, unit: OuiUnit::OUI_UNIT_VH }
    }
    pub const fn fr(value: f32) -> Self {
        Self { value, unit: OuiUnit::OUI_UNIT_FR }
    }
    pub const fn auto() -> Self {
        Self { value: 0.0, unit: OuiUnit::OUI_UNIT_AUTO }
    }
    pub const fn none() -> Self {
        Self { value: 0.0, unit: OuiUnit::OUI_UNIT_NONE }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OuiRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

// ─── CSS display values ─────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiDisplay {
    OUI_DISPLAY_BLOCK = 0,
    OUI_DISPLAY_INLINE = 1,
    OUI_DISPLAY_INLINE_BLOCK = 2,
    OUI_DISPLAY_FLEX = 3,
    OUI_DISPLAY_INLINE_FLEX = 4,
    OUI_DISPLAY_GRID = 5,
    OUI_DISPLAY_INLINE_GRID = 6,
    OUI_DISPLAY_TABLE = 7,
    OUI_DISPLAY_TABLE_ROW = 8,
    OUI_DISPLAY_TABLE_CELL = 9,
    OUI_DISPLAY_NONE = 10,
    OUI_DISPLAY_CONTENTS = 11,
}

// ─── CSS position values ────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiPosition {
    OUI_POSITION_STATIC = 0,
    OUI_POSITION_RELATIVE = 1,
    OUI_POSITION_ABSOLUTE = 2,
    OUI_POSITION_FIXED = 3,
    OUI_POSITION_STICKY = 4,
}

// ─── CSS flex-direction values ──────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiFlexDirection {
    OUI_FLEX_ROW = 0,
    OUI_FLEX_ROW_REVERSE = 1,
    OUI_FLEX_COLUMN = 2,
    OUI_FLEX_COLUMN_REVERSE = 3,
}

// ─── CSS overflow values ────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiOverflow {
    OUI_OVERFLOW_VISIBLE = 0,
    OUI_OVERFLOW_HIDDEN = 1,
    OUI_OVERFLOW_SCROLL = 2,
    OUI_OVERFLOW_AUTO = 3,
}

// ─── CSS align-items values ─────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiAlignItems {
    OUI_ALIGN_STRETCH = 0,
    OUI_ALIGN_FLEX_START = 1,
    OUI_ALIGN_FLEX_END = 2,
    OUI_ALIGN_CENTER = 3,
    OUI_ALIGN_BASELINE = 4,
}

// ─── CSS justify-content values ─────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiJustifyContent {
    OUI_JUSTIFY_FLEX_START = 0,
    OUI_JUSTIFY_FLEX_END = 1,
    OUI_JUSTIFY_CENTER = 2,
    OUI_JUSTIFY_SPACE_BETWEEN = 3,
    OUI_JUSTIFY_SPACE_AROUND = 4,
    OUI_JUSTIFY_SPACE_EVENLY = 5,
}

// ─── CSS flex-wrap values ───────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiFlexWrap {
    OUI_FLEX_WRAP_NOWRAP = 0,
    OUI_FLEX_WRAP_WRAP = 1,
    OUI_FLEX_WRAP_WRAP_REVERSE = 2,
}

// ─── CSS text-align values ──────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiTextAlign {
    OUI_TEXT_ALIGN_LEFT = 0,
    OUI_TEXT_ALIGN_RIGHT = 1,
    OUI_TEXT_ALIGN_CENTER = 2,
    OUI_TEXT_ALIGN_JUSTIFY = 3,
}

// ─── CSS font-style values ──────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiFontStyle {
    OUI_FONT_STYLE_NORMAL = 0,
    OUI_FONT_STYLE_ITALIC = 1,
    OUI_FONT_STYLE_OBLIQUE = 2,
}

// ─── Initialization config ──────────────────────────────────

#[repr(C)]
pub struct OuiInitConfig {
    pub resource_pak_path: *const c_char,
}

// ─── Bitmap ─────────────────────────────────────────────────

#[repr(C)]
pub struct OuiBitmap {
    pub pixels: *mut u8,
    pub width: c_int,
    pub height: c_int,
    pub stride: c_int,
}

// ─── Mouse event types ──────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiMouseEventType {
    OUI_MOUSE_DOWN = 0,
    OUI_MOUSE_UP = 1,
    OUI_MOUSE_MOVE = 2,
}

// ─── Mouse button ───────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiMouseButton {
    OUI_BUTTON_LEFT = 0,
    OUI_BUTTON_MIDDLE = 1,
    OUI_BUTTON_RIGHT = 2,
}

// ─── Key event types ────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiKeyEventType {
    OUI_KEY_DOWN = 0,
    OUI_KEY_UP = 1,
    OUI_KEY_CHAR = 2,
}

// ─── Modifier flags (bitmask) ───────────────────────────────

pub type OuiModifiers = c_int;
pub const OUI_MOD_SHIFT: OuiModifiers = 1 << 0;
pub const OUI_MOD_CTRL: OuiModifiers = 1 << 1;
pub const OUI_MOD_ALT: OuiModifiers = 1 << 2;
pub const OUI_MOD_META: OuiModifiers = 1 << 3;

// ─── Resource provider ──────────────────────────────────────

pub type OuiResourceFreeFunc =
    Option<unsafe extern "C" fn(data: *mut u8, user_data: *mut c_void)>;

#[repr(C)]
pub struct OuiResourceResponse {
    pub data: *mut u8,
    pub length: usize,
    pub mime_type: *const c_char,
    pub free_func: OuiResourceFreeFunc,
    pub free_user_data: *mut c_void,
}

pub type OuiResourceProviderFunc = Option<
    unsafe extern "C" fn(
        url: *const c_char,
        response: *mut OuiResourceResponse,
        user_data: *mut c_void,
    ) -> c_int,
>;

// ─── Event types ────────────────────────────────────────────

#[repr(C)]
pub struct OuiEvent {
    pub type_: *const c_char,
    pub target: *mut OuiElement,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_button: c_int,
    pub key_code: c_int,
    pub key_text: *const c_char,
    pub modifiers: c_int,
    pub default_prevented: c_int,
}

pub type OuiEventCallback =
    Option<unsafe extern "C" fn(event: *mut OuiEvent, user_data: *mut c_void)>;

// ─── Functions (89 exported symbols) ────────────────────────

unsafe extern "C" {
    // ═══ Initialization ═══════════════════════════════════════

    pub unsafe fn oui_init(config: *const OuiInitConfig) -> OuiStatus;
    pub unsafe fn oui_shutdown();

    // ═══ Document ═════════════════════════════════════════════

    pub unsafe fn oui_document_create(
        viewport_width: c_int,
        viewport_height: c_int,
    ) -> *mut OuiDocument;
    pub unsafe fn oui_document_destroy(doc: *mut OuiDocument);
    pub unsafe fn oui_document_set_viewport(
        doc: *mut OuiDocument,
        width: c_int,
        height: c_int,
    );
    pub unsafe fn oui_document_layout(doc: *mut OuiDocument) -> OuiStatus;
    pub unsafe fn oui_document_update_all(doc: *mut OuiDocument) -> OuiStatus;
    pub unsafe fn oui_document_load_html(
        doc: *mut OuiDocument,
        html: *const c_char,
    ) -> OuiStatus;

    // ═══ Element lifecycle ════════════════════════════════════

    pub unsafe fn oui_element_create(
        doc: *mut OuiDocument,
        tag: *const c_char,
    ) -> *mut OuiElement;
    pub unsafe fn oui_element_destroy(elem: *mut OuiElement);
    pub unsafe fn oui_document_body(doc: *mut OuiDocument) -> *mut OuiElement;

    // ═══ DOM tree manipulation ════════════════════════════════

    pub unsafe fn oui_element_append_child(
        parent: *mut OuiElement,
        child: *mut OuiElement,
    );
    pub unsafe fn oui_element_remove_child(
        parent: *mut OuiElement,
        child: *mut OuiElement,
    );
    pub unsafe fn oui_element_insert_before(
        parent: *mut OuiElement,
        child: *mut OuiElement,
        before: *mut OuiElement,
    );
    pub unsafe fn oui_element_first_child(
        parent: *const OuiElement,
    ) -> *mut OuiElement;
    pub unsafe fn oui_element_next_sibling(
        elem: *const OuiElement,
    ) -> *mut OuiElement;
    pub unsafe fn oui_element_parent(
        elem: *const OuiElement,
    ) -> *mut OuiElement;

    // ═══ Generic style ════════════════════════════════════════

    pub unsafe fn oui_element_set_style(
        e: *mut OuiElement,
        property: *const c_char,
        value: *const c_char,
    ) -> OuiStatus;
    pub unsafe fn oui_element_remove_style(
        e: *mut OuiElement,
        property: *const c_char,
    ) -> OuiStatus;
    pub unsafe fn oui_element_clear_styles(e: *mut OuiElement);

    // ═══ HTML attributes ══════════════════════════════════════

    pub unsafe fn oui_element_set_attribute(
        e: *mut OuiElement,
        name: *const c_char,
        value: *const c_char,
    ) -> OuiStatus;
    pub unsafe fn oui_element_remove_attribute(
        e: *mut OuiElement,
        name: *const c_char,
    ) -> OuiStatus;
    pub unsafe fn oui_element_get_attribute(
        e: *const OuiElement,
        name: *const c_char,
    ) -> *mut c_char;
    pub unsafe fn oui_element_set_id(
        e: *mut OuiElement,
        id: *const c_char,
    ) -> OuiStatus;
    pub unsafe fn oui_element_set_class(
        e: *mut OuiElement,
        classes: *const c_char,
    ) -> OuiStatus;

    // ═══ Layout dimensions ════════════════════════════════════

    pub unsafe fn oui_element_set_width(e: *mut OuiElement, len: OuiLength);
    pub unsafe fn oui_element_set_height(e: *mut OuiElement, len: OuiLength);
    pub unsafe fn oui_element_set_min_width(e: *mut OuiElement, len: OuiLength);
    pub unsafe fn oui_element_set_min_height(
        e: *mut OuiElement,
        len: OuiLength,
    );
    pub unsafe fn oui_element_set_max_width(e: *mut OuiElement, len: OuiLength);
    pub unsafe fn oui_element_set_max_height(
        e: *mut OuiElement,
        len: OuiLength,
    );

    // ═══ Box model ════════════════════════════════════════════

    pub unsafe fn oui_element_set_margin(
        e: *mut OuiElement,
        top: OuiLength,
        right: OuiLength,
        bottom: OuiLength,
        left: OuiLength,
    );
    pub unsafe fn oui_element_set_padding(
        e: *mut OuiElement,
        top: OuiLength,
        right: OuiLength,
        bottom: OuiLength,
        left: OuiLength,
    );

    // ═══ Display & positioning ════════════════════════════════

    pub unsafe fn oui_element_set_display(
        e: *mut OuiElement,
        display: OuiDisplay,
    );
    pub unsafe fn oui_element_set_position(
        e: *mut OuiElement,
        pos: OuiPosition,
    );
    pub unsafe fn oui_element_set_overflow(
        e: *mut OuiElement,
        overflow: OuiOverflow,
    );

    // ═══ Flexbox ══════════════════════════════════════════════

    pub unsafe fn oui_element_set_flex_direction(
        e: *mut OuiElement,
        dir: OuiFlexDirection,
    );
    pub unsafe fn oui_element_set_flex_wrap(
        e: *mut OuiElement,
        wrap: OuiFlexWrap,
    );
    pub unsafe fn oui_element_set_flex_grow(e: *mut OuiElement, grow: f32);
    pub unsafe fn oui_element_set_flex_shrink(e: *mut OuiElement, shrink: f32);
    pub unsafe fn oui_element_set_flex_basis(
        e: *mut OuiElement,
        basis: OuiLength,
    );
    pub unsafe fn oui_element_set_align_items(
        e: *mut OuiElement,
        align: OuiAlignItems,
    );
    pub unsafe fn oui_element_set_justify_content(
        e: *mut OuiElement,
        jc: OuiJustifyContent,
    );

    // ═══ Colors & visuals ═════════════════════════════════════

    pub unsafe fn oui_element_set_color(e: *mut OuiElement, rgba: u32);
    pub unsafe fn oui_element_set_background_color(
        e: *mut OuiElement,
        rgba: u32,
    );
    pub unsafe fn oui_element_set_opacity(e: *mut OuiElement, opacity: f32);
    pub unsafe fn oui_element_set_z_index(e: *mut OuiElement, z: c_int);

    // ═══ Text content ═════════════════════════════════════════

    pub unsafe fn oui_element_set_text_content(
        e: *mut OuiElement,
        text: *const c_char,
    );

    // ═══ Font ═════════════════════════════════════════════════

    pub unsafe fn oui_element_set_font_family(
        e: *mut OuiElement,
        family: *const c_char,
    );
    pub unsafe fn oui_element_set_font_size(
        e: *mut OuiElement,
        size: OuiLength,
    );
    pub unsafe fn oui_element_set_font_weight(
        e: *mut OuiElement,
        weight: c_int,
    );
    pub unsafe fn oui_element_set_font_style(
        e: *mut OuiElement,
        style: OuiFontStyle,
    );
    pub unsafe fn oui_element_set_line_height(
        e: *mut OuiElement,
        lh: OuiLength,
    );
    pub unsafe fn oui_element_set_text_align(
        e: *mut OuiElement,
        align: OuiTextAlign,
    );

    // ═══ Geometry queries ═════════════════════════════════════

    pub unsafe fn oui_element_get_offset_x(e: *const OuiElement) -> f32;
    pub unsafe fn oui_element_get_offset_y(e: *const OuiElement) -> f32;
    pub unsafe fn oui_element_get_width(e: *const OuiElement) -> f32;
    pub unsafe fn oui_element_get_height(e: *const OuiElement) -> f32;
    pub unsafe fn oui_element_get_bounding_rect(
        e: *const OuiElement,
    ) -> OuiRect;

    // ═══ Computed style ═══════════════════════════════════════

    pub unsafe fn oui_element_get_computed_style(
        e: *const OuiElement,
        property: *const c_char,
    ) -> *mut c_char;

    // ═══ Hit testing ══════════════════════════════════════════

    pub unsafe fn oui_document_hit_test(
        doc: *mut OuiDocument,
        x: f32,
        y: f32,
    ) -> *mut OuiElement;

    // ═══ Scroll geometry & control ════════════════════════════

    pub unsafe fn oui_element_get_scroll_width(e: *const OuiElement) -> f32;
    pub unsafe fn oui_element_get_scroll_height(e: *const OuiElement) -> f32;
    pub unsafe fn oui_element_get_scroll_left(e: *const OuiElement) -> f64;
    pub unsafe fn oui_element_get_scroll_top(e: *const OuiElement) -> f64;
    pub unsafe fn oui_element_scroll_to(
        e: *mut OuiElement,
        x: f64,
        y: f64,
    ) -> OuiStatus;
    pub unsafe fn oui_element_scroll_by(
        e: *mut OuiElement,
        dx: f64,
        dy: f64,
    ) -> OuiStatus;

    // ═══ Offscreen rendering ══════════════════════════════════

    pub unsafe fn oui_document_render_to_bitmap(
        doc: *mut OuiDocument,
        out_bitmap: *mut OuiBitmap,
    ) -> OuiStatus;
    pub unsafe fn oui_bitmap_free(bitmap: *mut OuiBitmap);
    pub unsafe fn oui_document_render_to_png(
        doc: *mut OuiDocument,
        file_path: *const c_char,
    ) -> OuiStatus;
    pub unsafe fn oui_document_render_to_png_buffer(
        doc: *mut OuiDocument,
        out_data: *mut *mut u8,
        out_size: *mut usize,
    ) -> OuiStatus;
    pub unsafe fn oui_free(ptr: *mut c_void);

    // ═══ Resource provider ════════════════════════════════════

    pub unsafe fn oui_document_set_resource_provider(
        doc: *mut OuiDocument,
        provider: OuiResourceProviderFunc,
        user_data: *mut c_void,
    ) -> OuiStatus;

    // ═══ Direct image injection ═══════════════════════════════

    pub unsafe fn oui_element_set_image_data(
        elem: *mut OuiElement,
        rgba_pixels: *const u8,
        width: c_int,
        height: c_int,
    ) -> OuiStatus;
    pub unsafe fn oui_element_set_image_encoded(
        elem: *mut OuiElement,
        data: *const u8,
        length: usize,
    ) -> OuiStatus;

    // ═══ Frame & time management ══════════════════════════════

    pub unsafe fn oui_document_advance_time(
        doc: *mut OuiDocument,
        time_ms: f64,
    ) -> OuiStatus;
    pub unsafe fn oui_document_advance_time_by(
        doc: *mut OuiDocument,
        delta_ms: f64,
    ) -> OuiStatus;
    pub unsafe fn oui_document_get_time(doc: *mut OuiDocument) -> f64;
    pub unsafe fn oui_document_begin_frame(
        doc: *mut OuiDocument,
        time_ms: f64,
    ) -> OuiStatus;

    // ═══ Input event dispatch ═════════════════════════════════

    pub unsafe fn oui_document_dispatch_mouse_event(
        doc: *mut OuiDocument,
        event_type: OuiMouseEventType,
        x: f32,
        y: f32,
        button: OuiMouseButton,
        modifiers: c_int,
    ) -> OuiStatus;
    pub unsafe fn oui_document_dispatch_key_event(
        doc: *mut OuiDocument,
        event_type: OuiKeyEventType,
        key_code: c_int,
        key_text: *const c_char,
        modifiers: c_int,
    ) -> OuiStatus;
    pub unsafe fn oui_document_dispatch_wheel_event(
        doc: *mut OuiDocument,
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
        modifiers: c_int,
    ) -> OuiStatus;

    // ═══ Event callbacks ══════════════════════════════════════

    pub unsafe fn oui_element_set_event_callback(
        elem: *mut OuiElement,
        event_type: *const c_char,
        callback: OuiEventCallback,
        user_data: *mut c_void,
    ) -> OuiStatus;
    pub unsafe fn oui_element_remove_event_callback(
        elem: *mut OuiElement,
        event_type: *const c_char,
    ) -> OuiStatus;

    // ═══ Focus management ═════════════════════════════════════

    pub unsafe fn oui_element_focus(elem: *mut OuiElement) -> OuiStatus;
    pub unsafe fn oui_element_blur(elem: *mut OuiElement) -> OuiStatus;
    pub unsafe fn oui_document_get_focused_element(
        doc: *mut OuiDocument,
    ) -> *mut OuiElement;
    pub unsafe fn oui_document_advance_focus(
        doc: *mut OuiDocument,
        direction: c_int,
    ) -> OuiStatus;
    pub unsafe fn oui_element_has_focus(elem: *const OuiElement) -> c_int;
}
