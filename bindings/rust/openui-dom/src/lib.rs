//! Native element tree — lightweight arena-allocated nodes.
//!
//! This replaces Blink's full DOM with a minimal tree structure that stores:
//! - Parent/child relationships
//! - ComputedStyle per node
//! - Element type tag (div, span, text, etc.)
//!
//! No parsing, no DOM API, no event handling — just a tree of styled nodes
//! that the layout algorithm traverses.

mod tree;

pub use tree::{Document, NodeId, NodeData, ElementTag};
