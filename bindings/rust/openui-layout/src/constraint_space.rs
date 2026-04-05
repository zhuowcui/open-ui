//! ConstraintSpace — the input to a layout algorithm.
//!
//! Extracted from Blink's `ConstraintSpace` (core/layout/constraint_space.h).
//! This carries the available size, percentage resolution size, BFC state,
//! exclusion space, and various flags that the parent layout passes to each child.
//!
//! Extended in SP10 with flex-specific fields and SP12 with BFC, float, and
//! fragmentation fields.

use openui_geometry::{BfcOffset, LayoutUnit};
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
        Self {
            available_inline_size: width,
            available_block_size: height,
            percentage_resolution_inline_size: width,
            percentage_resolution_block_size: height,
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
        Self {
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

    /// Create a constraint space for a flex child with externally determined sizes.
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

    /// Whether this space has a fragmentation context (non-zero fragmentainer size).
    #[inline]
    pub fn has_block_fragmentation(&self) -> bool {
        self.fragmentainer_block_size > LayoutUnit::zero()
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

    pub fn set_available_size(
        mut self,
        inline_size: LayoutUnit,
        block_size: LayoutUnit,
    ) -> Self {
        self.space.available_inline_size = inline_size;
        self.space.available_block_size = block_size;
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
