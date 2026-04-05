//! LayoutResult — output of a layout algorithm with status and metadata.
//!
//! Source: core/layout/layout_result.h (~1,111 lines in Blink)
//!
//! A `LayoutResult` wraps a `Fragment` with additional metadata needed by
//! the parent layout algorithm: BFC offset, end margin strut, self-collapsing
//! flag, and a status code indicating whether layout succeeded or needs a
//! retry with updated information.
//!
//! Blink's `LayoutResult` is significantly larger (caching, paint, accessibility),
//! but we extract the block-layout-relevant subset.

use openui_geometry::{LayoutUnit, MarginStrut};

use crate::fragment::Fragment;

/// Status of a layout operation.
///
/// Most layouts succeed, but block layout in particular can produce non-success
/// outcomes that require the parent to retry with updated state.
///
/// Source: `LayoutResult::EStatus` in `layout_result.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutStatus {
    /// Layout completed successfully.
    Success,

    /// The child resolved its BFC block offset (e.g., due to content or
    /// clearance). The parent should abort its current pass and relayout
    /// with the now-known BFC offset.
    BfcBlockOffsetResolved,

    /// An earlier break point was found that produces better fragmentation.
    /// The parent should abort and restart with the earlier break.
    NeedsEarlierBreak,

    /// The fragmentainer ran out of space. The parent should break before
    /// this child and continue in the next fragmentainer.
    OutOfFragmentainerSpace,

    /// Line clamping requires a relayout pass.
    NeedsLineClampRelayout,

    /// Fragmentation should be disabled for this subtree (e.g., monolithic
    /// content that cannot break).
    DisableFragmentation,
}

/// The result of running a layout algorithm on a node.
///
/// Contains the positioned fragment, BFC coordinates, margin strut state,
/// and a status indicating whether the layout succeeded or needs retry.
///
/// Source: `LayoutResult` in `layout_result.h`.
#[derive(Debug)]
pub struct LayoutResult {
    /// The positioned fragment tree produced by layout.
    pub fragment: Fragment,

    /// Layout status — `Success` for normal completion.
    pub status: LayoutStatus,

    // ── BFC state ────────────────────────────────────────────────────

    /// Line offset of this fragment within its BFC.
    pub bfc_line_offset: LayoutUnit,

    /// Block offset of this fragment within its BFC.
    /// `None` if the BFC block offset has not been resolved yet
    /// (the element's position depends on margin collapsing or float
    /// interaction that hasn't been finalized).
    pub bfc_block_offset: Option<LayoutUnit>,

    // ── Margin collapsing ────────────────────────────────────────────

    /// The margin strut at the end of this fragment. The parent uses this
    /// to continue margin collapsing with subsequent siblings.
    pub end_margin_strut: MarginStrut,

    // ── Flags ────────────────────────────────────────────────────────

    /// True if this fragment is self-collapsing (zero block-size, no
    /// border, no padding, no content that prevents margin collapse-through).
    /// When true, the top and bottom margins of this element collapse together.
    pub is_self_collapsing: bool,

    /// True if this fragment's block offset was increased by float exclusions.
    pub is_pushed_by_floats: bool,

    /// Types of adjoining objects preceding this fragment.
    /// Used for margin collapsing decisions.
    pub adjoining_object_types: AdjoiningObjectTypes,

    /// True if a descendant in the subtree modified the margin strut.
    pub subtree_modified_margin_strut: bool,

    // ── Fragmentation ────────────────────────────────────────────────

    /// The `break-before` value of the first child, propagated up for the
    /// parent's fragmentation decision.
    pub initial_break_before: BreakBetween,

    /// The `break-after` value of the last child, propagated up.
    pub final_break_after: BreakBetween,

    /// Block size to use in the fragmentation context (may differ from
    /// fragment size due to overflow).
    pub block_size_for_fragmentation: Option<LayoutUnit>,

    /// Block-end annotation space of the last line.
    pub block_end_annotation_space: LayoutUnit,

    // ── Baselines ────────────────────────────────────────────────────

    /// First baseline of this fragment (for alignment by parent).
    pub first_baseline: Option<LayoutUnit>,

    /// Last baseline of this fragment.
    pub last_baseline: Option<LayoutUnit>,
}

/// Types of objects that can adjoin in margin collapsing.
///
/// Source: `AdjoiningObjectTypes` in Blink (3-bit bitfield).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AdjoiningObjectTypes(u8);

impl AdjoiningObjectTypes {
    pub const NONE: Self = Self(0);
    pub const FLOATING: Self = Self(1);
    pub const INLINE: Self = Self(2);
    pub const BLOCK_START: Self = Self(4);

    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[inline]
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Break between values for fragmentation.
///
/// Source: `EBreakBetween` in Blink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BreakBetween {
    #[default]
    Auto,
    Avoid,
    AvoidPage,
    AvoidColumn,
    Page,
    Left,
    Right,
    Recto,
    Verso,
    Column,
}

impl LayoutResult {
    /// Create a successful layout result from a fragment.
    pub fn success(fragment: Fragment) -> Self {
        Self {
            fragment,
            status: LayoutStatus::Success,
            bfc_line_offset: LayoutUnit::zero(),
            bfc_block_offset: None,
            end_margin_strut: MarginStrut::new(),
            is_self_collapsing: false,
            is_pushed_by_floats: false,
            adjoining_object_types: AdjoiningObjectTypes::NONE,
            subtree_modified_margin_strut: false,
            initial_break_before: BreakBetween::Auto,
            final_break_after: BreakBetween::Auto,
            block_size_for_fragmentation: None,
            block_end_annotation_space: LayoutUnit::zero(),
            first_baseline: None,
            last_baseline: None,
        }
    }

    /// Create a non-success result indicating BFC block offset was resolved.
    pub fn bfc_offset_resolved(fragment: Fragment, resolved_offset: LayoutUnit) -> Self {
        Self {
            status: LayoutStatus::BfcBlockOffsetResolved,
            bfc_block_offset: Some(resolved_offset),
            ..Self::success(fragment)
        }
    }

    /// Whether this result indicates a successful layout.
    #[inline]
    pub fn is_success(&self) -> bool {
        self.status == LayoutStatus::Success
    }

    /// Access the fragment.
    #[inline]
    pub fn fragment(&self) -> &Fragment {
        &self.fragment
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_geometry::PhysicalSize;
    use openui_dom::NodeId;

    fn dummy_fragment() -> Fragment {
        Fragment::new_box(NodeId::NONE, PhysicalSize::zero())
    }

    #[test]
    fn success_result_defaults() {
        let result = LayoutResult::success(dummy_fragment());
        assert!(result.is_success());
        assert_eq!(result.status, LayoutStatus::Success);
        assert!(!result.is_self_collapsing);
        assert!(!result.is_pushed_by_floats);
        assert!(result.end_margin_strut.is_empty());
        assert_eq!(result.bfc_block_offset, None);
    }

    #[test]
    fn bfc_offset_resolved_status() {
        let result = LayoutResult::bfc_offset_resolved(
            dummy_fragment(),
            LayoutUnit::from_i32(50),
        );
        assert!(!result.is_success());
        assert_eq!(result.status, LayoutStatus::BfcBlockOffsetResolved);
        assert_eq!(result.bfc_block_offset, Some(LayoutUnit::from_i32(50)));
    }

    #[test]
    fn adjoining_object_types() {
        let mut t = AdjoiningObjectTypes::NONE;
        assert!(t.is_empty());
        t = t.union(AdjoiningObjectTypes::FLOATING);
        assert!(!t.is_empty());
        assert!(t.contains(AdjoiningObjectTypes::FLOATING));
        assert!(!t.contains(AdjoiningObjectTypes::INLINE));
    }
}
