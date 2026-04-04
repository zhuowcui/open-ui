//! Event types, modifier flags, and callback infrastructure.
//!
//! This module bridges Rust closures to the C event callback system.
//! Closures are stored in a global registry and invoked through a C-compatible
//! trampoline function.

use openui_sys::{OuiKeyEventType, OuiMouseButton, OuiMouseEventType};
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::sync::{LazyLock, Mutex};

// ─── Event ──────────────────────────────────────────────────

/// A DOM event passed to event listener callbacks.
#[derive(Debug, Clone)]
pub struct Event {
    /// The event type name (e.g. `"click"`, `"keydown"`).
    pub event_type: String,
    /// Mouse X coordinate in viewport pixels.
    pub mouse_x: f32,
    /// Mouse Y coordinate in viewport pixels.
    pub mouse_y: f32,
    /// Which mouse button was pressed (0 = left, 1 = middle, 2 = right).
    pub mouse_button: i32,
    /// Virtual key code for keyboard events.
    pub key_code: i32,
    /// Character text produced by a key press.
    pub key_text: String,
    /// Bitmask of active modifier keys.
    pub modifiers: i32,
}

impl Event {
    /// Construct a Rust `Event` from a raw `OuiEvent` pointer.
    ///
    /// # Safety
    ///
    /// `raw` must point to a valid, fully-initialised `OuiEvent`.
    pub(crate) unsafe fn from_raw(raw: &openui_sys::OuiEvent) -> Self {
        let event_type = if raw.type_.is_null() {
            String::new()
        } else {
            CStr::from_ptr(raw.type_).to_string_lossy().into_owned()
        };
        let key_text = if raw.key_text.is_null() {
            String::new()
        } else {
            CStr::from_ptr(raw.key_text).to_string_lossy().into_owned()
        };
        Event {
            event_type,
            mouse_x: raw.mouse_x,
            mouse_y: raw.mouse_y,
            mouse_button: raw.mouse_button,
            key_code: raw.key_code,
            key_text,
            modifiers: raw.modifiers,
        }
    }
}

// ─── Input event enums ──────────────────────────────────────

/// Mouse event types for `Document::dispatch_mouse_event`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventType {
    /// A mouse button was pressed.
    Down,
    /// A mouse button was released.
    Up,
    /// The mouse pointer moved.
    Move,
}

impl From<MouseEventType> for OuiMouseEventType {
    fn from(t: MouseEventType) -> Self {
        match t {
            MouseEventType::Down => OuiMouseEventType::OUI_MOUSE_DOWN,
            MouseEventType::Up => OuiMouseEventType::OUI_MOUSE_UP,
            MouseEventType::Move => OuiMouseEventType::OUI_MOUSE_MOVE,
        }
    }
}

/// Mouse button identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Primary (left) button.
    Left,
    /// Auxiliary (middle / wheel) button.
    Middle,
    /// Secondary (right) button.
    Right,
}

impl From<MouseButton> for OuiMouseButton {
    fn from(b: MouseButton) -> Self {
        match b {
            MouseButton::Left => OuiMouseButton::OUI_BUTTON_LEFT,
            MouseButton::Middle => OuiMouseButton::OUI_BUTTON_MIDDLE,
            MouseButton::Right => OuiMouseButton::OUI_BUTTON_RIGHT,
        }
    }
}

/// Keyboard event types for `Document::dispatch_key_event`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventType {
    /// A key was pressed.
    Down,
    /// A key was released.
    Up,
    /// A character was produced.
    Char,
}

impl From<KeyEventType> for OuiKeyEventType {
    fn from(t: KeyEventType) -> Self {
        match t {
            KeyEventType::Down => OuiKeyEventType::OUI_KEY_DOWN,
            KeyEventType::Up => OuiKeyEventType::OUI_KEY_UP,
            KeyEventType::Char => OuiKeyEventType::OUI_KEY_CHAR,
        }
    }
}

// ─── Modifier flags ─────────────────────────────────────────

/// Keyboard modifier flags (bitwise-OR them together).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers(pub u32);

impl Modifiers {
    /// No modifiers active.
    pub const NONE: Self = Modifiers(0);
    /// Shift key.
    pub const SHIFT: Self = Modifiers(1);
    /// Control key.
    pub const CTRL: Self = Modifiers(2);
    /// Alt / Option key.
    pub const ALT: Self = Modifiers(4);
    /// Meta / Command / Windows key.
    pub const META: Self = Modifiers(8);

    /// Return the raw bitmask value.
    pub fn bits(self) -> u32 {
        self.0
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Modifiers(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

// ─── Callback registry ──────────────────────────────────────

/// Key: (element raw pointer as usize, event type string).
type CallbackKey = (usize, String);

/// Value: raw pointer to a `Box<dyn Fn(&Event)>`, cast to `usize` for `Send`.
type CallbackPtr = usize;

/// Global registry of active event callbacks.
///
/// When `Element::on()` is called the closure is boxed, leaked into a raw
/// pointer, and stored here so it can be freed later in `remove_event()` or
/// when the `Element` is dropped.
pub(crate) static CALLBACK_REGISTRY: LazyLock<Mutex<HashMap<CallbackKey, CallbackPtr>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// C-compatible trampoline invoked by Blink for every DOM event.
///
/// `user_data` is a raw pointer to a `Box<dyn Fn(&Event)>` that was stored
/// via `Element::on()`.
///
/// # Safety
///
/// Must only be called by the C event dispatch mechanism with a valid
/// `OuiEvent` pointer and the `user_data` originally passed to
/// `oui_element_set_event_callback`.
pub(crate) unsafe extern "C" fn event_trampoline(
    event: *mut openui_sys::OuiEvent,
    user_data: *mut c_void,
) {
    if event.is_null() || user_data.is_null() {
        return;
    }
    let closure = &*(user_data as *const Box<dyn Fn(&Event)>);
    let rust_event = Event::from_raw(&*event);
    closure(&rust_event);
}

/// Free a callback previously stored via `Element::on()`.
///
/// Removes it from the global registry and drops the boxed closure.
pub(crate) fn free_callback(element_ptr: usize, event_type: &str) {
    let key = (element_ptr, event_type.to_string());
    if let Some(ptr) = CALLBACK_REGISTRY.lock().unwrap().remove(&key) {
        // Reconstruct and drop the Box<Box<dyn Fn(&Event)>> to free memory.
        let _ = unsafe { Box::from_raw(ptr as *mut Box<dyn Fn(&Event)>) };
    }
}

/// Store a callback in the registry, freeing any previous callback for the
/// same (element, event_type) pair.
pub(crate) fn store_callback(
    element_ptr: usize,
    event_type: &str,
    user_data: *mut c_void,
) {
    let key = (element_ptr, event_type.to_string());
    let mut map = CALLBACK_REGISTRY.lock().unwrap();
    // Free any previous callback for this key.
    if let Some(old) = map.remove(&key) {
        let _ = unsafe { Box::from_raw(old as *mut Box<dyn Fn(&Event)>) };
    }
    map.insert(key, user_data as usize);
}

/// Free all callbacks registered for a given element pointer.
pub(crate) fn free_all_callbacks_for(element_ptr: usize) {
    let mut map = CALLBACK_REGISTRY.lock().unwrap();
    let keys: Vec<CallbackKey> = map
        .keys()
        .filter(|(ptr, _)| *ptr == element_ptr)
        .cloned()
        .collect();
    for key in keys {
        if let Some(ptr) = map.remove(&key) {
            let _ = unsafe { Box::from_raw(ptr as *mut Box<dyn Fn(&Event)>) };
        }
    }
}

// ─── Resource provider registry ─────────────────────────────

/// Global registry for resource provider callbacks (one per document).
pub(crate) static RESOURCE_PROVIDER_REGISTRY: LazyLock<Mutex<HashMap<usize, usize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// C-compatible trampoline for the resource provider callback.
///
/// # Safety
///
/// Must only be called by the C resource loading mechanism.
pub(crate) unsafe extern "C" fn resource_provider_trampoline(
    url: *const std::os::raw::c_char,
    response: *mut openui_sys::OuiResourceResponse,
    user_data: *mut c_void,
) -> std::os::raw::c_int {
    if url.is_null() || response.is_null() || user_data.is_null() {
        return 0;
    }
    let closure = &*(user_data as *const Box<dyn Fn(&str) -> Option<Vec<u8>>>);
    let url_str = match CStr::from_ptr(url).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    match closure(url_str) {
        Some(data) => {
            let len = data.len();
            // Box the Vec so we can recover it in the free callback.
            let mut boxed_data = Box::new(data);
            let ptr = boxed_data.as_mut_ptr();
            let rd_ptr = Box::into_raw(boxed_data);

            (*response).data = ptr;
            (*response).length = len;
            (*response).mime_type = std::ptr::null();
            (*response).free_func = Some(resource_free_trampoline);
            (*response).free_user_data = rd_ptr as *mut c_void;
            1
        }
        None => 0,
    }
}

/// Free function called by Blink when it no longer needs the resource data.
unsafe extern "C" fn resource_free_trampoline(_data: *mut u8, user_data: *mut c_void) {
    if !user_data.is_null() {
        // user_data is a Box<Vec<u8>> — reconstruct and drop.
        let _ = Box::from_raw(user_data as *mut Vec<u8>);
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_bitor() {
        let m = Modifiers::SHIFT | Modifiers::CTRL;
        assert_eq!(m.bits(), 3);
    }

    #[test]
    fn modifier_bitor_assign() {
        let mut m = Modifiers::NONE;
        m |= Modifiers::ALT;
        assert_eq!(m.bits(), 4);
    }

    #[test]
    fn mouse_event_type_conversion() {
        let _: OuiMouseEventType = MouseEventType::Down.into();
        let _: OuiMouseEventType = MouseEventType::Up.into();
        let _: OuiMouseEventType = MouseEventType::Move.into();
    }

    #[test]
    fn mouse_button_conversion() {
        let _: OuiMouseButton = MouseButton::Left.into();
        let _: OuiMouseButton = MouseButton::Middle.into();
        let _: OuiMouseButton = MouseButton::Right.into();
    }

    #[test]
    fn key_event_type_conversion() {
        let _: OuiKeyEventType = KeyEventType::Down.into();
        let _: OuiKeyEventType = KeyEventType::Up.into();
        let _: OuiKeyEventType = KeyEventType::Char.into();
    }
}
