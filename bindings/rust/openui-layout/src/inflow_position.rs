//! Inflow position tracking structs for block layout — extracted from Blink.
//!
//! Source: core/layout/block_layout_algorithm.h (lines 33-60)
//!
//! These structs track state as the block layout algorithm walks through
//! its children sequentially. `PreviousInflowPosition` is passed from one
//! child to the next; `InflowChildData` holds per-child computed data.

use openui_geometry::{BfcOffset, BoxStrut, LayoutUnit, MarginStrut};

/// Communicates the position of the previous inflow child to subsequent children.
///
/// After each child is positioned, a new `PreviousInflowPosition` is computed
/// and passed to the next child's layout. This is how margin collapsing
/// propagates sequentially through siblings.
///
/// Source: `PreviousInflowPosition` in `block_layout_algorithm.h`.
#[derive(Debug, Clone)]
pub struct PreviousInflowPosition {
    /// The block offset after the previous child (including collapsed margins).
    pub logical_block_offset: LayoutUnit,

    /// The margin strut carried forward from the previous child.
    /// Contains the accumulated margins awaiting collapse with the next sibling.
    pub margin_strut: MarginStrut,

    /// Block-end annotation space of the previous line.
    /// Positive means space is reserved; negative means overflow.
    pub block_end_annotation_space: LayoutUnit,

    /// Whether the previous inflow child was self-collapsing and had clearance.
    /// This affects whether subsequent margins can collapse through.
    pub self_collapsing_child_had_clearance: bool,
}

impl PreviousInflowPosition {
    /// Initial position at the start of a block container's child walk.
    pub fn new() -> Self {
        Self {
            logical_block_offset: LayoutUnit::zero(),
            margin_strut: MarginStrut::new(),
            block_end_annotation_space: LayoutUnit::zero(),
            self_collapsing_child_had_clearance: false,
        }
    }
}

impl Default for PreviousInflowPosition {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-child computed data for an inflow block child.
///
/// Created by `ComputeChildData()` before laying out each child, containing
/// the estimated BFC offset, margin strut at this position, and resolved
/// margins.
///
/// Source: `InflowChildData` in `block_layout_algorithm.h`.
#[derive(Debug, Clone)]
pub struct InflowChildData {
    /// Estimated BFC offset for this child (may change if BFC resolves later).
    pub bfc_offset_estimate: BfcOffset,

    /// The margin strut at this child's position (accumulated from parent
    /// start and preceding siblings).
    pub margin_strut: MarginStrut,

    /// The child's resolved margins (all four sides).
    pub margins: BoxStrut,

    /// Whether this child's block offset was increased by float exclusions.
    pub is_pushed_by_floats: bool,
}

impl InflowChildData {
    pub fn new(
        bfc_offset_estimate: BfcOffset,
        margin_strut: MarginStrut,
        margins: BoxStrut,
    ) -> Self {
        Self {
            bfc_offset_estimate,
            margin_strut,
            margins,
            is_pushed_by_floats: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previous_inflow_position_defaults() {
        let pos = PreviousInflowPosition::new();
        assert_eq!(pos.logical_block_offset, LayoutUnit::zero());
        assert!(pos.margin_strut.is_empty());
        assert!(!pos.self_collapsing_child_had_clearance);
    }

    #[test]
    fn inflow_child_data_construction() {
        let data = InflowChildData::new(
            BfcOffset::new(LayoutUnit::from_i32(10), LayoutUnit::from_i32(20)),
            MarginStrut::new(),
            BoxStrut::zero(),
        );
        assert_eq!(data.bfc_offset_estimate.line_offset, LayoutUnit::from_i32(10));
        assert_eq!(data.bfc_offset_estimate.block_offset, LayoutUnit::from_i32(20));
        assert!(!data.is_pushed_by_floats);
    }
}
