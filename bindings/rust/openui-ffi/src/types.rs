use std::ffi::c_void;

pub const OUI_ABI_VERSION: u32 = 0x0002_0000;

#[repr(C)]
pub struct OuiApp {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OuiDocument {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OuiElement {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OuiStyleCompound {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OuiResource {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OuiFontFace {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OuiListener {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OuiBuffer {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OuiAccessibilitySnapshot {
    _private: [u8; 0],
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuiStatus {
    Ok = 0,
    InvalidArgument = -1,
    AbiMismatch = -2,
    WrongThread = -3,
    InvalidHandle = -4,
    WrongDocument = -5,
    StaleHandle = -6,
    WrongValueType = -7,
    OutOfMemory = -8,
    InvalidState = -9,
    BufferTooSmall = -10,
    Reentrant = -11,
    Internal = -99,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiUtf8 {
    pub data: *const u8,
    pub length: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OuiLength {
    pub value: f32,
    pub unit: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OuiColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union OuiStylePayload {
    pub length: OuiLength,
    pub number: f32,
    pub integer: i32,
    pub color: OuiColor,
    pub enum_value: i32,
    pub compound: *const OuiStyleCompound,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiStyleValue {
    pub tag: u32,
    pub reserved: u32,
    pub data: OuiStylePayload,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiAppConfig {
    pub struct_size: u32,
    pub abi_version: u32,
    pub title: OuiUtf8,
    pub width: u32,
    pub height: u32,
    pub backend: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiViewportMetrics {
    pub logical_width: f64,
    pub logical_height: f64,
    pub physical_width: u32,
    pub physical_height: u32,
    pub device_scale_factor: f64,
    pub authority: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiDocumentConfig {
    pub struct_size: u32,
    pub abi_version: u32,
    pub viewport: OuiViewportMetrics,
}

/// Caller-owned client and content dimensions in logical CSS pixels.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OuiScrollMetricsV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub client_width: f64,
    pub client_height: f64,
    pub scroll_width: f64,
    pub scroll_height: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OuiFontUnicodeRange {
    pub start: u32,
    pub end: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OuiFontFeatureDefault {
    pub tag: [u8; 4],
    pub value: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiFontFaceDescriptor {
    pub struct_size: u32,
    pub abi_version: u32,
    pub family: OuiUtf8,
    pub face_index: u32,
    pub style: u32,
    pub style_min: f32,
    pub style_max: f32,
    pub weight_min: f32,
    pub weight_max: f32,
    pub stretch_min: f32,
    pub stretch_max: f32,
    pub unicode_ranges: *const OuiFontUnicodeRange,
    pub unicode_range_count: usize,
    pub feature_defaults: *const OuiFontFeatureDefault,
    pub feature_default_count: usize,
    pub size_adjust: f32,
    pub ascent_override: f32,
    pub descent_override: f32,
    pub line_gap_override: f32,
    pub flags: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OuiFontFaceInfo {
    pub struct_size: u32,
    pub abi_version: u32,
    pub collection_id: u64,
    pub face_id: u64,
    pub collection_generation: u64,
    pub byte_length: usize,
    pub format: u32,
    pub face_index: u32,
    pub style: u32,
    pub flags: u32,
    pub style_min: f32,
    pub style_max: f32,
    pub weight_min: f32,
    pub weight_max: f32,
    pub stretch_min: f32,
    pub stretch_max: f32,
    pub size_adjust: f32,
    pub ascent_override: f32,
    pub descent_override: f32,
    pub line_gap_override: f32,
    pub family_length: usize,
    pub unicode_range_count: usize,
    pub feature_default_count: usize,
    pub sha256: [u8; 32],
}

pub const OUI_FONT_FACE_STYLE_NORMAL: u32 = 0;
pub const OUI_FONT_FACE_STYLE_ITALIC: u32 = 1;
pub const OUI_FONT_FACE_STYLE_OBLIQUE: u32 = 2;
pub const OUI_FONT_CONTAINER_TTF: u32 = 1;
pub const OUI_FONT_CONTAINER_OTF: u32 = 2;
pub const OUI_FONT_CONTAINER_COLLECTION: u32 = 3;
pub const OUI_FONT_CONTAINER_WOFF: u32 = 4;
pub const OUI_FONT_CONTAINER_WOFF2: u32 = 5;
pub const OUI_FONT_FACE_HAS_SIZE_ADJUST: u32 = 1 << 0;
pub const OUI_FONT_FACE_HAS_ASCENT_OVERRIDE: u32 = 1 << 1;
pub const OUI_FONT_FACE_HAS_DESCENT_OVERRIDE: u32 = 1 << 2;
pub const OUI_FONT_FACE_HAS_LINE_GAP_OVERRIDE: u32 = 1 << 3;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OuiRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiBitmap {
    pub struct_size: u32,
    pub abi_version: u32,
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub pixels: *mut OuiBuffer,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OuiTransformOperation {
    pub kind: u32,
    pub reserved: u32,
    pub x: OuiLength,
    pub y: OuiLength,
    pub values: [f32; 6],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OuiEasing {
    pub kind: u32,
    pub step_position: u32,
    pub step_count: u32,
    pub reserved: u32,
    pub values: [f64; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OuiAnimationOptions {
    pub struct_size: u32,
    pub abi_version: u32,
    pub delay_ms: f64,
    pub duration_ms: f64,
    pub iterations: f64,
    pub playback_rate: f64,
    pub direction: u32,
    pub fill: u32,
    pub play_state: u32,
    pub composite: u32,
    pub easing: OuiEasing,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiAnimationTimeline {
    pub struct_size: u32,
    pub abi_version: u32,
    pub kind: u32,
    pub axis: u32,
    pub source: *mut OuiElement,
    pub range_start: f64,
    pub range_end: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiKeyframe {
    pub offset: f64,
    pub value: OuiStyleValue,
    pub easing: OuiEasing,
    pub has_easing: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OuiAnimationState {
    pub struct_size: u32,
    pub abi_version: u32,
    pub animation_id: u64,
    pub current_time_ms: f64,
    pub iteration: u64,
    pub property: i32,
    pub play_state: u32,
    pub phase: u32,
    pub finished: u8,
    pub reserved: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiAnimationEvent {
    pub struct_size: u32,
    pub abi_version: u32,
    pub animation_id: u64,
    pub kind: u32,
    pub property: i32,
    pub elapsed_time_ms: f64,
    pub iteration: u64,
    pub target: *mut OuiElement,
}

pub const OUI_EVENT_FLAG_DEFAULT_PREVENTED: u32 = 1 << 0;
pub const OUI_EVENT_FLAG_PROPAGATION_STOPPED: u32 = 1 << 1;
pub const OUI_EVENT_FLAG_IMMEDIATE_PROPAGATION_STOPPED: u32 = 1 << 2;
pub const OUI_EDIT_MOVE: u32 = 0;
pub const OUI_EDIT_DELETE: u32 = 1;
pub const OUI_EDIT_SELECT_ALL: u32 = 2;
pub const OUI_EDIT_UNDO: u32 = 3;
pub const OUI_EDIT_REDO: u32 = 4;
pub const OUI_TEXT_BACKWARD: u32 = 0;
pub const OUI_TEXT_FORWARD: u32 = 1;
pub const OUI_TEXT_GRAPHEME: u32 = 0;
pub const OUI_TEXT_WORD: u32 = 1;
pub const OUI_TEXT_LINE: u32 = 2;
pub const OUI_TEXT_DOCUMENT: u32 = 3;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OuiEditCommandV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub command: u32,
    pub direction: u32,
    pub unit: u32,
    pub extend_selection: u32,
    pub reserved: [u32; 2],
}

pub const OUI_CONTROL_DISABLED: u32 = 1 << 0;
pub const OUI_CONTROL_CHECKED: u32 = 1 << 1;
pub const OUI_CONTROL_SELECTED: u32 = 1 << 2;
pub const OUI_CONTROL_OPEN: u32 = 1 << 3;
pub const OUI_CONTROL_INDETERMINATE: u32 = 1 << 4;
pub const OUI_CONTROL_PASSWORD: u32 = 1 << 5;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiAccessibilityUpdate {
    pub struct_size: u32,
    pub abi_version: u32,
    pub generation: u64,
    pub updated_nodes: usize,
    pub focus_id: u64,
    pub full_tree: u8,
    pub reduced_motion: u8,
    pub reserved: u16,
}

pub const OUI_ACCESSIBILITY_NODE_HIDDEN: u32 = 1 << 0;
pub const OUI_ACCESSIBILITY_NODE_REQUIRED: u32 = 1 << 1;
pub const OUI_ACCESSIBILITY_NODE_READ_ONLY: u32 = 1 << 2;
pub const OUI_ACCESSIBILITY_NODE_MODAL: u32 = 1 << 3;
pub const OUI_ACCESSIBILITY_NODE_HAS_BOUNDS: u32 = 1 << 4;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct OuiAccessibilitySnapshotInfo {
    pub struct_size: u32,
    pub abi_version: u32,
    pub generation: u64,
    pub focus_id: u64,
    pub node_count: usize,
    pub changed_count: usize,
    pub removed_count: usize,
    pub full_tree: u8,
    pub reduced_motion: u8,
    pub reserved: [u8; 6],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiAccessibilityNodeInfo {
    pub struct_size: u32,
    pub abi_version: u32,
    pub id: u64,
    pub role: u32,
    pub flags: u32,
    pub actions: u32,
    pub reserved: u32,
    pub bounds: OuiRect,
    pub child_count: usize,
    pub labelled_by_count: usize,
    pub described_by_count: usize,
    pub controls_count: usize,
    pub details_count: usize,
    pub label_length: usize,
    pub description_length: usize,
    pub value_length: usize,
    pub role_name_length: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct OuiAccessibilityNodeState {
    pub struct_size: u32,
    pub abi_version: u32,
    pub flags: u32,
    pub live: u32,
    pub numeric_value: f64,
    pub numeric_min: f64,
    pub numeric_max: f64,
    pub numeric_step: f64,
    pub scroll_x: f64,
    pub scroll_y: f64,
    pub placeholder_length: usize,
    pub character_lengths_count: usize,
    pub selection_anchor_node: u64,
    pub selection_anchor_index: usize,
    pub selection_focus_node: u64,
    pub selection_focus_index: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiEvent {
    pub struct_size: u32,
    pub abi_version: u32,
    pub event_type: u32,
    pub phase: u32,
    pub flags: u32,
    pub modifiers: u32,
    pub timestamp_ns: u64,
    pub x: f32,
    pub y: f32,
    pub delta_x: f32,
    pub delta_y: f32,
    pub key_code: i32,
    pub pointer_id: u32,
    pub text: OuiUtf8,
    pub target: *mut OuiElement,
    pub current_target: *mut OuiElement,
}

/// Owned metadata copied from a currently executing native focus callback.
/// The related element handle must be released with oui_element_destroy.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiFocusEventInfoV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub bubbles: u32,
    pub cancelable: u32,
    pub related_target: *mut OuiElement,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiErrorInfo {
    pub struct_size: u32,
    pub abi_version: u32,
    pub status: i32,
    pub detail: u32,
    pub message_length: usize,
}

pub type OuiEventCallback = unsafe extern "C" fn(event: *mut OuiEvent, user_data: *mut c_void);

/// Borrowed platform event; all text/path storage is valid only during callback.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiPlatformEvent {
    pub struct_size: u32,
    pub abi_version: u32,
    pub event_type: u32,
    pub phase: u32,
    pub flags: u32,
    pub backend: u32,
    pub pointer_id: u64,
    pub x: f32,
    pub y: f32,
    pub delta_x: f32,
    pub delta_y: f32,
    pub key_code: i32,
    pub modifiers: u32,
    pub button: u32,
    pub reserved: u32,
    pub frame_number: u64,
    pub time_ms: f64,
    pub text: OuiUtf8,
    pub path: *const u8,
    pub path_length: usize,
    pub viewport: OuiViewportMetrics,
}

pub type OuiPlatformEventCallback =
    unsafe extern "C" fn(app: *mut OuiApp, event: *const OuiPlatformEvent, user_data: *mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiAppRunConfig {
    pub struct_size: u32,
    pub abi_version: u32,
    pub flags: u32,
    pub reserved: u32,
    pub callback: Option<OuiPlatformEventCallback>,
    pub user_data: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiInputEventInfoV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub input_type: u32,
    pub has_data: u32,
    pub is_composing: u32,
    pub bubbles: u32,
    pub cancelable: u32,
    pub data: *mut OuiBuffer,
}

/// Immutable event properties for an active native callback.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OuiEventPropertiesV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub bubbles: u32,
    pub cancelable: u32,
}
