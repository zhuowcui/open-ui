//! Safe wrapper around an Open UI document (rendering context).
//!
//! A [`Document`] owns a Blink viewport and the DOM tree rooted at its body.
//! It is the entry point for creating elements, running layout, rendering,
//! and dispatching input events.

use crate::element::Element;
use crate::events::{
    resource_provider_trampoline, RESOURCE_PROVIDER_REGISTRY,
};
use crate::style::{check_status, Bitmap, OuiError};
use openui_sys::OuiBitmap;
use std::ffi::{c_void, CString};

/// A document represents a Blink rendering context with a viewport.
///
/// It owns the underlying C `OuiDocument` and destroys it on drop.
/// All elements created within this document are logically owned by it.
pub struct Document {
    raw: *mut openui_sys::OuiDocument,
}

impl Document {
    /// Create a new document with the given viewport dimensions.
    pub fn new(width: i32, height: i32) -> Result<Self, OuiError> {
        let raw = unsafe { openui_sys::oui_document_create(width, height) };
        if raw.is_null() {
            return Err(OuiError::CreationFailed);
        }
        Ok(Document { raw })
    }

    /// Get the document body (root element).
    ///
    /// The returned element is *borrowed* — it is owned by the document and
    /// will not be destroyed when dropped.
    pub fn body(&self) -> Option<Element> {
        let raw = unsafe { openui_sys::oui_document_body(self.raw) };
        if raw.is_null() {
            None
        } else {
            Some(Element::from_raw(raw, false))
        }
    }

    /// Set the viewport size.
    pub fn set_viewport(&self, width: i32, height: i32) {
        unsafe { openui_sys::oui_document_set_viewport(self.raw, width, height) };
    }

    /// Trigger layout computation.
    pub fn layout(&self) -> Result<(), OuiError> {
        check_status(unsafe { openui_sys::oui_document_layout(self.raw) })
    }

    /// Full lifecycle update (style recalc + layout + compositing).
    pub fn update_all(&self) -> Result<(), OuiError> {
        check_status(unsafe { openui_sys::oui_document_update_all(self.raw) })
    }

    /// Load and parse HTML content into the document.
    pub fn load_html(&self, html: &str) -> Result<(), OuiError> {
        let c_html = CString::new(html).map_err(|_| OuiError::InvalidArgument)?;
        check_status(unsafe { openui_sys::oui_document_load_html(self.raw, c_html.as_ptr()) })
    }

    /// Render the document to a PNG file on disk.
    pub fn render_to_png(&self, path: &str) -> Result<(), OuiError> {
        let c_path = CString::new(path).map_err(|_| OuiError::InvalidArgument)?;
        check_status(unsafe { openui_sys::oui_document_render_to_png(self.raw, c_path.as_ptr()) })
    }

    /// Render the document to an in-memory RGBA bitmap.
    pub fn render_to_bitmap(&self) -> Result<Bitmap, OuiError> {
        let mut bitmap = OuiBitmap {
            pixels: std::ptr::null_mut(),
            width: 0,
            height: 0,
            stride: 0,
        };
        check_status(unsafe { openui_sys::oui_document_render_to_bitmap(self.raw, &mut bitmap) })?;
        Ok(Bitmap { raw: bitmap })
    }

    /// Render the document to an in-memory PNG buffer.
    pub fn render_to_png_buffer(&self) -> Result<Vec<u8>, OuiError> {
        let mut data: *mut u8 = std::ptr::null_mut();
        let mut size: usize = 0;
        check_status(unsafe {
            openui_sys::oui_document_render_to_png_buffer(self.raw, &mut data, &mut size)
        })?;
        if data.is_null() {
            return Err(OuiError::Internal);
        }
        let vec = unsafe { std::slice::from_raw_parts(data, size) }.to_vec();
        unsafe { openui_sys::oui_free(data as *mut c_void) };
        Ok(vec)
    }

    /// Hit-test at viewport coordinates and return the topmost element.
    ///
    /// Returns `None` if no element was hit.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<Element> {
        let raw = unsafe { openui_sys::oui_document_hit_test(self.raw, x, y) };
        if raw.is_null() {
            None
        } else {
            Some(Element::from_raw(raw, false))
        }
    }

    // ─── Time & animation ───────────────────────────────────

    /// Advance the animation clock to an absolute time in milliseconds.
    pub fn advance_time(&self, time_ms: f64) -> Result<(), OuiError> {
        check_status(unsafe { openui_sys::oui_document_advance_time(self.raw, time_ms) })
    }

    /// Advance the animation clock by a delta in milliseconds.
    pub fn advance_time_by(&self, delta_ms: f64) -> Result<(), OuiError> {
        check_status(unsafe { openui_sys::oui_document_advance_time_by(self.raw, delta_ms) })
    }

    /// Get the current animation time in milliseconds.
    pub fn get_time(&self) -> f64 {
        unsafe { openui_sys::oui_document_get_time(self.raw) }
    }

    /// Full frame tick: advance time, run animations, and update lifecycle.
    pub fn begin_frame(&self, time_ms: f64) -> Result<(), OuiError> {
        check_status(unsafe { openui_sys::oui_document_begin_frame(self.raw, time_ms) })
    }

    // ─── Input event dispatch ───────────────────────────────

    /// Dispatch a mouse event into the document.
    pub fn dispatch_mouse_event(
        &self,
        event_type: crate::events::MouseEventType,
        x: f32,
        y: f32,
        button: crate::events::MouseButton,
        modifiers: crate::events::Modifiers,
    ) -> Result<(), OuiError> {
        check_status(unsafe {
            openui_sys::oui_document_dispatch_mouse_event(
                self.raw,
                event_type.into(),
                x,
                y,
                button.into(),
                modifiers.bits() as i32,
            )
        })
    }

    /// Dispatch a keyboard event into the document.
    pub fn dispatch_key_event(
        &self,
        event_type: crate::events::KeyEventType,
        key_code: i32,
        key_text: Option<&str>,
        modifiers: crate::events::Modifiers,
    ) -> Result<(), OuiError> {
        let c_text = key_text
            .map(|t| CString::new(t))
            .transpose()
            .map_err(|_| OuiError::InvalidArgument)?;
        let text_ptr = c_text.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());
        check_status(unsafe {
            openui_sys::oui_document_dispatch_key_event(
                self.raw,
                event_type.into(),
                key_code,
                text_ptr,
                modifiers.bits() as i32,
            )
        })
    }

    /// Dispatch a wheel (scroll) event into the document.
    pub fn dispatch_wheel_event(
        &self,
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
        modifiers: crate::events::Modifiers,
    ) -> Result<(), OuiError> {
        check_status(unsafe {
            openui_sys::oui_document_dispatch_wheel_event(
                self.raw,
                x,
                y,
                delta_x,
                delta_y,
                modifiers.bits() as i32,
            )
        })
    }

    // ─── Focus management ───────────────────────────────────

    /// Get the currently focused element, if any.
    pub fn get_focused_element(&self) -> Option<Element> {
        let raw = unsafe { openui_sys::oui_document_get_focused_element(self.raw) };
        if raw.is_null() {
            None
        } else {
            Some(Element::from_raw(raw, false))
        }
    }

    /// Advance focus in the given direction (1 = Tab, -1 = Shift+Tab).
    pub fn advance_focus(&self, direction: i32) -> Result<(), OuiError> {
        check_status(unsafe { openui_sys::oui_document_advance_focus(self.raw, direction) })
    }

    // ─── Resource provider ──────────────────────────────────

    /// Set a resource provider callback.
    ///
    /// The callback receives a URL string and should return `Some(bytes)` to
    /// supply the resource data, or `None` to let the engine handle it.
    pub fn set_resource_provider<F>(&self, callback: F) -> Result<(), OuiError>
    where
        F: Fn(&str) -> Option<Vec<u8>> + 'static,
    {
        let boxed: Box<dyn Fn(&str) -> Option<Vec<u8>>> = Box::new(callback);
        let user_data = Box::into_raw(Box::new(boxed)) as *mut c_void;

        // Store for cleanup, freeing any previous provider.
        {
            let mut map = RESOURCE_PROVIDER_REGISTRY.lock().unwrap();
            if let Some(old) = map.remove(&(self.raw as usize)) {
                let _ = unsafe {
                    Box::from_raw(old as *mut Box<dyn Fn(&str) -> Option<Vec<u8>>>)
                };
            }
            map.insert(self.raw as usize, user_data as usize);
        }

        check_status(unsafe {
            openui_sys::oui_document_set_resource_provider(
                self.raw,
                Some(resource_provider_trampoline),
                user_data,
            )
        })
    }

    // ─── Raw access ─────────────────────────────────────────

    /// Get the underlying raw FFI pointer (for advanced use).
    pub fn as_raw(&self) -> *mut openui_sys::OuiDocument {
        self.raw
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // Free any resource provider closure.
            if let Some(ptr) = RESOURCE_PROVIDER_REGISTRY
                .lock()
                .unwrap()
                .remove(&(self.raw as usize))
            {
                let _ = unsafe {
                    Box::from_raw(ptr as *mut Box<dyn Fn(&str) -> Option<Vec<u8>>>)
                };
            }
            unsafe { openui_sys::oui_document_destroy(self.raw) };
        }
    }
}
