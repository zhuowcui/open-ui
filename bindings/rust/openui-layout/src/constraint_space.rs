//! ConstraintSpace — the input to a layout algorithm.
//!
//! Extracted from Blink's `ConstraintSpace` (core/layout/constraint_space.h).
//! This carries the available size, percentage resolution size, and various
//! flags that the parent layout passes to each child.

use openui_geometry::LayoutUnit;

/// Layout input constraints passed from parent to child.
///
/// Mirrors Blink's `ConstraintSpace`. For SP9 (block layout) we only need
/// the available size and percentage resolution size.
#[derive(Debug, Clone)]
pub struct ConstraintSpace {
    /// Available inline size (width in horizontal-tb).
    /// The maximum width this child can expand to.
    pub available_inline_size: LayoutUnit,

    /// Available block size (height in horizontal-tb).
    /// `LayoutUnit::indefinite()` if unconstrained (common for height).
    pub available_block_size: LayoutUnit,

    /// The size to use for resolving percentage widths.
    /// Usually the same as available_inline_size.
    pub percentage_resolution_inline_size: LayoutUnit,

    /// The size to use for resolving percentage heights.
    /// Can be indefinite.
    pub percentage_resolution_block_size: LayoutUnit,

    /// True if this element is at the start of a new BFC.
    pub is_new_formatting_context: bool,
}

impl ConstraintSpace {
    /// Create a constraint space for the root viewport.
    pub fn for_root(width: LayoutUnit, height: LayoutUnit) -> Self {
        Self {
            available_inline_size: width,
            available_block_size: height,
            percentage_resolution_inline_size: width,
            percentage_resolution_block_size: height,
            is_new_formatting_context: true,
        }
    }

    /// Create a constraint space for a child in normal block flow.
    pub fn for_block_child(
        available_inline_size: LayoutUnit,
        available_block_size: LayoutUnit,
        percentage_inline: LayoutUnit,
        percentage_block: LayoutUnit,
        is_new_fc: bool,
    ) -> Self {
        Self {
            available_inline_size,
            available_block_size,
            percentage_resolution_inline_size: percentage_inline,
            percentage_resolution_block_size: percentage_block,
            is_new_formatting_context: is_new_fc,
        }
    }
}
