//! Layout fragments — the output of a layout algorithm.
//!
//! Extracted from Blink's `PhysicalFragment` / `PhysicalBoxFragment`.
//! A fragment represents a positioned piece of the layout tree, ready for
//! painting. The fragment tree mirrors the element tree but with concrete
//! sizes and offsets.

use openui_geometry::{LayoutUnit, PhysicalOffset, PhysicalSize, BoxStrut};
use openui_dom::NodeId;

/// What kind of fragment this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentKind {
    /// A box fragment (from a block, flex, grid, or inline-block element).
    Box,
    /// A text fragment (from a text node).
    Text,
    /// The root viewport fragment.
    Viewport,
}

/// A positioned layout fragment, ready for painting.
///
/// Mirrors Blink's `PhysicalBoxFragment`. Contains the resolved size,
/// position relative to parent, and references back to the DOM node
/// and style for painting.
#[derive(Debug)]
pub struct Fragment {
    /// Which DOM node produced this fragment.
    pub node_id: NodeId,

    /// What type of fragment.
    pub kind: FragmentKind,

    /// Offset from the parent fragment's top-left corner.
    pub offset: PhysicalOffset,

    /// The fragment's border-box size.
    pub size: PhysicalSize,

    /// Resolved padding (in LayoutUnit).
    pub padding: BoxStrut,

    /// Resolved border widths (in LayoutUnit).
    pub border: BoxStrut,

    /// Resolved margin (in LayoutUnit, for debug/paint use).
    pub margin: BoxStrut,

    /// Child fragments, positioned relative to this fragment.
    pub children: Vec<Fragment>,
}

impl Fragment {
    /// Create a new box fragment.
    pub fn new_box(node_id: NodeId, size: PhysicalSize) -> Self {
        Self {
            node_id,
            kind: FragmentKind::Box,
            offset: PhysicalOffset::zero(),
            size,
            padding: BoxStrut::zero(),
            border: BoxStrut::zero(),
            margin: BoxStrut::zero(),
            children: Vec::new(),
        }
    }

    /// The content box rect (border-box minus border minus padding).
    pub fn content_offset(&self) -> PhysicalOffset {
        PhysicalOffset::new(
            self.border.left + self.padding.left,
            self.border.top + self.padding.top,
        )
    }

    /// The content box size.
    pub fn content_size(&self) -> PhysicalSize {
        PhysicalSize::new(
            self.size.width - self.border.left - self.border.right
                - self.padding.left - self.padding.right,
            self.size.height - self.border.top - self.border.bottom
                - self.padding.top - self.padding.bottom,
        )
    }

    /// The padding box size (border-box minus border).
    pub fn padding_box_size(&self) -> PhysicalSize {
        PhysicalSize::new(
            self.size.width - self.border.left - self.border.right,
            self.size.height - self.border.top - self.border.bottom,
        )
    }

    /// Width of the border-box.
    #[inline]
    pub fn width(&self) -> LayoutUnit { self.size.width }

    /// Height of the border-box.
    #[inline]
    pub fn height(&self) -> LayoutUnit { self.size.height }
}
