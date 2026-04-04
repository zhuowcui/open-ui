//! ConstraintSpace — the input to a layout algorithm.
//!
//! Extracted from Blink's `ConstraintSpace` (core/layout/constraint_space.h).
//! This carries the available size, percentage resolution size, and various
//! flags that the parent layout passes to each child.

use openui_geometry::LayoutUnit;

/// Layout input constraints passed from parent to child.
///
/// Mirrors Blink's `ConstraintSpace`. Extended in SP10 with flex-specific
/// fields that control how flex children resolve their sizes.
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

    // ── Flex-specific fields (SP10) ──────────────────────────────────

    /// True when the inline size is externally determined (e.g., row flex main axis).
    /// Child should use available_inline_size as its exact inline size.
    /// Blink: `SetIsFixedInlineSize(true)` in `BuildSpaceForLayout`.
    pub is_fixed_inline_size: bool,

    /// True when the block size is externally determined (e.g., column flex main axis).
    /// Child should use available_block_size as its exact block size.
    /// Blink: `SetIsFixedBlockSize(true)` in `BuildSpaceForLayout`.
    pub is_fixed_block_size: bool,

    /// True when the child should stretch its inline size to fill the
    /// cross axis (column flex with align-self: stretch).
    /// Blink: `SetInlineAutoBehavior(AutoSizeBehavior::kStretchExplicit)`.
    pub stretch_inline_size: bool,

    /// True when the child should stretch its block size to fill the
    /// cross axis (row flex with align-self: stretch).
    /// Blink: `SetBlockAutoBehavior(AutoSizeBehavior::kStretchExplicit)`.
    pub stretch_block_size: bool,

    /// True for column flex children where the container's block size is
    /// indefinite. Prevents percent heights from resolving.
    /// Blink: `SetIsInitialBlockSizeIndefinite(true)`.
    pub is_initial_block_size_indefinite: bool,
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
            is_fixed_inline_size: false,
            is_fixed_block_size: false,
            stretch_inline_size: false,
            stretch_block_size: false,
            is_initial_block_size_indefinite: false,
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
            is_fixed_inline_size: false,
            is_fixed_block_size: false,
            stretch_inline_size: false,
            stretch_block_size: false,
            is_initial_block_size_indefinite: false,
        }
    }

    /// Create a constraint space for a flex child with externally determined sizes.
    /// Blink: `BuildSpaceForLayout` in `flex_layout_algorithm.cc:694`.
    pub fn for_flex_child(
        available_inline_size: LayoutUnit,
        available_block_size: LayoutUnit,
        percentage_inline: LayoutUnit,
        percentage_block: LayoutUnit,
    ) -> Self {
        Self {
            available_inline_size,
            available_block_size,
            percentage_resolution_inline_size: percentage_inline,
            percentage_resolution_block_size: percentage_block,
            is_new_formatting_context: true, // flex children always establish new FC
            is_fixed_inline_size: false,
            is_fixed_block_size: false,
            stretch_inline_size: false,
            stretch_block_size: false,
            is_initial_block_size_indefinite: false,
        }
    }
}
