//! ConstraintSpace — the input to a layout algorithm.
//!
//! Extracted from Blink's `ConstraintSpace` (core/layout/constraint_space.h).
//! This carries the available size, percentage resolution size, BFC state,
//! exclusion space, and various flags that the parent layout passes to each child.
//!
//! Extended in SP10 with flex-specific fields, SP12 with BFC, float, and
//! fragmentation fields, and SP17 with an authoritative writing direction.

use openui_geometry::{BfcOffset, LayoutUnit, WritingDirectionMode};
use std::sync::Arc;

use crate::exclusions::ExclusionSpace;

/// Layout input constraints passed from parent to child.
///
/// Mirrors Blink's `ConstraintSpace`. Contains available space, percentage
/// bases, BFC coordinates, exclusion space reference, and formatting context
/// flags.
///
/// Source: `constraint_space.h` (1,652 lines in Blink).
#[derive(Debug, Clone)]
pub struct ConstraintSpace {
    // ── Coordinate system ───────────────────────────────────────────
    /// Writing direction that defines the inline/block axes of every logical
    /// size and offset stored in this space.
    pub writing_direction: WritingDirectionMode,

    // ── Available space ──────────────────────────────────────────────
    /// Available inline size (width in horizontal-tb).
    pub available_inline_size: LayoutUnit,

    /// Available block size (height in horizontal-tb).
    /// `INDEFINITE_SIZE` if unconstrained (common for height).
    pub available_block_size: LayoutUnit,

    // ── Percentage resolution ────────────────────────────────────────
    /// The size to use for resolving percentage widths.
    pub percentage_resolution_inline_size: LayoutUnit,

    /// The size to use for resolving percentage heights. Can be indefinite.
    pub percentage_resolution_block_size: LayoutUnit,

    // ── BFC state (SP12) ─────────────────────────────────────────────
    /// The offset of this element within its block formatting context.
    /// `None` if the BFC offset is not yet known (pending resolution).
    pub bfc_offset: BfcOffset,

    /// The BFC block offset at which floats were last positioned. Used for
    /// float avoidance queries when BFC offset is still pending.
    pub floats_bfc_block_offset: Option<LayoutUnit>,

    /// Shared exclusion space tracking float exclusion rectangles in this BFC.
    /// `None` when no floats are present or when establishing a new BFC.
    pub exclusion_space: Option<Arc<ExclusionSpace>>,

    // ── Formatting context flags ─────────────────────────────────────
    /// True if this element establishes a new BFC. Elements with overflow
    /// != visible, floats, absolutely positioned elements, inline-blocks,
    /// flex/grid containers, etc. all establish new BFCs.
    pub is_new_formatting_context: bool,

    // ── Flex-specific fields (SP10) ──────────────────────────────────
    /// True when the inline size is externally determined (e.g., row flex main axis).
    pub is_fixed_inline_size: bool,

    /// True when the block size is externally determined (e.g., column flex main axis).
    pub is_fixed_block_size: bool,

    /// True when the child should stretch its inline size to fill the cross axis.
    pub stretch_inline_size: bool,

    /// True when the child should stretch its block size to fill the cross axis.
    pub stretch_block_size: bool,

    /// True for column flex children where the container's block size is indefinite.
    pub is_initial_block_size_indefinite: bool,

    // ── Fragmentation fields (SP12) ──────────────────────────────────
    /// Block size of the current fragmentainer (column, page). Zero means
    /// no fragmentation context.
    pub fragmentainer_block_size: LayoutUnit,

    /// How far into the current fragmentainer this element starts.
    pub block_offset_in_fragmentainer: LayoutUnit,

    /// Whether the layout is being resumed from a previous fragmentainer break.
    pub is_resuming: bool,

    // ── Baseline request (SP12) ──────────────────────────────────────
    /// Whether the parent needs a first baseline from this child.
    pub needs_first_baseline: bool,

    /// Whether the parent needs a last baseline from this child.
    pub needs_last_baseline: bool,
}

impl ConstraintSpace {
    /// Create a constraint space for the root viewport.
    pub fn for_root(width: LayoutUnit, height: LayoutUnit) -> Self {
        Self::for_root_with_writing_direction(width, height, WritingDirectionMode::horizontal_ltr())
    }

    /// Create a root space from physical viewport dimensions.
    ///
    /// The stored sizes are logical in `writing_direction`: vertical and
    /// sideways roots therefore use viewport height as their inline size and
    /// viewport width as their block size.
    pub fn for_root_with_writing_direction(
        physical_width: LayoutUnit,
        physical_height: LayoutUnit,
        writing_direction: WritingDirectionMode,
    ) -> Self {
        let (inline_size, block_size) = if writing_direction.is_horizontal() {
            (physical_width, physical_height)
        } else {
            (physical_height, physical_width)
        };
        Self {
            writing_direction,
            available_inline_size: inline_size,
            available_block_size: block_size,
            percentage_resolution_inline_size: inline_size,
            percentage_resolution_block_size: block_size,
            bfc_offset: BfcOffset::zero(),
            floats_bfc_block_offset: None,
            exclusion_space: None,
            is_new_formatting_context: true,
            is_fixed_inline_size: false,
            is_fixed_block_size: false,
            stretch_inline_size: false,
            stretch_block_size: false,
            is_initial_block_size_indefinite: false,
            fragmentainer_block_size: LayoutUnit::zero(),
            block_offset_in_fragmentainer: LayoutUnit::zero(),
            is_resuming: false,
            needs_first_baseline: false,
            needs_last_baseline: false,
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
        Self::for_block_child_with_writing_direction(
            available_inline_size,
            available_block_size,
            percentage_inline,
            percentage_block,
            is_new_fc,
            WritingDirectionMode::horizontal_ltr(),
        )
    }

    /// Create a block-child space from sizes already expressed in the child's
    /// logical coordinate system.
    pub fn for_block_child_with_writing_direction(
        available_inline_size: LayoutUnit,
        available_block_size: LayoutUnit,
        percentage_inline: LayoutUnit,
        percentage_block: LayoutUnit,
        is_new_fc: bool,
        writing_direction: WritingDirectionMode,
    ) -> Self {
        Self {
            writing_direction,
            available_inline_size,
            available_block_size,
            percentage_resolution_inline_size: percentage_inline,
            percentage_resolution_block_size: percentage_block,
            bfc_offset: BfcOffset::zero(),
            floats_bfc_block_offset: None,
            exclusion_space: None,
            is_new_formatting_context: is_new_fc,
            is_fixed_inline_size: false,
            is_fixed_block_size: false,
            stretch_inline_size: false,
            stretch_block_size: false,
            is_initial_block_size_indefinite: false,
            fragmentainer_block_size: LayoutUnit::zero(),
            block_offset_in_fragmentainer: LayoutUnit::zero(),
            is_resuming: false,
            needs_first_baseline: false,
            needs_last_baseline: false,
        }
    }

    /// Create a block-child space from sizes expressed in the parent's logical
    /// axes, transposing both available and percentage bases for an orthogonal
    /// child.
    #[allow(clippy::too_many_arguments)]
    pub fn for_block_child_from_parent(
        parent: &ConstraintSpace,
        available_inline_size: LayoutUnit,
        available_block_size: LayoutUnit,
        percentage_inline: LayoutUnit,
        percentage_block: LayoutUnit,
        is_new_fc: bool,
        child_writing_direction: WritingDirectionMode,
    ) -> Self {
        let (available_inline_size, available_block_size) = Self::convert_logical_size_between(
            available_inline_size,
            available_block_size,
            parent.writing_direction,
            child_writing_direction,
        );
        let (percentage_inline, percentage_block) = Self::convert_logical_size_between(
            percentage_inline,
            percentage_block,
            parent.writing_direction,
            child_writing_direction,
        );
        Self::for_block_child_with_writing_direction(
            available_inline_size,
            available_block_size,
            percentage_inline,
            percentage_block,
            is_new_fc,
            child_writing_direction,
        )
    }

    /// Create a constraint space for a flex child with externally determined sizes.
    pub fn for_flex_child(
        available_inline_size: LayoutUnit,
        available_block_size: LayoutUnit,
        percentage_inline: LayoutUnit,
        percentage_block: LayoutUnit,
    ) -> Self {
        Self::for_flex_child_with_writing_direction(
            available_inline_size,
            available_block_size,
            percentage_inline,
            percentage_block,
            WritingDirectionMode::horizontal_ltr(),
        )
    }

    /// Create a flex-child space from sizes already expressed in the child's
    /// logical coordinate system.
    pub fn for_flex_child_with_writing_direction(
        available_inline_size: LayoutUnit,
        available_block_size: LayoutUnit,
        percentage_inline: LayoutUnit,
        percentage_block: LayoutUnit,
        writing_direction: WritingDirectionMode,
    ) -> Self {
        Self {
            writing_direction,
            available_inline_size,
            available_block_size,
            percentage_resolution_inline_size: percentage_inline,
            percentage_resolution_block_size: percentage_block,
            bfc_offset: BfcOffset::zero(),
            floats_bfc_block_offset: None,
            exclusion_space: None,
            is_new_formatting_context: true,
            is_fixed_inline_size: false,
            is_fixed_block_size: false,
            stretch_inline_size: false,
            stretch_block_size: false,
            is_initial_block_size_indefinite: false,
            fragmentainer_block_size: LayoutUnit::zero(),
            block_offset_in_fragmentainer: LayoutUnit::zero(),
            is_resuming: false,
            needs_first_baseline: false,
            needs_last_baseline: false,
        }
    }

    /// Create a flex-child space from sizes expressed in the flex container's
    /// logical axes.
    pub fn for_flex_child_from_parent(
        parent: &ConstraintSpace,
        available_inline_size: LayoutUnit,
        available_block_size: LayoutUnit,
        percentage_inline: LayoutUnit,
        percentage_block: LayoutUnit,
        child_writing_direction: WritingDirectionMode,
    ) -> Self {
        let (available_inline_size, available_block_size) = Self::convert_logical_size_between(
            available_inline_size,
            available_block_size,
            parent.writing_direction,
            child_writing_direction,
        );
        let (percentage_inline, percentage_block) = Self::convert_logical_size_between(
            percentage_inline,
            percentage_block,
            parent.writing_direction,
            child_writing_direction,
        );
        Self::for_flex_child_with_writing_direction(
            available_inline_size,
            available_block_size,
            percentage_inline,
            percentage_block,
            child_writing_direction,
        )
    }

    /// Whether this space has a fragmentation context (non-zero fragmentainer size).
    #[inline]
    pub fn has_block_fragmentation(&self) -> bool {
        self.fragmentainer_block_size > LayoutUnit::zero()
    }

    /// Convert a logical size pair between parent and child axes.
    ///
    /// Direction and line/block flipping affect offsets, not extents. Sizes
    /// transpose only when one writing mode is horizontal and the other is
    /// vertical/sideways.
    #[inline]
    pub fn convert_logical_size_between(
        inline_size: LayoutUnit,
        block_size: LayoutUnit,
        from: WritingDirectionMode,
        to: WritingDirectionMode,
    ) -> (LayoutUnit, LayoutUnit) {
        if from.is_horizontal() == to.is_horizontal() {
            (inline_size, block_size)
        } else {
            (block_size, inline_size)
        }
    }
}

/// Builder for constructing `ConstraintSpace` values incrementally.
///
/// Mirrors Blink's `ConstraintSpaceBuilder`. Starts from either a parent space
/// or defaults, then sets fields via chained methods.
///
/// Source: `constraint_space_builder.h` (735 lines in Blink).
pub struct ConstraintSpaceBuilder {
    space: ConstraintSpace,
}

impl ConstraintSpaceBuilder {
    /// Create a builder with all defaults (zero sizes, no BFC, no fragmentation).
    pub fn new() -> Self {
        Self {
            space: ConstraintSpace::for_root(LayoutUnit::zero(), LayoutUnit::zero()),
        }
    }

    /// Create a builder from a parent space, inheriting BFC and fragmentation state.
    pub fn from_parent(parent: &ConstraintSpace) -> Self {
        let mut space = parent.clone();
        // Child starts as non-fixed, non-stretch by default
        space.is_fixed_inline_size = false;
        space.is_fixed_block_size = false;
        space.stretch_inline_size = false;
        space.stretch_block_size = false;
        space.is_initial_block_size_indefinite = false;
        space.needs_first_baseline = false;
        space.needs_last_baseline = false;
        Self { space }
    }

    pub fn set_available_size(mut self, inline_size: LayoutUnit, block_size: LayoutUnit) -> Self {
        self.space.available_inline_size = inline_size;
        self.space.available_block_size = block_size;
        self
    }

    /// Set the coordinate system for sizes supplied by subsequent setters.
    /// This does not transpose existing values; use
    /// `set_writing_direction_from_parent` at a parent/child boundary.
    pub fn set_writing_direction(mut self, writing_direction: WritingDirectionMode) -> Self {
        self.space.writing_direction = writing_direction;
        self
    }

    /// Change from the inherited parent axes to child axes, transposing all
    /// inherited size pairs exactly once when the modes are orthogonal.
    pub fn set_writing_direction_from_parent(
        mut self,
        writing_direction: WritingDirectionMode,
    ) -> Self {
        let parent_direction = self.space.writing_direction;
        (
            self.space.available_inline_size,
            self.space.available_block_size,
        ) = ConstraintSpace::convert_logical_size_between(
            self.space.available_inline_size,
            self.space.available_block_size,
            parent_direction,
            writing_direction,
        );
        (
            self.space.percentage_resolution_inline_size,
            self.space.percentage_resolution_block_size,
        ) = ConstraintSpace::convert_logical_size_between(
            self.space.percentage_resolution_inline_size,
            self.space.percentage_resolution_block_size,
            parent_direction,
            writing_direction,
        );
        self.space.writing_direction = writing_direction;
        self
    }

    pub fn set_percentage_resolution_size(
        mut self,
        inline_size: LayoutUnit,
        block_size: LayoutUnit,
    ) -> Self {
        self.space.percentage_resolution_inline_size = inline_size;
        self.space.percentage_resolution_block_size = block_size;
        self
    }

    pub fn set_bfc_offset(mut self, offset: BfcOffset) -> Self {
        self.space.bfc_offset = offset;
        self
    }

    pub fn set_floats_bfc_block_offset(mut self, offset: Option<LayoutUnit>) -> Self {
        self.space.floats_bfc_block_offset = offset;
        self
    }

    pub fn set_exclusion_space(mut self, exclusion_space: Option<Arc<ExclusionSpace>>) -> Self {
        self.space.exclusion_space = exclusion_space;
        self
    }

    pub fn set_is_new_formatting_context(mut self, is_new_fc: bool) -> Self {
        self.space.is_new_formatting_context = is_new_fc;
        self
    }

    pub fn set_is_fixed_inline_size(mut self, v: bool) -> Self {
        self.space.is_fixed_inline_size = v;
        self
    }

    pub fn set_is_fixed_block_size(mut self, v: bool) -> Self {
        self.space.is_fixed_block_size = v;
        self
    }

    pub fn set_stretch_inline_size(mut self, v: bool) -> Self {
        self.space.stretch_inline_size = v;
        self
    }

    pub fn set_stretch_block_size(mut self, v: bool) -> Self {
        self.space.stretch_block_size = v;
        self
    }

    pub fn set_is_initial_block_size_indefinite(mut self, v: bool) -> Self {
        self.space.is_initial_block_size_indefinite = v;
        self
    }

    pub fn set_fragmentainer_block_size(mut self, size: LayoutUnit) -> Self {
        self.space.fragmentainer_block_size = size;
        self
    }

    pub fn set_block_offset_in_fragmentainer(mut self, offset: LayoutUnit) -> Self {
        self.space.block_offset_in_fragmentainer = offset;
        self
    }

    pub fn set_is_resuming(mut self, v: bool) -> Self {
        self.space.is_resuming = v;
        self
    }

    pub fn set_needs_first_baseline(mut self, v: bool) -> Self {
        self.space.needs_first_baseline = v;
        self
    }

    pub fn set_needs_last_baseline(mut self, v: bool) -> Self {
        self.space.needs_last_baseline = v;
        self
    }

    /// Consume the builder and produce the final `ConstraintSpace`.
    pub fn build(self) -> ConstraintSpace {
        self.space
    }
}

impl Default for ConstraintSpaceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lu(value: i32) -> LayoutUnit {
        LayoutUnit::from_i32(value)
    }

    fn vertical_rl_ltr() -> WritingDirectionMode {
        WritingDirectionMode::new(false, true, false, false)
    }

    #[test]
    fn legacy_root_constructor_remains_horizontal_ltr() {
        let space = ConstraintSpace::for_root(lu(800), lu(600));
        assert_eq!(
            space.writing_direction,
            WritingDirectionMode::horizontal_ltr()
        );
        assert_eq!(space.available_inline_size, lu(800));
        assert_eq!(space.available_block_size, lu(600));
        assert_eq!(space.percentage_resolution_inline_size, lu(800));
        assert_eq!(space.percentage_resolution_block_size, lu(600));
    }

    #[test]
    fn vertical_root_converts_physical_viewport_to_logical_size() {
        let space =
            ConstraintSpace::for_root_with_writing_direction(lu(800), lu(600), vertical_rl_ltr());
        assert_eq!(space.writing_direction, vertical_rl_ltr());
        assert_eq!(space.available_inline_size, lu(600));
        assert_eq!(space.available_block_size, lu(800));
        assert_eq!(space.percentage_resolution_inline_size, lu(600));
        assert_eq!(space.percentage_resolution_block_size, lu(800));
    }

    #[test]
    fn orthogonal_child_boundary_transposes_size_pairs_once() {
        let parent = ConstraintSpaceBuilder::new()
            .set_available_size(lu(700), lu(500))
            .set_percentage_resolution_size(lu(600), lu(400))
            .build();
        let child = ConstraintSpaceBuilder::from_parent(&parent)
            .set_writing_direction_from_parent(vertical_rl_ltr())
            .build();

        assert_eq!(child.available_inline_size, lu(500));
        assert_eq!(child.available_block_size, lu(700));
        assert_eq!(child.percentage_resolution_inline_size, lu(400));
        assert_eq!(child.percentage_resolution_block_size, lu(600));
        assert_eq!(child.writing_direction, vertical_rl_ltr());
    }

    #[test]
    fn explicit_block_child_constructor_converts_parent_axes() {
        let parent = ConstraintSpace::for_root(lu(800), lu(600));
        let child = ConstraintSpace::for_block_child_from_parent(
            &parent,
            lu(700),
            lu(500),
            lu(600),
            lu(400),
            false,
            vertical_rl_ltr(),
        );
        assert_eq!(child.available_inline_size, lu(500));
        assert_eq!(child.available_block_size, lu(700));
        assert_eq!(child.percentage_resolution_inline_size, lu(400));
        assert_eq!(child.percentage_resolution_block_size, lu(600));
        assert_eq!(child.writing_direction, vertical_rl_ltr());
    }

    #[test]
    fn parallel_or_direction_only_child_boundary_preserves_extents() {
        let vertical_parent = ConstraintSpaceBuilder::new()
            .set_writing_direction(vertical_rl_ltr())
            .set_available_size(lu(500), lu(700))
            .set_percentage_resolution_size(lu(400), lu(600))
            .build();
        let vertical_rtl = WritingDirectionMode::new(false, true, false, true);
        let child = ConstraintSpaceBuilder::from_parent(&vertical_parent)
            .set_writing_direction_from_parent(vertical_rtl)
            .build();

        assert_eq!(child.available_inline_size, lu(500));
        assert_eq!(child.available_block_size, lu(700));
        assert_eq!(child.percentage_resolution_inline_size, lu(400));
        assert_eq!(child.percentage_resolution_block_size, lu(600));
    }
}
