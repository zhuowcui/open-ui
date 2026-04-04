//! # Open UI
//!
//! Safe, idiomatic Rust bindings for the Open UI C API (Chromium/Blink
//! rendering pipeline) together with a Leptos-style fine-grained reactive
//! runtime.
//!
//! ## Reactive primitives
//!
//! | Primitive | Purpose |
//! |-----------|---------|
//! | [`Signal`] | Readable/writable reactive value |
//! | [`Memo`] | Cached derived value |
//! | [`create_effect`] | Side-effect that re-runs on dependency changes |
//! | [`batch`] | Defer effect execution across multiple updates |
//! | [`create_scope`] | Group effects for batch disposal |
//! | [`dispose_scope`] | Tear down a scope and its children |
//! | [`on_cleanup`] | Register a cleanup callback in the current scope |
//!
//! ## DOM wrappers
//!
//! | Type | Purpose |
//! |------|---------|
//! | [`Document`] | Blink viewport and rendering context |
//! | [`Element`] | A node in the DOM tree |
//!
//! ## View system
//!
//! | Type / Macro | Purpose |
//! |--------------|---------|
//! | [`view!`] | JSX-like declarative UI macro |
//! | [`component`] | Attribute macro to define a component |
//! | [`ViewNode`] | A renderable node in the UI tree |
//! | [`IntoView`] | Trait for converting values into view nodes |
//! | [`mount_view`] | Mount a view node onto a parent element |
//! | [`with_document`] | Set the render context document |
//! | [`current_document`] | Access the current render context document |

// ─── Reactive runtime modules ───────────────────────────────

pub mod effect;
pub mod runtime;
pub mod scope;
pub mod signal;

// ─── DOM wrapper modules ────────────────────────────────────

pub mod document;
pub mod element;
pub mod events;
pub mod style;

// ─── View system modules ────────────────────────────────────

pub mod context;
pub mod view_node;

/// Convenience prelude for common imports.
pub mod prelude;

// ─── Re-exports: reactive primitives ────────────────────────

pub use effect::{batch, create_effect};
pub use runtime::{EffectId, ScopeId, SignalId};
pub use scope::{create_scope, dispose_scope, on_cleanup};
pub use signal::{create_memo, create_signal, Memo, Signal};

// ─── Re-exports: DOM wrappers ───────────────────────────────

pub use document::Document;
pub use element::Element;
pub use events::{Event, KeyEventType, Modifiers, MouseButton, MouseEventType};
pub use style::{
    AlignItems, Bitmap, Display, FlexDirection, FlexWrap, FontStyle, JustifyContent, Length,
    OuiError, Overflow, Position, Rect, TextAlign,
};

// ─── Re-exports: view system ────────────────────────────────

pub use context::{current_document, with_document};
pub use view_node::{mount_view, IntoView, ViewNode};

// ─── Re-exports: proc macros ────────────────────────────────

pub use openui_macros::{component, view};

#[cfg(test)]
mod tests;
