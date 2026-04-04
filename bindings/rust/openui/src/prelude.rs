//! Convenience prelude for common imports.
//!
//! ```ignore
//! use openui::prelude::*;
//! ```

pub use crate::app::App;
pub use crate::context::{current_document, with_document};
pub use crate::document::Document;
pub use crate::element::Element;
pub use crate::events::*;
pub use crate::renderer::{DynChild, For, Show};
pub use crate::style::*;
pub use crate::view_node::{mount_view, IntoView, ViewNode};
pub use openui_macros::{component, view};

// Reactive primitives
pub use crate::effect::{batch, create_effect};
pub use crate::runtime::{EffectId, ScopeId, SignalId};
pub use crate::scope::{create_scope, dispose_scope, on_cleanup};
pub use crate::signal::{create_memo, create_signal, Memo, Signal};
