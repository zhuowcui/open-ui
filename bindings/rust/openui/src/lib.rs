//! # Open UI
//!
//! Safe, direct Rust framework over the pure-Rust Open UI retained engine.
//! Application interaction uses Rust callbacks and native [`Document`] and
//! [`Element`] methods. Browser-like element behavior needed by applications
//! belongs in this public native Rust API. Open UI does not execute JavaScript
//! or expose browser script bindings.
//!
//! ```no_run
//! use openui::prelude::*;
//!
//! # fn main() -> Result<(), Error> {
//! let count = create_signal(0_i32);
//! let app = App::builder()
//!     .title("Open UI")
//!     .size(LogicalSize::new(800.0, 600.0))
//!     .backend(BackendPreference::Auto)
//!     .build()?;
//!
//! app.run(move || view! {
//!     <button
//!         style:display={Display::Flex}
//!         style:padding="8px 16px"
//!         on:click={move |_| count.update(|value| *value += 1)}
//!     >
//!         {count.get()}
//!     </button>
//! })?;
//! # Ok(())
//! # }
//! ```
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
//! ## Native document and element handles
//!
//! | Type | Purpose |
//! |------|---------|
//! | [`Document`] | Native retained document and rendering context |
//! | [`Element`] | A node in the retained document tree |
//! | [`TextNode`] | Native text data that can be created, edited, attached, and moved |
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

extern crate self as openui;

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
pub mod text_node;

// ─── View system modules ────────────────────────────────────

pub mod context;
pub mod renderer;
pub mod view_node;

mod generated_style_setters;

// ─── Application shell ─────────────────────────────────────

pub mod app;

/// Convenience prelude for common imports.
pub mod prelude;

// ─── Re-exports: reactive primitives ────────────────────────

pub use effect::{batch, create_effect};
pub use runtime::{EffectId, ScopeId, SignalId};
pub use scope::{create_scope, dispose_scope, on_cleanup};
pub use signal::{create_memo, create_signal, Memo, Signal};

// ─── Re-exports: DOM wrappers ───────────────────────────────

pub use document::Document;
pub use element::{Element, WeakElement};
pub use events::{Event, EventPhase, KeyEventType, Modifiers, MouseButton, MouseEventType};
pub use openui_dom::ElementTag;
pub use style::{Bitmap, Error, Rect};
pub use text_node::{TextNode, WeakTextNode};

/// Canonical typed style values shared with the engine, macro, and C schema.
pub mod typed_style {
    pub use openui_style::*;
}
pub use openui_style::{ComputedStyle, Style, StyleProperty, StyleValue};
pub use openui_text::{
    FontAxisRange, FontCollection, FontCollectionError, FontCollectionStats, FontContainerFormat,
    FontFaceDescriptor, FontFaceHandle, FontFaceInfo, FontFeatureDefault, FontMetricOverrides,
    FontPaletteBase, FontPaletteEntryOverride, FontPaletteHandle, FontPaletteValuesDescriptor,
    FontStyleRange, FontUnicodeRange, HyphenationDictionaryHandle, HyphenationRegistry,
    HyphenationRegistryError,
};

// ─── Re-exports: view system ────────────────────────────────

pub use context::{current_document, with_document};
pub use renderer::{DynChild, For, Show};
pub use view_node::{mount_view, IntoView, ViewNode};

// ─── Re-exports: application shell ─────────────────────────

pub use app::{
    App, AppBuilder, AppExitHandle, BackendPreference, HeadlessApp, LogicalSize, RenderOptions,
    WindowOptions,
};
pub use openui_engine::{
    AccessibilityAction, AccessibilityLive, AccessibilityNode, AccessibilityNodeId,
    AccessibilityPlatformAction, AccessibilityRelation, AccessibilityRole, AccessibilityTreeUpdate,
    AnimationEvent, AnimationEventKind, AnimationId, AnimationState, ControlAdjustment,
    EditCommand, EngineOptions, FocusOrigin, PointerEventKind, RasterBackend, RasterConfiguration,
    RasterPixelGeometry, ScrollAlignment, ScrollAnimationId, ScrollIntoViewContainer,
    ScrollIntoViewOptions, ScrollMetrics, TextDirection, TextEdging, TextHinting,
    TextRasterConfiguration, TextUnit, ViewportAuthority, ViewportMetrics, ViewportMetricsError,
};
#[cfg(all(feature = "linux", target_os = "linux"))]
pub use openui_platform::{KeyboardInput, PlatformEvent};

// ─── Re-exports: proc macros ────────────────────────────────

pub use openui_macros::{component, view};

#[cfg(test)]
mod tests;
