//! SP12 H6 — Comprehensive CSS Sizing Tests.
//!
//! Tests for width/height resolution, min/max constraints, intrinsic sizing,
//! CSS Sizing L3 keywords, replaced element sizing, aspect-ratio, and edge cases.

#[path = "sp12_wpt_helpers.rs"]
mod sp12_wpt_helpers;

use sp12_wpt_helpers::*;

use openui_geometry::{LayoutUnit, Length, MinMaxSizes, INDEFINITE_SIZE};
use openui_dom::{Document, ElementTag, NodeId};
use openui_layout::css_sizing::{
    SizingKeyword, apply_aspect_ratio, apply_aspect_ratio_with_auto,
    compute_automatic_size, compute_definite_size, resolve_preferred_size,
    resolve_sizing_keyword,
};
use openui_layout::intrinsic_sizing::{
    IntrinsicSizes, compute_intrinsic_block_sizes, compute_intrinsic_inline_sizes,
    compute_block_size_from_content, shrink_to_fit_inline_size,
    compute_replaced_intrinsic_sizes,
};
use openui_layout::size_constraints::{
    SizeConstraint, resolve_size_constraints,
    constrain_inline_size, constrain_block_size,
    resolve_inline_size, resolve_block_size,
    apply_box_sizing_adjustment,
};
use openui_layout::ConstraintSpace;
use openui_style::{AspectRatio, BoxSizing, ComputedStyle, Display, BorderStyle, Overflow, Float, Position};

fn luf(v: f32) -> LayoutUnit {
    LayoutUnit::from_f32(v)
}

// ═══════════════════════════════════════════════════════════════════════════
// §1  WIDTH RESOLUTION  (80+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// ── 1.1  width: auto = fill available ────────────────────────────────────

#[test]
fn w_auto_fills_container_800() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_auto().height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 800, 50);
}

#[test]
fn w_auto_fills_container_400() {
    let mut b = BlockTestBuilder::new(400, 300);
    b.add_child().width_auto().height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 30);
}

#[test]
fn w_auto_fills_container_1200() {
    let mut b = BlockTestBuilder::new(1200, 800);
    b.add_child().height(40.0).done();
    let r = b.build();
    r.assert_child_size(0, 1200, 40);
}

#[test]
fn w_auto_with_margins_subtracts() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_auto().height(50.0).margin(0, 50, 0, 50).done();
    let r = b.build();
    r.assert_child_size(0, 700, 50);
}

#[test]
fn w_auto_with_large_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_auto().height(50.0).margin(0, 200, 0, 200).done();
    let r = b.build();
    r.assert_child_size(0, 400, 50);
}

#[test]
fn w_auto_fills_narrow_container() {
    let mut b = BlockTestBuilder::new(100, 100);
    b.add_child().height(20.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 20);
}

#[test]
fn w_auto_multiple_children_all_fill() {
    let mut b = BlockTestBuilder::new(600, 400);
    b.add_child().height(30.0).done();
    b.add_child().height(40.0).done();
    b.add_child().height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 600, 30);
    r.assert_child_size(1, 600, 40);
    r.assert_child_size(2, 600, 50);
}

#[test]
fn w_auto_with_padding_reduces_content() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_auto().height(50.0).padding(0, 30, 0, 30).done();
    let r = b.build();
    // border-box width = 800, content = 800 - 60 = 740
    r.assert_child_size(0, 800, 50);
}

#[test]
fn w_auto_with_border_reduces_content() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_auto().height(50.0).border(0, 10, 0, 10).done();
    let r = b.build();
    r.assert_child_size(0, 800, 50);
}

// ── 1.2  width: fixed px ─────────────────────────────────────────────────

#[test]
fn w_200px() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

#[test]
fn w_0px() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(0.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 0, 50);
}

#[test]
fn w_1px() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(1.0).height(10.0).done();
    let r = b.build();
    r.assert_child_size(0, 1, 10);
}

#[test]
fn w_exact_container_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(800.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 800, 50);
}

#[test]
fn w_exceeds_container() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(1000.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 1000, 50);
}

#[test]
fn w_500px() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(500.0).height(100.0).done();
    let r = b.build();
    r.assert_child_size(0, 500, 100);
}

#[test]
fn w_multiple_fixed_widths() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(20.0).done();
    b.add_child().width(200.0).height(30.0).done();
    b.add_child().width(300.0).height(40.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 20);
    r.assert_child_size(1, 200, 30);
    r.assert_child_size(2, 300, 40);
}

// ── 1.3  width: percentage ───────────────────────────────────────────────

#[test]
fn w_50pct() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(50.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 50);
}

#[test]
fn w_100pct() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(100.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 800, 50);
}

#[test]
fn w_25pct() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(25.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

#[test]
fn w_10pct_of_1000() {
    let mut b = BlockTestBuilder::new(1000, 500);
    b.add_child().width_pct(10.0).height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 30);
}

#[test]
fn w_75pct() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(75.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 600, 50);
}

#[test]
fn w_33pct() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::percent(33.0), lu(900), lu(900), BoxSizing::ContentBox, lu(0), &c,
    );
    assert_eq!(result, LayoutUnit::from_f32(297.0));
}

// ── 1.4  width + padding + border = total ────────────────────────────────

#[test]
fn w_plus_padding_plus_border_content_box() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .padding(10, 20, 10, 20)
        .border(5, 5, 5, 5)
        .done();
    let r = b.build();
    // content-box: border-box width = 200 + 20+20 + 5+5 = 250
    r.assert_child_size(0, 250, 130);
}

#[test]
fn w_only_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(80.0).padding(15, 25, 15, 25).done();
    let r = b.build();
    // border-box = 300 + 25+25 = 350 wide, 80 + 15+15 = 110 tall
    r.assert_child_size(0, 350, 110);
}

#[test]
fn w_only_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(60.0).border(3, 3, 3, 3).done();
    let r = b.build();
    r.assert_child_size(0, 206, 66);
}

#[test]
fn w_large_padding_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).padding(20, 30, 20, 30).border(10, 10, 10, 10).done();
    let r = b.build();
    // 100 + 30+30 + 10+10 = 180 wide; 50 + 20+20 + 10+10 = 110 tall
    r.assert_child_size(0, 180, 110);
}

// ── 1.5  box-sizing: border-box ──────────────────────────────────────────

#[test]
fn w_border_box_200_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .padding(10, 20, 10, 20)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    // border-box: total = 200 (content shrinks to 200-40=160)
    r.assert_child_size(0, 200, 100);
}

#[test]
fn w_border_box_300_with_padding_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(300.0).height(150.0)
        .padding(10, 15, 10, 15)
        .border(5, 5, 5, 5)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    r.assert_child_size(0, 300, 150);
}

#[test]
fn w_border_box_vs_content_box_same_content() {
    // Two children with same content area: content-box 200px, border-box 240px with 20px padding
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).padding(0, 20, 0, 20).done();
    b.add_child().width(240.0).height(50.0).padding(0, 20, 0, 20).box_sizing_border_box().done();
    let r = b.build();
    // child 0: content-box → border-box = 200 + 40 = 240
    r.assert_child_size(0, 240, 50);
    // child 1: border-box → stays 240
    r.assert_child_size(1, 240, 50);
}

// ── 1.6  box-sizing: content-box (explicit) ──────────────────────────────

#[test]
fn w_content_box_explicit() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::px(300.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c,
    );
    assert_eq!(result, lu(300));
}

#[test]
fn w_content_box_with_pb() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::px(300.0), lu(800), lu(800), BoxSizing::ContentBox, lu(40), &c,
    );
    assert_eq!(result, lu(300));
}

#[test]
fn w_border_box_with_pb_subtracts() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::px(300.0), lu(800), lu(800), BoxSizing::BorderBox, lu(40), &c,
    );
    assert_eq!(result, lu(260));
}

// ── 1.7  width: auto via resolve_inline_size ─────────────────────────────

#[test]
fn resolve_inline_auto_fills_available_600() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(&Length::auto(), lu(600), lu(600), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(600));
}

#[test]
fn resolve_inline_auto_fills_available_1024() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(&Length::auto(), lu(1024), lu(1024), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(1024));
}

// ── 1.8  width positioning ───────────────────────────────────────────────

#[test]
fn w_fixed_at_position_0() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn w_fixed_with_left_margin() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).margin(0, 0, 0, 30).done();
    let r = b.build();
    r.assert_child_position(0, 30, 0);
}

#[test]
fn w_auto_centered_with_auto_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(50.0).margin_auto_horizontal().done();
    let r = b.build();
    r.assert_child_position(0, 200, 0);
}

#[test]
fn w_200_centered_in_1000() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width(200.0).height(50.0).margin_auto_horizontal().done();
    let r = b.build();
    r.assert_child_position(0, 400, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// §2  HEIGHT RESOLUTION  (80+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// ── 2.1  height: auto = content height ───────────────────────────────────

#[test]
fn h_auto_empty_is_zero() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height_auto().done();
    let r = b.build();
    r.assert_child_size(0, 200, 0);
}

#[test]
fn h_auto_from_child_50() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .add_child().width(100.0).height(50.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 800, 50);
}

#[test]
fn h_auto_from_multiple_children() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .add_child().width(100.0).height(30.0).done()
        .add_child().width(100.0).height(40.0).done()
        .add_child().width(100.0).height(20.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 800, 90);
}

#[test]
fn h_auto_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .padding(15, 0, 15, 0)
        .add_child().width(100.0).height(50.0).done()
        .done();
    let r = b.build();
    // height = child(50) + padding(15+15) = 80
    r.assert_child_size(0, 800, 80);
}

#[test]
fn h_auto_with_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .border(5, 0, 5, 0)
        .add_child().width(100.0).height(60.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 800, 70);
}

// ── 2.2  height: fixed px ────────────────────────────────────────────────

#[test]
fn h_200px() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(200.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 200);
}

#[test]
fn h_0px() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(0.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 0);
}

#[test]
fn h_1px() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(1.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 1);
}

#[test]
fn h_matches_container() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(600.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 600);
}

#[test]
fn h_exceeds_container() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(1000.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 1000);
}

// ── 2.3  height: percentage with definite parent ─────────────────────────

#[test]
fn h_50pct_definite_parent() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height_pct(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 300);
}

#[test]
fn h_100pct_definite_parent() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height_pct(100.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 600);
}

#[test]
fn h_25pct_definite_parent() {
    let mut b = BlockTestBuilder::new(800, 400);
    b.add_child().width(200.0).height_pct(25.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn h_10pct_of_1000() {
    let mut b = BlockTestBuilder::new(800, 1000);
    b.add_child().width(200.0).height_pct(10.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

// ── 2.4  height: pct with auto parent (treated as auto) ─────────────────

#[test]
fn h_pct_indefinite_cb_treated_as_auto() {
    let c = SizeConstraint::unconstrained();
    let indef = LayoutUnit::from_raw(-64);
    let result = resolve_block_size(
        &Length::percent(50.0), lu(100), indef, BoxSizing::ContentBox, lu(0), &c,
    );
    assert_eq!(result, lu(100));
}

#[test]
fn h_pct_indefinite_cb_fallback_200() {
    let c = SizeConstraint::unconstrained();
    let indef = LayoutUnit::from_raw(-64);
    let result = resolve_block_size(
        &Length::percent(75.0), lu(200), indef, BoxSizing::ContentBox, lu(0), &c,
    );
    assert_eq!(result, lu(200));
}

// ── 2.5  height: via resolve_block_size ──────────────────────────────────

#[test]
fn resolve_block_auto_returns_content() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(&Length::auto(), lu(130), lu(600), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(130));
}

#[test]
fn resolve_block_fixed_200() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(&Length::px(200.0), lu(50), lu(600), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(200));
}

#[test]
fn resolve_block_pct_50_of_600() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(&Length::percent(50.0), lu(50), lu(600), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(300));
}

#[test]
fn resolve_block_border_box_subtracts_pb() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(&Length::px(300.0), lu(50), lu(600), BoxSizing::BorderBox, lu(40), &c);
    assert_eq!(result, lu(260));
}

#[test]
fn resolve_block_border_box_auto_subtraction() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(&Length::auto(), lu(150), lu(600), BoxSizing::BorderBox, lu(40), &c);
    assert_eq!(result, lu(110));
}

// ── 2.6  height stacking ─────────────────────────────────────────────────

#[test]
fn h_two_children_stacked() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).done();
    b.add_child().width(200.0).height(150.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 100);
}

#[test]
fn h_three_children_stacked() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).done();
    b.add_child().width(200.0).height(60.0).done();
    b.add_child().width(200.0).height(70.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 50);
    r.assert_child_position(2, 0, 110);
}

#[test]
fn container_height_from_children() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).done();
    b.add_child().width(200.0).height(50.0).done();
    let r = b.build();
    assert_layout!(r, container height 600);
}

#[test]
fn h_children_with_margins_stacking() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).margin(0, 0, 20, 0).done();
    b.add_child().width(200.0).height(60.0).margin(10, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // Margin collapse: max(20, 10) = 20 from top of first child's bottom
    r.assert_child_position(1, 0, 70);
}

// ── 2.7  height with box model ───────────────────────────────────────────

#[test]
fn h_with_padding_content_box() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).padding(20, 0, 20, 0).done();
    let r = b.build();
    // content-box: border-box height = 100 + 20 + 20 = 140
    r.assert_child_size(0, 200, 140);
}

#[test]
fn h_with_border_box() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(200.0).padding(20, 0, 20, 0).box_sizing_border_box().done();
    let r = b.build();
    r.assert_child_size(0, 200, 200);
}

#[test]
fn h_border_box_with_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(200.0).border(10, 0, 10, 0).box_sizing_border_box().done();
    let r = b.build();
    r.assert_child_size(0, 200, 200);
}


// ═══════════════════════════════════════════════════════════════════════════
// §3  MIN/MAX WIDTH  (80+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// ── 3.1  min-width clamps up ─────────────────────────────────────────────

#[test]
fn min_w_clamps_up_fixed() {
    let c = SizeConstraint {
        min_inline_size: lu(200),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(100), &c), lu(200));
}

#[test]
fn min_w_no_effect_when_larger() {
    let c = SizeConstraint {
        min_inline_size: lu(100),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(300), &c), lu(300));
}

#[test]
fn min_w_equals_size() {
    let c = SizeConstraint {
        min_inline_size: lu(200),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(200), &c), lu(200));
}

#[test]
fn min_w_layout_clamps_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).min_width(200.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

#[test]
fn min_w_with_auto_width_no_effect() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().min_width(200.0).height(50.0).done();
    let r = b.build();
    // auto width = 800, min=200 → still 800
    r.assert_child_size(0, 800, 50);
}

// ── 3.2  max-width clamps down ───────────────────────────────────────────

#[test]
fn max_w_clamps_down_fixed() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: lu(300),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(500), &c), lu(300));
}

#[test]
fn max_w_no_effect_when_smaller() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: lu(500),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(300), &c), lu(300));
}

#[test]
fn max_w_equals_size() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: lu(300),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(300), &c), lu(300));
}

#[test]
fn max_w_layout_clamps_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(500.0).max_width(300.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 300, 50);
}

#[test]
fn max_w_on_auto_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().max_width(300.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 300, 50);
}

// ── 3.3  min-width > max-width (min wins) ────────────────────────────────

#[test]
fn min_w_gt_max_w_normalized() {
    let c = resolve_size_constraints(
        &Length::px(400.0), &Length::px(200.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, lu(400));
    assert_eq!(c.max_inline_size, lu(400));
}

#[test]
fn min_w_gt_max_w_all_clamp_to_min() {
    let c = resolve_size_constraints(
        &Length::px(300.0), &Length::px(100.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(constrain_inline_size(lu(50), &c), lu(300));
    assert_eq!(constrain_inline_size(lu(200), &c), lu(300));
    assert_eq!(constrain_inline_size(lu(500), &c), lu(300));
}

#[test]
fn min_w_gt_max_w_layout() {
    let mut b = BlockTestBuilder::new(800, 600);
    // In layout, max-width=200 clamps first, then min can't override
    b.add_child().width(250.0).min_width(400.0).max_width(200.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

// ── 3.4  min-width: 0 (default) ─────────────────────────────────────────

#[test]
fn min_w_zero_default() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, LayoutUnit::zero());
}

#[test]
fn min_w_explicit_zero() {
    let c = resolve_size_constraints(
        &Length::px(0.0), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, LayoutUnit::zero());
    assert_eq!(constrain_inline_size(LayoutUnit::zero(), &c), LayoutUnit::zero());
}

// ── 3.5  max-width: none (default, no constraint) ───────────────────────

#[test]
fn max_w_none_default() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.max_inline_size, LayoutUnit::max());
}

#[test]
fn max_w_none_no_clamp() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(constrain_inline_size(lu(99999), &c), lu(99999));
}

// ── 3.6  Percentage min/max-width ────────────────────────────────────────

#[test]
fn min_w_percentage_20_of_500() {
    let c = resolve_size_constraints(
        &Length::percent(20.0), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(500), lu(400), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, lu(100));
}

#[test]
fn max_w_percentage_50_of_800() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::percent(50.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.max_inline_size, lu(400));
}

#[test]
fn min_w_percentage_against_indefinite_is_zero() {
    let indef = LayoutUnit::from_raw(-64);
    let c = resolve_size_constraints(
        &Length::percent(50.0), &Length::none(),
        &Length::auto(), &Length::none(),
        indef, lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, LayoutUnit::zero());
}

// ── 3.7  min-width on auto-width element ─────────────────────────────────

#[test]
fn min_w_on_auto_width_element() {
    let c = resolve_size_constraints(
        &Length::px(500.0), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(400), lu(300), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_inline_size(&Length::auto(), lu(400), lu(400), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(500));
}

#[test]
fn min_w_300_on_auto_width_in_200_container() {
    let c = resolve_size_constraints(
        &Length::px(300.0), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(200), lu(300), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_inline_size(&Length::auto(), lu(200), lu(200), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(300));
}

// ── 3.8  max-width on percentage-width element ───────────────────────────

#[test]
fn max_w_on_pct_width() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::px(300.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_inline_size(&Length::percent(50.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c);
    // 50% of 800 = 400, clamped to max 300
    assert_eq!(result, lu(300));
}

#[test]
fn max_w_on_pct_width_no_clamp() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::px(500.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_inline_size(&Length::percent(50.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c);
    // 50% of 800 = 400, max=500 → 400
    assert_eq!(result, lu(400));
}

// ── 3.9  Combined min+max width chain ────────────────────────────────────

#[test]
fn combined_min_max_width_clamp_to_max() {
    let c = resolve_size_constraints(
        &Length::px(100.0), &Length::px(400.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_inline_size(&Length::auto(), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(400));
}

#[test]
fn combined_min_max_width_clamp_to_min() {
    let c = resolve_size_constraints(
        &Length::px(200.0), &Length::px(400.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_inline_size(&Length::px(100.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(200));
}

#[test]
fn combined_min_max_width_within_bounds() {
    let c = resolve_size_constraints(
        &Length::px(100.0), &Length::px(400.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_inline_size(&Length::px(250.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(250));
}

// ── 3.10  border-box with min/max ────────────────────────────────────────

#[test]
fn min_w_border_box_subtracts_pb() {
    let c = resolve_size_constraints(
        &Length::px(200.0), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::BorderBox, lu(40), lu(0),
    );
    assert_eq!(c.min_inline_size, lu(160));
}

#[test]
fn max_w_border_box_subtracts_pb() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::px(500.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::BorderBox, lu(60), lu(0),
    );
    assert_eq!(c.max_inline_size, lu(440));
}

#[test]
fn min_w_border_box_small_clamps_to_zero() {
    let c = resolve_size_constraints(
        &Length::px(10.0), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::BorderBox, lu(50), lu(0),
    );
    assert_eq!(c.min_inline_size, LayoutUnit::zero());
}

// ═══════════════════════════════════════════════════════════════════════════
// §4  MIN/MAX HEIGHT  (80+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// ── 4.1  min-height clamps up ────────────────────────────────────────────

#[test]
fn min_h_clamps_up() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(200),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_block_size(lu(100), &c), lu(200));
}

#[test]
fn min_h_no_effect_when_taller() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(100),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_block_size(lu(300), &c), lu(300));
}

#[test]
fn min_h_layout_clamps() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).min_height(150.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 150);
}

#[test]
fn min_h_with_auto_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height_auto().min_height(100.0).done();
    let r = b.build();
    // auto height (empty=0) clamped to min 100
    r.assert_child_size(0, 200, 100);
}

#[test]
fn min_h_zero_default() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_block_size, LayoutUnit::zero());
}

// ── 4.2  max-height clamps down ──────────────────────────────────────────

#[test]
fn max_h_clamps_down() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(0),
        max_block_size: lu(200),
    };
    assert_eq!(constrain_block_size(lu(400), &c), lu(200));
}

#[test]
fn max_h_no_effect_when_shorter() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(0),
        max_block_size: lu(500),
    };
    assert_eq!(constrain_block_size(lu(300), &c), lu(300));
}

#[test]
fn max_h_layout_clamps() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(400.0).max_height(250.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 250);
}

#[test]
fn max_h_on_auto_height_from_content() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().max_height(60.0)
        .add_child().width(100.0).height(100.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 800, 60);
}

#[test]
fn max_h_none_default() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.max_block_size, LayoutUnit::max());
}

#[test]
fn max_h_none_no_clamp() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(constrain_block_size(lu(99999), &c), lu(99999));
}

// ── 4.3  min-height > max-height (min wins) ─────────────────────────────

#[test]
fn min_h_gt_max_h_normalized() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::px(400.0), &Length::px(200.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_block_size, lu(400));
    assert_eq!(c.max_block_size, lu(400));
}

#[test]
fn min_h_gt_max_h_all_clamp() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::px(300.0), &Length::px(100.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(constrain_block_size(lu(50), &c), lu(300));
    assert_eq!(constrain_block_size(lu(200), &c), lu(300));
    assert_eq!(constrain_block_size(lu(500), &c), lu(300));
}

#[test]
fn min_h_gt_max_h_layout() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(250.0).min_height(400.0).max_height(200.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 200);
}

// ── 4.4  Percentage min/max height ───────────────────────────────────────

#[test]
fn min_h_pct_25_of_400() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::percent(25.0), &Length::none(),
        lu(800), lu(400), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_block_size, lu(100));
}

#[test]
fn max_h_pct_50_of_600() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::auto(), &Length::percent(50.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.max_block_size, lu(300));
}

#[test]
fn min_h_pct_indefinite_is_zero() {
    let indef = LayoutUnit::from_raw(-64);
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::percent(50.0), &Length::none(),
        lu(800), indef, BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_block_size, LayoutUnit::zero());
}

// ── 4.5  min-height with auto height ─────────────────────────────────────

#[test]
fn min_h_auto_height_empty_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).min_height(100.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn min_h_auto_height_small_content() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().min_height(100.0)
        .add_child().width(50.0).height(30.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 800, 100);
}

#[test]
fn min_h_auto_height_large_content() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().min_height(50.0)
        .add_child().width(50.0).height(200.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 800, 200);
}

// ── 4.6  max-height causing overflow ─────────────────────────────────────

#[test]
fn max_h_overflow_hidden() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .max_height(50.0).overflow_hidden()
        .add_child().width(100.0).height(200.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 800, 50);
}

#[test]
fn max_h_constrains_percentage_height() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::auto(), &Length::px(100.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_block_size(&Length::percent(50.0), lu(50), lu(600), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(100));
}

// ── 4.7  border-box height constraints ───────────────────────────────────

#[test]
fn min_h_border_box_subtracts_pb() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::px(200.0), &Length::none(),
        lu(800), lu(600), BoxSizing::BorderBox, lu(0), lu(40),
    );
    assert_eq!(c.min_block_size, lu(160));
}

#[test]
fn max_h_border_box_subtracts_pb() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::auto(), &Length::px(400.0),
        lu(800), lu(600), BoxSizing::BorderBox, lu(0), lu(60),
    );
    assert_eq!(c.max_block_size, lu(340));
}

#[test]
fn min_h_border_box_small_clamps_zero() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::px(10.0), &Length::none(),
        lu(800), lu(600), BoxSizing::BorderBox, lu(0), lu(50),
    );
    assert_eq!(c.min_block_size, LayoutUnit::zero());
}

// ── 4.8  Additional height constraint tests ──────────────────────────────

#[test]
fn combined_min_max_height_within() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::px(50.0), &Length::px(300.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(constrain_block_size(lu(150), &c), lu(150));
}

#[test]
fn combined_min_max_height_clamp_up() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::px(100.0), &Length::px(300.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(constrain_block_size(lu(50), &c), lu(100));
}

#[test]
fn combined_min_max_height_clamp_down() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::px(50.0), &Length::px(200.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(constrain_block_size(lu(400), &c), lu(200));
}


// ═══════════════════════════════════════════════════════════════════════════
// §5  INTRINSIC SIZING  (80+ tests)
// ═══════════════════════════════════════════════════════════════════════════

fn doc_with_one_child(child_w: f32, child_h: f32) -> (Document, NodeId) {
    let mut doc = Document::new();
    let root = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(child_w);
    doc.node_mut(child).style.height = Length::px(child_h);
    doc.append_child(parent, child);
    (doc, parent)
}

fn doc_with_children(sizes: &[(f32, f32)]) -> (Document, NodeId) {
    let mut doc = Document::new();
    let root = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);
    for &(w, h) in sizes {
        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.width = Length::px(w);
        doc.node_mut(child).style.height = Length::px(h);
        doc.append_child(parent, child);
    }
    (doc, parent)
}

// ── 5.1  min-content width ───────────────────────────────────────────────

#[test]
fn intrinsic_min_content_single_child_150() {
    let (doc, parent) = doc_with_one_child(150.0, 50.0);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.min_content_inline_size, luf(150.0));
}

#[test]
fn intrinsic_min_content_multiple_takes_max() {
    let (doc, parent) = doc_with_children(&[(100.0, 30.0), (200.0, 40.0), (150.0, 20.0)]);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.min_content_inline_size, luf(200.0));
}

#[test]
fn intrinsic_min_content_empty_is_zero() {
    let mut doc = Document::new();
    let root = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.min_content_inline_size, lu(0));
}

#[test]
fn intrinsic_min_content_single_child_1px() {
    let (doc, parent) = doc_with_one_child(1.0, 1.0);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.min_content_inline_size, luf(1.0));
}

#[test]
fn intrinsic_min_content_two_same_width() {
    let (doc, parent) = doc_with_children(&[(100.0, 20.0), (100.0, 30.0)]);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.min_content_inline_size, luf(100.0));
}

// ── 5.2  max-content width ───────────────────────────────────────────────

#[test]
fn intrinsic_max_content_single_250() {
    let (doc, parent) = doc_with_one_child(250.0, 80.0);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.max_content_inline_size, luf(250.0));
}

#[test]
fn intrinsic_max_content_multiple() {
    let (doc, parent) = doc_with_children(&[(100.0, 10.0), (300.0, 20.0), (200.0, 15.0)]);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.max_content_inline_size, luf(300.0));
}

#[test]
fn intrinsic_max_content_zero_child() {
    let (doc, parent) = doc_with_one_child(0.0, 50.0);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.max_content_inline_size, luf(0.0));
}

// ── 5.3  fit-content width (shrink-to-fit) ───────────────────────────────

#[test]
fn shrink_to_fit_available_gt_max() {
    assert_eq!(shrink_to_fit_inline_size(lu(50), lu(200), lu(300)), lu(200));
}

#[test]
fn shrink_to_fit_available_between() {
    assert_eq!(shrink_to_fit_inline_size(lu(80), lu(400), lu(250)), lu(250));
}

#[test]
fn shrink_to_fit_available_lt_min() {
    assert_eq!(shrink_to_fit_inline_size(lu(120), lu(300), lu(60)), lu(120));
}

#[test]
fn shrink_to_fit_all_equal() {
    assert_eq!(shrink_to_fit_inline_size(lu(100), lu(100), lu(100)), lu(100));
}

#[test]
fn shrink_to_fit_zero_min() {
    assert_eq!(shrink_to_fit_inline_size(lu(0), lu(200), lu(150)), lu(150));
}

#[test]
fn shrink_to_fit_zero_available() {
    assert_eq!(shrink_to_fit_inline_size(lu(50), lu(200), lu(0)), lu(50));
}

#[test]
fn shrink_to_fit_min_equals_max() {
    assert_eq!(shrink_to_fit_inline_size(lu(150), lu(150), lu(200)), lu(150));
}

#[test]
fn shrink_to_fit_large_values() {
    assert_eq!(shrink_to_fit_inline_size(lu(1000), lu(5000), lu(3000)), lu(3000));
}

// ── 5.4  Shrink-to-fit for floats ────────────────────────────────────────

#[test]
fn float_shrink_to_fit_auto_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().float_left().width(150.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 150, 50);
}

#[test]
fn float_shrink_to_fit_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().float_left().width(100.0).height(50.0).padding(10, 10, 10, 10).done();
    let r = b.build();
    r.assert_child_size(0, 120, 70);
}

// ── 5.5  Intrinsic width with children ───────────────────────────────────

#[test]
fn intrinsic_block_size_sum_of_heights() {
    let (doc, parent) = doc_with_children(&[(100.0, 30.0), (200.0, 40.0), (150.0, 20.0)]);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(
        sizes.max_content_block_size,
        luf(30.0) + luf(40.0) + luf(20.0)
    );
}

#[test]
fn intrinsic_block_size_single() {
    let (doc, parent) = doc_with_one_child(100.0, 60.0);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.max_content_block_size, luf(60.0));
}

// ── 5.6  Intrinsic width with padding/border ─────────────────────────────

#[test]
fn intrinsic_with_parent_padding() {
    let mut doc = Document::new();
    let root = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.padding_top = Length::px(10.0);
    doc.node_mut(parent).style.padding_right = Length::px(10.0);
    doc.node_mut(parent).style.padding_bottom = Length::px(10.0);
    doc.node_mut(parent).style.padding_left = Length::px(10.0);
    doc.append_child(root, parent);
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(100.0);
    doc.node_mut(child).style.height = Length::px(40.0);
    doc.append_child(parent, child);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    // 100 + padding(10+10) = 120
    assert_eq!(sizes.max_content_inline_size, luf(100.0) + luf(20.0));
}

#[test]
fn intrinsic_with_parent_border() {
    let mut doc = Document::new();
    let root = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.border_top_width = 5;
    doc.node_mut(parent).style.border_right_width = 5;
    doc.node_mut(parent).style.border_bottom_width = 5;
    doc.node_mut(parent).style.border_left_width = 5;
    doc.node_mut(parent).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_left_style = BorderStyle::Solid;
    doc.append_child(root, parent);
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(80.0);
    doc.node_mut(child).style.height = Length::px(30.0);
    doc.append_child(parent, child);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.max_content_inline_size, luf(80.0) + LayoutUnit::from_i32(10));
}

// ── 5.7  Nested block intrinsic sizing ───────────────────────────────────

#[test]
fn intrinsic_nested_propagation() {
    let mut doc = Document::new();
    let root = doc.root();
    let grandparent = doc.create_node(ElementTag::Div);
    doc.node_mut(grandparent).style.display = Display::Block;
    doc.append_child(root, grandparent);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(grandparent, parent);
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(200.0);
    doc.node_mut(child).style.height = Length::px(60.0);
    doc.append_child(parent, child);
    let sizes = compute_intrinsic_block_sizes(&doc, grandparent);
    assert_eq!(sizes.min_content_inline_size, luf(200.0));
    assert_eq!(sizes.max_content_inline_size, luf(200.0));
    assert_eq!(sizes.max_content_block_size, luf(60.0));
}

// ── 5.8  Display:none skipped ────────────────────────────────────────────

#[test]
fn intrinsic_display_none_skipped() {
    let mut doc = Document::new();
    let root = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);
    let visible = doc.create_node(ElementTag::Div);
    doc.node_mut(visible).style.display = Display::Block;
    doc.node_mut(visible).style.width = Length::px(100.0);
    doc.node_mut(visible).style.height = Length::px(50.0);
    doc.append_child(parent, visible);
    let hidden = doc.create_node(ElementTag::Div);
    doc.node_mut(hidden).style.display = Display::None;
    doc.node_mut(hidden).style.width = Length::px(300.0);
    doc.node_mut(hidden).style.height = Length::px(200.0);
    doc.append_child(parent, hidden);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.min_content_inline_size, luf(100.0));
    assert_eq!(sizes.max_content_inline_size, luf(100.0));
}

// ── 5.9  Block size from content ─────────────────────────────────────────

#[test]
fn block_size_from_content_simple_sum() {
    let mut doc = Document::new();
    let root = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);
    let child_boxes = [lu(50), lu(30), lu(40)];
    let result = compute_block_size_from_content(&doc, parent, &child_boxes);
    assert_eq!(result, lu(120));
}

#[test]
fn block_size_from_content_single() {
    let mut doc = Document::new();
    let root = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);
    let child_boxes = [lu(75)];
    let result = compute_block_size_from_content(&doc, parent, &child_boxes);
    assert_eq!(result, lu(75));
}

#[test]
fn block_size_from_content_empty() {
    let mut doc = Document::new();
    let root = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);
    let child_boxes: [LayoutUnit; 0] = [];
    let result = compute_block_size_from_content(&doc, parent, &child_boxes);
    assert_eq!(result, lu(0));
}

#[test]
fn block_size_content_clamped_by_min_h() {
    let mut doc = Document::new();
    let root = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.min_height = Length::px(200.0);
    doc.append_child(root, parent);
    let child_boxes = [lu(50)];
    let result = compute_block_size_from_content(&doc, parent, &child_boxes);
    assert_eq!(result, lu(200));
}

#[test]
fn block_size_content_clamped_by_max_h() {
    let mut doc = Document::new();
    let root = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.max_height = Length::px(80.0);
    doc.append_child(root, parent);
    let child_boxes = [lu(60), lu(90)];
    let result = compute_block_size_from_content(&doc, parent, &child_boxes);
    assert_eq!(result, lu(80));
}

// ── 5.10  Intrinsic text sizing ──────────────────────────────────────────

#[test]
fn intrinsic_text_hello() {
    let mut doc = Document::new();
    let root = doc.root();
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("hello world".to_string());
    doc.append_child(root, text);
    let sizes = compute_intrinsic_inline_sizes(&doc, text);
    // widest word = 5 chars * 8px = 40
    assert_eq!(sizes.min, luf(40.0));
    // full text = 11 chars * 8px = 88
    assert_eq!(sizes.max, luf(88.0));
}

#[test]
fn intrinsic_text_single_word() {
    let mut doc = Document::new();
    let root = doc.root();
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("test".to_string());
    doc.append_child(root, text);
    let sizes = compute_intrinsic_inline_sizes(&doc, text);
    assert_eq!(sizes.min, luf(32.0)); // 4 * 8
    assert_eq!(sizes.max, luf(32.0)); // same, single word
}

// ── 5.11  Child with min-width in intrinsic ──────────────────────────────

#[test]
fn intrinsic_child_min_width_constraint() {
    let mut doc = Document::new();
    let root = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(80.0);
    doc.node_mut(child).style.min_width = Length::px(120.0);
    doc.node_mut(child).style.height = Length::px(30.0);
    doc.append_child(parent, child);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.min_content_inline_size, luf(120.0));
    assert_eq!(sizes.max_content_inline_size, luf(120.0));
}


// ═══════════════════════════════════════════════════════════════════════════
// §6  CSS SIZING L3 KEYWORDS  (80+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// ── 6.1  width: min-content ──────────────────────────────────────────────

#[test]
fn sizing_keyword_min_content_resolves() {
    let intrinsic = MinMaxSizes::new(lu(60), lu(300));
    let result = resolve_sizing_keyword(SizingKeyword::MinContent, &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(60));
}

#[test]
fn sizing_keyword_min_content_with_margins() {
    let intrinsic = MinMaxSizes::new(lu(100), lu(400));
    let result = resolve_sizing_keyword(SizingKeyword::MinContent, &intrinsic, lu(800), lu(50));
    assert_eq!(result, lu(100));
}

#[test]
fn sizing_keyword_min_content_zero_intrinsic() {
    let intrinsic = MinMaxSizes::new(lu(0), lu(0));
    let result = resolve_sizing_keyword(SizingKeyword::MinContent, &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(0));
}

// ── 6.2  width: max-content ──────────────────────────────────────────────

#[test]
fn sizing_keyword_max_content_resolves() {
    let intrinsic = MinMaxSizes::new(lu(60), lu(300));
    let result = resolve_sizing_keyword(SizingKeyword::MaxContent, &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(300));
}

#[test]
fn sizing_keyword_max_content_large() {
    let intrinsic = MinMaxSizes::new(lu(200), lu(1500));
    let result = resolve_sizing_keyword(SizingKeyword::MaxContent, &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(1500));
}

#[test]
fn sizing_keyword_max_content_zero() {
    let intrinsic = MinMaxSizes::new(lu(0), lu(0));
    let result = resolve_sizing_keyword(SizingKeyword::MaxContent, &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(0));
}

// ── 6.3  width: fit-content ──────────────────────────────────────────────

#[test]
fn sizing_keyword_fit_content_clamps_below_min() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(200));
    let result = resolve_sizing_keyword(SizingKeyword::FitContent(lu(20)), &intrinsic, lu(500), lu(0));
    assert_eq!(result, lu(50));
}

#[test]
fn sizing_keyword_fit_content_clamps_above_max() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(200));
    let result = resolve_sizing_keyword(SizingKeyword::FitContent(lu(500)), &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(200));
}

#[test]
fn sizing_keyword_fit_content_within_range() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(200));
    let result = resolve_sizing_keyword(SizingKeyword::FitContent(lu(150)), &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(150));
}

#[test]
fn sizing_keyword_fit_content_exactly_min() {
    let intrinsic = MinMaxSizes::new(lu(100), lu(300));
    let result = resolve_sizing_keyword(SizingKeyword::FitContent(lu(100)), &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(100));
}

#[test]
fn sizing_keyword_fit_content_exactly_max() {
    let intrinsic = MinMaxSizes::new(lu(100), lu(300));
    let result = resolve_sizing_keyword(SizingKeyword::FitContent(lu(300)), &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(300));
}

// ── 6.4  width: stretch (fill-available) ─────────────────────────────────

#[test]
fn sizing_keyword_stretch_fills_available() {
    let intrinsic = MinMaxSizes::new(lu(0), lu(0));
    let result = resolve_sizing_keyword(SizingKeyword::Stretch, &intrinsic, lu(600), lu(0));
    assert_eq!(result, lu(600));
}

#[test]
fn sizing_keyword_stretch_with_margins() {
    let intrinsic = MinMaxSizes::new(lu(0), lu(0));
    let result = resolve_sizing_keyword(SizingKeyword::Stretch, &intrinsic, lu(600), lu(50));
    assert_eq!(result, lu(550));
}

#[test]
fn sizing_keyword_stretch_narrow_container() {
    let intrinsic = MinMaxSizes::new(lu(0), lu(0));
    let result = resolve_sizing_keyword(SizingKeyword::Stretch, &intrinsic, lu(100), lu(0));
    assert_eq!(result, lu(100));
}

#[test]
fn sizing_keyword_stretch_large_margins() {
    let intrinsic = MinMaxSizes::new(lu(0), lu(0));
    let result = resolve_sizing_keyword(SizingKeyword::Stretch, &intrinsic, lu(800), lu(200));
    assert_eq!(result, lu(600));
}

// ── 6.5  SizingKeyword enum ──────────────────────────────────────────────

#[test]
fn sizing_keyword_enum_auto_distinct() {
    let auto = SizingKeyword::Auto;
    let min_c = SizingKeyword::MinContent;
    assert_ne!(format!("{:?}", auto), format!("{:?}", min_c));
}

#[test]
fn sizing_keyword_enum_fit_content_carries_value() {
    let fit = SizingKeyword::FitContent(lu(100));
    if let SizingKeyword::FitContent(v) = fit {
        assert_eq!(v, lu(100));
    } else {
        panic!("Expected FitContent");
    }
}

#[test]
fn sizing_keyword_enum_all_distinct() {
    let variants = [
        format!("{:?}", SizingKeyword::Auto),
        format!("{:?}", SizingKeyword::MinContent),
        format!("{:?}", SizingKeyword::MaxContent),
        format!("{:?}", SizingKeyword::FitContent(lu(0))),
        format!("{:?}", SizingKeyword::Stretch),
    ];
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(variants[i], variants[j]);
        }
    }
}

// ── 6.6  aspect-ratio: 16/9 ─────────────────────────────────────────────

#[test]
fn aspect_ratio_16_9_width_to_height() {
    let (w, h) = apply_aspect_ratio(lu(320), INDEFINITE_SIZE, (16.0, 9.0));
    assert_eq!(w, lu(320));
    assert_eq!(h, lu(180));
}

#[test]
fn aspect_ratio_16_9_height_to_width() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(180), (16.0, 9.0));
    assert_eq!(w, lu(320));
    assert_eq!(h, lu(180));
}

#[test]
fn aspect_ratio_16_9_width_640() {
    let (w, h) = apply_aspect_ratio(lu(640), INDEFINITE_SIZE, (16.0, 9.0));
    assert_eq!(w, lu(640));
    assert_eq!(h, lu(360));
}

#[test]
fn aspect_ratio_16_9_height_360() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(360), (16.0, 9.0));
    assert_eq!(w, lu(640));
    assert_eq!(h, lu(360));
}

// ── 6.7  aspect-ratio: auto ─────────────────────────────────────────────

#[test]
fn aspect_ratio_auto_flag_prefers_intrinsic() {
    let ar = AspectRatio { ratio: (16.0, 9.0), auto_flag: true };
    let intrinsic = Some((4.0, 3.0));
    let (w, h) = apply_aspect_ratio_with_auto(INDEFINITE_SIZE, lu(300), &ar, intrinsic);
    assert_eq!(w, lu(400));
    assert_eq!(h, lu(300));
}

#[test]
fn aspect_ratio_auto_flag_no_intrinsic_uses_specified() {
    let ar = AspectRatio { ratio: (16.0, 9.0), auto_flag: true };
    let (w, h) = apply_aspect_ratio_with_auto(INDEFINITE_SIZE, lu(180), &ar, None);
    assert_eq!(w, lu(320));
    assert_eq!(h, lu(180));
}

#[test]
fn aspect_ratio_no_auto_flag_uses_specified() {
    let ar = AspectRatio { ratio: (16.0, 9.0), auto_flag: false };
    let intrinsic = Some((4.0, 3.0));
    let (w, h) = apply_aspect_ratio_with_auto(INDEFINITE_SIZE, lu(180), &ar, intrinsic);
    assert_eq!(w, lu(320));
    assert_eq!(h, lu(180));
}

// ── 6.8  aspect-ratio with fixed width → computed height ─────────────────

#[test]
fn aspect_ratio_2_1_width_200() {
    let (w, h) = apply_aspect_ratio(lu(200), INDEFINITE_SIZE, (2.0, 1.0));
    assert_eq!(w, lu(200));
    assert_eq!(h, lu(100));
}

#[test]
fn aspect_ratio_4_3_width_400() {
    let (w, h) = apply_aspect_ratio(lu(400), INDEFINITE_SIZE, (4.0, 3.0));
    assert_eq!(w, lu(400));
    assert_eq!(h, lu(300));
}

#[test]
fn aspect_ratio_1_1_square() {
    let (w, h) = apply_aspect_ratio(lu(200), INDEFINITE_SIZE, (1.0, 1.0));
    assert_eq!(w, lu(200));
    assert_eq!(h, lu(200));
}

#[test]
fn aspect_ratio_3_2_width_300() {
    let (w, h) = apply_aspect_ratio(lu(300), INDEFINITE_SIZE, (3.0, 2.0));
    assert_eq!(w, lu(300));
    assert_eq!(h, lu(200));
}

// ── 6.9  aspect-ratio with fixed height → computed width ─────────────────

#[test]
fn aspect_ratio_2_1_height_100() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(100), (2.0, 1.0));
    assert_eq!(w, lu(200));
    assert_eq!(h, lu(100));
}

#[test]
fn aspect_ratio_4_3_height_300() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(300), (4.0, 3.0));
    assert_eq!(w, lu(400));
    assert_eq!(h, lu(300));
}

#[test]
fn aspect_ratio_1_1_height_150() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(150), (1.0, 1.0));
    assert_eq!(w, lu(150));
    assert_eq!(h, lu(150));
}

// ── 6.10  aspect-ratio + min/max constraints ─────────────────────────────

#[test]
fn aspect_ratio_plus_min_width() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(400));
    let ar = AspectRatio { ratio: (2.0, 1.0), auto_flag: false };
    let result = resolve_preferred_size(
        &Length::auto(), &Length::px(100.0), &Length::none(),
        lu(800), &intrinsic, lu(800), lu(0), lu(30), Some(&ar), true,
    );
    // width from AR: 30*2 = 60, clamped to min 100
    assert_eq!(result, lu(100));
}

#[test]
fn aspect_ratio_plus_max_width() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(400));
    let ar = AspectRatio { ratio: (2.0, 1.0), auto_flag: false };
    let result = resolve_preferred_size(
        &Length::auto(), &Length::auto(), &Length::px(150.0),
        lu(800), &intrinsic, lu(800), lu(0), lu(200), Some(&ar), true,
    );
    // width from AR: 200*2 = 400, clamped to max 150
    assert_eq!(result, lu(150));
}

#[test]
fn aspect_ratio_both_definite_ignored() {
    let (w, h) = apply_aspect_ratio(lu(200), lu(100), (16.0, 9.0));
    assert_eq!(w, lu(200));
    assert_eq!(h, lu(100));
}

#[test]
fn aspect_ratio_both_indefinite_no_resolution() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, INDEFINITE_SIZE, (16.0, 9.0));
    assert_eq!(w, INDEFINITE_SIZE);
    assert_eq!(h, INDEFINITE_SIZE);
}

#[test]
fn zero_aspect_ratio_width_zero() {
    let (w, h) = apply_aspect_ratio(lu(200), INDEFINITE_SIZE, (0.0, 9.0));
    assert_eq!(w, lu(200));
    assert_eq!(h, INDEFINITE_SIZE);
}

#[test]
fn zero_aspect_ratio_height_zero() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(100), (16.0, 0.0));
    assert_eq!(w, INDEFINITE_SIZE);
    assert_eq!(h, lu(100));
}

// ── 6.11  Height keywords ────────────────────────────────────────────────

#[test]
fn automatic_block_size_fit_content_uses_max() {
    let intrinsic = MinMaxSizes::new(lu(100), lu(300));
    let result = compute_automatic_size(false, &intrinsic, lu(600), lu(0));
    assert_eq!(result, lu(300));
}

#[test]
fn automatic_block_size_clamped_to_available() {
    let intrinsic = MinMaxSizes::new(lu(100), lu(500));
    let result = compute_automatic_size(false, &intrinsic, lu(200), lu(0));
    assert_eq!(result, lu(200));
}

#[test]
fn automatic_inline_size_stretch() {
    let intrinsic = MinMaxSizes::new(lu(100), lu(300));
    let result = compute_automatic_size(true, &intrinsic, lu(600), lu(40));
    assert_eq!(result, lu(560));
}

#[test]
fn automatic_inline_size_stretch_no_margins() {
    let intrinsic = MinMaxSizes::new(lu(100), lu(300));
    let result = compute_automatic_size(true, &intrinsic, lu(600), lu(0));
    assert_eq!(result, lu(600));
}

// ── 6.12  Definite size detection ────────────────────────────────────────

#[test]
fn definite_size_px_200() {
    let space = ConstraintSpace::for_root(lu(800), lu(600));
    let result = compute_definite_size(&Length::px(200.0), lu(800), &space, true);
    assert_eq!(result, Some(lu(200)));
}

#[test]
fn definite_size_pct_50() {
    let space = ConstraintSpace::for_root(lu(800), lu(600));
    let result = compute_definite_size(&Length::percent(50.0), lu(800), &space, true);
    assert_eq!(result, Some(lu(400)));
}

#[test]
fn definite_size_auto_without_fixed() {
    let space = ConstraintSpace::for_root(lu(500), lu(400));
    let result = compute_definite_size(&Length::auto(), lu(500), &space, true);
    assert_eq!(result, None);
}

#[test]
fn definite_size_auto_with_fixed() {
    let mut space = ConstraintSpace::for_root(lu(500), lu(400));
    space.is_fixed_inline_size = true;
    let result = compute_definite_size(&Length::auto(), lu(500), &space, true);
    assert_eq!(result, Some(lu(500)));
}

#[test]
fn definite_size_min_content_is_indefinite() {
    let space = ConstraintSpace::for_root(lu(500), lu(400));
    let result = compute_definite_size(&Length::min_content(), lu(500), &space, true);
    assert_eq!(result, None);
}

#[test]
fn definite_size_pct_indefinite_cb() {
    let space = ConstraintSpace::for_root(lu(800), INDEFINITE_SIZE);
    let result = compute_definite_size(&Length::percent(50.0), INDEFINITE_SIZE, &space, false);
    assert_eq!(result, None);
}

// ── 6.13  Preferred size resolution ──────────────────────────────────────

#[test]
fn preferred_size_fixed_150() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(300));
    let result = resolve_preferred_size(
        &Length::px(150.0), &Length::auto(), &Length::none(),
        lu(800), &intrinsic, lu(800), lu(0), INDEFINITE_SIZE, None, true,
    );
    assert_eq!(result, lu(150));
}

#[test]
fn preferred_size_percentage_50() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(300));
    let result = resolve_preferred_size(
        &Length::percent(50.0), &Length::auto(), &Length::none(),
        lu(400), &intrinsic, lu(400), lu(0), INDEFINITE_SIZE, None, true,
    );
    assert_eq!(result, lu(200));
}

#[test]
fn preferred_size_max_content_clamped_by_max() {
    let intrinsic = MinMaxSizes::new(lu(80), lu(250));
    let result = resolve_preferred_size(
        &Length::max_content(), &Length::px(100.0), &Length::px(200.0),
        lu(800), &intrinsic, lu(800), lu(0), INDEFINITE_SIZE, None, true,
    );
    assert_eq!(result, lu(200));
}

#[test]
fn preferred_size_min_content_clamped_by_min() {
    let intrinsic = MinMaxSizes::new(lu(30), lu(200));
    let result = resolve_preferred_size(
        &Length::min_content(), &Length::px(50.0), &Length::none(),
        lu(800), &intrinsic, lu(800), lu(0), INDEFINITE_SIZE, None, true,
    );
    assert_eq!(result, lu(50));
}

// ── 6.14  Stretch definite detection ─────────────────────────────────────

#[test]
fn stretch_definite_in_flex() {
    let mut space = ConstraintSpace::for_root(lu(600), lu(400));
    space.stretch_inline_size = true;
    let result = compute_definite_size(&Length::stretch(), lu(600), &space, true);
    assert_eq!(result, Some(lu(600)));
}

#[test]
fn stretch_indefinite_without_flex() {
    let space = ConstraintSpace::for_root(lu(600), lu(400));
    let result = compute_definite_size(&Length::stretch(), lu(600), &space, true);
    assert_eq!(result, None);
}


// ═══════════════════════════════════════════════════════════════════════════
// §7  REPLACED ELEMENT SIZING  (60+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// ── 7.1  img with intrinsic width + height ───────────────────────────────

#[test]
fn replaced_explicit_both_640x480() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(640.0);
    style.height = Length::px(480.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(640.0));
    assert_eq!(sizes.min_content_block_size, luf(480.0));
}

#[test]
fn replaced_explicit_both_100x100() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(100.0);
    style.height = Length::px(100.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(100.0));
    assert_eq!(sizes.min_content_block_size, luf(100.0));
}

#[test]
fn replaced_explicit_both_1x1() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(1.0);
    style.height = Length::px(1.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(1.0));
    assert_eq!(sizes.min_content_block_size, luf(1.0));
}

#[test]
fn replaced_explicit_both_800x600() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(800.0);
    style.height = Length::px(600.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(800.0));
    assert_eq!(sizes.max_content_inline_size, luf(800.0));
    assert_eq!(sizes.min_content_block_size, luf(600.0));
    assert_eq!(sizes.max_content_block_size, luf(600.0));
}

// ── 7.2  img with width only (height from ratio) ─────────────────────────

#[test]
fn replaced_width_only_600() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(600.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(600.0));
    // Default AR = 300:150 = 2:1, height = 600/2 = 300
    assert_eq!(sizes.min_content_block_size, luf(300.0));
}

#[test]
fn replaced_width_only_150() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(150.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(150.0));
    // 150/2 = 75
    assert_eq!(sizes.min_content_block_size, luf(75.0));
}

#[test]
fn replaced_width_only_300() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(300.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(300.0));
    assert_eq!(sizes.min_content_block_size, luf(150.0));
}

#[test]
fn replaced_width_only_900() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(900.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(900.0));
    assert_eq!(sizes.min_content_block_size, luf(450.0));
}

// ── 7.3  img with height only (width from ratio) ─────────────────────────

#[test]
fn replaced_height_only_300() {
    let mut style = ComputedStyle::initial();
    style.height = Length::px(300.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    // Default AR = 2:1, width = 300*2 = 600
    assert_eq!(sizes.min_content_inline_size, luf(600.0));
    assert_eq!(sizes.min_content_block_size, luf(300.0));
}

#[test]
fn replaced_height_only_75() {
    let mut style = ComputedStyle::initial();
    style.height = Length::px(75.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(150.0));
    assert_eq!(sizes.min_content_block_size, luf(75.0));
}

#[test]
fn replaced_height_only_150() {
    let mut style = ComputedStyle::initial();
    style.height = Length::px(150.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(300.0));
    assert_eq!(sizes.min_content_block_size, luf(150.0));
}

#[test]
fn replaced_height_only_450() {
    let mut style = ComputedStyle::initial();
    style.height = Length::px(450.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(900.0));
    assert_eq!(sizes.min_content_block_size, luf(450.0));
}

// ── 7.4  img with neither (300×150 default) ──────────────────────────────

#[test]
fn replaced_default_size_300x150() {
    let style = ComputedStyle::initial();
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, lu(300));
    assert_eq!(sizes.max_content_inline_size, lu(300));
    assert_eq!(sizes.min_content_block_size, lu(150));
    assert_eq!(sizes.max_content_block_size, lu(150));
}

#[test]
fn replaced_default_min_equals_max() {
    let style = ComputedStyle::initial();
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, sizes.max_content_inline_size);
    assert_eq!(sizes.min_content_block_size, sizes.max_content_block_size);
}

// ── 7.5  max-width on image ──────────────────────────────────────────────

#[test]
fn replaced_max_width_clamps() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(500.0);
    style.height = Length::px(250.0);
    style.max_width = Length::px(300.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    // Intrinsic sizes report what the style says; constraint applied during layout
    assert_eq!(sizes.min_content_inline_size, luf(500.0));
}

#[test]
fn replaced_max_width_via_constraints() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::px(300.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = constrain_inline_size(lu(500), &c);
    assert_eq!(result, lu(300));
}

// ── 7.6  Percentage width on image ───────────────────────────────────────

#[test]
fn replaced_pct_width_50_of_800() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::percent(50.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c,
    );
    assert_eq!(result, lu(400));
}

#[test]
fn replaced_pct_width_100() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::percent(100.0), lu(600), lu(600), BoxSizing::ContentBox, lu(0), &c,
    );
    assert_eq!(result, lu(600));
}

// ── 7.7  Replaced element intrinsic sizes consistency ────────────────────

#[test]
fn replaced_min_equals_max_explicit() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(400.0);
    style.height = Length::px(300.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, sizes.max_content_inline_size);
    assert_eq!(sizes.min_content_block_size, sizes.max_content_block_size);
}

#[test]
fn replaced_width_only_min_eq_max() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(200.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, sizes.max_content_inline_size);
}

#[test]
fn replaced_height_only_min_eq_max() {
    let mut style = ComputedStyle::initial();
    style.height = Length::px(100.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_block_size, sizes.max_content_block_size);
}

// ── 7.8  Layout-level replaced sizing ────────────────────────────────────

#[test]
fn layout_replaced_fixed_size() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().fixed_size(320.0, 240.0).done();
    let r = b.build();
    r.assert_child_size(0, 320, 240);
}

#[test]
fn layout_replaced_with_margin() {
    let mut b = BlockTestBuilder::new(800, 600);
    // Top margin collapses, only left margin visible in offset
    b.add_child().fixed_size(200.0, 150.0).margin(0, 20, 10, 20).done();
    let r = b.build();
    r.assert_child_size(0, 200, 150);
    r.assert_child_position(0, 20, 0);
}

#[test]
fn layout_replaced_centered() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().fixed_size(400.0, 200.0).margin_auto_horizontal().done();
    let r = b.build();
    r.assert_child_position(0, 200, 0);
}


// ═══════════════════════════════════════════════════════════════════════════
// §8  EDGE CASES  (60+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// ── 8.1  Very large percentage ───────────────────────────────────────────

#[test]
fn edge_200pct_width() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::percent(200.0), lu(400), lu(400), BoxSizing::ContentBox, lu(0), &c,
    );
    assert_eq!(result, lu(800));
}

#[test]
fn edge_1000pct_width() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::percent(1000.0), lu(100), lu(100), BoxSizing::ContentBox, lu(0), &c,
    );
    assert_eq!(result, lu(1000));
}

#[test]
fn edge_0pct_width() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::percent(0.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c,
    );
    assert_eq!(result, lu(0));
}

// ── 8.2  Box-sizing adjustment edge cases ────────────────────────────────

#[test]
fn edge_box_sizing_bb_subtracts_zero() {
    let result = apply_box_sizing_adjustment(lu(200), BoxSizing::BorderBox, lu(0));
    assert_eq!(result, lu(200));
}

#[test]
fn edge_box_sizing_cb_no_change() {
    let result = apply_box_sizing_adjustment(lu(200), BoxSizing::ContentBox, lu(40));
    assert_eq!(result, lu(200));
}

#[test]
fn edge_box_sizing_bb_clamps_negative_to_zero() {
    let result = apply_box_sizing_adjustment(lu(20), BoxSizing::BorderBox, lu(50));
    assert_eq!(result, lu(0));
}

#[test]
fn edge_box_sizing_bb_exact_subtraction() {
    let result = apply_box_sizing_adjustment(lu(100), BoxSizing::BorderBox, lu(100));
    assert_eq!(result, lu(0));
}

// ── 8.3  Auto + min-width + max-width interaction ────────────────────────

#[test]
fn edge_auto_min_max_clamp_to_max() {
    let c = resolve_size_constraints(
        &Length::px(100.0), &Length::px(300.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_inline_size(&Length::auto(), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c);
    // auto fills 800, clamped to max 300
    assert_eq!(result, lu(300));
}

#[test]
fn edge_auto_min_max_clamp_to_min() {
    let c = resolve_size_constraints(
        &Length::px(500.0), &Length::px(800.0),
        &Length::auto(), &Length::none(),
        lu(400), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_inline_size(&Length::auto(), lu(400), lu(400), BoxSizing::ContentBox, lu(0), &c);
    // auto fills 400, clamped to min 500
    assert_eq!(result, lu(500));
}

#[test]
fn edge_auto_min_max_no_clamp() {
    let c = resolve_size_constraints(
        &Length::px(100.0), &Length::px(900.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_inline_size(&Length::auto(), lu(600), lu(600), BoxSizing::ContentBox, lu(0), &c);
    // auto fills 600, within [100, 900]
    assert_eq!(result, lu(600));
}

// ── 8.4  Height auto + min + max ─────────────────────────────────────────

#[test]
fn edge_h_auto_min_max_clamp_to_min() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::px(200.0), &Length::px(500.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_block_size(&Length::auto(), lu(50), lu(600), BoxSizing::ContentBox, lu(0), &c);
    // content=50, clamped to min 200
    assert_eq!(result, lu(200));
}

#[test]
fn edge_h_auto_min_max_clamp_to_max() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::px(100.0), &Length::px(300.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_block_size(&Length::auto(), lu(500), lu(600), BoxSizing::ContentBox, lu(0), &c);
    // content=500, clamped to max 300
    assert_eq!(result, lu(300));
}

#[test]
fn edge_h_auto_min_max_within() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::px(50.0), &Length::px(400.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_block_size(&Length::auto(), lu(200), lu(600), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(200));
}

// ── 8.5  Deeply nested percentage chains ─────────────────────────────────

#[test]
fn edge_nested_pct_50_of_50() {
    // parent = 50% of 800 = 400, child = 50% of parent = 200
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(50.0).height(50.0)
        .add_child().with_style(|s| s.width = Length::percent(50.0)).height(30.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 50);
    r.assert_nested_child_size(0, 0, 200, 30);
}

#[test]
fn edge_nested_pct_25_of_50() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(50.0).height(50.0)
        .add_child().with_style(|s| s.width = Length::percent(25.0)).height(20.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 50);
    r.assert_nested_child_size(0, 0, 100, 20);
}

#[test]
fn edge_nested_pct_100_of_100() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(100.0).height(50.0)
        .add_child().with_style(|s| s.width = Length::percent(100.0)).height(30.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 800, 50);
    r.assert_nested_child_size(0, 0, 800, 30);
}

// ── 8.6  Zero-size elements ──────────────────────────────────────────────

#[test]
fn edge_zero_width_zero_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(0.0).height(0.0).done();
    let r = b.build();
    r.assert_child_size(0, 0, 0);
}

#[test]
fn edge_zero_width_fixed_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(0.0).height(100.0).done();
    let r = b.build();
    r.assert_child_size(0, 0, 100);
}

#[test]
fn edge_fixed_width_zero_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(0.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 0);
}

// ── 8.7  Mixed sizing strategies ─────────────────────────────────────────

#[test]
fn edge_mixed_fixed_pct_auto() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).done();
    b.add_child().width_pct(50.0).height(60.0).done();
    b.add_child().height(70.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
    r.assert_child_size(1, 400, 60);
    r.assert_child_size(2, 800, 70);
}

#[test]
fn edge_fixed_width_pct_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height_pct(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 300);
}

#[test]
fn edge_pct_width_fixed_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(75.0).height(100.0).done();
    let r = b.build();
    r.assert_child_size(0, 600, 100);
}

// ── 8.8  Many children layout ────────────────────────────────────────────

#[test]
fn edge_ten_children_stacked() {
    let mut b = BlockTestBuilder::new(800, 2000);
    for _ in 0..10 {
        b.add_child().width(200.0).height(50.0).done();
    }
    let r = b.build();
    for i in 0..10 {
        r.assert_child_size(i, 200, 50);
        r.assert_child_position(i, 0, (i as i32) * 50);
    }
    r.assert_child_count(10);
}

#[test]
fn edge_five_varying_heights() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(10.0).done();
    b.add_child().width(100.0).height(20.0).done();
    b.add_child().width(100.0).height(30.0).done();
    b.add_child().width(100.0).height(40.0).done();
    b.add_child().width(100.0).height(50.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 10);
    r.assert_child_position(2, 0, 30);
    r.assert_child_position(3, 0, 60);
    r.assert_child_position(4, 0, 100);
}

// ── 8.9  Constraint chain: all three axis ────────────────────────────────

#[test]
fn edge_inline_block_all_constraints() {
    let c = resolve_size_constraints(
        &Length::px(100.0), &Length::px(500.0),
        &Length::px(50.0), &Length::px(300.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, lu(100));
    assert_eq!(c.max_inline_size, lu(500));
    assert_eq!(c.min_block_size, lu(50));
    assert_eq!(c.max_block_size, lu(300));
    assert_eq!(constrain_inline_size(lu(50), &c), lu(100));
    assert_eq!(constrain_inline_size(lu(250), &c), lu(250));
    assert_eq!(constrain_inline_size(lu(700), &c), lu(500));
    assert_eq!(constrain_block_size(lu(20), &c), lu(50));
    assert_eq!(constrain_block_size(lu(200), &c), lu(200));
    assert_eq!(constrain_block_size(lu(400), &c), lu(300));
}

// ── 8.10  Width/height interaction edge cases ────────────────────────────

#[test]
fn edge_wide_content_narrow_container() {
    let mut b = BlockTestBuilder::new(200, 200);
    b.add_child().width(500.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 500, 50);
}

#[test]
fn edge_tall_content_short_container() {
    let mut b = BlockTestBuilder::new(200, 200);
    b.add_child().width(100.0).height(500.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 500);
}

#[test]
fn edge_auto_both_empty() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().done();
    let r = b.build();
    r.assert_child_size(0, 800, 0);
}

// ── 8.11  Constrain unconstrained ────────────────────────────────────────

#[test]
fn edge_unconstrained_no_clamp() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(constrain_inline_size(lu(500), &c), lu(500));
    assert_eq!(constrain_block_size(lu(300), &c), lu(300));
}

#[test]
fn edge_unconstrained_zero() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(constrain_inline_size(lu(0), &c), lu(0));
    assert_eq!(constrain_block_size(lu(0), &c), lu(0));
}

#[test]
fn edge_unconstrained_large() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(constrain_inline_size(lu(100000), &c), lu(100000));
}

// ── 8.12  Border-box with percentage ─────────────────────────────────────

#[test]
fn edge_border_box_pct_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width_pct(50.0).height(50.0)
        .padding(0, 20, 0, 20)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    // 50% of 800 = 400 border-box
    r.assert_child_size(0, 400, 50);
}

#[test]
fn edge_border_box_pct_100() {
    let mut b = BlockTestBuilder::new(600, 400);
    b.add_child()
        .width_pct(100.0).height(50.0)
        .padding(0, 30, 0, 30)
        .border(0, 5, 0, 5)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    r.assert_child_size(0, 600, 50);
}

// ── 8.13  Overflow hidden with height constraints ────────────────────────

#[test]
fn edge_overflow_hidden_fixed_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).overflow_hidden()
        .add_child().width(150.0).height(300.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn edge_overflow_hidden_max_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().max_height(80.0).overflow_hidden()
        .add_child().width(100.0).height(200.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 800, 80);
}

// ── 8.14  Position and sizing interaction ────────────────────────────────

#[test]
fn edge_relative_does_not_affect_sizing() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative().done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn edge_multiple_auto_margin_centering() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width(200.0).height(50.0).margin_auto_horizontal().done();
    b.add_child().width(600.0).height(60.0).margin_auto_horizontal().done();
    let r = b.build();
    r.assert_child_position(0, 400, 0);
    r.assert_child_position(1, 200, 50);
}

// ── 8.15  Min-content / max-content as min/max constraints ───────────────

#[test]
fn edge_min_content_as_min_w_treated_zero() {
    let c = resolve_size_constraints(
        &Length::min_content(), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, LayoutUnit::zero());
}

#[test]
fn edge_max_content_as_max_w_treated_none() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::max_content(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.max_inline_size, LayoutUnit::max());
}

#[test]
fn edge_fit_content_as_min_w_treated_zero() {
    let c = resolve_size_constraints(
        &Length::fit_content(), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, LayoutUnit::zero());
}

#[test]
fn edge_fit_content_as_max_w_treated_none() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::fit_content(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.max_inline_size, LayoutUnit::max());
}

// ── 8.16  Various container sizes ────────────────────────────────────────

#[test]
fn edge_container_1x1() {
    let mut b = BlockTestBuilder::new(1, 1);
    b.add_child().height(1.0).done();
    let r = b.build();
    r.assert_child_size(0, 1, 1);
}

#[test]
fn edge_container_large() {
    let mut b = BlockTestBuilder::new(10000, 10000);
    b.add_child().height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 10000, 50);
}

#[test]
fn edge_container_width_gt_height() {
    let mut b = BlockTestBuilder::new(1920, 100);
    b.add_child().width_pct(50.0).height_pct(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 960, 50);
}

// ── 8.17  Aspect ratio edge cases ────────────────────────────────────────

#[test]
fn edge_aspect_ratio_very_wide() {
    let (w, h) = apply_aspect_ratio(lu(1000), INDEFINITE_SIZE, (100.0, 1.0));
    assert_eq!(w, lu(1000));
    assert_eq!(h, lu(10));
}

#[test]
fn edge_aspect_ratio_very_tall() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(1000), (1.0, 100.0));
    assert_eq!(w, lu(10));
    assert_eq!(h, lu(1000));
}

#[test]
fn edge_aspect_ratio_equal() {
    let (w, h) = apply_aspect_ratio(lu(500), INDEFINITE_SIZE, (1.0, 1.0));
    assert_eq!(w, lu(500));
    assert_eq!(h, lu(500));
}

// ── 8.18  Float sizing ───────────────────────────────────────────────────

#[test]
fn edge_float_left_with_fixed_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

#[test]
fn edge_float_right_with_fixed_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_right().done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

#[test]
fn edge_float_shrinks_to_zero_no_children() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(0.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 0, 50);
}

// ── 8.19  Multiple constraints combined ──────────────────────────────────

#[test]
fn edge_all_box_model_combined() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .padding(10, 15, 10, 15)
        .border(3, 3, 3, 3)
        .margin(0, 10, 0, 10)
        .done();
    let r = b.build();
    // width = 200+15+15+3+3 = 236; height = 100+10+10+3+3 = 126
    r.assert_child_size(0, 236, 126);
    r.assert_child_position(0, 10, 0);
}

#[test]
fn edge_border_box_all_combined() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .padding(10, 15, 10, 15)
        .border(3, 3, 3, 3)
        .margin(0, 10, 0, 10)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    // border-box: stays 200 x 100
    r.assert_child_size(0, 200, 100);
    r.assert_child_position(0, 10, 0);
}

// ── 8.20  IntrinsicSizes struct ──────────────────────────────────────────

#[test]
fn edge_intrinsic_sizes_struct() {
    let sizes = IntrinsicSizes {
        min_content_inline_size: lu(100),
        max_content_inline_size: lu(200),
        min_content_block_size: lu(50),
        max_content_block_size: lu(80),
    };
    assert_eq!(sizes.min_content_inline_size, lu(100));
    assert_eq!(sizes.max_content_inline_size, lu(200));
    assert_eq!(sizes.min_content_block_size, lu(50));
    assert_eq!(sizes.max_content_block_size, lu(80));
}

#[test]
fn edge_intrinsic_sizes_zero() {
    let sizes = IntrinsicSizes {
        min_content_inline_size: lu(0),
        max_content_inline_size: lu(0),
        min_content_block_size: lu(0),
        max_content_block_size: lu(0),
    };
    assert_eq!(sizes.min_content_inline_size, lu(0));
    assert_eq!(sizes.max_content_inline_size, lu(0));
}

// ── 8.21  Additional layout integration ──────────────────────────────────

#[test]
fn edge_child_exceeds_container_both_axes() {
    let mut b = BlockTestBuilder::new(200, 200);
    b.add_child().width(500.0).height(500.0).done();
    let r = b.build();
    r.assert_child_size(0, 500, 500);
}

#[test]
fn edge_container_builder_dimensions() {
    let r = BlockTestBuilder::new(1024, 768).build();
    r.assert_container_width(1024);
    r.assert_container_height(768);
}

#[test]
fn edge_empty_container() {
    let r = BlockTestBuilder::new(800, 600).build();
    r.assert_child_count(0);
    r.assert_container_width(800);
    r.assert_container_height(600);
}

// ── 8.22  Additional resolve tests ───────────────────────────────────────

#[test]
fn edge_resolve_inline_pct_100() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(&Length::percent(100.0), lu(500), lu(500), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(500));
}

#[test]
fn edge_resolve_inline_pct_1() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(&Length::percent(1.0), lu(1000), lu(1000), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(10));
}

#[test]
fn edge_resolve_block_auto_zero_content() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(&Length::auto(), lu(0), lu(600), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(0));
}

#[test]
fn edge_resolve_block_pct_100() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(&Length::percent(100.0), lu(50), lu(600), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(600));
}

#[test]
fn edge_resolve_block_pct_1() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(&Length::percent(1.0), lu(50), lu(1000), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(10));
}

// ── 8.23  Constraint normalization edge cases ────────────────────────────

#[test]
fn edge_both_min_gt_max_inline_and_block() {
    let c = resolve_size_constraints(
        &Length::px(500.0), &Length::px(200.0),
        &Length::px(400.0), &Length::px(100.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, lu(500));
    assert_eq!(c.max_inline_size, lu(500));
    assert_eq!(c.min_block_size, lu(400));
    assert_eq!(c.max_block_size, lu(400));
}

#[test]
fn edge_pct_max_width_30_of_600() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::percent(30.0),
        &Length::auto(), &Length::none(),
        lu(600), lu(400), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.max_inline_size, lu(180));
}

#[test]
fn edge_pct_min_height_10_of_500() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::percent(10.0), &Length::none(),
        lu(800), lu(500), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_block_size, lu(50));
}


// ═══════════════════════════════════════════════════════════════════════════
// §9  ADDITIONAL WIDTH TESTS  (supplementary to reach 600+)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn w_150px() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(150.0).height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 150, 30);
}

#[test]
fn w_400px_in_500_container() {
    let mut b = BlockTestBuilder::new(500, 300);
    b.add_child().width(400.0).height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 30);
}

#[test]
fn w_pct_60() {
    let mut b = BlockTestBuilder::new(1000, 500);
    b.add_child().width_pct(60.0).height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 600, 30);
}

#[test]
fn w_pct_80() {
    let mut b = BlockTestBuilder::new(500, 300);
    b.add_child().width_pct(80.0).height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 30);
}

#[test]
fn w_pct_90_of_1000() {
    let mut b = BlockTestBuilder::new(1000, 500);
    b.add_child().width_pct(90.0).height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 900, 30);
}

#[test]
fn w_auto_with_margin_and_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_auto().height(50.0).margin(0, 20, 0, 20).padding(0, 10, 0, 10).done();
    let r = b.build();
    // 800 - 20 - 20 margin = 760 border-box; padding inside that
    r.assert_child_size(0, 760, 50);
}

#[test]
fn w_border_box_400_padding_30() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(100.0).padding(0, 30, 0, 30).box_sizing_border_box().done();
    let r = b.build();
    r.assert_child_size(0, 400, 100);
}

#[test]
fn w_border_box_with_margin_centering() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(50.0).padding(0, 20, 0, 20).box_sizing_border_box().margin_auto_horizontal().done();
    let r = b.build();
    r.assert_child_position(0, 200, 0);
    r.assert_child_size(0, 400, 50);
}

#[test]
fn resolve_inline_fixed_250() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(&Length::px(250.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(250));
}

#[test]
fn resolve_inline_fixed_750() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(&Length::px(750.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(750));
}

#[test]
fn resolve_inline_pct_33_of_600() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(&Length::percent(33.0), lu(600), lu(600), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, luf(198.0));
}

#[test]
fn resolve_inline_pct_66_of_600() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(&Length::percent(66.0), lu(600), lu(600), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, luf(396.0));
}

#[test]
fn resolve_inline_border_box_500_pb_80() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(&Length::px(500.0), lu(800), lu(800), BoxSizing::BorderBox, lu(80), &c);
    assert_eq!(result, lu(420));
}

#[test]
fn resolve_inline_border_box_200_pb_200() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(&Length::px(200.0), lu(800), lu(800), BoxSizing::BorderBox, lu(200), &c);
    assert_eq!(result, lu(0));
}

// ═══════════════════════════════════════════════════════════════════════════
// §10  ADDITIONAL HEIGHT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn h_300px() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(300.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 300);
}

#[test]
fn h_500px() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(500.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 500);
}

#[test]
fn h_75pct() {
    let mut b = BlockTestBuilder::new(800, 400);
    b.add_child().width(200.0).height_pct(75.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 300);
}

#[test]
fn h_33pct() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(&Length::percent(33.0), lu(50), lu(300), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, luf(99.0));
}

#[test]
fn h_border_box_300_pb_60() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(&Length::px(300.0), lu(50), lu(600), BoxSizing::BorderBox, lu(60), &c);
    assert_eq!(result, lu(240));
}

#[test]
fn h_auto_nested_two_levels() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .add_child().height(40.0).width(100.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 800, 40);
}

#[test]
fn h_auto_with_padding_and_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .padding(10, 0, 10, 0).border(5, 0, 5, 0)
        .add_child().width(100.0).height(40.0).done()
        .done();
    let r = b.build();
    // 40 + 10+10 + 5+5 = 70
    r.assert_child_size(0, 800, 70);
}

#[test]
fn h_pct_80_of_500() {
    let mut b = BlockTestBuilder::new(800, 500);
    b.add_child().width(200.0).height_pct(80.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 400);
}

#[test]
fn h_pct_10_of_1000() {
    let mut b = BlockTestBuilder::new(800, 1000);
    b.add_child().width(200.0).height_pct(10.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn h_border_box_with_padding_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(200.0).padding(30, 0, 30, 0).box_sizing_border_box().done();
    let r = b.build();
    r.assert_child_size(0, 200, 200);
}

// ═══════════════════════════════════════════════════════════════════════════
// §11  ADDITIONAL MIN/MAX TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn min_w_150_layout() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(80.0).min_width(150.0).height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 150, 30);
}

#[test]
fn max_w_250_layout() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).max_width(250.0).height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 250, 30);
}

#[test]
fn min_h_200_layout() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).min_height(200.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 200);
}

#[test]
fn max_h_100_layout() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(300.0).max_height(100.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn min_max_w_equal() {
    let c = resolve_size_constraints(
        &Length::px(200.0), &Length::px(200.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, lu(200));
    assert_eq!(c.max_inline_size, lu(200));
    assert_eq!(constrain_inline_size(lu(100), &c), lu(200));
    assert_eq!(constrain_inline_size(lu(300), &c), lu(200));
}

#[test]
fn min_max_h_equal() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::px(150.0), &Length::px(150.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_block_size, lu(150));
    assert_eq!(c.max_block_size, lu(150));
}

#[test]
fn min_w_pct_40_of_600() {
    let c = resolve_size_constraints(
        &Length::percent(40.0), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(600), lu(400), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, lu(240));
}

#[test]
fn max_w_pct_80_of_500() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::percent(80.0),
        &Length::auto(), &Length::none(),
        lu(500), lu(400), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.max_inline_size, lu(400));
}

#[test]
fn min_h_pct_50_of_800() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::percent(50.0), &Length::none(),
        lu(800), lu(800), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_block_size, lu(400));
}

#[test]
fn max_h_pct_25_of_400() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::auto(), &Length::percent(25.0),
        lu(800), lu(400), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.max_block_size, lu(100));
}

#[test]
fn constrain_inline_boundary_at_min() {
    let c = SizeConstraint {
        min_inline_size: lu(100),
        max_inline_size: lu(500),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(100), &c), lu(100));
}

#[test]
fn constrain_inline_boundary_at_max() {
    let c = SizeConstraint {
        min_inline_size: lu(100),
        max_inline_size: lu(500),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(500), &c), lu(500));
}

#[test]
fn constrain_block_boundary_at_min() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(80),
        max_block_size: lu(300),
    };
    assert_eq!(constrain_block_size(lu(80), &c), lu(80));
}

#[test]
fn constrain_block_boundary_at_max() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(80),
        max_block_size: lu(300),
    };
    assert_eq!(constrain_block_size(lu(300), &c), lu(300));
}

// ═══════════════════════════════════════════════════════════════════════════
// §12  ADDITIONAL INTRINSIC SIZING TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn intrinsic_min_content_wide_child() {
    let (doc, parent) = doc_with_one_child(500.0, 20.0);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.min_content_inline_size, luf(500.0));
}

#[test]
fn intrinsic_max_content_narrow_child() {
    let (doc, parent) = doc_with_one_child(10.0, 10.0);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.max_content_inline_size, luf(10.0));
}

#[test]
fn intrinsic_block_single_tall_child() {
    let (doc, parent) = doc_with_one_child(100.0, 500.0);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.max_content_block_size, luf(500.0));
}

#[test]
fn intrinsic_four_children() {
    let (doc, parent) = doc_with_children(&[
        (80.0, 10.0), (120.0, 20.0), (90.0, 15.0), (110.0, 25.0),
    ]);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.min_content_inline_size, luf(120.0));
    assert_eq!(sizes.max_content_inline_size, luf(120.0));
    assert_eq!(
        sizes.max_content_block_size,
        luf(10.0) + luf(20.0) + luf(15.0) + luf(25.0)
    );
}

#[test]
fn intrinsic_five_same_size_children() {
    let (doc, parent) = doc_with_children(&[
        (100.0, 20.0), (100.0, 20.0), (100.0, 20.0), (100.0, 20.0), (100.0, 20.0),
    ]);
    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    assert_eq!(sizes.min_content_inline_size, luf(100.0));
    assert_eq!(sizes.max_content_block_size, luf(100.0));
}

#[test]
fn shrink_to_fit_min_zero_max_zero() {
    assert_eq!(shrink_to_fit_inline_size(lu(0), lu(0), lu(100)), lu(0));
}

#[test]
fn shrink_to_fit_available_very_large() {
    assert_eq!(shrink_to_fit_inline_size(lu(50), lu(200), lu(10000)), lu(200));
}

#[test]
fn shrink_to_fit_min_1_max_1() {
    assert_eq!(shrink_to_fit_inline_size(lu(1), lu(1), lu(500)), lu(1));
}


// ═══════════════════════════════════════════════════════════════════════════
// §13  ADDITIONAL CSS SIZING L3 KEYWORD TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn sizing_kw_min_content_large_intrinsic() {
    let intrinsic = MinMaxSizes::new(lu(500), lu(1000));
    let result = resolve_sizing_keyword(SizingKeyword::MinContent, &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(500));
}

#[test]
fn sizing_kw_max_content_small_intrinsic() {
    let intrinsic = MinMaxSizes::new(lu(10), lu(50));
    let result = resolve_sizing_keyword(SizingKeyword::MaxContent, &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(50));
}

#[test]
fn sizing_kw_fit_content_0_equals_min() {
    let intrinsic = MinMaxSizes::new(lu(80), lu(300));
    let result = resolve_sizing_keyword(SizingKeyword::FitContent(lu(0)), &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(80));
}

#[test]
fn sizing_kw_fit_content_large_equals_max() {
    let intrinsic = MinMaxSizes::new(lu(80), lu(300));
    let result = resolve_sizing_keyword(SizingKeyword::FitContent(lu(10000)), &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(300));
}

#[test]
fn sizing_kw_stretch_zero_available() {
    let intrinsic = MinMaxSizes::new(lu(0), lu(0));
    let result = resolve_sizing_keyword(SizingKeyword::Stretch, &intrinsic, lu(0), lu(0));
    assert_eq!(result, lu(0));
}

#[test]
fn sizing_kw_stretch_1000() {
    let intrinsic = MinMaxSizes::new(lu(0), lu(0));
    let result = resolve_sizing_keyword(SizingKeyword::Stretch, &intrinsic, lu(1000), lu(0));
    assert_eq!(result, lu(1000));
}

#[test]
fn sizing_kw_fit_content_200_with_wide_intrinsic() {
    let intrinsic = MinMaxSizes::new(lu(100), lu(500));
    let result = resolve_sizing_keyword(SizingKeyword::FitContent(lu(200)), &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(200));
}

#[test]
fn sizing_kw_fit_content_300_narrow_intrinsic() {
    let intrinsic = MinMaxSizes::new(lu(10), lu(50));
    let result = resolve_sizing_keyword(SizingKeyword::FitContent(lu(300)), &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(50));
}

// ── Additional aspect-ratio tests ────────────────────────────────────────

#[test]
fn ar_3_1_w_300() {
    let (w, h) = apply_aspect_ratio(lu(300), INDEFINITE_SIZE, (3.0, 1.0));
    assert_eq!(w, lu(300));
    assert_eq!(h, lu(100));
}

#[test]
fn ar_1_3_h_300() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(300), (1.0, 3.0));
    assert_eq!(w, lu(100));
    assert_eq!(h, lu(300));
}

#[test]
fn ar_2_3_w_200() {
    let (w, h) = apply_aspect_ratio(lu(200), INDEFINITE_SIZE, (2.0, 3.0));
    assert_eq!(w, lu(200));
    assert_eq!(h, lu(300));
}

#[test]
fn ar_3_2_h_200() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(200), (3.0, 2.0));
    assert_eq!(w, lu(300));
    assert_eq!(h, lu(200));
}

#[test]
fn ar_5_4_w_500() {
    let (w, h) = apply_aspect_ratio(lu(500), INDEFINITE_SIZE, (5.0, 4.0));
    assert_eq!(w, lu(500));
    assert_eq!(h, lu(400));
}

#[test]
fn ar_4_5_h_400() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(400), (4.0, 5.0));
    assert_eq!(w, lu(320));
    assert_eq!(h, lu(400));
}

#[test]
fn ar_8_5_w_800() {
    let (w, h) = apply_aspect_ratio(lu(800), INDEFINITE_SIZE, (8.0, 5.0));
    assert_eq!(w, lu(800));
    assert_eq!(h, lu(500));
}

#[test]
fn ar_auto_flag_true_no_intrinsic() {
    let ar = AspectRatio { ratio: (2.0, 1.0), auto_flag: true };
    let (w, h) = apply_aspect_ratio_with_auto(lu(400), INDEFINITE_SIZE, &ar, None);
    assert_eq!(w, lu(400));
    assert_eq!(h, lu(200));
}

#[test]
fn ar_auto_flag_true_with_intrinsic_3_2() {
    let ar = AspectRatio { ratio: (16.0, 9.0), auto_flag: true };
    let (w, h) = apply_aspect_ratio_with_auto(lu(300), INDEFINITE_SIZE, &ar, Some((3.0, 2.0)));
    assert_eq!(w, lu(300));
    assert_eq!(h, lu(200));
}

#[test]
fn ar_auto_flag_false_ignores_intrinsic() {
    let ar = AspectRatio { ratio: (2.0, 1.0), auto_flag: false };
    let (w, h) = apply_aspect_ratio_with_auto(lu(400), INDEFINITE_SIZE, &ar, Some((3.0, 2.0)));
    assert_eq!(w, lu(400));
    assert_eq!(h, lu(200));
}

// ── Additional preferred size tests ──────────────────────────────────────

#[test]
fn preferred_auto_stretch_inline() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(200));
    let result = resolve_preferred_size(
        &Length::auto(), &Length::auto(), &Length::none(),
        lu(600), &intrinsic, lu(600), lu(0), INDEFINITE_SIZE, None, true,
    );
    assert_eq!(result, lu(600));
}

#[test]
fn preferred_min_content_directly() {
    let intrinsic = MinMaxSizes::new(lu(80), lu(300));
    let result = resolve_preferred_size(
        &Length::min_content(), &Length::auto(), &Length::none(),
        lu(800), &intrinsic, lu(800), lu(0), INDEFINITE_SIZE, None, true,
    );
    assert_eq!(result, lu(80));
}

#[test]
fn preferred_max_content_directly() {
    let intrinsic = MinMaxSizes::new(lu(80), lu(300));
    let result = resolve_preferred_size(
        &Length::max_content(), &Length::auto(), &Length::none(),
        lu(800), &intrinsic, lu(800), lu(0), INDEFINITE_SIZE, None, true,
    );
    assert_eq!(result, lu(300));
}

#[test]
fn preferred_pct_25() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(300));
    let result = resolve_preferred_size(
        &Length::percent(25.0), &Length::auto(), &Length::none(),
        lu(800), &intrinsic, lu(800), lu(0), INDEFINITE_SIZE, None, true,
    );
    assert_eq!(result, lu(200));
}

#[test]
fn preferred_pct_75() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(300));
    let result = resolve_preferred_size(
        &Length::percent(75.0), &Length::auto(), &Length::none(),
        lu(800), &intrinsic, lu(800), lu(0), INDEFINITE_SIZE, None, true,
    );
    assert_eq!(result, lu(600));
}

#[test]
fn preferred_fixed_clamped_by_min() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(300));
    let result = resolve_preferred_size(
        &Length::px(80.0), &Length::px(150.0), &Length::none(),
        lu(800), &intrinsic, lu(800), lu(0), INDEFINITE_SIZE, None, true,
    );
    assert_eq!(result, lu(150));
}

#[test]
fn preferred_fixed_clamped_by_max() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(300));
    let result = resolve_preferred_size(
        &Length::px(400.0), &Length::auto(), &Length::px(250.0),
        lu(800), &intrinsic, lu(800), lu(0), INDEFINITE_SIZE, None, true,
    );
    assert_eq!(result, lu(250));
}

#[test]
fn preferred_fixed_within_bounds() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(300));
    let result = resolve_preferred_size(
        &Length::px(200.0), &Length::px(100.0), &Length::px(400.0),
        lu(800), &intrinsic, lu(800), lu(0), INDEFINITE_SIZE, None, true,
    );
    assert_eq!(result, lu(200));
}

// ── Additional definite size tests ───────────────────────────────────────

#[test]
fn definite_size_px_500() {
    let space = ConstraintSpace::for_root(lu(800), lu(600));
    let result = compute_definite_size(&Length::px(500.0), lu(800), &space, true);
    assert_eq!(result, Some(lu(500)));
}

#[test]
fn definite_size_px_1() {
    let space = ConstraintSpace::for_root(lu(800), lu(600));
    let result = compute_definite_size(&Length::px(1.0), lu(800), &space, true);
    assert_eq!(result, Some(lu(1)));
}

#[test]
fn definite_size_pct_25() {
    let space = ConstraintSpace::for_root(lu(800), lu(600));
    let result = compute_definite_size(&Length::percent(25.0), lu(800), &space, true);
    assert_eq!(result, Some(lu(200)));
}

#[test]
fn definite_size_pct_100() {
    let space = ConstraintSpace::for_root(lu(800), lu(600));
    let result = compute_definite_size(&Length::percent(100.0), lu(800), &space, true);
    assert_eq!(result, Some(lu(800)));
}

#[test]
fn definite_size_max_content_is_none() {
    let space = ConstraintSpace::for_root(lu(800), lu(600));
    let result = compute_definite_size(&Length::max_content(), lu(800), &space, true);
    assert_eq!(result, None);
}

#[test]
fn definite_size_fit_content_is_none() {
    let space = ConstraintSpace::for_root(lu(800), lu(600));
    let result = compute_definite_size(&Length::fit_content(), lu(800), &space, true);
    assert_eq!(result, None);
}

// ── Additional automatic size tests ──────────────────────────────────────

#[test]
fn automatic_inline_stretch_200() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(100));
    let result = compute_automatic_size(true, &intrinsic, lu(200), lu(0));
    assert_eq!(result, lu(200));
}

#[test]
fn automatic_inline_stretch_with_margins_50() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(100));
    let result = compute_automatic_size(true, &intrinsic, lu(200), lu(50));
    assert_eq!(result, lu(150));
}

#[test]
fn automatic_block_fit_content_large_available() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(100));
    let result = compute_automatic_size(false, &intrinsic, lu(1000), lu(0));
    assert_eq!(result, lu(100));
}

#[test]
fn automatic_block_fit_content_small_available() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(300));
    let result = compute_automatic_size(false, &intrinsic, lu(100), lu(0));
    assert_eq!(result, lu(100));
}

#[test]
fn automatic_block_fit_content_tiny_available() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(300));
    let result = compute_automatic_size(false, &intrinsic, lu(30), lu(0));
    assert_eq!(result, lu(50));
}


// ═══════════════════════════════════════════════════════════════════════════
// §14  LAYOUT INTEGRATION — WIDTH RESOLUTION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn layout_w_300_h_100() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(100.0).done();
    let r = b.build();
    assert_layout!(r, child(0) at (0, 0) size (300, 100));
}

#[test]
fn layout_w_pct_50_h_80() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(50.0).height(80.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (400, 80));
}

#[test]
fn layout_w_auto_fills_600() {
    let mut b = BlockTestBuilder::new(600, 400);
    b.add_child().height(40.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (600, 40));
}

#[test]
fn layout_w_border_box_250() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(250.0).height(80.0).padding(0, 25, 0, 25).box_sizing_border_box().done();
    let r = b.build();
    assert_layout!(r, child(0) size (250, 80));
}

#[test]
fn layout_w_auto_margin_20() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(40.0).margin(0, 20, 0, 20).done();
    let r = b.build();
    assert_layout!(r, child(0) at (20, 0) size (760, 40));
}

#[test]
fn layout_w_two_children_100_200() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(30.0).done();
    b.add_child().width(200.0).height(40.0).done();
    let r = b.build();
    assert_layout!(r, child(0) at (0, 0) size (100, 30));
    assert_layout!(r, child(1) at (0, 30) size (200, 40));
}

#[test]
fn layout_w_three_varying() {
    let mut b = BlockTestBuilder::new(1000, 800);
    b.add_child().width(250.0).height(50.0).done();
    b.add_child().width_pct(40.0).height(60.0).done();
    b.add_child().height(70.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (250, 50));
    assert_layout!(r, child(1) size (400, 60));
    assert_layout!(r, child(2) size (1000, 70));
}

#[test]
fn layout_w_centered_300_in_800() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(50.0).margin_auto_horizontal().done();
    let r = b.build();
    assert_layout!(r, child(0) at (250, 0) size (300, 50));
}

#[test]
fn layout_w_border_box_500_pb_100() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(500.0).height(200.0).padding(0, 50, 0, 50).box_sizing_border_box().done();
    let r = b.build();
    assert_layout!(r, child(0) size (500, 200));
}

#[test]
fn layout_w_content_box_500_pb_100() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(500.0).height(200.0).padding(0, 50, 0, 50).done();
    let r = b.build();
    assert_layout!(r, child(0) size (600, 200));
}

// ═══════════════════════════════════════════════════════════════════════════
// §15  LAYOUT INTEGRATION — HEIGHT RESOLUTION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn layout_h_fixed_200() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(200.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 200));
}

#[test]
fn layout_h_pct_50_of_600() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height_pct(50.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 300));
}

#[test]
fn layout_h_auto_empty() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 0));
}

#[test]
fn layout_h_auto_from_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .add_child().width(50.0).height(75.0).done()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) size (800, 75));
}

#[test]
fn layout_h_auto_from_two_children() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .add_child().width(50.0).height(30.0).done()
        .add_child().width(50.0).height(45.0).done()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) size (800, 75));
}

#[test]
fn layout_h_border_box_200_pb_40() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(200.0).padding(20, 0, 20, 0).box_sizing_border_box().done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 200));
}

#[test]
fn layout_h_content_box_200_pb_40() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(200.0).padding(20, 0, 20, 0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 240));
}

#[test]
fn layout_h_stacking_three() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(80.0).done();
    b.add_child().width(100.0).height(90.0).done();
    b.add_child().width(100.0).height(100.0).done();
    let r = b.build();
    assert_layout!(r, child(0) at (0, 0));
    assert_layout!(r, child(1) at (0, 80));
    assert_layout!(r, child(2) at (0, 170));
}

#[test]
fn layout_h_pct_25_of_800() {
    let mut b = BlockTestBuilder::new(600, 800);
    b.add_child().width(100.0).height_pct(25.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 200));
}

#[test]
fn layout_h_pct_100() {
    let mut b = BlockTestBuilder::new(600, 400);
    b.add_child().width(100.0).height_pct(100.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 400));
}

// ═══════════════════════════════════════════════════════════════════════════
// §16  LAYOUT INTEGRATION — MIN/MAX
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn layout_min_w_clamps_narrow() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).min_width(100.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 30));
}

#[test]
fn layout_max_w_clamps_wide() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(600.0).max_width(400.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (400, 30));
}

#[test]
fn layout_min_h_clamps_short() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(20.0).min_height(80.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 80));
}

#[test]
fn layout_max_h_clamps_tall() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(500.0).max_height(200.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 200));
}

#[test]
fn layout_min_w_gt_max_w() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(250.0).min_width(300.0).max_width(150.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (150, 30));
}

#[test]
fn layout_min_h_gt_max_h() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(250.0).min_height(300.0).max_height(150.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 150));
}

#[test]
fn layout_max_w_on_auto() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().max_width(400.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (400, 30));
}

#[test]
fn layout_min_w_on_auto_with_content() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().min_width(200.0).height(30.0).done();
    let r = b.build();
    // auto fills 800 which is > min 200
    assert_layout!(r, child(0) size (800, 30));
}

#[test]
fn layout_max_h_on_auto_from_content() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().max_height(50.0)
        .add_child().width(100.0).height(200.0).done()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) size (800, 50));
}

#[test]
fn layout_min_h_on_auto_from_content() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().min_height(150.0)
        .add_child().width(100.0).height(30.0).done()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) size (800, 150));
}

// ═══════════════════════════════════════════════════════════════════════════
// §17  LAYOUT INTEGRATION — NESTED PERCENTAGES
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn layout_nested_50_50_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(50.0).height(40.0)
        .add_child().with_style(|s| s.width = Length::percent(50.0)).height(20.0).done()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) size (400, 40));
    r.assert_nested_child_size(0, 0, 200, 20);
}

#[test]
fn layout_nested_100_50_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(100.0).height(40.0)
        .add_child().with_style(|s| s.width = Length::percent(50.0)).height(20.0).done()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) size (800, 40));
    r.assert_nested_child_size(0, 0, 400, 20);
}

#[test]
fn layout_nested_75_50_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(75.0).height(40.0)
        .add_child().with_style(|s| s.width = Length::percent(50.0)).height(20.0).done()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) size (600, 40));
    r.assert_nested_child_size(0, 0, 300, 20);
}

#[test]
fn layout_nested_auto_50_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(40.0)
        .add_child().with_style(|s| s.width = Length::percent(50.0)).height(20.0).done()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) size (800, 40));
    r.assert_nested_child_size(0, 0, 400, 20);
}

#[test]
fn layout_nested_fixed_pct_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(40.0)
        .add_child().with_style(|s| s.width = Length::percent(50.0)).height(20.0).done()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) size (400, 40));
    r.assert_nested_child_size(0, 0, 200, 20);
}

#[test]
fn layout_nested_position() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(80.0)
        .add_child().width(200.0).height(30.0).done()
        .add_child().width(200.0).height(30.0).done()
        .done();
    let r = b.build();
    r.assert_nested_child_position(0, 0, 0, 0);
    r.assert_nested_child_position(0, 1, 0, 30);
}

// ═══════════════════════════════════════════════════════════════════════════
// §18  LAYOUT INTEGRATION — BOX MODEL COMBINATIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn layout_padding_all_10() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).padding(10, 10, 10, 10).done();
    let r = b.build();
    assert_layout!(r, child(0) size (220, 120));
}

#[test]
fn layout_border_all_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).border(5, 5, 5, 5).done();
    let r = b.build();
    assert_layout!(r, child(0) size (210, 110));
}

#[test]
fn layout_padding_border_margin() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .padding(10, 10, 10, 10)
        .border(2, 2, 2, 2)
        .margin(0, 5, 0, 5)
        .done();
    let r = b.build();
    // width = 200 + 10+10 + 2+2 = 224; height = 100 + 10+10 + 2+2 = 124
    assert_layout!(r, child(0) at (5, 0) size (224, 124));
}

#[test]
fn layout_border_box_padding_10() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).padding(10, 10, 10, 10).box_sizing_border_box().done();
    let r = b.build();
    assert_layout!(r, child(0) size (200, 100));
}

#[test]
fn layout_border_box_border_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).border(5, 5, 5, 5).box_sizing_border_box().done();
    let r = b.build();
    assert_layout!(r, child(0) size (200, 100));
}

#[test]
fn layout_border_box_padding_border_margin() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .padding(10, 10, 10, 10)
        .border(2, 2, 2, 2)
        .margin(0, 5, 0, 5)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) at (5, 0) size (200, 100));
}

#[test]
fn layout_asymmetric_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).padding(5, 10, 15, 20).done();
    let r = b.build();
    // width = 200 + 10 + 20 = 230; height = 100 + 5 + 15 = 120
    assert_layout!(r, child(0) size (230, 120));
}

#[test]
fn layout_asymmetric_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).border(1, 2, 3, 4).done();
    let r = b.build();
    // width = 200 + 2 + 4 = 206; height = 100 + 1 + 3 = 104
    assert_layout!(r, child(0) size (206, 104));
}

#[test]
fn layout_asymmetric_margin() {
    let mut b = BlockTestBuilder::new(800, 600);
    // Top margin collapses through parent, only horizontal margins affect position
    b.add_child().width(200.0).height(100.0).margin(0, 30, 20, 40).done();
    let r = b.build();
    assert_layout!(r, child(0) at (40, 0) size (200, 100));
}

#[test]
fn layout_large_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).padding(50, 50, 50, 50).done();
    let r = b.build();
    assert_layout!(r, child(0) size (200, 150));
}

// ═══════════════════════════════════════════════════════════════════════════
// §19  ADDITIONAL REPLACED ELEMENT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn replaced_w_400_h_200() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(400.0);
    style.height = Length::px(200.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(400.0));
    assert_eq!(sizes.max_content_inline_size, luf(400.0));
    assert_eq!(sizes.min_content_block_size, luf(200.0));
    assert_eq!(sizes.max_content_block_size, luf(200.0));
}

#[test]
fn replaced_w_1920_h_1080() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(1920.0);
    style.height = Length::px(1080.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(1920.0));
    assert_eq!(sizes.min_content_block_size, luf(1080.0));
}

#[test]
fn replaced_w_only_1200() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(1200.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(1200.0));
    assert_eq!(sizes.min_content_block_size, luf(600.0));
}

#[test]
fn replaced_h_only_600() {
    let mut style = ComputedStyle::initial();
    style.height = Length::px(600.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(1200.0));
    assert_eq!(sizes.min_content_block_size, luf(600.0));
}

#[test]
fn replaced_w_only_50() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(50.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(50.0));
    assert_eq!(sizes.min_content_block_size, luf(25.0));
}

#[test]
fn replaced_h_only_25() {
    let mut style = ComputedStyle::initial();
    style.height = Length::px(25.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(50.0));
    assert_eq!(sizes.min_content_block_size, luf(25.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// §20  MORE EDGE CASES AND CONSTRAINT RESOLUTION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn edge_constrain_inline_just_below_min() {
    let c = SizeConstraint {
        min_inline_size: lu(100),
        max_inline_size: lu(500),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(99), &c), lu(100));
}

#[test]
fn edge_constrain_inline_just_above_max() {
    let c = SizeConstraint {
        min_inline_size: lu(100),
        max_inline_size: lu(500),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(501), &c), lu(500));
}

#[test]
fn edge_constrain_block_just_below_min() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(50),
        max_block_size: lu(300),
    };
    assert_eq!(constrain_block_size(lu(49), &c), lu(50));
}

#[test]
fn edge_constrain_block_just_above_max() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(50),
        max_block_size: lu(300),
    };
    assert_eq!(constrain_block_size(lu(301), &c), lu(300));
}

#[test]
fn edge_resolve_constraints_all_fixed() {
    let c = resolve_size_constraints(
        &Length::px(100.0), &Length::px(500.0),
        &Length::px(50.0), &Length::px(300.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, lu(100));
    assert_eq!(c.max_inline_size, lu(500));
    assert_eq!(c.min_block_size, lu(50));
    assert_eq!(c.max_block_size, lu(300));
}

#[test]
fn edge_resolve_constraints_all_pct() {
    let c = resolve_size_constraints(
        &Length::percent(10.0), &Length::percent(80.0),
        &Length::percent(5.0), &Length::percent(50.0),
        lu(1000), lu(800), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, lu(100));
    assert_eq!(c.max_inline_size, lu(800));
    assert_eq!(c.min_block_size, lu(40));
    assert_eq!(c.max_block_size, lu(400));
}

#[test]
fn edge_resolve_constraints_mixed() {
    let c = resolve_size_constraints(
        &Length::px(50.0), &Length::percent(50.0),
        &Length::percent(10.0), &Length::px(400.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    assert_eq!(c.min_inline_size, lu(50));
    assert_eq!(c.max_inline_size, lu(400));
    assert_eq!(c.min_block_size, lu(60));
    assert_eq!(c.max_block_size, lu(400));
}

#[test]
fn edge_w_auto_one_side_margin() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(30.0).margin(0, 0, 0, 100).done();
    let r = b.build();
    assert_layout!(r, child(0) at (100, 0) size (700, 30));
}

#[test]
fn edge_w_auto_right_margin() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(30.0).margin(0, 100, 0, 0).done();
    let r = b.build();
    assert_layout!(r, child(0) at (0, 0) size (700, 30));
}

#[test]
fn edge_width_exactly_1() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(1.0).height(1.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (1, 1));
}

#[test]
fn edge_many_small_children() {
    let mut b = BlockTestBuilder::new(800, 600);
    for _ in 0..20 {
        b.add_child().width(50.0).height(10.0).done();
    }
    let r = b.build();
    r.assert_child_count(20);
    for i in 0..20 {
        r.assert_child_size(i, 50, 10);
        r.assert_child_position(i, 0, (i as i32) * 10);
    }
}

#[test]
fn edge_container_200_child_pct_200() {
    let mut b = BlockTestBuilder::new(200, 200);
    b.add_child().width_pct(200.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (400, 30));
}

#[test]
fn edge_container_child_pct_0() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(0.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (0, 30));
}

#[test]
fn edge_h_pct_0() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height_pct(0.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 0));
}

#[test]
fn edge_h_pct_200() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height_pct(200.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 1200));
}

// ═══════════════════════════════════════════════════════════════════════════
// §21  ADDITIONAL SHRINK-TO-FIT AND FLOAT SIZING
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn float_left_fixed_200() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    assert_layout!(r, child(0) size (200, 50));
}

#[test]
fn float_right_fixed_300() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(60.0).float_right().done();
    let r = b.build();
    assert_layout!(r, child(0) size (300, 60));
}

#[test]
fn float_left_with_margin() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().margin(0, 10, 0, 10).done();
    let r = b.build();
    assert_layout!(r, child(0) size (200, 50));
}

#[test]
fn float_shrink_with_nested_fixed_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().float_left().width(200.0).height(50.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (200, 50));
}

#[test]
fn shrink_to_fit_min_gt_max() {
    // min > max (degenerate): min(max(300, 200), 100) = 100
    assert_eq!(shrink_to_fit_inline_size(lu(300), lu(100), lu(200)), lu(100));
}

#[test]
fn shrink_to_fit_1_1_1() {
    assert_eq!(shrink_to_fit_inline_size(lu(1), lu(1), lu(1)), lu(1));
}

#[test]
fn shrink_to_fit_0_0_0() {
    assert_eq!(shrink_to_fit_inline_size(lu(0), lu(0), lu(0)), lu(0));
}

#[test]
fn shrink_to_fit_0_1000_500() {
    assert_eq!(shrink_to_fit_inline_size(lu(0), lu(1000), lu(500)), lu(500));
}

// ═══════════════════════════════════════════════════════════════════════════
// §22  ADDITIONAL ASPECT RATIO TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ar_10_1_w_100() {
    let (w, h) = apply_aspect_ratio(lu(100), INDEFINITE_SIZE, (10.0, 1.0));
    assert_eq!(w, lu(100));
    assert_eq!(h, lu(10));
}

#[test]
fn ar_1_10_h_100() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(100), (1.0, 10.0));
    assert_eq!(w, lu(10));
    assert_eq!(h, lu(100));
}

#[test]
fn ar_16_10_w_1600() {
    let (w, h) = apply_aspect_ratio(lu(1600), INDEFINITE_SIZE, (16.0, 10.0));
    assert_eq!(w, lu(1600));
    assert_eq!(h, lu(1000));
}

#[test]
fn ar_21_9_w_2100() {
    let (w, h) = apply_aspect_ratio(lu(2100), INDEFINITE_SIZE, (21.0, 9.0));
    assert_eq!(w, lu(2100));
    assert_eq!(h, lu(900));
}

#[test]
fn ar_9_16_h_1600() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(1600), (9.0, 16.0));
    assert_eq!(w, lu(900));
    assert_eq!(h, lu(1600));
}

#[test]
fn ar_preferred_with_ar_and_both_constrained() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(400));
    let ar = AspectRatio { ratio: (4.0, 3.0), auto_flag: false };
    let result = resolve_preferred_size(
        &Length::auto(), &Length::px(100.0), &Length::px(300.0),
        lu(800), &intrinsic, lu(800), lu(0), lu(300),
        Some(&ar), true,
    );
    // width from AR: 300 * 4/3 = 400, clamped to max 300
    assert_eq!(result, lu(300));
}

#[test]
fn ar_preferred_with_ar_no_constraints() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(400));
    let ar = AspectRatio { ratio: (2.0, 1.0), auto_flag: false };
    let result = resolve_preferred_size(
        &Length::auto(), &Length::auto(), &Length::none(),
        lu(800), &intrinsic, lu(800), lu(0), lu(100),
        Some(&ar), true,
    );
    // width from AR: 100 * 2 = 200
    assert_eq!(result, lu(200));
}

// ═══════════════════════════════════════════════════════════════════════════
// §23  ADDITIONAL CONTAINER AND MACRO TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn macro_child_at_size() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).done();
    let r = b.build();
    assert_layout!(r, child(0) at (0, 0) size (200, 100));
}

#[test]
fn macro_container_width() {
    let r = BlockTestBuilder::new(1024, 768).build();
    assert_layout!(r, container width 1024);
}

#[test]
fn macro_container_height() {
    let r = BlockTestBuilder::new(1024, 768).build();
    assert_layout!(r, container height 768);
}

#[test]
fn macro_child_count_0() {
    let r = BlockTestBuilder::new(800, 600).build();
    assert_layout!(r, child_count 0);
}

#[test]
fn macro_child_count_3() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(30.0).done();
    b.add_child().width(100.0).height(30.0).done();
    b.add_child().width(100.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child_count 3);
}

#[test]
fn macro_child_at() {
    let mut b = BlockTestBuilder::new(800, 600);
    // Top margin collapses, only left margin affects position
    b.add_child().width(100.0).height(50.0).margin(0, 0, 0, 20).done();
    let r = b.build();
    assert_layout!(r, child(0) at (20, 0));
}

#[test]
fn macro_child_size() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(350.0).height(175.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (350, 175));
}

#[test]
fn with_container_style_override() {
    let r = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.padding_top = Length::px(10.0);
            s.padding_bottom = Length::px(10.0);
        })
        .build();
    r.assert_container_width(800);
}

#[test]
fn with_style_closure_on_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .with_style(|s| {
            s.margin_left = Length::px(50.0);
        })
        .done();
    let r = b.build();
    r.assert_child_position(0, 50, 0);
}

#[test]
fn style_builder_basic() {
    let style = style_builder().width(200.0).height(100.0).build();
    assert_eq!(style.width, Length::px(200.0));
    assert_eq!(style.height, Length::px(100.0));
}

#[test]
fn style_builder_pct() {
    let style = style_builder().width_pct(50.0).height_pct(25.0).build();
    assert_eq!(style.width, Length::percent(50.0));
    assert_eq!(style.height, Length::percent(25.0));
}

#[test]
fn style_builder_auto() {
    let style = style_builder().width_auto().height_auto().build();
    assert_eq!(style.width, Length::auto());
    assert_eq!(style.height, Length::auto());
}

#[test]
fn style_builder_border_box() {
    let style = style_builder().box_sizing_border_box().build();
    assert_eq!(style.box_sizing, BoxSizing::BorderBox);
}

#[test]
fn style_builder_overflow_hidden() {
    let style = style_builder().overflow_hidden().build();
    assert_eq!(style.overflow_x, Overflow::Hidden);
    assert_eq!(style.overflow_y, Overflow::Hidden);
}

#[test]
fn style_builder_float_left() {
    let style = style_builder().float_left().build();
    assert_eq!(style.float, Float::Left);
}

#[test]
fn style_builder_float_right() {
    let style = style_builder().float_right().build();
    assert_eq!(style.float, Float::Right);
}

#[test]
fn style_builder_position_absolute() {
    let style = style_builder().position_absolute().build();
    assert_eq!(style.position, Position::Absolute);
}

#[test]
fn style_builder_position_relative() {
    let style = style_builder().position_relative().build();
    assert_eq!(style.position, Position::Relative);
}

#[test]
fn style_builder_display_inline_block() {
    let style = style_builder().display(Display::InlineBlock).build();
    assert_eq!(style.display, Display::InlineBlock);
}

#[test]
fn style_builder_with_closure() {
    let style = style_builder().with(|s| {
        s.margin_top = Length::px(20.0);
    }).build();
    assert_eq!(style.margin_top, Length::px(20.0));
}

#[test]
fn style_builder_padding() {
    let style = style_builder().padding(10, 20, 30, 40).build();
    assert_eq!(style.padding_top, Length::px(10.0));
    assert_eq!(style.padding_right, Length::px(20.0));
    assert_eq!(style.padding_bottom, Length::px(30.0));
    assert_eq!(style.padding_left, Length::px(40.0));
}

#[test]
fn style_builder_margin() {
    let style = style_builder().margin(5, 10, 15, 20).build();
    assert_eq!(style.margin_top, Length::px(5.0));
    assert_eq!(style.margin_right, Length::px(10.0));
    assert_eq!(style.margin_bottom, Length::px(15.0));
    assert_eq!(style.margin_left, Length::px(20.0));
}

#[test]
fn style_builder_border_width() {
    let style = style_builder().border_width(1, 2, 3, 4).build();
    assert_eq!(style.border_top_width, 1);
    assert_eq!(style.border_right_width, 2);
    assert_eq!(style.border_bottom_width, 3);
    assert_eq!(style.border_left_width, 4);
    assert_eq!(style.border_top_style, BorderStyle::Solid);
}


// ═══════════════════════════════════════════════════════════════════════════
// §24  COMPREHENSIVE CONSTRAINT PARAMETERIZATION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn constrain_inline_size_at_200_in_100_500() {
    let c = SizeConstraint { min_inline_size: lu(100), max_inline_size: lu(500), min_block_size: lu(0), max_block_size: LayoutUnit::max() };
    assert_eq!(constrain_inline_size(lu(200), &c), lu(200));
}

#[test]
fn constrain_inline_size_at_50_in_100_500() {
    let c = SizeConstraint { min_inline_size: lu(100), max_inline_size: lu(500), min_block_size: lu(0), max_block_size: LayoutUnit::max() };
    assert_eq!(constrain_inline_size(lu(50), &c), lu(100));
}

#[test]
fn constrain_inline_size_at_600_in_100_500() {
    let c = SizeConstraint { min_inline_size: lu(100), max_inline_size: lu(500), min_block_size: lu(0), max_block_size: LayoutUnit::max() };
    assert_eq!(constrain_inline_size(lu(600), &c), lu(500));
}

#[test]
fn constrain_inline_size_at_0_in_0_max() {
    let c = SizeConstraint { min_inline_size: lu(0), max_inline_size: LayoutUnit::max(), min_block_size: lu(0), max_block_size: LayoutUnit::max() };
    assert_eq!(constrain_inline_size(lu(0), &c), lu(0));
}

#[test]
fn constrain_block_at_150_in_50_300() {
    let c = SizeConstraint { min_inline_size: lu(0), max_inline_size: LayoutUnit::max(), min_block_size: lu(50), max_block_size: lu(300) };
    assert_eq!(constrain_block_size(lu(150), &c), lu(150));
}

#[test]
fn constrain_block_at_20_in_50_300() {
    let c = SizeConstraint { min_inline_size: lu(0), max_inline_size: LayoutUnit::max(), min_block_size: lu(50), max_block_size: lu(300) };
    assert_eq!(constrain_block_size(lu(20), &c), lu(50));
}

#[test]
fn constrain_block_at_400_in_50_300() {
    let c = SizeConstraint { min_inline_size: lu(0), max_inline_size: LayoutUnit::max(), min_block_size: lu(50), max_block_size: lu(300) };
    assert_eq!(constrain_block_size(lu(400), &c), lu(300));
}

#[test]
fn resolve_inline_px_100_unconstrained() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_inline_size(&Length::px(100.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c), lu(100));
}

#[test]
fn resolve_inline_px_800_unconstrained() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_inline_size(&Length::px(800.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c), lu(800));
}

#[test]
fn resolve_inline_pct_10_of_500() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_inline_size(&Length::percent(10.0), lu(500), lu(500), BoxSizing::ContentBox, lu(0), &c), lu(50));
}

#[test]
fn resolve_inline_pct_90_of_500() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_inline_size(&Length::percent(90.0), lu(500), lu(500), BoxSizing::ContentBox, lu(0), &c), lu(450));
}

#[test]
fn resolve_block_px_50() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_block_size(&Length::px(50.0), lu(100), lu(600), BoxSizing::ContentBox, lu(0), &c), lu(50));
}

#[test]
fn resolve_block_px_600() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_block_size(&Length::px(600.0), lu(100), lu(600), BoxSizing::ContentBox, lu(0), &c), lu(600));
}

#[test]
fn resolve_block_pct_10_of_500() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_block_size(&Length::percent(10.0), lu(100), lu(500), BoxSizing::ContentBox, lu(0), &c), lu(50));
}

#[test]
fn resolve_block_pct_90_of_500() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_block_size(&Length::percent(90.0), lu(100), lu(500), BoxSizing::ContentBox, lu(0), &c), lu(450));
}

#[test]
fn resolve_block_auto_75() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_block_size(&Length::auto(), lu(75), lu(600), BoxSizing::ContentBox, lu(0), &c), lu(75));
}

#[test]
fn resolve_block_auto_0() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_block_size(&Length::auto(), lu(0), lu(600), BoxSizing::ContentBox, lu(0), &c), lu(0));
}

#[test]
fn resolve_inline_auto_200() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_inline_size(&Length::auto(), lu(200), lu(200), BoxSizing::ContentBox, lu(0), &c), lu(200));
}

// ═══════════════════════════════════════════════════════════════════════════
// §25  MORE LAYOUT INTEGRATION FOR SPECIFIC SCENARIOS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn layout_w_100_h_100_margin_10_all() {
    let mut b = BlockTestBuilder::new(800, 600);
    // Top margin collapses through parent, left margin applies
    b.add_child().width(100.0).height(100.0).margin(0, 10, 10, 10).done();
    let r = b.build();
    assert_layout!(r, child(0) at (10, 0) size (100, 100));
}

#[test]
fn layout_two_stacked_with_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    // Bottom margin doesn't collapse through, but top margin of first collapses
    b.add_child().width(100.0).height(50.0).margin(0, 0, 10, 0).done();
    b.add_child().width(100.0).height(50.0).margin(10, 0, 10, 0).done();
    let r = b.build();
    assert_layout!(r, child(0) at (0, 0) size (100, 50));
    // margin collapse: max(10, 10) = 10 between them
    assert_layout!(r, child(1) at (0, 60));
}

#[test]
fn layout_child_with_left_margin_50() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).margin(0, 0, 0, 50).done();
    let r = b.build();
    assert_layout!(r, child(0) at (50, 0) size (200, 50));
}

#[test]
fn layout_child_with_top_margin_30() {
    let mut b = BlockTestBuilder::new(800, 600);
    // Top margin collapses through parent boundary
    b.add_child().width(200.0).height(50.0).margin(30, 0, 0, 0).done();
    let r = b.build();
    assert_layout!(r, child(0) at (0, 0) size (200, 50));
}

#[test]
fn layout_w_pct_100_with_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(100.0).height(30.0).margin(0, 0, 0, 0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (800, 30));
}

#[test]
fn layout_nested_margin_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(80.0)
        .add_child().width(200.0).height(40.0).margin(0, 0, 0, 20).done()
        .done();
    let r = b.build();
    r.assert_nested_child_position(0, 0, 20, 0);
    r.assert_nested_child_size(0, 0, 200, 40);
}

#[test]
fn layout_pct_width_70() {
    let mut b = BlockTestBuilder::new(1000, 500);
    b.add_child().width_pct(70.0).height(50.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (700, 50));
}

#[test]
fn layout_pct_width_15() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(15.0).height(40.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (120, 40));
}

#[test]
fn layout_pct_height_75() {
    let mut b = BlockTestBuilder::new(800, 400);
    b.add_child().width(100.0).height_pct(75.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 300));
}

#[test]
fn layout_pct_height_10() {
    let mut b = BlockTestBuilder::new(800, 1000);
    b.add_child().width(100.0).height_pct(10.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 100));
}

// ═══════════════════════════════════════════════════════════════════════════
// §26  COMPREHENSIVE BOX-SIZING ADJUSTMENT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn box_sizing_cb_100_pb_0() {
    assert_eq!(apply_box_sizing_adjustment(lu(100), BoxSizing::ContentBox, lu(0)), lu(100));
}

#[test]
fn box_sizing_cb_100_pb_50() {
    assert_eq!(apply_box_sizing_adjustment(lu(100), BoxSizing::ContentBox, lu(50)), lu(100));
}

#[test]
fn box_sizing_bb_100_pb_0() {
    assert_eq!(apply_box_sizing_adjustment(lu(100), BoxSizing::BorderBox, lu(0)), lu(100));
}

#[test]
fn box_sizing_bb_100_pb_30() {
    assert_eq!(apply_box_sizing_adjustment(lu(100), BoxSizing::BorderBox, lu(30)), lu(70));
}

#[test]
fn box_sizing_bb_100_pb_100() {
    assert_eq!(apply_box_sizing_adjustment(lu(100), BoxSizing::BorderBox, lu(100)), lu(0));
}

#[test]
fn box_sizing_bb_100_pb_150() {
    assert_eq!(apply_box_sizing_adjustment(lu(100), BoxSizing::BorderBox, lu(150)), lu(0));
}

#[test]
fn box_sizing_bb_500_pb_80() {
    assert_eq!(apply_box_sizing_adjustment(lu(500), BoxSizing::BorderBox, lu(80)), lu(420));
}

#[test]
fn box_sizing_bb_1_pb_0() {
    assert_eq!(apply_box_sizing_adjustment(lu(1), BoxSizing::BorderBox, lu(0)), lu(1));
}

#[test]
fn box_sizing_bb_1_pb_1() {
    assert_eq!(apply_box_sizing_adjustment(lu(1), BoxSizing::BorderBox, lu(1)), lu(0));
}

// ═══════════════════════════════════════════════════════════════════════════
// §27  REMAINING TESTS TO REACH 600+
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn layout_four_children_stacked() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(40.0).done();
    b.add_child().width(100.0).height(50.0).done();
    b.add_child().width(100.0).height(60.0).done();
    b.add_child().width(100.0).height(70.0).done();
    let r = b.build();
    assert_layout!(r, child(0) at (0, 0) size (100, 40));
    assert_layout!(r, child(1) at (0, 40) size (100, 50));
    assert_layout!(r, child(2) at (0, 90) size (100, 60));
    assert_layout!(r, child(3) at (0, 150) size (100, 70));
    assert_layout!(r, child_count 4);
}

#[test]
fn layout_child_wider_than_container() {
    let mut b = BlockTestBuilder::new(300, 200);
    b.add_child().width(500.0).height(50.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (500, 50));
}

#[test]
fn layout_child_taller_than_container() {
    let mut b = BlockTestBuilder::new(300, 200);
    b.add_child().width(100.0).height(500.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 500));
}

#[test]
fn resolve_constraints_border_box_all() {
    let c = resolve_size_constraints(
        &Length::px(200.0), &Length::px(600.0),
        &Length::px(100.0), &Length::px(400.0),
        lu(800), lu(600), BoxSizing::BorderBox, lu(40), lu(30),
    );
    assert_eq!(c.min_inline_size, lu(160));
    assert_eq!(c.max_inline_size, lu(560));
    assert_eq!(c.min_block_size, lu(70));
    assert_eq!(c.max_block_size, lu(370));
}

#[test]
fn layout_nested_three_levels_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(600.0).height(100.0)
        .add_child().width(400.0).height(80.0).done()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) size (600, 100));
    r.assert_nested_child_size(0, 0, 400, 80);
}

#[test]
fn layout_min_w_250_width_200() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).min_width(250.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (250, 30));
}

#[test]
fn layout_max_w_150_width_200() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).max_width(150.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (150, 30));
}

#[test]
fn layout_min_h_120_height_80() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(80.0).min_height(120.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 120));
}

#[test]
fn layout_max_h_60_height_100() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(100.0).max_height(60.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 60));
}

#[test]
fn layout_min_w_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).min_width(200.0).height(30.0).padding(0, 10, 0, 10).done();
    let r = b.build();
    // content width clamped to min 200, border-box = 200 + 10+10 = 220
    assert_layout!(r, child(0) size (220, 30));
}

#[test]
fn layout_max_w_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).max_width(300.0).height(30.0).padding(0, 10, 0, 10).done();
    let r = b.build();
    // content width clamped to max 300, border-box = 300 + 10+10 = 320
    assert_layout!(r, child(0) size (320, 30));
}

#[test]
fn layout_min_h_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(30.0).min_height(100.0).padding(10, 0, 10, 0).done();
    let r = b.build();
    // content height clamped to min 100, border-box = 100 + 10+10 = 120
    assert_layout!(r, child(0) size (100, 120));
}

#[test]
fn layout_max_h_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(300.0).max_height(200.0).padding(10, 0, 10, 0).done();
    let r = b.build();
    // content height clamped to max 200, border-box = 200 + 10+10 = 220
    assert_layout!(r, child(0) size (100, 220));
}

#[test]
fn layout_border_box_min_w() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).min_width(200.0).height(30.0).padding(0, 10, 0, 10).box_sizing_border_box().done();
    let r = b.build();
    assert_layout!(r, child(0) size (200, 30));
}

#[test]
fn layout_border_box_max_w() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).max_width(300.0).height(30.0).padding(0, 10, 0, 10).box_sizing_border_box().done();
    let r = b.build();
    assert_layout!(r, child(0) size (300, 30));
}

#[test]
fn layout_border_box_min_h() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).min_height(100.0).padding(10, 0, 10, 0).box_sizing_border_box().done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 100));
}

#[test]
fn layout_border_box_max_h() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(400.0).max_height(200.0).padding(10, 0, 10, 0).box_sizing_border_box().done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 200));
}

#[test]
fn intrinsic_text_a_b_c() {
    let mut doc = Document::new();
    let root = doc.root();
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("a b c".to_string());
    doc.append_child(root, text);
    let sizes = compute_intrinsic_inline_sizes(&doc, text);
    assert_eq!(sizes.min, luf(8.0)); // single char * 8
    assert_eq!(sizes.max, luf(40.0)); // 5 chars * 8
}

#[test]
fn intrinsic_text_empty() {
    let mut doc = Document::new();
    let root = doc.root();
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("".to_string());
    doc.append_child(root, text);
    let sizes = compute_intrinsic_inline_sizes(&doc, text);
    assert_eq!(sizes.min, luf(0.0));
    assert_eq!(sizes.max, luf(0.0));
}

#[test]
fn intrinsic_text_single_long_word() {
    let mut doc = Document::new();
    let root = doc.root();
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("supercalifragilistic".to_string());
    doc.append_child(root, text);
    let sizes = compute_intrinsic_inline_sizes(&doc, text);
    // 20 chars * 8 = 160
    assert_eq!(sizes.min, luf(160.0));
    assert_eq!(sizes.max, luf(160.0));
}

#[test]
fn min_max_sizes_new() {
    let s = MinMaxSizes::new(lu(50), lu(200));
    assert_eq!(s.min, lu(50));
    assert_eq!(s.max, lu(200));
}

#[test]
fn min_max_sizes_equal() {
    let s = MinMaxSizes::new(lu(100), lu(100));
    assert_eq!(s.min, lu(100));
    assert_eq!(s.max, lu(100));
}

#[test]
fn min_max_sizes_zero() {
    let s = MinMaxSizes::new(lu(0), lu(0));
    assert_eq!(s.min, lu(0));
    assert_eq!(s.max, lu(0));
}

#[test]
fn layout_container_300_300() {
    let r = BlockTestBuilder::new(300, 300).build();
    assert_layout!(r, container width 300);
    assert_layout!(r, container height 300);
}

#[test]
fn layout_container_1920_1080() {
    let r = BlockTestBuilder::new(1920, 1080).build();
    assert_layout!(r, container width 1920);
    assert_layout!(r, container height 1080);
}

#[test]
fn layout_w_10_in_10_container() {
    let mut b = BlockTestBuilder::new(10, 10);
    b.add_child().width(10.0).height(10.0).done();
    let r = b.build();
    assert_layout!(r, child(0) at (0, 0) size (10, 10));
}

#[test]
fn layout_auto_both_with_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .add_child().width(300.0).height(150.0).done()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) size (800, 150));
}

#[test]
fn layout_auto_both_with_two_children() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .add_child().width(200.0).height(80.0).done()
        .add_child().width(300.0).height(90.0).done()
        .done();
    let r = b.build();
    assert_layout!(r, child(0) size (800, 170));
}

#[test]
fn layout_nested_border_box_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(200.0)
        .add_child().width(300.0).height(100.0).padding(10, 10, 10, 10).done()
        .done();
    let r = b.build();
    r.assert_nested_child_size(0, 0, 320, 120);
}

#[test]
fn layout_nested_border_box_inner() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(200.0)
        .add_child().width(300.0).height(100.0).padding(10, 10, 10, 10)
            .with_style(|s| s.box_sizing = BoxSizing::BorderBox).done()
        .done();
    let r = b.build();
    r.assert_nested_child_size(0, 0, 300, 100);
}


// ═══════════════════════════════════════════════════════════════════════════
// §28  FINAL BATCH — SIZING COMPLETENESS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn layout_w_350() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(350.0).height(50.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (350, 50));
}

#[test]
fn layout_w_450() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(450.0).height(50.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (450, 50));
}

#[test]
fn layout_w_550() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(550.0).height(50.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (550, 50));
}

#[test]
fn layout_w_650() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(650.0).height(50.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (650, 50));
}

#[test]
fn layout_h_25() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(25.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 25));
}

#[test]
fn layout_h_75() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(75.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 75));
}

#[test]
fn layout_h_125() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(125.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 125));
}

#[test]
fn layout_h_175() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(175.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 175));
}

#[test]
fn layout_w_pct_5() {
    let mut b = BlockTestBuilder::new(1000, 500);
    b.add_child().width_pct(5.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (50, 30));
}

#[test]
fn layout_w_pct_95() {
    let mut b = BlockTestBuilder::new(1000, 500);
    b.add_child().width_pct(95.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (950, 30));
}

#[test]
fn layout_h_pct_5() {
    let mut b = BlockTestBuilder::new(800, 1000);
    b.add_child().width(100.0).height_pct(5.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 50));
}

#[test]
fn layout_h_pct_95() {
    let mut b = BlockTestBuilder::new(800, 1000);
    b.add_child().width(100.0).height_pct(95.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 950));
}

#[test]
fn layout_centered_600_in_1200() {
    let mut b = BlockTestBuilder::new(1200, 600);
    b.add_child().width(600.0).height(50.0).margin_auto_horizontal().done();
    let r = b.build();
    assert_layout!(r, child(0) at (300, 0) size (600, 50));
}

#[test]
fn layout_centered_100_in_800() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(30.0).margin_auto_horizontal().done();
    let r = b.build();
    assert_layout!(r, child(0) at (350, 0) size (100, 30));
}

#[test]
fn resolve_inline_constrained_min_200() {
    let c = resolve_size_constraints(
        &Length::px(200.0), &Length::none(),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_inline_size(&Length::px(150.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(200));
}

#[test]
fn resolve_inline_constrained_max_300() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::px(300.0),
        &Length::auto(), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_inline_size(&Length::px(500.0), lu(800), lu(800), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(300));
}

#[test]
fn resolve_block_constrained_min_100() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::px(100.0), &Length::none(),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_block_size(&Length::auto(), lu(50), lu(600), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(100));
}

#[test]
fn resolve_block_constrained_max_200() {
    let c = resolve_size_constraints(
        &Length::auto(), &Length::none(),
        &Length::auto(), &Length::px(200.0),
        lu(800), lu(600), BoxSizing::ContentBox, lu(0), lu(0),
    );
    let result = resolve_block_size(&Length::auto(), lu(500), lu(600), BoxSizing::ContentBox, lu(0), &c);
    assert_eq!(result, lu(200));
}

#[test]
fn ar_9_21_h_900() {
    let (_w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(900), (9.0, 21.0));
    assert_eq!(h, lu(900));
}

#[test]
fn ar_1_2_w_100() {
    let (w, h) = apply_aspect_ratio(lu(100), INDEFINITE_SIZE, (1.0, 2.0));
    assert_eq!(w, lu(100));
    assert_eq!(h, lu(200));
}

#[test]
fn ar_2_1_w_50() {
    let (w, h) = apply_aspect_ratio(lu(50), INDEFINITE_SIZE, (2.0, 1.0));
    assert_eq!(w, lu(50));
    assert_eq!(h, lu(25));
}

#[test]
fn ar_3_4_w_300() {
    let (w, h) = apply_aspect_ratio(lu(300), INDEFINITE_SIZE, (3.0, 4.0));
    assert_eq!(w, lu(300));
    assert_eq!(h, lu(400));
}

#[test]
fn ar_4_3_w_120() {
    let (w, h) = apply_aspect_ratio(lu(120), INDEFINITE_SIZE, (4.0, 3.0));
    assert_eq!(w, lu(120));
    assert_eq!(h, lu(90));
}

#[test]
fn intrinsic_sizes_large_values() {
    let sizes = IntrinsicSizes {
        min_content_inline_size: lu(10000),
        max_content_inline_size: lu(50000),
        min_content_block_size: lu(5000),
        max_content_block_size: lu(20000),
    };
    assert_eq!(sizes.min_content_inline_size, lu(10000));
    assert_eq!(sizes.max_content_inline_size, lu(50000));
}

#[test]
fn layout_nested_pct_width_in_fixed() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(500.0).height(100.0)
        .add_child().with_style(|s| s.width = Length::percent(60.0)).height(50.0).done()
        .done();
    let r = b.build();
    r.assert_nested_child_size(0, 0, 300, 50);
}

#[test]
fn layout_nested_auto_width_fills_parent() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(100.0)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_nested_child_size(0, 0, 400, 50);
}

#[test]
fn layout_child_count_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    for _ in 0..5 {
        b.add_child().width(100.0).height(20.0).done();
    }
    let r = b.build();
    assert_layout!(r, child_count 5);
}

#[test]
fn layout_child_count_1() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(20.0).done();
    let r = b.build();
    assert_layout!(r, child_count 1);
}

#[test]
fn resolve_inline_auto_small_container() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_inline_size(&Length::auto(), lu(50), lu(50), BoxSizing::ContentBox, lu(0), &c), lu(50));
}

#[test]
fn resolve_block_pct_25_of_800() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_block_size(&Length::percent(25.0), lu(50), lu(800), BoxSizing::ContentBox, lu(0), &c), lu(200));
}

#[test]
fn resolve_block_pct_75_of_400() {
    let c = SizeConstraint::unconstrained();
    assert_eq!(resolve_block_size(&Length::percent(75.0), lu(50), lu(400), BoxSizing::ContentBox, lu(0), &c), lu(300));
}


// ═══════════════════════════════════════════════════════════════════════════
// §29  FINAL PUSH TO 600+
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn layout_w_750_in_800() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(750.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (750, 30));
}

#[test]
fn layout_w_799_in_800() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(799.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (799, 30));
}

#[test]
fn layout_w_801_in_800() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(801.0).height(30.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (801, 30));
}

#[test]
fn layout_h_599() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(599.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 599));
}

#[test]
fn layout_h_601() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(601.0).done();
    let r = b.build();
    assert_layout!(r, child(0) size (100, 601));
}

#[test]
fn resolve_inline_bb_pct_50() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(&Length::percent(50.0), lu(800), lu(800), BoxSizing::BorderBox, lu(40), &c);
    // 50% of 800 = 400 border-box, content = 400 - 40 = 360
    assert_eq!(result, lu(360));
}

#[test]
fn resolve_block_bb_pct_50() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(&Length::percent(50.0), lu(100), lu(600), BoxSizing::BorderBox, lu(40), &c);
    // 50% of 600 = 300 border-box, content = 300 - 40 = 260
    assert_eq!(result, lu(260));
}

#[test]
fn resolve_inline_bb_auto_no_subtraction() {
    let c = SizeConstraint::unconstrained();
    // auto with border-box still subtracts pb from the fill-available value
    let result = resolve_inline_size(&Length::auto(), lu(500), lu(500), BoxSizing::BorderBox, lu(40), &c);
    assert_eq!(result, lu(460));
}

#[test]
fn constrain_inline_mid_range() {
    let c = SizeConstraint { min_inline_size: lu(50), max_inline_size: lu(250), min_block_size: lu(0), max_block_size: LayoutUnit::max() };
    assert_eq!(constrain_inline_size(lu(150), &c), lu(150));
}

#[test]
fn constrain_block_mid_range() {
    let c = SizeConstraint { min_inline_size: lu(0), max_inline_size: LayoutUnit::max(), min_block_size: lu(30), max_block_size: lu(200) };
    assert_eq!(constrain_block_size(lu(100), &c), lu(100));
}

#[test]
fn ar_1_1_h_350() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(350), (1.0, 1.0));
    assert_eq!(w, lu(350));
    assert_eq!(h, lu(350));
}

#[test]
fn ar_16_9_both_definite() {
    let (w, h) = apply_aspect_ratio(lu(1920), lu(1080), (16.0, 9.0));
    assert_eq!(w, lu(1920));
    assert_eq!(h, lu(1080));
}

#[test]
fn sizing_kw_auto_debug() {
    let auto = SizingKeyword::Auto;
    let debug = format!("{:?}", auto);
    assert!(debug.contains("Auto"));
}

#[test]
fn sizing_kw_stretch_debug() {
    let stretch = SizingKeyword::Stretch;
    let debug = format!("{:?}", stretch);
    assert!(debug.contains("Stretch"));
}

#[test]
fn replaced_w_only_1_px() {
    let mut style = ComputedStyle::initial();
    style.width = Length::px(1.0);
    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, luf(1.0));
}

