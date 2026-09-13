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
pub struct OuiListener {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OuiBuffer {
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
pub struct OuiDocumentConfig {
    pub struct_size: u32,
    pub abi_version: u32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

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

pub const OUI_EVENT_FLAG_DEFAULT_PREVENTED: u32 = 1 << 0;
pub const OUI_EVENT_FLAG_PROPAGATION_STOPPED: u32 = 1 << 1;

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
