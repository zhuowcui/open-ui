//! Convenience prelude for common imports.
//!
//! ```ignore
//! use openui::prelude::*;
//! ```

pub use crate::context::{current_document, with_document};
pub use crate::document::Document;
pub use crate::element::Element;
pub use crate::events::*;
pub use crate::style::*;
pub use crate::view_node::{mount_view, IntoView, ViewNode};
pub use openui_macros::{component, view};
