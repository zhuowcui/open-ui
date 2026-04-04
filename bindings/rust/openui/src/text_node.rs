//! Safe wrapper around `OuiTextNode` — a mutable DOM Text node handle.
//!
//! Unlike [`Element`], a `TextNode` wraps `blink::Text` (not `blink::Element`).
//! It is created via [`Element::create_text_child`] and provides a
//! [`set_data`](TextNode::set_data) method for reactive text updates without
//! introducing wrapper elements like `<span>`.

use std::ffi::CString;

/// RAII handle to a DOM Text node.
///
/// When dropped (if owned), removes the text node from the DOM and frees
/// the underlying C object.
pub struct TextNode {
    raw: *mut openui_sys::OuiTextNode,
    owned: bool,
}

impl TextNode {
    /// Wrap a raw pointer, taking ownership.
    ///
    /// # Safety
    /// `raw` must be a valid pointer returned by `oui_element_create_text_child`
    /// that has not been destroyed.
    pub(crate) unsafe fn from_raw(raw: *mut openui_sys::OuiTextNode) -> Self {
        TextNode { raw, owned: true }
    }

    /// Wrap a raw pointer *without* ownership — drop will not destroy it.
    ///
    /// # Safety
    /// `raw` must be a valid, non-null `OuiTextNode` pointer that outlives
    /// the returned `TextNode`.
    pub unsafe fn from_raw_borrowed(raw: *mut openui_sys::OuiTextNode) -> Self {
        TextNode { raw, owned: false }
    }

    /// Returns the underlying raw pointer.
    pub fn as_raw(&self) -> *mut openui_sys::OuiTextNode {
        self.raw
    }

    /// Update the text content of this node.
    pub fn set_data(&self, data: &str) {
        let c = CString::new(data).unwrap_or_default();
        unsafe { openui_sys::oui_text_node_set_data(self.raw, c.as_ptr()) };
    }
}

impl Drop for TextNode {
    fn drop(&mut self) {
        if self.owned && !self.raw.is_null() {
            unsafe { openui_sys::oui_text_node_destroy(self.raw) };
        }
    }
}
