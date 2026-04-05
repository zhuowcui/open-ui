//! SP12 C2 — Full margin collapsing tests (CSS 2.1 §8.3.1).

use openui_geometry::{BoxStrut, LayoutUnit, MarginStrut};
use openui_layout::margin_collapsing::{
    adjoining_margin_resolve, clearance_prevents_collapsing, collapse_margins,
    establishes_new_bfc_for_collapsing, finalize_margins, float_prevents_collapsing,
    handle_margin_after_child, handle_margin_before_child, merge_struts,
    should_margins_collapse_through, ChildMarginInfo, CollapseCheckParams,
    MarginCollapsingState, ParentMarginInfo,
};
use openui_style::{Clear, Display, Float, Overflow, Position};

fn lu(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

fn parent_no_separator(first: bool, last: bool) -> ParentMarginInfo {
    ParentMarginInfo {
        border: BoxStrut::zero(),
        padding: BoxStrut::zero(),
        is_first_child: first,
        is_last_child: last,
        block_size: None,
    }
}

fn normal_child(top: i32, bottom: i32) -> ChildMarginInfo {
    ChildMarginInfo {
        margin_top: lu(top),
        margin_bottom: lu(bottom),
        establishes_bfc: false,
        is_float: false,
        has_clearance: false,
        collapsed_through: false,
        child_margin_strut: MarginStrut::new(),
    }
}

// ── 1. Adjacent sibling positive margins (larger wins) ──────────────────

#[test]
fn adjacent_sibling_positive_margins_larger_wins() {
    // Two siblings: A has margin-bottom 20, B has margin-top 30.
    // Collapsed = max(20, 30) = 30.
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    // After child A: bottom margin 20
    let child_a = normal_child(10, 20);
    handle_margin_after_child(&mut state, &child_a);

    // Before child B: top margin 30
    let child_b = normal_child(30, 15);
    let parent = parent_no_separator(false, false);
    let resolved = handle_margin_before_child(&mut state, &child_b, &parent);

    assert_eq!(resolved, lu(30));
}

// ── 2. Adjacent sibling one negative (sum) ──────────────────────────────

#[test]
fn adjacent_sibling_one_negative() {
    // A: margin-bottom 20, B: margin-top -10.
    // Collapsed = 20 + (-10) = 10.
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    let child_a = normal_child(0, 20);
    handle_margin_after_child(&mut state, &child_a);

    let child_b = normal_child(-10, 0);
    let parent = parent_no_separator(false, false);
    let resolved = handle_margin_before_child(&mut state, &child_b, &parent);

    assert_eq!(resolved, lu(10));
}

// ── 3. Adjacent sibling both negative (most negative wins) ──────────────

#[test]
fn adjacent_sibling_both_negative() {
    // A: margin-bottom -5, B: margin-top -15.
    // Collapsed = min(-5, -15) = -15.
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    let child_a = normal_child(0, -5);
    handle_margin_after_child(&mut state, &child_a);

    let child_b = normal_child(-15, 0);
    let parent = parent_no_separator(false, false);
    let resolved = handle_margin_before_child(&mut state, &child_b, &parent);

    assert_eq!(resolved, lu(-15));
}

// ── 4. Parent-first-child collapsing ────────────────────────────────────

#[test]
fn parent_first_child_collapsing() {
    // Parent has margin-top 10 in strut, first child has margin-top 25.
    // No border/padding => they collapse. Result should be max(10, 25) = 25.
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(10));

    let child = normal_child(25, 0);
    let parent = parent_no_separator(true, false);
    let resolved = handle_margin_before_child(&mut state, &child, &parent);

    // Margins collapse upward — no spacing produced here.
    assert_eq!(resolved, lu(0));
    // The strut now holds max(10, 25) = 25.
    assert_eq!(collapse_margins(&state.margin_strut), lu(25));
}

// ── 5. Parent-first-child blocked by border ─────────────────────────────

#[test]
fn parent_first_child_blocked_by_border() {
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(10));

    let child = normal_child(25, 0);
    let parent = ParentMarginInfo {
        border: BoxStrut::new(lu(1), lu(0), lu(0), lu(0)),
        padding: BoxStrut::zero(),
        is_first_child: true,
        is_last_child: false,
        block_size: None,
    };
    let resolved = handle_margin_before_child(&mut state, &child, &parent);

    // Border separates them: parent strut (10) resolves, child starts new.
    assert_eq!(resolved, lu(10));
    assert_eq!(collapse_margins(&state.margin_strut), lu(25));
}

// ── 6. Parent-first-child blocked by padding ────────────────────────────

#[test]
fn parent_first_child_blocked_by_padding() {
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(15));

    let child = normal_child(20, 0);
    let parent = ParentMarginInfo {
        border: BoxStrut::zero(),
        padding: BoxStrut::new(lu(5), lu(0), lu(0), lu(0)),
        is_first_child: true,
        is_last_child: false,
        block_size: None,
    };
    let resolved = handle_margin_before_child(&mut state, &child, &parent);

    // Padding separates them.
    assert_eq!(resolved, lu(15));
    assert_eq!(collapse_margins(&state.margin_strut), lu(20));
}

// ── 7. Parent-last-child collapsing ─────────────────────────────────────

#[test]
fn parent_last_child_collapsing() {
    // Last child's bottom margin (in strut) collapses with parent's bottom
    // margin when no border/padding/height separates them.
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;
    state.margin_strut.append_normal(lu(20)); // last child's bottom margin

    let parent = parent_no_separator(false, true);
    let (resolved_bottom, propagated) =
        finalize_margins(&mut state, &parent, lu(15), false);

    // Margins collapse: nothing resolved at content edge.
    assert_eq!(resolved_bottom, lu(0));
    // Propagated strut = max(20, 15) = 20.
    assert_eq!(collapse_margins(&propagated), lu(20));
}

// ── 8. Parent-last-child blocked by height ──────────────────────────────

#[test]
fn parent_last_child_blocked_by_height() {
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;
    state.margin_strut.append_normal(lu(20));

    let parent = ParentMarginInfo {
        border: BoxStrut::zero(),
        padding: BoxStrut::zero(),
        is_first_child: false,
        is_last_child: true,
        block_size: Some(lu(100)),
    };
    let (resolved_bottom, propagated) =
        finalize_margins(&mut state, &parent, lu(15), false);

    // Height separates: strut resolved at content bottom edge.
    assert_eq!(resolved_bottom, lu(20));
    // Parent's own bottom margin propagates separately.
    assert_eq!(collapse_margins(&propagated), lu(15));
}

// ── 9. Empty block collapsing through ───────────────────────────────────

#[test]
fn empty_block_collapses_through() {
    let params = CollapseCheckParams {
        border: BoxStrut::zero(),
        padding: BoxStrut::zero(),
        block_size: None,
        min_block_size: lu(0),
        in_flow_child_count: 0,
        has_line_boxes: false,
    };
    assert!(should_margins_collapse_through(&params));
}

// ── 10. Empty block with border (doesn't collapse through) ──────────────

#[test]
fn empty_block_with_border_no_collapse_through() {
    let params = CollapseCheckParams {
        border: BoxStrut::new(lu(1), lu(0), lu(1), lu(0)),
        padding: BoxStrut::zero(),
        block_size: None,
        min_block_size: lu(0),
        in_flow_child_count: 0,
        has_line_boxes: false,
    };
    assert!(!should_margins_collapse_through(&params));
}

// ── 11. Empty block with min-height (doesn't collapse through) ──────────

#[test]
fn empty_block_with_min_height_no_collapse_through() {
    let params = CollapseCheckParams {
        border: BoxStrut::zero(),
        padding: BoxStrut::zero(),
        block_size: None,
        min_block_size: lu(1),
        in_flow_child_count: 0,
        has_line_boxes: false,
    };
    assert!(!should_margins_collapse_through(&params));
}

// ── 12. New BFC prevents collapsing ─────────────────────────────────────

#[test]
fn new_bfc_prevents_collapsing() {
    // Child that establishes BFC should not collapse with parent.
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(10));

    let child = ChildMarginInfo {
        margin_top: lu(20),
        margin_bottom: lu(5),
        establishes_bfc: true,
        is_float: false,
        has_clearance: false,
        collapsed_through: false,
        child_margin_strut: MarginStrut::new(),
    };
    let parent = parent_no_separator(true, false);
    let resolved = handle_margin_before_child(&mut state, &child, &parent);

    // Parent strut (10) resolved, child starts fresh.
    assert_eq!(resolved, lu(10));
    assert!(state.bfc_offset_resolved);
}

// ── 13. Float prevents collapsing ───────────────────────────────────────

#[test]
fn float_prevents_margin_collapsing() {
    assert!(float_prevents_collapsing(Float::Left));
    assert!(float_prevents_collapsing(Float::Right));
    assert!(!float_prevents_collapsing(Float::None));
}

#[test]
fn float_child_does_not_affect_strut() {
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(10));

    let child = ChildMarginInfo {
        margin_top: lu(50),
        margin_bottom: lu(50),
        establishes_bfc: false,
        is_float: true,
        has_clearance: false,
        collapsed_through: false,
        child_margin_strut: MarginStrut::new(),
    };

    let parent = parent_no_separator(true, false);
    let resolved = handle_margin_before_child(&mut state, &child, &parent);

    // Float returns 0 and doesn't touch strut.
    assert_eq!(resolved, lu(0));
    assert_eq!(collapse_margins(&state.margin_strut), lu(10));

    handle_margin_after_child(&mut state, &child);
    assert_eq!(collapse_margins(&state.margin_strut), lu(10));
}

// ── 14. Clearance prevents collapsing ───────────────────────────────────

#[test]
fn clearance_prevents_margin_collapsing() {
    assert!(clearance_prevents_collapsing(Clear::Left));
    assert!(clearance_prevents_collapsing(Clear::Right));
    assert!(clearance_prevents_collapsing(Clear::Both));
    assert!(!clearance_prevents_collapsing(Clear::None));
}

#[test]
fn child_with_clearance_severs_chain() {
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(10));

    let child = ChildMarginInfo {
        margin_top: lu(20),
        margin_bottom: lu(0),
        establishes_bfc: false,
        is_float: false,
        has_clearance: true,
        collapsed_through: false,
        child_margin_strut: MarginStrut::new(),
    };

    let parent = parent_no_separator(true, false);
    let resolved = handle_margin_before_child(&mut state, &child, &parent);

    // Clearance resolves strut(10), child margin starts fresh.
    assert_eq!(resolved, lu(10));
    assert!(state.bfc_offset_resolved);
}

// ── 15. Multiple consecutive empty blocks ───────────────────────────────

#[test]
fn multiple_consecutive_empty_blocks() {
    // Three consecutive empty blocks with margins 10, 20, 5.
    // All collapse together: max(10, 20, 5) = 20.
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    let margins = [10, 20, 5];
    for &m in &margins {
        let mut child_strut = MarginStrut::new();
        child_strut.append_normal(lu(m));
        // For empty blocks, top and bottom margins are identical, so the
        // strut carries both.

        let child = ChildMarginInfo {
            margin_top: lu(m),
            margin_bottom: lu(m),
            establishes_bfc: false,
            is_float: false,
            has_clearance: false,
            collapsed_through: true,
            child_margin_strut: child_strut,
        };

        // Before: appends child's top margin to strut, then resolves
        // (but for collapsed-through children the behavior is: the resolve
        // emits spacing, and after-child merges the child strut back in).
        let parent = parent_no_separator(false, false);
        let _resolved = handle_margin_before_child(&mut state, &child, &parent);
        handle_margin_after_child(&mut state, &child);
    }

    // After all three empty blocks, the strut carries max(10, 20, 5) = 20.
    assert_eq!(collapse_margins(&state.margin_strut), lu(20));
}

// ── 16. Deeply nested parent-child collapsing ───────────────────────────

#[test]
fn deeply_nested_parent_child_collapsing() {
    // Grandparent(margin-top:5) > Parent(margin-top:10) > Child(margin-top:20)
    // All collapse together (no border/padding). Result = max(5,10,20) = 20.
    let mut gp_state = MarginCollapsingState::new();
    gp_state.margin_strut.append_normal(lu(5));

    // Parent is first child of grandparent, no separator.
    let parent_as_child = normal_child(10, 0);
    let gp_info = parent_no_separator(true, false);
    let resolved = handle_margin_before_child(&mut gp_state, &parent_as_child, &gp_info);
    assert_eq!(resolved, lu(0)); // collapses upward

    // Now the strut has max(5, 10) = 10.
    // Child is first child of parent — simulate by appending to same strut.
    gp_state.margin_strut.append_normal(lu(20));

    // Strut should now be max(5, 10, 20) = 20.
    assert_eq!(collapse_margins(&gp_state.margin_strut), lu(20));
}

// ── 17. Negative and positive margin chain ──────────────────────────────

#[test]
fn negative_and_positive_margin_chain() {
    // Chain: +30, -10, +5, -20
    // positive_max = max(30, 5) = 30
    // negative_min = min(-10, -20) = -20
    // result = 30 + (-20) = 10
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(30));
    strut.append_normal(lu(-10));
    strut.append_normal(lu(5));
    strut.append_normal(lu(-20));
    assert_eq!(adjoining_margin_resolve(&strut), lu(10));
}

// ── 18. Quirky margin handling ──────────────────────────────────────────

#[test]
fn quirky_margin_handling() {
    let mut strut = MarginStrut::new();
    strut.append(lu(15), true); // quirky
    strut.append_normal(lu(10));
    // max(quirky=15, normal=10) + 0 = 15
    assert_eq!(adjoining_margin_resolve(&strut), lu(15));

    // Quirky smaller than normal:
    let mut strut2 = MarginStrut::new();
    strut2.append(lu(5), true);
    strut2.append_normal(lu(20));
    assert_eq!(adjoining_margin_resolve(&strut2), lu(20));
}

#[test]
fn quirky_container_start_ignores_quirky() {
    let mut strut = MarginStrut::new();
    strut.is_quirky_container_start = true;
    strut.append(lu(30), true); // ignored
    strut.append_normal(lu(10));
    assert_eq!(adjoining_margin_resolve(&strut), lu(10));
}

// ── 19. Zero-margin edge cases ──────────────────────────────────────────

#[test]
fn zero_margins_collapse_to_zero() {
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    let child_a = normal_child(0, 0);
    handle_margin_after_child(&mut state, &child_a);

    let child_b = normal_child(0, 0);
    let parent = parent_no_separator(false, false);
    let resolved = handle_margin_before_child(&mut state, &child_b, &parent);

    assert_eq!(resolved, lu(0));
}

#[test]
fn zero_and_positive_margin() {
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(0));
    strut.append_normal(lu(15));
    assert_eq!(collapse_margins(&strut), lu(15));
}

#[test]
fn zero_and_negative_margin() {
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(0));
    strut.append_normal(lu(-7));
    assert_eq!(collapse_margins(&strut), lu(-7));
}

// ── 20. MarginStrut resolve positive only ───────────────────────────────

#[test]
fn margin_strut_resolve_positive_only() {
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(10));
    strut.append_normal(lu(40));
    strut.append_normal(lu(25));
    assert_eq!(collapse_margins(&strut), lu(40));
}

// ── 21. MarginStrut resolve negative only ───────────────────────────────

#[test]
fn margin_strut_resolve_negative_only() {
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(-3));
    strut.append_normal(lu(-12));
    strut.append_normal(lu(-8));
    assert_eq!(collapse_margins(&strut), lu(-12));
}

// ── 22. MarginStrut resolve mixed ───────────────────────────────────────

#[test]
fn margin_strut_resolve_mixed() {
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(50));
    strut.append_normal(lu(-30));
    // 50 + (-30) = 20
    assert_eq!(collapse_margins(&strut), lu(20));
}

// ── Additional tests ────────────────────────────────────────────────────

#[test]
fn bfc_detection_overflow_scroll() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::Block,
        Overflow::Scroll,
        Overflow::Visible,
        Float::None,
        Position::Static,
    ));
}

#[test]
fn bfc_detection_overflow_auto() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::Block,
        Overflow::Visible,
        Overflow::Auto,
        Float::None,
        Position::Static,
    ));
}

#[test]
fn bfc_detection_fixed_position() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::Block,
        Overflow::Visible,
        Overflow::Visible,
        Float::None,
        Position::Fixed,
    ));
}

#[test]
fn bfc_detection_grid() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::Grid,
        Overflow::Visible,
        Overflow::Visible,
        Float::None,
        Position::Static,
    ));
}

#[test]
fn merge_struts_combines_correctly() {
    let mut target = MarginStrut::new();
    target.append_normal(lu(10));
    target.append_normal(lu(-5));

    let mut source = MarginStrut::new();
    source.append_normal(lu(20));
    source.append_normal(lu(-15));

    merge_struts(&mut target, &source);

    // positive = max(10, 20) = 20
    // negative = min(-5, -15) = -15
    // sum = 20 + (-15) = 5
    assert_eq!(collapse_margins(&target), lu(5));
}

#[test]
fn empty_block_finalize_propagates_strut() {
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(10)); // top margin

    let parent = parent_no_separator(false, false);
    let (resolved, propagated) =
        finalize_margins(&mut state, &parent, lu(15), true);

    // Empty block: resolved = 0, propagated has max(10, 15) = 15.
    assert_eq!(resolved, lu(0));
    assert_eq!(collapse_margins(&propagated), lu(15));
}

#[test]
fn state_new_resolved_has_bfc_set() {
    let state = MarginCollapsingState::new_resolved();
    assert!(state.bfc_offset_resolved);
    assert!(!state.is_empty_block);
}

#[test]
fn empty_block_with_padding_no_collapse_through() {
    let params = CollapseCheckParams {
        border: BoxStrut::zero(),
        padding: BoxStrut::new(lu(0), lu(0), lu(3), lu(0)),
        block_size: None,
        min_block_size: lu(0),
        in_flow_child_count: 0,
        has_line_boxes: false,
    };
    assert!(!should_margins_collapse_through(&params));
}

#[test]
fn empty_block_with_explicit_height_no_collapse_through() {
    let params = CollapseCheckParams {
        border: BoxStrut::zero(),
        padding: BoxStrut::zero(),
        block_size: Some(lu(50)),
        min_block_size: lu(0),
        in_flow_child_count: 0,
        has_line_boxes: false,
    };
    assert!(!should_margins_collapse_through(&params));
}

#[test]
fn parent_last_child_blocked_by_border() {
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;
    state.margin_strut.append_normal(lu(25));

    let parent = ParentMarginInfo {
        border: BoxStrut::new(lu(0), lu(0), lu(1), lu(0)),
        padding: BoxStrut::zero(),
        is_first_child: false,
        is_last_child: true,
        block_size: None,
    };
    let (resolved_bottom, propagated) =
        finalize_margins(&mut state, &parent, lu(10), false);

    // Border separates.
    assert_eq!(resolved_bottom, lu(25));
    assert_eq!(collapse_margins(&propagated), lu(10));
}

#[test]
fn parent_last_child_blocked_by_padding() {
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;
    state.margin_strut.append_normal(lu(15));

    let parent = ParentMarginInfo {
        border: BoxStrut::zero(),
        padding: BoxStrut::new(lu(0), lu(0), lu(5), lu(0)),
        is_first_child: false,
        is_last_child: true,
        block_size: None,
    };
    let (resolved_bottom, propagated) =
        finalize_margins(&mut state, &parent, lu(10), false);

    assert_eq!(resolved_bottom, lu(15));
    assert_eq!(collapse_margins(&propagated), lu(10));
}

#[test]
fn bfc_child_after_resets_strut() {
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;
    state.margin_strut.append_normal(lu(100));

    let child = ChildMarginInfo {
        margin_top: lu(10),
        margin_bottom: lu(30),
        establishes_bfc: true,
        is_float: false,
        has_clearance: false,
        collapsed_through: false,
        child_margin_strut: MarginStrut::new(),
    };

    handle_margin_after_child(&mut state, &child);

    // After a BFC child, strut is reset to just the child's bottom margin.
    assert_eq!(collapse_margins(&state.margin_strut), lu(30));
}

#[test]
fn discard_margins_produces_zero() {
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(100));
    strut.discard_margins = true;
    assert_eq!(collapse_margins(&strut), lu(0));
    assert_eq!(adjoining_margin_resolve(&strut), lu(0));
}

#[test]
fn normal_block_does_not_establish_bfc() {
    assert!(!establishes_new_bfc_for_collapsing(
        Display::Block,
        Overflow::Visible,
        Overflow::Visible,
        Float::None,
        Position::Static,
    ));
}
