//! SP12 H4 — Comprehensive margin collapsing tests (CSS 2.1 §8.3.1).
//!
//! Covers adjacent sibling collapsing, parent-child collapsing, empty block
//! collapse-through, negative margins, collapsing prevention, and complex
//! multi-level scenarios.  Uses both the unit-level margin collapsing API
//! and the integration-level `BlockTestBuilder` from the WPT helpers.

#[path = "sp12_wpt_helpers.rs"]
mod sp12_wpt_helpers;

use openui_geometry::{BoxStrut, LayoutUnit, MarginStrut};
use openui_layout::margin_collapsing::{
    adjoining_margin_resolve, clearance_prevents_collapsing, collapse_margins,
    establishes_new_bfc_for_collapsing, finalize_margins, float_prevents_collapsing,
    handle_margin_after_child, handle_margin_before_child, merge_struts,
    should_margins_collapse_through, ChildMarginInfo, CollapseCheckParams,
    MarginCollapsingState, ParentMarginInfo,
};
use openui_style::{Clear, Display, Float, Overflow, Position};
use sp12_wpt_helpers::*;

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

fn empty_child(top: i32, bottom: i32) -> ChildMarginInfo {
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(top));
    strut.append_normal(lu(bottom));
    ChildMarginInfo {
        margin_top: lu(top),
        margin_bottom: lu(bottom),
        establishes_bfc: false,
        is_float: false,
        has_clearance: false,
        collapsed_through: true,
        child_margin_strut: strut,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. Adjacent Sibling Collapsing (20+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn sibling_equal_positive_margins() {
    // Both 20px → max(20,20) = 20
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    let a = normal_child(0, 20);
    handle_margin_after_child(&mut state, &a);

    let b = normal_child(20, 0);
    let resolved = handle_margin_before_child(&mut state, &b, &parent_no_separator(false, false));
    assert_eq!(resolved, lu(20));
}

#[test]
fn sibling_different_positive_larger_wins() {
    // 15 bottom + 30 top → 30
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 15));
    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(30, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(resolved, lu(30));
}

#[test]
fn sibling_zero_bottom_positive_top() {
    // margin-bottom:0, margin-top:20 → 20
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 0));
    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(20, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(resolved, lu(20));
}

#[test]
fn sibling_positive_bottom_zero_top() {
    // margin-bottom:25, margin-top:0 → 25
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 25));
    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(0, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(resolved, lu(25));
}

#[test]
fn three_siblings_pairwise_collapsing() {
    // A(mb:10) B(mt:20, mb:5) C(mt:15)
    // A-B gap = max(10,20) = 20, B-C gap = max(5,15) = 15
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 10));
    let gap_ab = handle_margin_before_child(
        &mut state,
        &normal_child(20, 5),
        &parent_no_separator(false, false),
    );
    assert_eq!(gap_ab, lu(20));

    handle_margin_after_child(&mut state, &normal_child(20, 5));
    let gap_bc = handle_margin_before_child(
        &mut state,
        &normal_child(15, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(gap_bc, lu(15));
}

#[test]
fn sibling_large_gap_both_contributing() {
    // 100 bottom + 80 top → 100
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 100));
    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(80, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(resolved, lu(100));
}

#[test]
fn sibling_first_child_bottom_margin_only() {
    // A(mb:40) B(mt:0) → 40
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 40));
    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(0, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(resolved, lu(40));
}

#[test]
fn sibling_second_child_top_margin_only() {
    // A(mb:0) B(mt:35) → 35
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 0));
    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(35, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(resolved, lu(35));
}

#[test]
fn sibling_very_small_margins() {
    // 1 + 2 → 2
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 1));
    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(2, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(resolved, lu(2));
}

#[test]
fn four_siblings_sequential_collapsing() {
    // A(mb:10) B(mt:20,mb:30) C(mt:5,mb:15) D(mt:25)
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 10));
    let ab = handle_margin_before_child(
        &mut state,
        &normal_child(20, 30),
        &parent_no_separator(false, false),
    );
    assert_eq!(ab, lu(20));

    handle_margin_after_child(&mut state, &normal_child(20, 30));
    let bc = handle_margin_before_child(
        &mut state,
        &normal_child(5, 15),
        &parent_no_separator(false, false),
    );
    assert_eq!(bc, lu(30));

    handle_margin_after_child(&mut state, &normal_child(5, 15));
    let cd = handle_margin_before_child(
        &mut state,
        &normal_child(25, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(cd, lu(25));
}

#[test]
fn sibling_identical_margins_all_sides() {
    // Both have 10 all around — collapsed gap = max(10,10) = 10
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(10, 10));
    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(10, 10),
        &parent_no_separator(false, false),
    );
    assert_eq!(resolved, lu(10));
}

#[test]
fn sibling_strut_positive_only_accumulation() {
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(5));
    strut.append_normal(lu(10));
    strut.append_normal(lu(3));
    assert_eq!(collapse_margins(&strut), lu(10));
}

#[test]
fn sibling_integration_two_blocks_equal_margins() {
    // Integration: two 50px-tall blocks each with margin 20 on all sides.
    // First child's margin-top collapses with container (no border/padding) → y=0.
    // Gap between them = max(20,20) = 20. Child 1 at y=0+50+20 = 70.
    let mut builder = BlockTestBuilder::new(400, 600);
    builder.add_child().height(50.0).margin(20, 0, 20, 0).done();
    builder.add_child().height(50.0).margin(20, 0, 20, 0).done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_child_position(1, 0, 70);
}

#[test]
fn sibling_integration_different_positive() {
    // A: mb=30, B: mt=10 → gap = 30.  A at y=0, h=40.  B at y=40+30=70.
    let mut builder = BlockTestBuilder::new(400, 600);
    builder.add_child().height(40.0).margin(0, 0, 30, 0).done();
    builder.add_child().height(40.0).margin(10, 0, 0, 0).done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_child_position(1, 0, 70);
}

#[test]
fn sibling_integration_three_blocks_collapsing() {
    // A(h=20,mb=10) B(h=20,mt=20,mb=5) C(h=20,mt=15)
    // A at y=0. A-B gap=max(10,20)=20. B at y=20+20=40.
    // B-C gap=max(5,15)=15. C at y=40+20+15=75.
    let mut builder = BlockTestBuilder::new(400, 600);
    builder.add_child().height(20.0).margin(0, 0, 10, 0).done();
    builder
        .add_child()
        .height(20.0)
        .margin(20, 0, 5, 0)
        .done();
    builder.add_child().height(20.0).margin(15, 0, 0, 0).done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_child_position(1, 0, 40);
    result.assert_child_position(2, 0, 75);
}

#[test]
fn sibling_integration_zero_and_positive() {
    // A(mb=0) B(mt=50) → gap = 50
    let mut builder = BlockTestBuilder::new(400, 600);
    builder.add_child().height(30.0).margin(0, 0, 0, 0).done();
    builder.add_child().height(30.0).margin(50, 0, 0, 0).done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_child_position(1, 0, 80);
}

#[test]
fn sibling_integration_large_margin() {
    // A(h=10,mb=200) B(h=10,mt=100) → gap=200
    let mut builder = BlockTestBuilder::new(400, 600);
    builder
        .add_child()
        .height(10.0)
        .margin(0, 0, 200, 0)
        .done();
    builder
        .add_child()
        .height(10.0)
        .margin(100, 0, 0, 0)
        .done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_child_position(1, 0, 210);
}

#[test]
fn sibling_integration_five_equal_blocks() {
    // 5 blocks: h=20, mt=mb=10.  First child collapses with container → y=0.
    // Subsequent gaps = max(10,10) = 10. Positions: 0, 30, 60, 90, 120.
    let mut builder = BlockTestBuilder::new(400, 600);
    for _ in 0..5 {
        builder.add_child().height(20.0).margin(10, 0, 10, 0).done();
    }
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_child_position(1, 0, 30);
    result.assert_child_position(2, 0, 60);
    result.assert_child_position(3, 0, 90);
    result.assert_child_position(4, 0, 120);
}

#[test]
fn sibling_both_zero_margins() {
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 0));
    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(0, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(resolved, lu(0));
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. Parent-Child Collapsing (20+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn parent_first_child_top_collapse() {
    // Parent mt=10 in strut, first child mt=25. No separator → max(10,25)=25
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(10));

    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(25, 0),
        &parent_no_separator(true, false),
    );
    assert_eq!(resolved, lu(0));
    assert_eq!(collapse_margins(&state.margin_strut), lu(25));
}

#[test]
fn parent_first_child_child_larger() {
    // Parent mt=5, first child mt=50 → 50
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(5));

    handle_margin_before_child(
        &mut state,
        &normal_child(50, 0),
        &parent_no_separator(true, false),
    );
    assert_eq!(collapse_margins(&state.margin_strut), lu(50));
}

#[test]
fn parent_first_child_parent_larger() {
    // Parent mt=40, first child mt=10 → 40
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(40));

    handle_margin_before_child(
        &mut state,
        &normal_child(10, 0),
        &parent_no_separator(true, false),
    );
    assert_eq!(collapse_margins(&state.margin_strut), lu(40));
}

#[test]
fn parent_last_child_bottom_collapse() {
    // Last child mb=20 in strut, parent mb=15. No separator → max(20,15)=20
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;
    state.margin_strut.append_normal(lu(20));

    let parent = parent_no_separator(false, true);
    let (resolved_bottom, propagated) = finalize_margins(&mut state, &parent, lu(15), false);
    assert_eq!(resolved_bottom, lu(0));
    assert_eq!(collapse_margins(&propagated), lu(20));
}

#[test]
fn parent_last_child_parent_margin_larger() {
    // Last child mb=5, parent mb=30 → propagated = 30
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;
    state.margin_strut.append_normal(lu(5));

    let (_, propagated) =
        finalize_margins(&mut state, &parent_no_separator(false, true), lu(30), false);
    assert_eq!(collapse_margins(&propagated), lu(30));
}

#[test]
fn parent_first_child_border_top_prevents() {
    // Border-top separates parent and first child.
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(10));

    let parent = ParentMarginInfo {
        border: BoxStrut::new(lu(1), lu(0), lu(0), lu(0)),
        padding: BoxStrut::zero(),
        is_first_child: true,
        is_last_child: false,
        block_size: None,
    };
    let resolved = handle_margin_before_child(&mut state, &normal_child(20, 0), &parent);
    // Parent strut (10) resolved, child starts fresh with 20.
    assert_eq!(resolved, lu(10));
    assert_eq!(collapse_margins(&state.margin_strut), lu(20));
}

#[test]
fn parent_first_child_padding_top_prevents() {
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(15));

    let parent = ParentMarginInfo {
        border: BoxStrut::zero(),
        padding: BoxStrut::new(lu(5), lu(0), lu(0), lu(0)),
        is_first_child: true,
        is_last_child: false,
        block_size: None,
    };
    let resolved = handle_margin_before_child(&mut state, &normal_child(20, 0), &parent);
    assert_eq!(resolved, lu(15));
}

#[test]
fn parent_last_child_border_bottom_prevents() {
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
    let (resolved_bottom, propagated) = finalize_margins(&mut state, &parent, lu(10), false);
    assert_eq!(resolved_bottom, lu(25));
    assert_eq!(collapse_margins(&propagated), lu(10));
}

#[test]
fn parent_last_child_padding_bottom_prevents() {
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;
    state.margin_strut.append_normal(lu(15));

    let parent = ParentMarginInfo {
        border: BoxStrut::zero(),
        padding: BoxStrut::new(lu(0), lu(0), lu(3), lu(0)),
        is_first_child: false,
        is_last_child: true,
        block_size: None,
    };
    let (resolved_bottom, _) = finalize_margins(&mut state, &parent, lu(10), false);
    assert_eq!(resolved_bottom, lu(15));
}

#[test]
fn parent_last_child_height_prevents() {
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
    let (resolved_bottom, propagated) = finalize_margins(&mut state, &parent, lu(15), false);
    assert_eq!(resolved_bottom, lu(20));
    assert_eq!(collapse_margins(&propagated), lu(15));
}

#[test]
fn parent_last_child_min_height_prevents() {
    // min-height doesn't directly appear in ParentMarginInfo, but an explicit
    // block_size (resulting from min-height clamping) prevents collapse.
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;
    state.margin_strut.append_normal(lu(30));

    let parent = ParentMarginInfo {
        border: BoxStrut::zero(),
        padding: BoxStrut::zero(),
        is_first_child: false,
        is_last_child: true,
        block_size: Some(lu(50)),
    };
    let (resolved_bottom, propagated) = finalize_margins(&mut state, &parent, lu(10), false);
    assert_eq!(resolved_bottom, lu(30));
    assert_eq!(collapse_margins(&propagated), lu(10));
}

#[test]
fn nested_grandchild_collapsing_chain() {
    // GP(mt:5) > P(mt:10) > Child(mt:20) — all collapse → 20
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(5));

    // Parent is first child of GP
    handle_margin_before_child(
        &mut state,
        &normal_child(10, 0),
        &parent_no_separator(true, false),
    );
    // Child is first child of Parent — continue accumulating
    state.margin_strut.append_normal(lu(20));

    assert_eq!(collapse_margins(&state.margin_strut), lu(20));
}

#[test]
fn parent_first_child_equal_margins() {
    // Both 15 → max(15,15) = 15
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(15));

    handle_margin_before_child(
        &mut state,
        &normal_child(15, 0),
        &parent_no_separator(true, false),
    );
    assert_eq!(collapse_margins(&state.margin_strut), lu(15));
}

#[test]
fn parent_first_child_zero_parent_margin() {
    // Parent mt=0, child mt=30 → 30
    let mut state = MarginCollapsingState::new();
    // No parent margin appended (it's 0).

    handle_margin_before_child(
        &mut state,
        &normal_child(30, 0),
        &parent_no_separator(true, false),
    );
    assert_eq!(collapse_margins(&state.margin_strut), lu(30));
}

#[test]
fn parent_first_child_zero_child_margin() {
    // Parent mt=20, child mt=0 → 20
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(20));

    handle_margin_before_child(
        &mut state,
        &normal_child(0, 0),
        &parent_no_separator(true, false),
    );
    assert_eq!(collapse_margins(&state.margin_strut), lu(20));
}

#[test]
fn parent_child_integration_top_collapse() {
    // Parent has no border/padding. First child mt=30, h=50.
    // Margin collapses with parent: child at y=0 (margin propagates out).
    let mut builder = BlockTestBuilder::new(400, 600);
    builder
        .add_child()
        .height(50.0)
        .margin(30, 0, 0, 0)
        .done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
}

#[test]
fn parent_child_integration_border_prevents_collapse() {
    // Container with border-top:1 prevents first-child collapse.
    // Child mt=30 doesn't collapse with container.
    let mut builder = BlockTestBuilder::new(400, 600);
    builder
        .add_child()
        .height(50.0)
        .margin(30, 0, 0, 0)
        .done();
    let result = builder
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = openui_style::BorderStyle::Solid;
        })
        .build();

    // With border, child is at y = border(1) + margin(30) = 31 within container.
    result.assert_child_position(0, 0, 31);
}

#[test]
fn parent_child_integration_padding_prevents_collapse() {
    let mut builder = BlockTestBuilder::new(400, 600);
    builder
        .add_child()
        .height(50.0)
        .margin(30, 0, 0, 0)
        .done();
    let result = builder
        .with_container_style(|s| {
            s.padding_top = openui_geometry::Length::px(5.0);
        })
        .build();

    // Child at y = padding(5) + margin(30) = 35
    result.assert_child_position(0, 0, 35);
}

#[test]
fn parent_child_both_margins_collapse() {
    // Parent mt=10, child mt=20. No separator.
    // Child's margin collapses with parent; child at y=0 inside container.
    // Container position reflects its own margin (10) since the viewport
    // resolves the container's BFC offset before child margin propagation.
    let mut builder = BlockTestBuilder::new(400, 600);
    builder
        .add_child()
        .height(50.0)
        .margin(20, 0, 0, 0)
        .done();
    let result = builder
        .with_container_style(|s| {
            s.margin_top = openui_geometry::Length::px(10.0);
        })
        .build();

    // Child collapses with parent — positioned at y=0 inside the container.
    result.assert_child_position(0, 0, 0);
}

#[test]
fn parent_first_child_not_first_child_no_collapse() {
    // Second child shouldn't collapse with parent's margin-top.
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;
    state.margin_strut.append_normal(lu(10));

    // First, handle first child normally
    let first = normal_child(5, 15);
    handle_margin_before_child(&mut state, &first, &parent_no_separator(true, false));
    handle_margin_after_child(&mut state, &first);

    // Second child — not first child
    let second = normal_child(20, 0);
    let resolved =
        handle_margin_before_child(&mut state, &second, &parent_no_separator(false, false));
    // Should collapse with first child's bottom margin, not parent's top.
    assert_eq!(resolved, lu(20));
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. Empty Block Collapsing (15+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn empty_block_collapses_through_basic() {
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

#[test]
fn empty_block_border_top_prevents() {
    let params = CollapseCheckParams {
        border: BoxStrut::new(lu(1), lu(0), lu(0), lu(0)),
        padding: BoxStrut::zero(),
        block_size: None,
        min_block_size: lu(0),
        in_flow_child_count: 0,
        has_line_boxes: false,
    };
    assert!(!should_margins_collapse_through(&params));
}

#[test]
fn empty_block_border_bottom_prevents() {
    let params = CollapseCheckParams {
        border: BoxStrut::new(lu(0), lu(0), lu(1), lu(0)),
        padding: BoxStrut::zero(),
        block_size: None,
        min_block_size: lu(0),
        in_flow_child_count: 0,
        has_line_boxes: false,
    };
    assert!(!should_margins_collapse_through(&params));
}

#[test]
fn empty_block_padding_top_prevents() {
    let params = CollapseCheckParams {
        border: BoxStrut::zero(),
        padding: BoxStrut::new(lu(1), lu(0), lu(0), lu(0)),
        block_size: None,
        min_block_size: lu(0),
        in_flow_child_count: 0,
        has_line_boxes: false,
    };
    assert!(!should_margins_collapse_through(&params));
}

#[test]
fn empty_block_padding_bottom_prevents() {
    let params = CollapseCheckParams {
        border: BoxStrut::zero(),
        padding: BoxStrut::new(lu(0), lu(0), lu(1), lu(0)),
        block_size: None,
        min_block_size: lu(0),
        in_flow_child_count: 0,
        has_line_boxes: false,
    };
    assert!(!should_margins_collapse_through(&params));
}

#[test]
fn empty_block_min_height_prevents() {
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

#[test]
fn empty_block_explicit_height_prevents() {
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
fn empty_block_height_zero_collapses() {
    // height:0 with no border/padding still collapses through.
    let params = CollapseCheckParams {
        border: BoxStrut::zero(),
        padding: BoxStrut::zero(),
        block_size: Some(lu(0)),
        min_block_size: lu(0),
        in_flow_child_count: 0,
        has_line_boxes: false,
    };
    // height:0 is explicit — spec says "has computed height of zero" doesn't prevent.
    // Actually per CSS 2.1 §8.3.1: collapse-through when "height computes to 0 or auto".
    assert!(should_margins_collapse_through(&params));
}

#[test]
fn empty_block_has_line_boxes_prevents() {
    let params = CollapseCheckParams {
        border: BoxStrut::zero(),
        padding: BoxStrut::zero(),
        block_size: None,
        min_block_size: lu(0),
        in_flow_child_count: 0,
        has_line_boxes: true,
    };
    assert!(!should_margins_collapse_through(&params));
}

#[test]
fn empty_block_has_in_flow_children_prevents() {
    let params = CollapseCheckParams {
        border: BoxStrut::zero(),
        padding: BoxStrut::zero(),
        block_size: None,
        min_block_size: lu(0),
        in_flow_child_count: 1,
        has_line_boxes: false,
    };
    assert!(!should_margins_collapse_through(&params));
}

#[test]
fn multiple_empty_blocks_collapse_chain() {
    // Three empty blocks: margins 10, 20, 5. All collapse → max(10,20,5) = 20.
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    for &m in &[10, 20, 5] {
        let child = empty_child(m, m);
        let parent = parent_no_separator(false, false);
        let _resolved = handle_margin_before_child(&mut state, &child, &parent);
        handle_margin_after_child(&mut state, &child);
    }
    assert_eq!(collapse_margins(&state.margin_strut), lu(20));
}

#[test]
fn empty_block_between_siblings() {
    // A(mb=10) Empty(mt=5,mb=5) B(mt=15)
    // After empty (collapsed_through), B's margin accumulates without resolving
    // (previous_child_collapsed_through = true → returns 0).
    // The strut carries all accumulated margins.
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 10));

    let empty = empty_child(5, 5);
    let gap_to_empty =
        handle_margin_before_child(&mut state, &empty, &parent_no_separator(false, false));
    // Empty resolves A's strut: max(10,5) = 10.
    assert_eq!(gap_to_empty, lu(10));

    handle_margin_after_child(&mut state, &empty);
    // previous_child_collapsed_through is now true.
    assert!(state.previous_child_collapsed_through);

    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(15, 0),
        &parent_no_separator(false, false),
    );
    // B accumulates (returns 0) because prev collapsed through.
    assert_eq!(resolved, lu(0));
    // Strut holds max(5,15) = 15.
    assert_eq!(collapse_margins(&state.margin_strut), lu(15));
}

#[test]
fn two_consecutive_empty_blocks() {
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    for &m in &[8, 12] {
        let child = empty_child(m, m);
        handle_margin_before_child(&mut state, &child, &parent_no_separator(false, false));
        handle_margin_after_child(&mut state, &child);
    }
    assert_eq!(collapse_margins(&state.margin_strut), lu(12));
}

#[test]
fn five_consecutive_empty_blocks() {
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    for &m in &[3, 7, 2, 9, 4] {
        let child = empty_child(m, m);
        handle_margin_before_child(&mut state, &child, &parent_no_separator(false, false));
        handle_margin_after_child(&mut state, &child);
    }
    assert_eq!(collapse_margins(&state.margin_strut), lu(9));
}

#[test]
fn empty_block_with_horizontal_padding_still_collapses() {
    // Horizontal padding doesn't prevent collapse-through.
    let params = CollapseCheckParams {
        border: BoxStrut::zero(),
        padding: BoxStrut::new(lu(0), lu(10), lu(0), lu(10)),
        block_size: None,
        min_block_size: lu(0),
        in_flow_child_count: 0,
        has_line_boxes: false,
    };
    assert!(should_margins_collapse_through(&params));
}

#[test]
fn empty_block_integration_between_content() {
    // Two content blocks with an empty block between them.
    // A(h=30,mb=10) Empty(mt=5,mb=5) B(h=30,mt=15)
    // Empty block is positioned as a separate sibling (h=0):
    //   A-Empty gap = max(10,5) = 10. Empty at y=40.
    //   Empty-B gap = max(5,15) = 15. B at y=40+0+15 = 55.
    let mut builder = BlockTestBuilder::new(400, 600);
    builder.add_child().height(30.0).margin(0, 0, 10, 0).done();
    builder.add_child().margin(5, 0, 5, 0).done(); // empty: no height
    builder.add_child().height(30.0).margin(15, 0, 0, 0).done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_child_position(2, 0, 55);
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. Negative Margins (15+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn negative_positive_sum_basic() {
    // +20 + (-10) = 10
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(20));
    strut.append_normal(lu(-10));
    assert_eq!(collapse_margins(&strut), lu(10));
}

#[test]
fn negative_minus10_plus20_equals_10() {
    // -10 + 20 = 10
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(-10));
    strut.append_normal(lu(20));
    assert_eq!(collapse_margins(&strut), lu(10));
}

#[test]
fn negative_minus20_plus10_equals_minus10() {
    // -20 + 10 = -10
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(-20));
    strut.append_normal(lu(10));
    assert_eq!(collapse_margins(&strut), lu(-10));
}

#[test]
fn two_negatives_most_negative_wins() {
    // min(-5, -15) = -15
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(-5));
    strut.append_normal(lu(-15));
    assert_eq!(collapse_margins(&strut), lu(-15));
}

#[test]
fn two_negatives_equal() {
    // min(-10, -10) = -10
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(-10));
    strut.append_normal(lu(-10));
    assert_eq!(collapse_margins(&strut), lu(-10));
}

#[test]
fn negative_sibling_one_negative() {
    // A(mb=20) B(mt=-10) → 20 + (-10) = 10
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 20));
    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(-10, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(resolved, lu(10));
}

#[test]
fn negative_sibling_both_negative() {
    // A(mb=-5) B(mt=-15) → min(-5,-15) = -15
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, -5));
    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(-15, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(resolved, lu(-15));
}

#[test]
fn negative_margin_causes_overlap() {
    // A(mb=-30) B(mt=10) → max(0,10) + min(0,-30) = 10 + (-30) = -20
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, -30));
    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(10, 0),
        &parent_no_separator(false, false),
    );
    assert_eq!(resolved, lu(-20));
}

#[test]
fn negative_chain_positive_and_negative() {
    // +30, -10, +5, -20 → max(30,5) + min(-10,-20) = 30 + (-20) = 10
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(30));
    strut.append_normal(lu(-10));
    strut.append_normal(lu(5));
    strut.append_normal(lu(-20));
    assert_eq!(adjoining_margin_resolve(&strut), lu(10));
}

#[test]
fn negative_only_single() {
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(-7));
    assert_eq!(collapse_margins(&strut), lu(-7));
}

#[test]
fn positive_only_single() {
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(42));
    assert_eq!(collapse_margins(&strut), lu(42));
}

#[test]
fn negative_large_positive_small() {
    // -100 + 5 = -95
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(-100));
    strut.append_normal(lu(5));
    assert_eq!(collapse_margins(&strut), lu(-95));
}

#[test]
fn negative_equal_magnitude() {
    // +10 + (-10) = 0
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(10));
    strut.append_normal(lu(-10));
    assert_eq!(collapse_margins(&strut), lu(0));
}

#[test]
fn negative_integration_overlap() {
    // A(h=50,mb=-20) B(h=50,mt=10) → gap = -20 + 10 = -10
    // A at y=0. B at y = 50 + (-10) = 40 (overlap by 10px).
    let mut builder = BlockTestBuilder::new(400, 600);
    builder
        .add_child()
        .height(50.0)
        .margin(0, 0, -20, 0)
        .done();
    builder
        .add_child()
        .height(50.0)
        .margin(10, 0, 0, 0)
        .done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_child_position(1, 0, 40);
}

#[test]
fn negative_integration_siblings() {
    // A(h=30,mb=20) B(h=30,mt=-10) → gap = max(20,0)+min(0,-10) = 20+(-10) = 10
    let mut builder = BlockTestBuilder::new(400, 600);
    builder
        .add_child()
        .height(30.0)
        .margin(0, 0, 20, 0)
        .done();
    builder
        .add_child()
        .height(30.0)
        .margin(-10, 0, 0, 0)
        .done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_child_position(1, 0, 40);
}

#[test]
fn three_negatives_most_negative() {
    // min(-3, -12, -8) = -12
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(-3));
    strut.append_normal(lu(-12));
    strut.append_normal(lu(-8));
    assert_eq!(collapse_margins(&strut), lu(-12));
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. Collapsing Prevention (15+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bfc_overflow_hidden_prevents() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::Block,
        Overflow::Hidden,
        Overflow::Visible,
        Float::None,
        Position::Static,
    ));
}

#[test]
fn bfc_overflow_scroll_prevents() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::Block,
        Overflow::Scroll,
        Overflow::Visible,
        Float::None,
        Position::Static,
    ));
}

#[test]
fn bfc_overflow_auto_prevents() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::Block,
        Overflow::Visible,
        Overflow::Auto,
        Float::None,
        Position::Static,
    ));
}

#[test]
fn bfc_float_left_prevents() {
    assert!(float_prevents_collapsing(Float::Left));
}

#[test]
fn bfc_float_right_prevents() {
    assert!(float_prevents_collapsing(Float::Right));
}

#[test]
fn bfc_float_none_does_not_prevent() {
    assert!(!float_prevents_collapsing(Float::None));
}

#[test]
fn bfc_absolute_position_prevents() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::Block,
        Overflow::Visible,
        Overflow::Visible,
        Float::None,
        Position::Absolute,
    ));
}

#[test]
fn bfc_fixed_position_prevents() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::Block,
        Overflow::Visible,
        Overflow::Visible,
        Float::None,
        Position::Fixed,
    ));
}

#[test]
fn clearance_left_prevents() {
    assert!(clearance_prevents_collapsing(Clear::Left));
}

#[test]
fn clearance_right_prevents() {
    assert!(clearance_prevents_collapsing(Clear::Right));
}

#[test]
fn clearance_both_prevents() {
    assert!(clearance_prevents_collapsing(Clear::Both));
}

#[test]
fn clearance_none_does_not_prevent() {
    assert!(!clearance_prevents_collapsing(Clear::None));
}

#[test]
fn flex_items_dont_collapse() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::Flex,
        Overflow::Visible,
        Overflow::Visible,
        Float::None,
        Position::Static,
    ));
}

#[test]
fn grid_items_dont_collapse() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::Grid,
        Overflow::Visible,
        Overflow::Visible,
        Float::None,
        Position::Static,
    ));
}

#[test]
fn inline_block_doesnt_collapse() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::InlineBlock,
        Overflow::Visible,
        Overflow::Visible,
        Float::None,
        Position::Static,
    ));
}

#[test]
fn flow_root_doesnt_collapse() {
    assert!(establishes_new_bfc_for_collapsing(
        Display::FlowRoot,
        Overflow::Visible,
        Overflow::Visible,
        Float::None,
        Position::Static,
    ));
}

#[test]
fn normal_block_allows_collapse() {
    assert!(!establishes_new_bfc_for_collapsing(
        Display::Block,
        Overflow::Visible,
        Overflow::Visible,
        Float::None,
        Position::Static,
    ));
}

#[test]
fn bfc_child_prevents_parent_collapse() {
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
    let resolved =
        handle_margin_before_child(&mut state, &child, &parent_no_separator(true, false));
    // BFC child forces parent strut to resolve first.
    assert_eq!(resolved, lu(10));
    assert!(state.bfc_offset_resolved);
}

#[test]
fn float_child_does_not_touch_strut() {
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
    let resolved =
        handle_margin_before_child(&mut state, &child, &parent_no_separator(true, false));
    assert_eq!(resolved, lu(0));
    assert_eq!(collapse_margins(&state.margin_strut), lu(10));

    handle_margin_after_child(&mut state, &child);
    assert_eq!(collapse_margins(&state.margin_strut), lu(10));
}

#[test]
fn clearance_child_severs_chain() {
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
    let resolved =
        handle_margin_before_child(&mut state, &child, &parent_no_separator(true, false));
    assert_eq!(resolved, lu(10));
    assert!(state.bfc_offset_resolved);
}

#[test]
fn overflow_hidden_integration_prevents_sibling_collapse() {
    // overflow:hidden establishes BFC — prevents parent-child collapse, but
    // sibling margins still collapse per CSS 2.1 §8.3.1.
    // Gap = max(20,10) = 20. Child 1 at y = 30 + 20 = 50.
    let mut builder = BlockTestBuilder::new(400, 600);
    builder
        .add_child()
        .height(30.0)
        .margin(0, 0, 20, 0)
        .done();
    builder
        .add_child()
        .height(30.0)
        .margin(10, 0, 0, 0)
        .overflow_hidden()
        .done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_child_position(1, 0, 50);
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. Complex Scenarios (10+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn chain_empty_blocks_between_content() {
    // A(mb=10) + 3 empty blocks (mt=mb=5 each) + B(mt=20)
    // First empty resolves A's strut. Subsequent empties accumulate
    // (previous_child_collapsed_through). B also accumulates (returns 0).
    // Final strut carries max(5,5,5,20) = 20.
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 10));

    for _ in 0..3 {
        let empty = empty_child(5, 5);
        handle_margin_before_child(&mut state, &empty, &parent_no_separator(false, false));
        handle_margin_after_child(&mut state, &empty);
    }

    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(20, 0),
        &parent_no_separator(false, false),
    );
    // B accumulates (prev collapsed through).
    assert_eq!(resolved, lu(0));
    // Strut holds max of all accumulated margins including B's mt=20.
    assert_eq!(collapse_margins(&state.margin_strut), lu(20));
}

#[test]
fn deeply_nested_five_levels() {
    // 5-level nesting: margins 5, 10, 15, 20, 25 all collapsing → 25
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(5));

    // Each level is first-child of parent, no separators.
    for &m in &[10, 15, 20, 25] {
        let child = normal_child(m, 0);
        handle_margin_before_child(&mut state, &child, &parent_no_separator(true, false));
    }
    assert_eq!(collapse_margins(&state.margin_strut), lu(25));
}

#[test]
fn merge_struts_mixed() {
    let mut target = MarginStrut::new();
    target.append_normal(lu(10));
    target.append_normal(lu(-5));

    let mut source = MarginStrut::new();
    source.append_normal(lu(20));
    source.append_normal(lu(-15));

    merge_struts(&mut target, &source);
    // pos = max(10,20) = 20, neg = min(-5,-15) = -15, sum = 5
    assert_eq!(collapse_margins(&target), lu(5));
}

#[test]
fn empty_block_finalize_propagation() {
    let mut state = MarginCollapsingState::new();
    state.margin_strut.append_normal(lu(10));

    let parent = parent_no_separator(false, false);
    let (resolved, propagated) = finalize_margins(&mut state, &parent, lu(15), true);
    assert_eq!(resolved, lu(0));
    assert_eq!(collapse_margins(&propagated), lu(15));
}

#[test]
fn mixed_positive_negative_through_empty_blocks() {
    // A(mb=30) Empty(mt=-10,mb=5) B(mt=15)
    // Empty's before-child resolves A's strut + empty's mt: max(30,0)+min(0,-10)=20.
    // After empty: merges child_strut (pos=5, neg=-10) into reset strut.
    // B accumulates (previous collapsed through), returns 0.
    let mut state = MarginCollapsingState::new();
    state.bfc_offset_resolved = true;

    handle_margin_after_child(&mut state, &normal_child(0, 30));

    let mut empty_strut = MarginStrut::new();
    empty_strut.append_normal(lu(-10));
    empty_strut.append_normal(lu(5));
    let empty = ChildMarginInfo {
        margin_top: lu(-10),
        margin_bottom: lu(5),
        establishes_bfc: false,
        is_float: false,
        has_clearance: false,
        collapsed_through: true,
        child_margin_strut: empty_strut,
    };
    let gap_to_empty =
        handle_margin_before_child(&mut state, &empty, &parent_no_separator(false, false));
    // Resolves: pos=30, neg=-10 → 20
    assert_eq!(gap_to_empty, lu(20));

    handle_margin_after_child(&mut state, &empty);

    let resolved = handle_margin_before_child(
        &mut state,
        &normal_child(15, 0),
        &parent_no_separator(false, false),
    );
    // B accumulates (prev collapsed through).
    assert_eq!(resolved, lu(0));
    // Strut: pos=max(5,15)=15, neg=-10. Sum=15+(-10)=5.
    assert_eq!(collapse_margins(&state.margin_strut), lu(5));
}

#[test]
fn bfc_child_after_resets_strut_complex() {
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
    // BFC resets: strut = just child's bottom margin
    assert_eq!(collapse_margins(&state.margin_strut), lu(30));
}

#[test]
fn discard_margins_flag() {
    let mut strut = MarginStrut::new();
    strut.append_normal(lu(100));
    strut.discard_margins = true;
    assert_eq!(collapse_margins(&strut), lu(0));
    assert_eq!(adjoining_margin_resolve(&strut), lu(0));
}

#[test]
fn complex_integration_mixed_children() {
    // Container with:
    //   A(h=40, mt=10, mb=20)
    //   B(h=0) — empty, mt=5, mb=5
    //   C(h=40, mt=15, mb=0)
    // A's mt=10 collapses with container → A at y=0.
    // A-B gap = max(20,5)=20. B at y=40+20=60.
    // B-C gap = max(5,15)=15. C at y=60+0+15=75.
    let mut builder = BlockTestBuilder::new(400, 600);
    builder
        .add_child()
        .height(40.0)
        .margin(10, 0, 20, 0)
        .done();
    builder.add_child().margin(5, 0, 5, 0).done(); // empty
    builder
        .add_child()
        .height(40.0)
        .margin(15, 0, 0, 0)
        .done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_child_position(2, 0, 75);
}

#[test]
fn complex_integration_nested_parent_child() {
    // Outer > Inner(mt=20) > Leaf(mt=30, h=50)
    // All collapse with container (no border/padding at any level).
    // Inner at y=0 inside container (margin propagates out).
    let mut builder = BlockTestBuilder::new(400, 600);
    builder
        .add_child()
        .margin(20, 0, 0, 0)
        .add_child()
        .margin(30, 0, 0, 0)
        .height(50.0)
        .done()
        .done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
}

#[test]
fn complex_integration_border_breaks_chain() {
    // Outer > Inner(mt=20, border-top=1) > Leaf(mt=30, h=50)
    // Inner's mt=20 collapses with container (no container border/padding).
    // Inner at y=0. Border on Inner prevents Inner-Leaf collapse.
    // Leaf at y = border(1) + margin(30) = 31 within Inner.
    let mut builder = BlockTestBuilder::new(400, 600);
    builder
        .add_child()
        .margin(20, 0, 0, 0)
        .border(1, 0, 0, 0)
        .add_child()
        .margin(30, 0, 0, 0)
        .height(50.0)
        .done()
        .done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_nested_child_position(0, 0, 0, 31);
}

#[test]
fn state_new_resolved_defaults() {
    let state = MarginCollapsingState::new_resolved();
    assert!(state.bfc_offset_resolved);
    assert!(!state.is_empty_block);
    assert!(!state.previous_child_collapsed_through);
}

#[test]
fn complex_five_siblings_alternating_margins() {
    // 5 siblings with alternating large/small margins.
    let mut builder = BlockTestBuilder::new(400, 800);
    builder
        .add_child()
        .height(20.0)
        .margin(0, 0, 50, 0)
        .done();
    builder
        .add_child()
        .height(20.0)
        .margin(10, 0, 30, 0)
        .done();
    builder
        .add_child()
        .height(20.0)
        .margin(5, 0, 40, 0)
        .done();
    builder
        .add_child()
        .height(20.0)
        .margin(15, 0, 25, 0)
        .done();
    builder
        .add_child()
        .height(20.0)
        .margin(35, 0, 0, 0)
        .done();
    let result = builder.build();

    // A at y=0.
    // A-B gap = max(50,10) = 50. B at 20+50=70.
    // B-C gap = max(30,5) = 30. C at 70+20+30=120.
    // C-D gap = max(40,15) = 40. D at 120+20+40=180.
    // D-E gap = max(25,35) = 35. E at 180+20+35=235.
    result.assert_child_position(0, 0, 0);
    result.assert_child_position(1, 0, 70);
    result.assert_child_position(2, 0, 120);
    result.assert_child_position(3, 0, 180);
    result.assert_child_position(4, 0, 235);
}

#[test]
fn complex_negative_sibling_overlap_integration() {
    // A(h=50, mb=-30) B(h=50, mt=-10)
    // pos_max=0, neg_min=min(-30,-10)=-30. Gap = -30. B at 50 + (-30) = 20.
    let mut builder = BlockTestBuilder::new(400, 600);
    builder
        .add_child()
        .height(50.0)
        .margin(0, 0, -30, 0)
        .done();
    builder
        .add_child()
        .height(50.0)
        .margin(-10, 0, 0, 0)
        .done();
    let result = builder.build();

    result.assert_child_position(0, 0, 0);
    result.assert_child_position(1, 0, 20);
}

#[test]
fn quirky_margin_smaller_than_normal() {
    let mut strut = MarginStrut::new();
    strut.append(lu(5), true); // quirky
    strut.append_normal(lu(20));
    assert_eq!(adjoining_margin_resolve(&strut), lu(20));
}

#[test]
fn quirky_container_start_ignores_quirky_margin() {
    let mut strut = MarginStrut::new();
    strut.is_quirky_container_start = true;
    strut.append(lu(30), true);
    strut.append_normal(lu(10));
    assert_eq!(adjoining_margin_resolve(&strut), lu(10));
}

#[test]
fn empty_strut_resolves_zero() {
    let strut = MarginStrut::new();
    assert_eq!(collapse_margins(&strut), lu(0));
    assert_eq!(adjoining_margin_resolve(&strut), lu(0));
}

#[test]
fn margin_strut_is_empty() {
    let strut = MarginStrut::new();
    assert!(strut.is_empty());

    let mut strut2 = MarginStrut::new();
    strut2.append_normal(lu(5));
    assert!(!strut2.is_empty());
}
