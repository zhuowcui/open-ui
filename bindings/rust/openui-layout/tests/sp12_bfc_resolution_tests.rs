//! SP12 C1 — BFC offset resolution integration tests.

use openui_geometry::{BfcOffset, BoxStrut, LayoutUnit, MarginStrut};
use openui_layout::bfc_resolution::{
    resolve_bfc_block_offset, should_resolve_bfc_offset,
    compute_bfc_block_offset_estimate, needs_relayout,
    BfcBlockOffsetState, PendingFloats,
};
use openui_layout::exclusions::{ClearType, ExclusionSpace};
use openui_layout::exclusions::float_utils::UnpositionedFloat;
use openui_dom::NodeId;

fn lu(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

fn make_float(origin_block: LayoutUnit, is_left: bool) -> UnpositionedFloat {
    UnpositionedFloat {
        node_id: NodeId::NONE,
        available_size: lu(500),
        origin_bfc_offset: BfcOffset::new(lu(0), origin_block),
        margins: BoxStrut::zero(),
        inline_size: lu(100),
        block_size: lu(50),
        is_left,
    }
}

// ── State lifecycle tests ────────────────────────────────────────────────

#[test]
fn bfc_starts_unresolved() {
    let state = BfcBlockOffsetState::default();
    assert!(state.is_unresolved());
    assert!(!state.is_resolved());
    assert_eq!(state.resolved_offset(), None);
}

#[test]
fn resolve_transitions_to_resolved() {
    let mut state = BfcBlockOffsetState::new_unresolved();
    let mut pending = PendingFloats::new();
    let mut space = ExclusionSpace::new();

    resolve_bfc_block_offset(&mut state, lu(100), &mut pending, &mut space);
    assert!(state.is_resolved());
    assert_eq!(state.resolved_offset(), Some(lu(100)));
}

#[test]
fn resolve_with_matching_estimate_no_relayout() {
    let mut state = BfcBlockOffsetState::new_with_estimate(lu(75));
    let mut pending = PendingFloats::new();
    let mut space = ExclusionSpace::new();

    let changed = resolve_bfc_block_offset(&mut state, lu(75), &mut pending, &mut space);
    assert!(!changed, "matching estimate should not trigger relayout");
}

#[test]
fn resolve_with_different_estimate_needs_relayout() {
    let mut state = BfcBlockOffsetState::new_with_estimate(lu(30));
    let mut pending = PendingFloats::new();
    let mut space = ExclusionSpace::new();

    let changed = resolve_bfc_block_offset(&mut state, lu(60), &mut pending, &mut space);
    assert!(changed, "mismatched estimate must trigger relayout");
}

#[test]
fn pending_floats_positioned_on_resolve() {
    let mut state = BfcBlockOffsetState::new_unresolved();
    let mut pending = PendingFloats::new();
    let mut space = ExclusionSpace::new();

    pending.add(make_float(lu(0), true));
    assert_eq!(pending.len(), 1);

    resolve_bfc_block_offset(&mut state, lu(40), &mut pending, &mut space);
    assert!(pending.is_empty());
    assert!(space.has_floats());
}

#[test]
fn multiple_pending_floats_all_positioned() {
    let mut state = BfcBlockOffsetState::new_unresolved();
    let mut pending = PendingFloats::new();
    let mut space = ExclusionSpace::new();

    pending.add(make_float(lu(0), true));
    pending.add(make_float(lu(0), false));
    pending.add(make_float(lu(0), true));

    resolve_bfc_block_offset(&mut state, lu(10), &mut pending, &mut space);
    assert!(pending.is_empty());
    assert_eq!(space.num_exclusions(), 3);
}

#[test]
fn border_padding_triggers_resolution() {
    let bp = BoxStrut::new(lu(5), lu(0), lu(0), lu(0));
    assert!(should_resolve_bfc_offset(&bp, false, ClearType::None, false));
}

#[test]
fn content_triggers_resolution() {
    assert!(should_resolve_bfc_offset(
        &BoxStrut::zero(),
        true,
        ClearType::None,
        false
    ));
}

#[test]
fn clear_triggers_resolution() {
    let bp = BoxStrut::zero();
    assert!(should_resolve_bfc_offset(&bp, false, ClearType::Left, false));
    assert!(should_resolve_bfc_offset(&bp, false, ClearType::Right, false));
    assert!(should_resolve_bfc_offset(&bp, false, ClearType::Both, false));
}

#[test]
fn empty_pending_floats() {
    let pending = PendingFloats::new();
    assert!(pending.is_empty());
    assert_eq!(pending.len(), 0);
}

#[test]
fn double_resolve_is_noop() {
    let mut state = BfcBlockOffsetState::new_with_estimate(lu(10));
    let mut pending = PendingFloats::new();
    let mut space = ExclusionSpace::new();

    let changed1 = resolve_bfc_block_offset(&mut state, lu(20), &mut pending, &mut space);
    assert!(changed1);

    let changed2 = resolve_bfc_block_offset(&mut state, lu(999), &mut pending, &mut space);
    assert!(!changed2, "second resolve must be a no-op");
    assert_eq!(state.resolved_offset(), Some(lu(20)));
}

#[test]
fn new_fc_triggers_resolution() {
    assert!(should_resolve_bfc_offset(
        &BoxStrut::zero(),
        false,
        ClearType::None,
        true
    ));
}

#[test]
fn no_conditions_does_not_trigger_resolution() {
    assert!(!should_resolve_bfc_offset(
        &BoxStrut::zero(),
        false,
        ClearType::None,
        false
    ));
}

#[test]
fn estimate_with_margin_strut() {
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(20));
    let est = compute_bfc_block_offset_estimate(lu(100), lu(10), &strut);
    assert_eq!(est, lu(130)); // 100 + 10 + 20
}

#[test]
fn needs_relayout_detects_mismatch() {
    let state = BfcBlockOffsetState::Resolved(lu(50));
    assert!(!needs_relayout(&state, lu(50)));
    assert!(needs_relayout(&state, lu(51)));
}

#[test]
fn needs_relayout_false_when_unresolved() {
    let state = BfcBlockOffsetState::new_unresolved();
    assert!(!needs_relayout(&state, lu(100)));
}
