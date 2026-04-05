//! Lazy BFC block-offset resolution — extracted from Blink's block layout.
//!
//! Source: core/layout/block_layout_algorithm.cc (ResolveBfcBlockOffset,
//!         NeedsAbortAndRelayout, ComputeBfcBlockOffsetEstimate)
//!
//! In Blink's block layout the BFC block offset is not known immediately.
//! It starts as `Unresolved` and gets committed when:
//!   - The element has border or padding on the block-start side
//!   - The first in-flow content is encountered
//!   - A child with `clear` is encountered
//!   - The element establishes a new formatting context
//!
//! When the BFC offset resolves, pending unpositioned floats are positioned.
//! If the resolved offset differs from the estimate the layout must restart
//! (abort-and-relayout).

use openui_geometry::{BfcOffset, BoxStrut, LayoutUnit, MarginStrut};

use crate::exclusions::{ExclusionSpace, ClearType};
use crate::exclusions::float_utils::{UnpositionedFloat, PositionedFloat, position_float};

// ─────────────────────────────────────────────────────────────────────────────
// BfcBlockOffsetState
// ─────────────────────────────────────────────────────────────────────────────

/// Tracks whether the BFC block offset has been resolved.
///
/// Starts as `Unresolved` (optionally with an estimate) and transitions to
/// `Resolved` once the offset is committed. The transition is one-way:
/// resolving an already-resolved state is a no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BfcBlockOffsetState {
    /// BFC block offset not yet known.
    Unresolved {
        /// An estimated offset used for speculative float placement.
        estimated: Option<LayoutUnit>,
    },
    /// BFC block offset has been committed.
    Resolved(LayoutUnit),
}

impl BfcBlockOffsetState {
    /// Create a new unresolved state with no estimate.
    #[inline]
    pub fn new_unresolved() -> Self {
        BfcBlockOffsetState::Unresolved { estimated: None }
    }

    /// Create a new unresolved state with an estimate.
    #[inline]
    pub fn new_with_estimate(estimate: LayoutUnit) -> Self {
        BfcBlockOffsetState::Unresolved {
            estimated: Some(estimate),
        }
    }

    /// Whether the BFC offset is still unresolved.
    #[inline]
    pub fn is_unresolved(&self) -> bool {
        matches!(self, BfcBlockOffsetState::Unresolved { .. })
    }

    /// Whether the BFC offset has been resolved.
    #[inline]
    pub fn is_resolved(&self) -> bool {
        matches!(self, BfcBlockOffsetState::Resolved(_))
    }

    /// Get the resolved offset, if any.
    #[inline]
    pub fn resolved_offset(&self) -> Option<LayoutUnit> {
        match self {
            BfcBlockOffsetState::Resolved(v) => Some(*v),
            _ => None,
        }
    }
}

impl Default for BfcBlockOffsetState {
    fn default() -> Self {
        Self::new_unresolved()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PendingFloats
// ─────────────────────────────────────────────────────────────────────────────

/// A collection of unpositioned floats awaiting BFC offset resolution.
///
/// Floats encountered before the BFC offset is resolved cannot be placed
/// immediately because their block position depends on the resolved offset.
/// They are queued here and bulk-positioned once the offset commits.
#[derive(Debug, Default)]
pub struct PendingFloats {
    floats: Vec<UnpositionedFloat>,
}

impl PendingFloats {
    pub fn new() -> Self {
        Self {
            floats: Vec::new(),
        }
    }

    /// Queue a float for later positioning.
    #[inline]
    pub fn add(&mut self, float: UnpositionedFloat) {
        self.floats.push(float);
    }

    /// Position all pending floats using the resolved BFC offset.
    ///
    /// Each float's `origin_bfc_offset.block_offset` is updated to
    /// `resolved_offset` before querying the exclusion space. Returns the
    /// positioned floats and corresponding exclusion areas. The caller should
    /// add each exclusion area to the exclusion space.
    pub fn position_all(
        &mut self,
        resolved_offset: LayoutUnit,
        exclusion_space: &mut ExclusionSpace,
    ) -> Vec<PositionedFloat> {
        let pending = std::mem::take(&mut self.floats);
        let mut positioned = Vec::with_capacity(pending.len());

        for mut float in pending {
            float.origin_bfc_offset.block_offset = resolved_offset;
            let (pos, exclusion) = position_float(&float, exclusion_space);
            exclusion_space.add(exclusion);
            positioned.push(pos);
        }

        positioned
    }

    /// Whether there are any pending floats.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.floats.is_empty()
    }

    /// Number of pending floats.
    #[inline]
    pub fn len(&self) -> usize {
        self.floats.len()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Core resolution functions
// ─────────────────────────────────────────────────────────────────────────────

/// Commit the BFC block offset, positioning all pending floats.
///
/// Transitions `state` from `Unresolved` to `Resolved(offset)`. If already
/// resolved this is a no-op and returns `false`.
///
/// Returns `true` if the resolved offset differs from the estimate, which
/// signals that the layout must restart (abort-and-relayout).
///
/// Source: `BlockLayoutAlgorithm::ResolveBfcBlockOffset` in Blink.
pub fn resolve_bfc_block_offset(
    state: &mut BfcBlockOffsetState,
    offset: LayoutUnit,
    pending: &mut PendingFloats,
    exclusion_space: &mut ExclusionSpace,
) -> bool {
    match *state {
        BfcBlockOffsetState::Resolved(_) => {
            // Already resolved — nothing to do.
            false
        }
        BfcBlockOffsetState::Unresolved { estimated } => {
            *state = BfcBlockOffsetState::Resolved(offset);

            // Position all pending floats at the resolved offset.
            let _positioned = pending.position_all(offset, exclusion_space);

            // If the resolved offset differs from the estimate we must relayout.
            match estimated {
                Some(est) => est != offset,
                None => false,
            }
        }
    }
}

/// Check whether the BFC block offset should be resolved at this point.
///
/// The offset resolves when any of the following conditions hold:
/// - The element has block-start border or padding (`border_padding.top > 0`)
/// - The element has in-flow content (`has_content`)
/// - A child has `clear` other than `none`
/// - The element establishes a new formatting context
///
/// Source: Logic scattered across `BlockLayoutAlgorithm::Layout` and
///         `BlockLayoutAlgorithm::HandleInflow` in Blink.
pub fn should_resolve_bfc_offset(
    border_padding: &BoxStrut,
    has_content: bool,
    child_clear: ClearType,
    establishes_new_fc: bool,
) -> bool {
    // Block-start border or padding separates the margin from the content.
    if border_padding.top > LayoutUnit::zero() {
        return true;
    }

    // First in-flow content anchors the BFC offset.
    if has_content {
        return true;
    }

    // `clear` on a child forces resolution so clearance can be computed.
    if !matches!(child_clear, ClearType::None) {
        return true;
    }

    // Establishing a new formatting context prevents margin collapsing
    // through the boundary, so the offset must be committed.
    if establishes_new_fc {
        return true;
    }

    false
}

// ─────────────────────────────────────────────────────────────────────────────
// Integration helpers (for use by block.rs in a later phase)
// ─────────────────────────────────────────────────────────────────────────────

/// Estimate the BFC block offset from the current position and margin strut.
///
/// The estimate is `container_bfc_block_offset + content_offset + margin_strut.sum()`.
/// This is used for speculative float placement before the real offset is known.
///
/// Source: Derived from `BlockLayoutAlgorithm::ComputeChildData` in Blink
///         where `bfc_offset_estimate` is computed.
pub fn compute_bfc_block_offset_estimate(
    container_bfc_block_offset: LayoutUnit,
    content_offset: LayoutUnit,
    margin_strut: &MarginStrut,
) -> LayoutUnit {
    container_bfc_block_offset + content_offset + margin_strut.sum()
}

/// Check whether the resolved BFC offset differs from the estimate,
/// indicating that the block layout must be restarted.
///
/// Source: `BlockLayoutAlgorithm::NeedsAbortAndRelayout` check in Blink.
pub fn needs_relayout(state: &BfcBlockOffsetState, estimate: LayoutUnit) -> bool {
    match state {
        BfcBlockOffsetState::Resolved(resolved) => *resolved != estimate,
        BfcBlockOffsetState::Unresolved { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lu(v: i32) -> LayoutUnit {
        LayoutUnit::from_i32(v)
    }

    #[test]
    fn state_starts_unresolved() {
        let state = BfcBlockOffsetState::default();
        assert!(state.is_unresolved());
        assert!(!state.is_resolved());
        assert_eq!(state.resolved_offset(), None);
    }

    #[test]
    fn state_with_estimate() {
        let state = BfcBlockOffsetState::new_with_estimate(lu(42));
        assert!(state.is_unresolved());
        match state {
            BfcBlockOffsetState::Unresolved { estimated } => {
                assert_eq!(estimated, Some(lu(42)));
            }
            _ => panic!("expected Unresolved"),
        }
    }

    #[test]
    fn resolve_transitions_state() {
        let mut state = BfcBlockOffsetState::new_unresolved();
        let mut pending = PendingFloats::new();
        let mut space = ExclusionSpace::new();

        let changed = resolve_bfc_block_offset(&mut state, lu(100), &mut pending, &mut space);
        assert!(!changed);
        assert!(state.is_resolved());
        assert_eq!(state.resolved_offset(), Some(lu(100)));
    }

    #[test]
    fn resolve_matching_estimate_no_relayout() {
        let mut state = BfcBlockOffsetState::new_with_estimate(lu(50));
        let mut pending = PendingFloats::new();
        let mut space = ExclusionSpace::new();

        let changed = resolve_bfc_block_offset(&mut state, lu(50), &mut pending, &mut space);
        assert!(!changed);
    }

    #[test]
    fn resolve_different_estimate_triggers_relayout() {
        let mut state = BfcBlockOffsetState::new_with_estimate(lu(50));
        let mut pending = PendingFloats::new();
        let mut space = ExclusionSpace::new();

        let changed = resolve_bfc_block_offset(&mut state, lu(80), &mut pending, &mut space);
        assert!(changed);
        assert_eq!(state.resolved_offset(), Some(lu(80)));
    }

    #[test]
    fn double_resolve_is_noop() {
        let mut state = BfcBlockOffsetState::new_with_estimate(lu(10));
        let mut pending = PendingFloats::new();
        let mut space = ExclusionSpace::new();

        let changed1 = resolve_bfc_block_offset(&mut state, lu(20), &mut pending, &mut space);
        assert!(changed1);

        // Second resolve should be a no-op.
        let changed2 = resolve_bfc_block_offset(&mut state, lu(999), &mut pending, &mut space);
        assert!(!changed2);
        assert_eq!(state.resolved_offset(), Some(lu(20)));
    }

    #[test]
    fn pending_floats_empty_by_default() {
        let pending = PendingFloats::new();
        assert!(pending.is_empty());
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn pending_floats_add_and_count() {
        let mut pending = PendingFloats::new();
        pending.add(make_test_float(lu(0), true));
        pending.add(make_test_float(lu(0), false));
        assert!(!pending.is_empty());
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn pending_floats_positioned_on_resolve() {
        let mut state = BfcBlockOffsetState::new_unresolved();
        let mut pending = PendingFloats::new();
        let mut space = ExclusionSpace::new();

        pending.add(make_test_float(lu(0), true));
        assert_eq!(pending.len(), 1);

        resolve_bfc_block_offset(&mut state, lu(50), &mut pending, &mut space);

        // Pending should be drained.
        assert!(pending.is_empty());
        // The exclusion space should now have the float.
        assert!(space.has_floats());
    }

    #[test]
    fn multiple_pending_floats_all_positioned() {
        let mut state = BfcBlockOffsetState::new_unresolved();
        let mut pending = PendingFloats::new();
        let mut space = ExclusionSpace::new();

        pending.add(make_test_float(lu(0), true));
        pending.add(make_test_float(lu(0), false));
        pending.add(make_test_float(lu(0), true));
        assert_eq!(pending.len(), 3);

        resolve_bfc_block_offset(&mut state, lu(10), &mut pending, &mut space);
        assert!(pending.is_empty());
        assert_eq!(space.num_exclusions(), 3);
    }

    #[test]
    fn should_resolve_border_padding() {
        let bp = BoxStrut::new(lu(1), lu(0), lu(0), lu(0));
        assert!(should_resolve_bfc_offset(&bp, false, ClearType::None, false));
    }

    #[test]
    fn should_resolve_content() {
        let bp = BoxStrut::zero();
        assert!(should_resolve_bfc_offset(&bp, true, ClearType::None, false));
    }

    #[test]
    fn should_resolve_clear() {
        let bp = BoxStrut::zero();
        assert!(should_resolve_bfc_offset(&bp, false, ClearType::Left, false));
        assert!(should_resolve_bfc_offset(&bp, false, ClearType::Right, false));
        assert!(should_resolve_bfc_offset(&bp, false, ClearType::Both, false));
    }

    #[test]
    fn should_resolve_new_fc() {
        let bp = BoxStrut::zero();
        assert!(should_resolve_bfc_offset(&bp, false, ClearType::None, true));
    }

    #[test]
    fn should_not_resolve_when_none_apply() {
        let bp = BoxStrut::zero();
        assert!(!should_resolve_bfc_offset(&bp, false, ClearType::None, false));
    }

    #[test]
    fn estimate_computation() {
        let est = compute_bfc_block_offset_estimate(lu(100), lu(20), &MarginStrut::new());
        assert_eq!(est, lu(120));

        let mut strut = MarginStrut::new();
        strut.append_normal(lu(15));
        let est2 = compute_bfc_block_offset_estimate(lu(100), lu(20), &strut);
        assert_eq!(est2, lu(135));
    }

    #[test]
    fn needs_relayout_when_mismatch() {
        let state = BfcBlockOffsetState::Resolved(lu(50));
        assert!(!needs_relayout(&state, lu(50)));
        assert!(needs_relayout(&state, lu(60)));
    }

    #[test]
    fn needs_relayout_false_when_unresolved() {
        let state = BfcBlockOffsetState::new_unresolved();
        assert!(!needs_relayout(&state, lu(100)));
    }

    // ── Test helpers ─────────────────────────────────────────────────────

    fn make_test_float(origin_block_offset: LayoutUnit, is_left: bool) -> UnpositionedFloat {
        use openui_dom::NodeId;
        UnpositionedFloat {
            node_id: NodeId::NONE,
            available_size: lu(500),
            origin_bfc_offset: BfcOffset::new(lu(0), origin_block_offset),
            margins: BoxStrut::zero(),
            inline_size: lu(100),
            block_size: lu(50),
            is_left,
        }
    }
}
