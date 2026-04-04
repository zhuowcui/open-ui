//! Safe wrapper around a DOM element in the Blink rendering tree.
//!
//! An [`Element`] is always associated with a [`Document`](crate::Document).
//! Elements obtained via tree traversal (e.g. `first_child`, `parent`) are
//! *borrowed* — they do not destroy the underlying C object on drop. Elements
//! created with [`Element::create`] are *owned* and will call
//! `oui_element_destroy` when dropped.

use crate::events::{
    event_trampoline, free_all_callbacks_for, free_callback, store_callback, Event,
};
use crate::style::{
    check_status, AlignItems, Display, FlexDirection, FlexWrap, FontStyle, JustifyContent, Length,
    OuiError, Overflow, Position, Rect, TextAlign,
};
use std::ffi::{c_void, CStr, CString};

/// An element in the document tree.
///
/// If `owned` is `true` (created via [`Element::create`]), the underlying C
/// element is destroyed when this value is dropped. Borrowed elements
/// (obtained from tree traversal or `Document::body`) do **not** destroy their
/// backing C object.
pub struct Element {
    raw: *mut openui_sys::OuiElement,
    owned: bool,
}

impl Element {
    /// Create a new element with the given HTML tag name.
    ///
    /// The element is initially detached — call `append_child` on a parent
    /// to insert it into the document tree.
    pub fn create(doc: &crate::Document, tag: &str) -> Result<Self, OuiError> {
        let c_tag = CString::new(tag).map_err(|_| OuiError::InvalidArgument)?;
        let raw = unsafe { openui_sys::oui_element_create(doc.as_raw(), c_tag.as_ptr()) };
        if raw.is_null() {
            return Err(OuiError::CreationFailed);
        }
        Ok(Element { raw, owned: true })
    }

    /// Wrap a raw FFI pointer.
    ///
    /// If `owned` is `true`, the element will be destroyed on drop.
    pub(crate) fn from_raw(raw: *mut openui_sys::OuiElement, owned: bool) -> Self {
        Element { raw, owned }
    }

    // ─── DOM tree manipulation ──────────────────────────────

    /// Append `child` as the last child of this element.
    pub fn append_child(&self, child: &Element) {
        unsafe { openui_sys::oui_element_append_child(self.raw, child.raw) };
    }

    /// Remove `child` from this element's children.
    pub fn remove_child(&self, child: &Element) {
        unsafe { openui_sys::oui_element_remove_child(self.raw, child.raw) };
    }

    /// Insert `child` before `before` in this element's child list.
    pub fn insert_before(&self, child: &Element, before: &Element) {
        unsafe { openui_sys::oui_element_insert_before(self.raw, child.raw, before.raw) };
    }

    /// Get the first child element, if any.
    pub fn first_child(&self) -> Option<Element> {
        let raw = unsafe { openui_sys::oui_element_first_child(self.raw) };
        if raw.is_null() {
            None
        } else {
            Some(Element::from_raw(raw, false))
        }
    }

    /// Get the next sibling element, if any.
    pub fn next_sibling(&self) -> Option<Element> {
        let raw = unsafe { openui_sys::oui_element_next_sibling(self.raw) };
        if raw.is_null() {
            None
        } else {
            Some(Element::from_raw(raw, false))
        }
    }

    /// Get the parent element, if any.
    pub fn parent(&self) -> Option<Element> {
        let raw = unsafe { openui_sys::oui_element_parent(self.raw) };
        if raw.is_null() {
            None
        } else {
            Some(Element::from_raw(raw, false))
        }
    }

    // ─── Generic style ──────────────────────────────────────

    /// Set a CSS property by name and string value.
    pub fn set_style(&self, property: &str, value: &str) -> Result<(), OuiError> {
        let c_prop = CString::new(property).map_err(|_| OuiError::InvalidArgument)?;
        let c_val = CString::new(value).map_err(|_| OuiError::InvalidArgument)?;
        check_status(unsafe {
            openui_sys::oui_element_set_style(self.raw, c_prop.as_ptr(), c_val.as_ptr())
        })
    }

    /// Remove a CSS property by name.
    pub fn remove_style(&self, property: &str) -> Result<(), OuiError> {
        let c_prop = CString::new(property).map_err(|_| OuiError::InvalidArgument)?;
        check_status(unsafe { openui_sys::oui_element_remove_style(self.raw, c_prop.as_ptr()) })
    }

    /// Clear all inline styles.
    pub fn clear_styles(&self) {
        unsafe { openui_sys::oui_element_clear_styles(self.raw) };
    }

    /// Get the computed value of a CSS property.
    ///
    /// Returns `None` if the property has no computed value.
    pub fn get_computed_style(&self, property: &str) -> Option<String> {
        let c_prop = CString::new(property).ok()?;
        let ptr = unsafe { openui_sys::oui_element_get_computed_style(self.raw, c_prop.as_ptr()) };
        if ptr.is_null() {
            None
        } else {
            let s = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
            unsafe { openui_sys::oui_free(ptr as *mut c_void) };
            Some(s)
        }
    }

    // ─── Attributes ─────────────────────────────────────────

    /// Set an HTML attribute.
    pub fn set_attribute(&self, name: &str, value: &str) -> Result<(), OuiError> {
        let c_name = CString::new(name).map_err(|_| OuiError::InvalidArgument)?;
        let c_val = CString::new(value).map_err(|_| OuiError::InvalidArgument)?;
        check_status(unsafe {
            openui_sys::oui_element_set_attribute(self.raw, c_name.as_ptr(), c_val.as_ptr())
        })
    }

    /// Remove an HTML attribute.
    pub fn remove_attribute(&self, name: &str) -> Result<(), OuiError> {
        let c_name = CString::new(name).map_err(|_| OuiError::InvalidArgument)?;
        check_status(unsafe {
            openui_sys::oui_element_remove_attribute(self.raw, c_name.as_ptr())
        })
    }

    /// Get the value of an HTML attribute.
    ///
    /// Returns `None` if the attribute is not set.
    pub fn get_attribute(&self, name: &str) -> Option<String> {
        let c_name = CString::new(name).ok()?;
        let ptr = unsafe { openui_sys::oui_element_get_attribute(self.raw, c_name.as_ptr()) };
        if ptr.is_null() {
            None
        } else {
            let s = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
            unsafe { openui_sys::oui_free(ptr as *mut c_void) };
            Some(s)
        }
    }

    /// Set the element's `id` attribute.
    pub fn set_id(&self, id: &str) -> Result<(), OuiError> {
        let c_id = CString::new(id).map_err(|_| OuiError::InvalidArgument)?;
        check_status(unsafe { openui_sys::oui_element_set_id(self.raw, c_id.as_ptr()) })
    }

    /// Set the element's `class` attribute (space-separated class names).
    pub fn set_class(&self, classes: &str) -> Result<(), OuiError> {
        let c_cls = CString::new(classes).map_err(|_| OuiError::InvalidArgument)?;
        check_status(unsafe { openui_sys::oui_element_set_class(self.raw, c_cls.as_ptr()) })
    }

    // ─── Text content ───────────────────────────────────────

    /// Set the text content of this element, replacing all children.
    pub fn set_text(&self, text: &str) -> Result<(), OuiError> {
        let c_text = CString::new(text).map_err(|_| OuiError::InvalidArgument)?;
        unsafe { openui_sys::oui_element_set_text_content(self.raw, c_text.as_ptr()) };
        Ok(())
    }

    // ─── Layout dimensions ──────────────────────────────────

    /// Set the element's width.
    pub fn set_width(&self, length: Length) {
        unsafe { openui_sys::oui_element_set_width(self.raw, length) };
    }

    /// Set the element's height.
    pub fn set_height(&self, length: Length) {
        unsafe { openui_sys::oui_element_set_height(self.raw, length) };
    }

    /// Set the element's minimum width.
    pub fn set_min_width(&self, length: Length) {
        unsafe { openui_sys::oui_element_set_min_width(self.raw, length) };
    }

    /// Set the element's minimum height.
    pub fn set_min_height(&self, length: Length) {
        unsafe { openui_sys::oui_element_set_min_height(self.raw, length) };
    }

    /// Set the element's maximum width.
    pub fn set_max_width(&self, length: Length) {
        unsafe { openui_sys::oui_element_set_max_width(self.raw, length) };
    }

    /// Set the element's maximum height.
    pub fn set_max_height(&self, length: Length) {
        unsafe { openui_sys::oui_element_set_max_height(self.raw, length) };
    }

    // ─── Box model ──────────────────────────────────────────

    /// Set margin on all four sides (values in pixels).
    pub fn set_margin(&self, top: f32, right: f32, bottom: f32, left: f32) {
        unsafe {
            openui_sys::oui_element_set_margin(
                self.raw,
                Length::px(top),
                Length::px(right),
                Length::px(bottom),
                Length::px(left),
            )
        };
    }

    /// Set padding on all four sides (values in pixels).
    pub fn set_padding(&self, top: f32, right: f32, bottom: f32, left: f32) {
        unsafe {
            openui_sys::oui_element_set_padding(
                self.raw,
                Length::px(top),
                Length::px(right),
                Length::px(bottom),
                Length::px(left),
            )
        };
    }

    // ─── Display, position, overflow ────────────────────────

    /// Set the CSS `display` property.
    pub fn set_display(&self, display: Display) {
        unsafe { openui_sys::oui_element_set_display(self.raw, display.into()) };
    }

    /// Set the CSS `position` property.
    pub fn set_position(&self, position: Position) {
        unsafe { openui_sys::oui_element_set_position(self.raw, position.into()) };
    }

    /// Set the CSS `overflow` property.
    pub fn set_overflow(&self, overflow: Overflow) {
        unsafe { openui_sys::oui_element_set_overflow(self.raw, overflow.into()) };
    }

    // ─── Flexbox ────────────────────────────────────────────

    /// Set the flex direction.
    pub fn set_flex_direction(&self, dir: FlexDirection) {
        unsafe { openui_sys::oui_element_set_flex_direction(self.raw, dir.into()) };
    }

    /// Set the flex wrap mode.
    pub fn set_flex_wrap(&self, wrap: FlexWrap) {
        unsafe { openui_sys::oui_element_set_flex_wrap(self.raw, wrap.into()) };
    }

    /// Set the `flex-grow` factor.
    pub fn set_flex_grow(&self, val: f32) {
        unsafe { openui_sys::oui_element_set_flex_grow(self.raw, val) };
    }

    /// Set the `flex-shrink` factor.
    pub fn set_flex_shrink(&self, val: f32) {
        unsafe { openui_sys::oui_element_set_flex_shrink(self.raw, val) };
    }

    /// Set the `flex-basis` length.
    pub fn set_flex_basis(&self, length: Length) {
        unsafe { openui_sys::oui_element_set_flex_basis(self.raw, length) };
    }

    /// Set the `align-items` property.
    pub fn set_align_items(&self, align: AlignItems) {
        unsafe { openui_sys::oui_element_set_align_items(self.raw, align.into()) };
    }

    /// Set the `justify-content` property.
    pub fn set_justify_content(&self, justify: JustifyContent) {
        unsafe { openui_sys::oui_element_set_justify_content(self.raw, justify.into()) };
    }

    // ─── Colors & visuals ───────────────────────────────────

    /// Set the text colour as an RGBA value (e.g. `0xFF0000FF` for red).
    pub fn set_color(&self, rgba: u32) {
        unsafe { openui_sys::oui_element_set_color(self.raw, rgba) };
    }

    /// Set the background colour as an RGBA value.
    pub fn set_background_color(&self, rgba: u32) {
        unsafe { openui_sys::oui_element_set_background_color(self.raw, rgba) };
    }

    /// Set the opacity (0.0 = transparent, 1.0 = opaque).
    pub fn set_opacity(&self, val: f32) {
        unsafe { openui_sys::oui_element_set_opacity(self.raw, val) };
    }

    /// Set the z-index stacking order.
    pub fn set_z_index(&self, val: i32) {
        unsafe { openui_sys::oui_element_set_z_index(self.raw, val) };
    }

    // ─── Font / text ────────────────────────────────────────

    /// Set the font family name.
    pub fn set_font_family(&self, family: &str) -> Result<(), OuiError> {
        let c_family = CString::new(family).map_err(|_| OuiError::InvalidArgument)?;
        unsafe { openui_sys::oui_element_set_font_family(self.raw, c_family.as_ptr()) };
        Ok(())
    }

    /// Set the font size in pixels.
    pub fn set_font_size(&self, size: f32) {
        unsafe { openui_sys::oui_element_set_font_size(self.raw, Length::px(size)) };
    }

    /// Set the font weight (100–900, with 400 = normal, 700 = bold).
    pub fn set_font_weight(&self, weight: i32) {
        unsafe { openui_sys::oui_element_set_font_weight(self.raw, weight) };
    }

    /// Set the font style.
    pub fn set_font_style(&self, style: FontStyle) {
        unsafe { openui_sys::oui_element_set_font_style(self.raw, style.into()) };
    }

    /// Set the line height.
    pub fn set_line_height(&self, length: Length) {
        unsafe { openui_sys::oui_element_set_line_height(self.raw, length) };
    }

    /// Set the text alignment.
    pub fn set_text_align(&self, align: TextAlign) {
        unsafe { openui_sys::oui_element_set_text_align(self.raw, align.into()) };
    }

    // ─── Geometry queries ───────────────────────────────────

    /// X offset relative to the offset parent.
    pub fn offset_x(&self) -> f32 {
        unsafe { openui_sys::oui_element_get_offset_x(self.raw) }
    }

    /// Y offset relative to the offset parent.
    pub fn offset_y(&self) -> f32 {
        unsafe { openui_sys::oui_element_get_offset_y(self.raw) }
    }

    /// Computed width after layout.
    pub fn width(&self) -> f32 {
        unsafe { openui_sys::oui_element_get_width(self.raw) }
    }

    /// Computed height after layout.
    pub fn height(&self) -> f32 {
        unsafe { openui_sys::oui_element_get_height(self.raw) }
    }

    /// Bounding rectangle in document coordinates.
    pub fn bounding_rect(&self) -> Rect {
        unsafe { openui_sys::oui_element_get_bounding_rect(self.raw) }.into()
    }

    // ─── Scroll ─────────────────────────────────────────────

    /// Total scrollable width.
    pub fn scroll_width(&self) -> f32 {
        unsafe { openui_sys::oui_element_get_scroll_width(self.raw) }
    }

    /// Total scrollable height.
    pub fn scroll_height(&self) -> f32 {
        unsafe { openui_sys::oui_element_get_scroll_height(self.raw) }
    }

    /// Current horizontal scroll position.
    pub fn scroll_left(&self) -> f64 {
        unsafe { openui_sys::oui_element_get_scroll_left(self.raw) }
    }

    /// Current vertical scroll position.
    pub fn scroll_top(&self) -> f64 {
        unsafe { openui_sys::oui_element_get_scroll_top(self.raw) }
    }

    /// Scroll to an absolute position.
    pub fn scroll_to(&self, x: f64, y: f64) -> Result<(), OuiError> {
        check_status(unsafe { openui_sys::oui_element_scroll_to(self.raw, x, y) })
    }

    /// Scroll by a relative delta.
    pub fn scroll_by(&self, dx: f64, dy: f64) -> Result<(), OuiError> {
        check_status(unsafe { openui_sys::oui_element_scroll_by(self.raw, dx, dy) })
    }

    // ─── Focus ──────────────────────────────────────────────

    /// Give this element keyboard focus.
    pub fn focus(&self) -> Result<(), OuiError> {
        check_status(unsafe { openui_sys::oui_element_focus(self.raw) })
    }

    /// Remove keyboard focus from this element.
    pub fn blur(&self) -> Result<(), OuiError> {
        check_status(unsafe { openui_sys::oui_element_blur(self.raw) })
    }

    /// Check whether this element currently has keyboard focus.
    pub fn has_focus(&self) -> bool {
        unsafe { openui_sys::oui_element_has_focus(self.raw) != 0 }
    }

    // ─── Events ─────────────────────────────────────────────

    /// Register an event listener on this element.
    ///
    /// The closure is stored in a global registry and invoked via a C
    /// trampoline whenever the event fires. Only one callback per event
    /// type per element is supported; calling `on()` again for the same
    /// event type replaces the previous callback.
    pub fn on<F>(&self, event_type: &str, callback: F) -> Result<(), OuiError>
    where
        F: Fn(&Event) + 'static,
    {
        let boxed: Box<dyn Fn(&Event)> = Box::new(callback);
        let user_data = Box::into_raw(Box::new(boxed)) as *mut c_void;

        store_callback(self.raw as usize, event_type, user_data);

        let c_event_type = CString::new(event_type).map_err(|_| OuiError::InvalidArgument)?;
        check_status(unsafe {
            openui_sys::oui_element_set_event_callback(
                self.raw,
                c_event_type.as_ptr(),
                Some(event_trampoline),
                user_data,
            )
        })
    }

    /// Remove a previously registered event listener.
    pub fn remove_event(&self, event_type: &str) -> Result<(), OuiError> {
        free_callback(self.raw as usize, event_type);
        let c_event_type = CString::new(event_type).map_err(|_| OuiError::InvalidArgument)?;
        check_status(unsafe {
            openui_sys::oui_element_remove_event_callback(self.raw, c_event_type.as_ptr())
        })
    }

    // ─── Image injection ────────────────────────────────────

    /// Set raw RGBA pixel data on an `<img>` element.
    pub fn set_image_data(
        &self,
        pixels: &[u8],
        width: i32,
        height: i32,
    ) -> Result<(), OuiError> {
        check_status(unsafe {
            openui_sys::oui_element_set_image_data(self.raw, pixels.as_ptr(), width, height)
        })
    }

    /// Set encoded image data (PNG, JPEG, etc.) on an `<img>` element.
    pub fn set_image_encoded(&self, data: &[u8]) -> Result<(), OuiError> {
        check_status(unsafe {
            openui_sys::oui_element_set_image_encoded(self.raw, data.as_ptr(), data.len())
        })
    }

    // ─── Raw access ─────────────────────────────────────────

    /// Get the underlying raw FFI pointer (for advanced use).
    pub fn as_raw(&self) -> *mut openui_sys::OuiElement {
        self.raw
    }

    /// Create a **borrowed** (non-owning) `Element` handle from a raw pointer.
    ///
    /// The returned element will **not** destroy the underlying C object
    /// when dropped. This is used by the `view!` macro to create references
    /// inside reactive effects.
    ///
    /// # Safety
    ///
    /// `raw` must point to a valid, live `OuiElement` for the entire
    /// lifetime of the returned handle.
    #[doc(hidden)]
    pub unsafe fn from_raw_borrowed(raw: *mut openui_sys::OuiElement) -> Self {
        Element { raw, owned: false }
    }
}

impl Drop for Element {
    fn drop(&mut self) {
        if self.owned && !self.raw.is_null() {
            free_all_callbacks_for(self.raw as usize);
            unsafe { openui_sys::oui_element_destroy(self.raw) };
        }
    }
}

// ─── Compile-time tests ─────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure Element is not accidentally Send/Sync (it wraps a raw pointer
    // into single-threaded Blink state).
    // Element wraps a *mut OuiElement, which is !Send and !Sync.
    // This compile-time assertion verifies Element is also !Send.
    const _: () = {
        fn _must_not_be_send<T: ?Sized>() {}
        // *mut OuiElement is !Send, so Element (which contains it) is !Send.
    };
}
