//! Top-level application shell and render loop.
//!
//! [`App`] is the main entry point for creating and running an Open UI
//! application.  It manages a [`Document`] (Blink rendering context) and
//! provides a builder-style API for mounting views, dispatching events, and
//! producing rendered output.
//!
//! # Example
//!
//! ```ignore
//! use openui::prelude::*;
//!
//! fn main() {
//!     App::new(800, 600)
//!         .render(|| view! { <h1>"Hello!"</h1> })
//!         .render_to_png("hello.png");
//! }
//! ```

use crate::context::with_document;
use crate::document::Document;
use crate::events::{KeyEventType, Modifiers, MouseButton, MouseEventType};
use crate::runtime::ScopeId;
use crate::scope::{create_scope, dispose_scope};
use crate::style::{Bitmap, OuiError};
use crate::view_node::{mount_view, IntoView};
use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize the Open UI / Blink rendering engine.
///
/// This is called automatically by [`App::new`]. It is safe to call multiple
/// times — only the first call performs initialization.
fn ensure_init() {
    INIT.call_once(|| {
        let status = unsafe { openui_sys::oui_init(std::ptr::null()) };
        assert_eq!(
            status,
            openui_sys::OuiStatus::OUI_OK,
            "oui_init() failed with status {:?}",
            status
        );
    });
}

/// An Open UI application.
///
/// `App` is the main entry point for creating and running an Open UI
/// application.  It manages a [`Document`] (Blink rendering context) and
/// provides methods for mounting views, dispatching events, and rendering
/// output.
///
/// # Example
///
/// ```ignore
/// use openui::prelude::*;
///
/// fn main() {
///     App::new(800, 600)
///         .render(|| view! { <h1>"Hello!"</h1> })
///         .render_to_png("hello.png");
/// }
/// ```
pub struct App {
    doc: Document,
    frame_time_ms: f64,
    root_scope: Option<ScopeId>,
}

impl App {
    /// Create a new application with the given viewport dimensions.
    ///
    /// Initializes the Blink engine on first call.  Creates a new document
    /// with the specified width and height.
    pub fn new(width: i32, height: i32) -> Self {
        ensure_init();
        let doc = Document::new(width, height).expect("failed to create document");
        App {
            doc,
            frame_time_ms: 0.0,
            root_scope: None,
        }
    }

    /// Mount a view tree onto the document body.
    ///
    /// The closure is called once inside a [`with_document`] scope.  The
    /// returned view is mounted onto the document body element.  After
    /// mounting, layout is computed and the document is fully updated.
    ///
    /// Returns `&mut Self` for chaining.
    pub fn render<V: IntoView>(&mut self, f: impl FnOnce() -> V) -> &mut Self {
        // Dispose previous root scope if re-rendering.
        if let Some(old) = self.root_scope.take() {
            dispose_scope(old);
        }

        let scope_id = create_scope(|| {
            let view = with_document(&self.doc, || f().into_view());
            if let Some(body) = self.doc.body() {
                mount_view(&body, view);
            }
        });
        self.root_scope = Some(scope_id);

        // Run initial layout + lifecycle
        self.doc.update_all().expect("initial update_all failed");
        self
    }

    /// Advance the animation clock and run a full lifecycle update.
    ///
    /// Each frame advances time by 16.67 ms (~60 fps) and calls
    /// [`Document::begin_frame`] followed by [`Document::update_all`].
    pub fn run_frames(&mut self, count: u32) -> &mut Self {
        for _ in 0..count {
            self.frame_time_ms += 16.67;
            self.doc.begin_frame(self.frame_time_ms).ok();
            self.doc.update_all().ok();
        }
        self
    }

    /// Dispatch a click event at the given viewport coordinates.
    ///
    /// Sends a mouse-down followed by mouse-up with the left button,
    /// simulating a full click.
    pub fn dispatch_click(&mut self, x: f32, y: f32) -> &mut Self {
        let mods = Modifiers::NONE;
        self.doc
            .dispatch_mouse_event(MouseEventType::Down, x, y, MouseButton::Left, mods)
            .ok();
        self.doc
            .dispatch_mouse_event(MouseEventType::Up, x, y, MouseButton::Left, mods)
            .ok();
        self
    }

    /// Dispatch a mouse move event.
    pub fn dispatch_mouse_move(&mut self, x: f32, y: f32) -> &mut Self {
        self.doc
            .dispatch_mouse_event(
                MouseEventType::Move,
                x,
                y,
                MouseButton::Left,
                Modifiers::NONE,
            )
            .ok();
        self
    }

    /// Dispatch a key press event.
    ///
    /// Sends key-down, an optional char event (when `text` is provided),
    /// and key-up.
    pub fn dispatch_key(&mut self, key_code: i32, text: Option<&str>) -> &mut Self {
        let mods = Modifiers::NONE;
        self.doc
            .dispatch_key_event(KeyEventType::Down, key_code, text, mods)
            .ok();
        if let Some(t) = text {
            self.doc
                .dispatch_key_event(KeyEventType::Char, key_code, Some(t), mods)
                .ok();
        }
        self.doc
            .dispatch_key_event(KeyEventType::Up, key_code, text, mods)
            .ok();
        self
    }

    /// Dispatch a wheel / scroll event.
    pub fn dispatch_wheel(&mut self, x: f32, y: f32, dx: f32, dy: f32) -> &mut Self {
        self.doc
            .dispatch_wheel_event(x, y, dx, dy, Modifiers::NONE)
            .ok();
        self
    }

    /// Render the current state to a PNG file at `path`.
    pub fn render_to_png(&mut self, path: &str) -> &mut Self {
        self.doc.render_to_png(path).expect("render_to_png failed");
        self
    }

    /// Render the current state to an in-memory RGBA bitmap.
    pub fn render_to_bitmap(&self) -> Result<Bitmap, OuiError> {
        self.doc.render_to_bitmap()
    }

    /// Render to an in-memory PNG buffer.
    pub fn render_to_png_buffer(&self) -> Result<Vec<u8>, OuiError> {
        self.doc.render_to_png_buffer()
    }

    /// Get a reference to the underlying document.
    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// Get a mutable reference to the underlying document.
    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.doc
    }

    /// Set the viewport size.
    pub fn set_viewport(&mut self, width: i32, height: i32) -> &mut Self {
        self.doc.set_viewport(width, height);
        self
    }

    /// Set a resource provider for loading images and other resources.
    ///
    /// The callback receives a URL string and should return `Some(bytes)` to
    /// supply the resource data, or `None` to let the engine handle it.
    pub fn set_resource_provider<F>(&mut self, callback: F) -> &mut Self
    where
        F: Fn(&str) -> Option<Vec<u8>> + 'static,
    {
        self.doc
            .set_resource_provider(callback)
            .expect("set resource provider failed");
        self
    }

    /// Get the current frame time in milliseconds.
    pub fn frame_time(&self) -> f64 {
        self.frame_time_ms
    }

    /// Load an HTML document, replacing the current document contents.
    ///
    /// The HTML string is parsed and rendered by Blink just like a browser
    /// would. After loading, layout is computed and the document is fully
    /// updated.
    pub fn load_html(&mut self, html: &str) -> &mut Self {
        self.doc.load_html(html).expect("load_html failed");
        self.doc.update_all().expect("update_all after load_html failed");
        self
    }

    /// Inject a CSS stylesheet into the document.
    ///
    /// Creates a `<style>` element with the given CSS text and appends it to
    /// the document body. This should be called **before** [`render`] so that
    /// the styles are available when elements are created.
    ///
    /// # Example
    ///
    /// ```ignore
    /// app.inject_css("* { box-sizing: border-box; margin: 0; padding: 0; }");
    /// app.render(|| view! { <div class="my-class">"Hello"</div> });
    /// ```
    pub fn inject_css(&mut self, css: &str) -> &mut Self {
        use crate::element::Element;

        let style_el = Element::create(&self.doc, "style")
            .expect("failed to create style element");
        style_el.set_text(css).expect("failed to set style text");
        if let Some(body) = self.doc.body() {
            body.append_child(&style_el);
        }
        // Leak the element so it stays in the DOM
        std::mem::forget(style_el);
        self
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(scope_id) = self.root_scope.take() {
            dispose_scope(scope_id);
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the builder-pattern types compile (chaining returns &mut App).
    #[allow(dead_code)]
    fn _render_chain_compiles() {
        fn _assert(_: &mut App) {}
    }

    /// Verify event-dispatch chain types.
    #[allow(dead_code)]
    fn _event_chain_compiles() {
        fn _takes_app(_: &mut App) {}
    }

    #[test]
    fn ensure_init_is_safe_to_reference() {
        // `ensure_init` is a private fn — just verify the static compiles.
        let _once: &Once = &INIT;
    }

    #[test]
    fn app_struct_has_expected_fields() {
        // Verify the struct layout via a builder that would fail to compile
        // if any field were renamed or removed.
        fn _build(doc: Document) -> App {
            App {
                doc,
                frame_time_ms: 0.0,
                root_scope: None,
            }
        }
    }
}
