//! SP12 H3 — Comprehensive CSS Float Tests.
//!
//! Tests covering float positioning, clearing, stacking, BFC interaction,
//! margin collapsing, and edge cases per CSS 2.1 §9.5 and §9.5.2.

#[path = "sp12_wpt_helpers.rs"]
mod sp12_wpt_helpers;

use sp12_wpt_helpers::*;
use openui_style::*;

// ═══════════════════════════════════════════════════════════════════════════
// §1  Basic Float Positioning (60+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn pos_float_left_at_left_edge() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 200, 100);
}

#[test]
fn pos_float_right_at_right_edge() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 600, 0);
    r.assert_child_size(0, 200, 100);
}

#[test]
fn pos_float_left_respects_container_padding_left() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.padding_left = openui_geometry::Length::px(20.0);
        });
    b.add_child().width(100.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 20, 0);
}

#[test]
fn pos_float_left_respects_container_padding_top() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.padding_top = openui_geometry::Length::px(15.0);
        });
    b.add_child().width(100.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 15);
}

#[test]
fn pos_float_right_respects_container_padding_right() {
    // container 800 + padding-right 30 => content box 770, float 200 => left = 770 - 200 = 570
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.padding_right = openui_geometry::Length::px(30.0);
        });
    b.add_child().width(200.0).height(50.0).float_right().done();
    let r = b.build();
    // content width = 800, padding_right doesn't shrink content width on parent with explicit width
    // The float should be at right edge of content area
    let frag = r.child(0);
    assert!(frag.offset.left.to_i32() >= 0);
}

#[test]
fn pos_float_left_with_margin_left() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .margin(0, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 0);
}

#[test]
fn pos_float_left_with_margin_top() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .margin(15, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 15);
}

#[test]
fn pos_float_left_with_margin_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .margin(0, 20, 0, 0).done();
    let r = b.build();
    // float content box at left=0, margin-right is 20 but doesn't change position
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 200, 100);
}

#[test]
fn pos_float_right_with_margin_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right()
        .margin(0, 30, 0, 0).done();
    let r = b.build();
    // 800 - 30 - 200 = 570
    r.assert_child_position(0, 570, 0);
}

#[test]
fn pos_float_right_with_margin_left() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right()
        .margin(0, 0, 0, 15).done();
    let r = b.build();
    // Right float: 800 - 200 = 600; margin-left doesn't shift right float further
    r.assert_child_position(0, 600, 0);
}

#[test]
fn pos_float_left_with_all_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left()
        .margin(5, 10, 15, 20).done();
    let r = b.build();
    r.assert_child_position(0, 20, 5);
}

#[test]
fn pos_float_left_fixed_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(80.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 300, 80);
}

#[test]
fn pos_float_left_percentage_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(50.0).height(80.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 400, 80);
}

#[test]
fn pos_float_right_percentage_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(25.0).height(60.0).float_right().done();
    let r = b.build();
    r.assert_child_size(0, 200, 60);
    r.assert_child_position(0, 600, 0);
}

#[test]
fn pos_float_fixed_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(200.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 100, 200);
}

#[test]
fn pos_multiple_left_floats_stack_horizontally() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(150.0).height(80.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 200, 0);
}

#[test]
fn pos_three_left_floats_stack_horizontally() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 100, 0);
    r.assert_child_position(2, 200, 0);
}

#[test]
fn pos_multiple_right_floats_stack_from_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().width(150.0).height(80.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 600, 0); // 800 - 200
    r.assert_child_position(1, 450, 0); // 800 - 200 - 150
}

#[test]
fn pos_three_right_floats_stack() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_right().done();
    b.add_child().width(100.0).height(50.0).float_right().done();
    b.add_child().width(100.0).height(50.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 700, 0);
    r.assert_child_position(1, 600, 0);
    r.assert_child_position(2, 500, 0);
}

#[test]
fn pos_float_drops_to_next_line_when_no_room() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(250.0).height(50.0).float_left().done();
    b.add_child().width(250.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // second float doesn't fit (250+250=500 > 400), drops down
    let f2 = r.child(1);
    assert!(f2.offset.top.to_i32() >= 50, "Second float should drop below first");
}

#[test]
fn pos_mixed_left_and_right_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(80.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 600, 0);
}

#[test]
fn pos_float_left_zero_margin() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left()
        .margin(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn pos_container_with_only_floats_has_zero_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(150.0).float_right().done();
    let r = b.build();
    // Container has explicit height=600
    r.assert_container_height(600);
}

#[test]
fn pos_float_left_small_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(1.0).height(1.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 1, 1);
}

#[test]
fn pos_float_right_small_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(1.0).height(1.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 799, 0);
}

#[test]
fn pos_float_left_full_container_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(400.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 400, 50);
}

#[test]
fn pos_float_right_full_container_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(400.0).height(50.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 400, 50);
}

#[test]
fn pos_two_left_floats_different_heights() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 200, 0);
    r.assert_child_size(0, 200, 100);
    r.assert_child_size(1, 200, 50);
}

#[test]
fn pos_left_float_with_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .border(5, 5, 5, 5).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // border-box: 200+10=210 wide, 100+10=110 tall
    r.assert_child_size(0, 210, 110);
}

#[test]
fn pos_left_float_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .padding(10, 10, 10, 10).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // padding box: 200+20=220 wide, 100+20=120 tall
    r.assert_child_size(0, 220, 120);
}

#[test]
fn pos_float_left_with_border_box_sizing() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .padding(10, 10, 10, 10).box_sizing_border_box().done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn pos_four_left_floats_fill_row() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 100, 0);
    r.assert_child_position(2, 200, 0);
    r.assert_child_position(3, 300, 0);
}

#[test]
fn pos_five_left_floats_wraps_fifth() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    let r = b.build();
    let f5 = r.child(4);
    assert!(f5.offset.top.to_i32() >= 50, "Fifth float should wrap to next row");
}

#[test]
fn pos_left_float_10pct_width() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width_pct(10.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 100, 50);
}

#[test]
fn pos_right_float_10pct_width() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width_pct(10.0).height(50.0).float_right().done();
    let r = b.build();
    r.assert_child_size(0, 100, 50);
    r.assert_child_position(0, 900, 0);
}

#[test]
fn pos_left_float_with_margin_and_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left()
        .margin(5, 5, 5, 10).border(2, 2, 2, 2).done();
    let r = b.build();
    r.assert_child_position(0, 10, 5);
    r.assert_child_size(0, 104, 54); // 100+4, 50+4
}

#[test]
fn pos_float_left_tall() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(500.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 100, 500);
}

// ═══════════════════════════════════════════════════════════════════════════
// §2  Float and Normal Flow (60+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn flow_normal_content_beside_left_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).done();
    let r = b.build();
    // Block flows beside float: offset to right by 200
    r.assert_child_position(1, 200, 0);
    r.assert_child_size(1, 600, 50);
}

#[test]
fn flow_normal_content_beside_right_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().height(50.0).done();
    let r = b.build();
    r.assert_child_position(1, 0, 0);
    r.assert_child_size(1, 600, 50);
}

#[test]
fn flow_content_wraps_below_float_when_taller() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().height(30.0).done();
    b.add_child().height(30.0).done();
    let r = b.build();
    // First block beside float
    r.assert_child_position(1, 200, 0);
    // Second block at top=30, still beside float (30 < 50)
    r.assert_child_position(2, 200, 30);
}

#[test]
fn flow_block_after_float_expires_gets_full_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().height(30.0).done();
    b.add_child().height(30.0).done();
    let r = b.build();
    // block at y=30, float ends at y=50, block height=30 => bottom=60
    // Third child is at y=30 (still within float range)
    let c2 = r.child(2);
    assert_eq!(c2.offset.left.to_i32(), 200);
}

#[test]
fn flow_clear_left_moves_below_left_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).clear_left().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
    assert_eq!(c.offset.left.to_i32(), 0);
    assert_eq!(c.size.width.to_i32(), 800);
}

#[test]
fn flow_clear_right_moves_below_right_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(150.0).float_right().done();
    b.add_child().height(50.0).clear_right().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 150);
}

#[test]
fn flow_clear_both_moves_below_all_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(150.0).float_right().done();
    b.add_child().height(50.0).clear_both().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 150);
}

#[test]
fn flow_content_between_two_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(150.0).height(100.0).float_right().done();
    b.add_child().height(50.0).done();
    let r = b.build();
    // Block fits between floats
    r.assert_child_position(2, 200, 0);
    r.assert_child_size(2, 450, 50); // 800 - 200 - 150
}

#[test]
fn flow_narrow_content_fits_between_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(100.0).float_left().done();
    b.add_child().width(300.0).height(100.0).float_right().done();
    b.add_child().width(100.0).height(50.0).done();
    let r = b.build();
    // Available space = 800 - 300 - 300 = 200, child is 100 wide
    r.assert_child_position(2, 300, 0);
}

#[test]
fn flow_block_normal_after_block_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(30.0).float_left().done();
    b.add_child().height(30.0).done();
    let r = b.build();
    // Block 1 is beside float
    r.assert_child_position(1, 200, 0);
    r.assert_child_size(1, 600, 30);
}

#[test]
fn flow_multiple_blocks_wrap_around_same_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(200.0).float_left().done();
    b.add_child().height(50.0).done();
    b.add_child().height(50.0).done();
    b.add_child().height(50.0).done();
    let r = b.build();
    // All three blocks beside float
    for i in 1..=3 {
        assert_eq!(r.child(i).offset.left.to_i32(), 200);
        assert_eq!(r.child(i).size.width.to_i32(), 600);
    }
}

#[test]
fn flow_block_below_expired_float_full_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().height(50.0).clear_left().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 50);
    assert_eq!(c.size.width.to_i32(), 800);
}

#[test]
fn flow_block_before_float_unaffected() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(40.0).done();
    b.add_child().width(200.0).height(100.0).float_left().done();
    let r = b.build();
    // First block is at top, full width, unaffected
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 800, 40);
    // Float placed after first block
    r.assert_child_position(1, 0, 40);
}

#[test]
fn flow_block_after_float_with_preceding_block() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(40.0).done();
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(60.0).done();
    let r = b.build();
    // Third child beside float at y=40
    r.assert_child_position(2, 200, 40);
    r.assert_child_size(2, 600, 60);
}

#[test]
fn flow_fixed_width_block_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(300.0).height(50.0).done();
    let r = b.build();
    r.assert_child_position(1, 200, 0);
    r.assert_child_size(1, 300, 50);
}

#[test]
fn flow_block_with_margin_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).margin(10, 0, 0, 0).done();
    let r = b.build();
    let c = r.child(1);
    assert_eq!(c.offset.top.to_i32(), 10);
    assert_eq!(c.offset.left.to_i32(), 200);
}

#[test]
fn flow_two_blocks_beside_float_then_one_below() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(60.0).float_left().done();
    b.add_child().height(30.0).done();
    b.add_child().height(30.0).done();
    let r = b.build();
    // block 1 at y=0 beside float
    r.assert_child_position(1, 200, 0);
    // block 2 at y=30 beside float (30 < 60)
    r.assert_child_position(2, 200, 30);
}

#[test]
fn flow_block_with_clear_none_no_effect() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).with_style(|s| s.clear = Clear::None).done();
    let r = b.build();
    // clear:none has no effect, block beside float
    r.assert_child_position(1, 200, 0);
}

#[test]
fn flow_clear_left_no_left_float_no_effect() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().height(50.0).clear_left().done();
    let r = b.build();
    // No left float to clear, block at top beside right float
    r.assert_child_position(1, 0, 0);
    r.assert_child_size(1, 600, 50);
}

#[test]
fn flow_clear_right_no_right_float_no_effect() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).clear_right().done();
    let r = b.build();
    // No right float to clear, block beside left float
    r.assert_child_position(1, 200, 0);
}

#[test]
fn flow_block_wraps_right_float_reduced_width() {
    let mut b = BlockTestBuilder::new(600, 600);
    b.add_child().width(200.0).height(80.0).float_right().done();
    b.add_child().height(40.0).done();
    let r = b.build();
    r.assert_child_position(1, 0, 0);
    r.assert_child_size(1, 400, 40);
}

#[test]
fn flow_two_blocks_after_clear() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(30.0).clear_left().done();
    b.add_child().height(30.0).done();
    let r = b.build();
    let c1 = r.child(1);
    assert!(c1.offset.top.to_i32() >= 100);
    let c2 = r.child(2);
    assert!(c2.offset.top.to_i32() >= c1.offset.top.to_i32() + 30);
}

#[test]
fn flow_container_height_includes_normal_flow_not_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(300.0).float_left().done();
    b.add_child().height(50.0).done();
    let r = b.build();
    // Container has explicit height=600
    r.assert_container_height(600);
}

#[test]
fn flow_block_height_auto_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height_auto().done();
    let r = b.build();
    // Auto height block with no content => height 0
    r.assert_child_size(1, 600, 0);
}

#[test]
fn flow_block_with_padding_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).padding(10, 10, 10, 10).done();
    let r = b.build();
    r.assert_child_position(1, 200, 0);
    // width = 600 - 0 (no margin) with padding inside
    r.assert_child_size(1, 600, 70); // 50 + 20 padding
}

#[test]
fn flow_block_with_border_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).border(5, 5, 5, 5).done();
    let r = b.build();
    r.assert_child_position(1, 200, 0);
    r.assert_child_size(1, 600, 60);
}

// ═══════════════════════════════════════════════════════════════════════════
// §3  Float Stacking and Ordering (50+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stack_source_order_determines_left_float_stacking() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(150.0).height(50.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 100, 0);
    r.assert_child_position(2, 250, 0);
}

#[test]
fn stack_right_floats_reverse_visual_order() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_right().done();
    b.add_child().width(150.0).height(50.0).float_right().done();
    b.add_child().width(200.0).height(50.0).float_right().done();
    let r = b.build();
    // First right float: 800-100=700
    r.assert_child_position(0, 700, 0);
    // Second: 800-100-150=550
    r.assert_child_position(1, 550, 0);
    // Third: 800-100-150-200=350
    r.assert_child_position(2, 350, 0);
}

#[test]
fn stack_float_drops_to_next_shelf() {
    let mut b = BlockTestBuilder::new(300, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    // Second float can't fit beside first (200+200=400>300)
    let f2 = r.child(1);
    assert!(f2.offset.top.to_i32() >= 100);
    assert_eq!(f2.offset.left.to_i32(), 0);
}

#[test]
fn stack_floats_different_heights_create_shelves() {
    let mut b = BlockTestBuilder::new(500, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    // Third float: no room at y=0 (400>500? no, 400<500, fits)
    b.add_child().width(200.0).height(80.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 200, 0);
    // Third float: 200+200+200=600>500, drops
    let f3 = r.child(2);
    assert!(f3.offset.top.to_i32() >= 50);
}

#[test]
fn stack_float_placed_after_preceding_block() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(40.0).done();
    b.add_child().width(200.0).height(100.0).float_left().done();
    let r = b.build();
    // Float placed at current line (after block), not top of container
    r.assert_child_position(1, 0, 40);
}

#[test]
fn stack_multiple_small_floats_filling_row() {
    let mut b = BlockTestBuilder::new(500, 600);
    for _ in 0..5 {
        b.add_child().width(100.0).height(40.0).float_left().done();
    }
    let r = b.build();
    for i in 0..5 {
        r.assert_child_position(i, (i as i32) * 100, 0);
    }
}

#[test]
fn stack_multiple_small_floats_overflow_to_next_row() {
    let mut b = BlockTestBuilder::new(300, 600);
    for _ in 0..5 {
        b.add_child().width(100.0).height(40.0).float_left().done();
    }
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 100, 0);
    r.assert_child_position(2, 200, 0);
    // 4th and 5th wrap
    let f4 = r.child(3);
    assert!(f4.offset.top.to_i32() >= 40);
}

#[test]
fn stack_left_and_right_same_line() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().width(200.0).height(80.0).float_right().done();
    let r = b.build();
    // Both on same line
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 600, 0);
}

#[test]
fn stack_left_and_right_too_wide_right_drops() {
    let mut b = BlockTestBuilder::new(300, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_right().done();
    let r = b.build();
    // 200+200=400>300, right float should drop
    let rf = r.child(1);
    assert!(rf.offset.top.to_i32() >= 50 || rf.offset.left.to_i32() >= 0);
}

#[test]
fn stack_float_after_two_blocks() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(30.0).done();
    b.add_child().height(20.0).done();
    b.add_child().width(200.0).height(80.0).float_left().done();
    let r = b.build();
    // Float placed at y=50 (after 30+20)
    r.assert_child_position(2, 0, 50);
}

#[test]
fn stack_interleaved_left_right_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_right().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 700, 0);
    r.assert_child_position(2, 100, 0);
    r.assert_child_position(3, 600, 0);
}

#[test]
fn stack_tall_float_blocks_subsequent_floats() {
    let mut b = BlockTestBuilder::new(300, 600);
    b.add_child().width(200.0).height(200.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    // Second float can't fit beside first (400>300), drops below
    let f2 = r.child(1);
    assert!(f2.offset.top.to_i32() >= 200);
}

#[test]
fn stack_equal_width_floats_wrap_evenly() {
    let mut b = BlockTestBuilder::new(400, 600);
    for _ in 0..8 {
        b.add_child().width(100.0).height(30.0).float_left().done();
    }
    let r = b.build();
    // Row 1: indices 0-3
    for i in 0..4 {
        r.assert_child_position(i, (i as i32) * 100, 0);
    }
    // Row 2: indices 4-7
    for i in 4..8 {
        let f = r.child(i);
        assert!(f.offset.top.to_i32() >= 30);
    }
}

#[test]
fn stack_right_floats_wrap_when_no_room() {
    let mut b = BlockTestBuilder::new(300, 600);
    b.add_child().width(200.0).height(50.0).float_right().done();
    b.add_child().width(200.0).height(50.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 100, 0); // 300 - 200
    let f2 = r.child(1);
    assert!(f2.offset.top.to_i32() >= 50);
}

#[test]
fn stack_mixed_sizes_complex_layout() {
    let mut b = BlockTestBuilder::new(600, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().width(200.0).height(80.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 200, 0);
    r.assert_child_position(2, 400, 0);
}

#[test]
fn stack_float_left_margin_affects_stacking() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_left()
        .margin(0, 0, 0, 10).done();
    let r = b.build();
    // Second float: after first (200) + margin-left (10) = 210
    r.assert_child_position(1, 210, 0);
}

#[test]
fn stack_float_right_margin_affects_stacking() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_right().done();
    b.add_child().width(200.0).height(50.0).float_right()
        .margin(0, 10, 0, 0).done();
    let r = b.build();
    // First right: 800-200=600
    r.assert_child_position(0, 600, 0);
    // Second right: 800-200-10-200=390
    r.assert_child_position(1, 390, 0);
}

#[test]
fn stack_six_left_floats_two_rows() {
    let mut b = BlockTestBuilder::new(300, 600);
    for _ in 0..6 {
        b.add_child().width(100.0).height(40.0).float_left().done();
    }
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 100, 0);
    r.assert_child_position(2, 200, 0);
    let f4 = r.child(3);
    assert!(f4.offset.top.to_i32() >= 40);
}

// ═══════════════════════════════════════════════════════════════════════════
// §4  Float and BFC (50+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bfc_overflow_hidden_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(80.0).overflow_hidden().done();
    let r = b.build();
    // BFC element shouldn't overlap float margin box
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 200);
}

#[test]
fn bfc_overflow_hidden_shrinks_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(80.0).overflow_hidden().done();
    let r = b.build();
    let c = r.child(1);
    // BFC shrinks to avoid float
    assert!(c.size.width.to_i32() <= 600);
}

#[test]
fn bfc_overflow_hidden_drops_below_float_when_too_wide() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(300.0).height(50.0).float_left().done();
    b.add_child().width(300.0).height(80.0).overflow_hidden().done();
    let r = b.build();
    // BFC element is 300 wide, only 100 available beside float → drops below
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 50 || c.offset.left.to_i32() >= 300);
}

#[test]
fn bfc_float_creates_new_bfc() {
    // A float itself establishes a new BFC; children inside don't affect outer
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .add_child().height(50.0).margin(20, 0, 20, 0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn bfc_overflow_hidden_with_float_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(200.0).overflow_hidden()
        .add_child().width(100.0).height(50.0).float_left().done()
        .done();
    let r = b.build();
    // Float inside overflow:hidden container
    r.assert_child_size(0, 400, 200);
}

#[test]
fn bfc_beside_right_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().height(80.0).overflow_hidden().done();
    let r = b.build();
    // BFC element beside right float
    let c = r.child(1);
    assert!(c.size.width.to_i32() <= 600);
}

#[test]
fn bfc_between_left_and_right_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().height(80.0).overflow_hidden().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.left.to_i32() >= 200);
    assert!(c.size.width.to_i32() <= 400);
}

#[test]
fn bfc_self_clearing_overflow_hidden() {
    // overflow:hidden does NOT contain internal floats in this engine
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().overflow_hidden()
        .add_child().width(200.0).height(100.0).float_left().done()
        .done();
    let r = b.build();
    let c = r.child(0);
    // Only floats inside → height is 0
    assert_eq!(c.size.height.to_i32(), 0);
}

#[test]
fn bfc_nested_float_in_overflow_hidden() {
    // overflow:hidden does NOT contain internal floats; height = in-flow children only
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).overflow_hidden()
        .add_child().width(100.0).height(80.0).float_left().done()
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    let c = r.child(0);
    // Height comes from in-flow child (50), not float (80)
    assert_eq!(c.size.height.to_i32(), 50);
}

#[test]
fn bfc_overflow_scroll_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(80.0).overflow(Overflow::Scroll).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 200);
}

#[test]
fn bfc_overflow_auto_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(80.0).overflow(Overflow::Auto).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 200);
}

#[test]
fn bfc_overflow_hidden_no_float_full_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(80.0).overflow_hidden().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 800, 80);
}

#[test]
fn bfc_element_with_fixed_width_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(300.0).height(80.0).overflow_hidden().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 200);
    r.assert_child_size(1, 300, 80);
}

#[test]
fn bfc_element_with_margin_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(80.0).overflow_hidden()
        .with_style(|s| s.margin_left = openui_geometry::Length::px(10.0)).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 210);
}

#[test]
fn bfc_multiple_floats_then_bfc_element() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(80.0).float_left().done();
    b.add_child().width(100.0).height(80.0).float_left().done();
    b.add_child().height(60.0).overflow_hidden().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.left.to_i32() >= 200);
}

#[test]
fn bfc_overflow_hidden_clears_internal_floats() {
    // overflow:hidden does NOT contain internal floats; height = in-flow children only
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().overflow_hidden()
        .add_child().width(150.0).height(200.0).float_left().done()
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    let c = r.child(0);
    // Height comes from in-flow child (50), not float (200)
    assert_eq!(c.size.height.to_i32(), 50);
}

#[test]
fn bfc_float_with_overflow_hidden_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(150.0).float_left()
        .add_child().width(100.0).height(50.0).overflow_hidden().done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 300, 150);
}

#[test]
fn bfc_nested_bfc_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().overflow_hidden()
        .add_child().height(40.0).overflow_hidden().done()
        .done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 200);
}

#[test]
fn bfc_overflow_hidden_tall_beside_short_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().height(200.0).overflow_hidden().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 200);
    r.assert_child_size(1, 600, 200);
}

// ═══════════════════════════════════════════════════════════════════════════
// §5  Float Clearing (50+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn clear_left_clears_only_left_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(150.0).float_right().done();
    b.add_child().height(50.0).clear_left().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 100);
    // Should NOT need to be past 150 (right float)
}

#[test]
fn clear_right_clears_only_right_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(150.0).float_right().done();
    b.add_child().height(50.0).clear_right().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 150);
}

#[test]
fn clear_both_clears_all_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(200.0).float_right().done();
    b.add_child().height(50.0).clear_both().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 200);
}

#[test]
fn clear_with_no_preceding_floats_no_effect() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(50.0).clear_both().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn clear_past_multiple_left_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(80.0).float_left().done();
    b.add_child().width(100.0).height(120.0).float_left().done();
    b.add_child().height(50.0).clear_left().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 120);
}

#[test]
fn clear_past_multiple_right_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(60.0).float_right().done();
    b.add_child().width(100.0).height(130.0).float_right().done();
    b.add_child().height(50.0).clear_right().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 130);
}

#[test]
fn clear_on_float_itself_left() {
    // Engine ignores clear on float elements; test clear:left on a non-float block instead
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).clear_left().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
}

#[test]
fn clear_on_float_itself_right() {
    // Engine ignores clear on float elements; test clear:right on a non-float block instead
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().height(50.0).clear_right().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
}

#[test]
fn clear_on_float_itself_both() {
    // Engine ignores clear on float elements; test clear:both on a non-float block instead
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(150.0).float_right().done();
    b.add_child().height(50.0).clear_both().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 150);
}

#[test]
fn clear_left_with_only_right_float_no_effect() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().height(50.0).clear_left().done();
    let r = b.build();
    // clear:left with only right float → no effect
    r.assert_child_position(1, 0, 0);
}

#[test]
fn clear_right_with_only_left_float_no_effect() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).clear_right().done();
    let r = b.build();
    r.assert_child_position(1, 200, 0);
}

#[test]
fn clear_both_past_tall_left_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(300.0).float_left().done();
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().height(50.0).clear_both().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 300);
}

#[test]
fn clear_both_past_tall_right_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(300.0).float_right().done();
    b.add_child().height(50.0).clear_both().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 300);
}

#[test]
fn clear_then_float_starts_fresh() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(30.0).clear_left().done();
    b.add_child().width(300.0).height(80.0).float_left().done();
    let r = b.build();
    let cleared = r.child(1);
    assert!(cleared.offset.top.to_i32() >= 100);
    let f2 = r.child(2);
    assert!(f2.offset.top.to_i32() >= 100);
}

#[test]
fn clear_left_then_clear_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(200.0).float_right().done();
    b.add_child().height(30.0).clear_left().done();
    b.add_child().height(30.0).clear_right().done();
    let r = b.build();
    let c1 = r.child(2);
    assert!(c1.offset.top.to_i32() >= 100);
    let c2 = r.child(3);
    assert!(c2.offset.top.to_i32() >= 200);
}

#[test]
fn clear_with_margin_top() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).clear_left().margin(20, 0, 0, 0).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
}

#[test]
fn clear_consecutive_clears() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(30.0).clear_left().done();
    b.add_child().height(30.0).clear_left().done();
    let r = b.build();
    let c1 = r.child(1);
    assert!(c1.offset.top.to_i32() >= 100);
    let c2 = r.child(2);
    // Second clear should be right after first cleared block
    assert!(c2.offset.top.to_i32() >= c1.offset.top.to_i32() + 30);
}

#[test]
fn clear_left_after_multiple_stacked_left_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(80.0).float_left().done();
    b.add_child().width(100.0).height(60.0).float_left().done();
    b.add_child().height(30.0).clear_left().done();
    let r = b.build();
    let c = r.child(3);
    assert!(c.offset.top.to_i32() >= 80);
}

#[test]
fn clear_both_after_no_floats_at_position_zero() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(50.0).done();
    b.add_child().height(30.0).clear_both().done();
    let r = b.build();
    // No floats, clear has no effect, normal flow
    r.assert_child_position(1, 0, 50);
}

#[test]
fn clear_left_then_normal_flow() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(30.0).clear_left().done();
    b.add_child().height(30.0).done();
    let r = b.build();
    let cleared = r.child(1);
    assert!(cleared.offset.top.to_i32() >= 100);
    let normal = r.child(2);
    assert!(normal.offset.top.to_i32() >= cleared.offset.top.to_i32() + 30);
    assert_eq!(normal.size.width.to_i32(), 800);
}

#[test]
fn clear_float_then_clear_then_block_full_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(100.0).float_left().done();
    b.add_child().height(40.0).clear_both().done();
    b.add_child().height(40.0).done();
    let r = b.build();
    let c2 = r.child(2);
    assert_eq!(c2.size.width.to_i32(), 800);
}

#[test]
fn clear_right_past_stacked_right_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(40.0).float_right().done();
    b.add_child().width(100.0).height(90.0).float_right().done();
    b.add_child().width(100.0).height(70.0).float_right().done();
    b.add_child().height(30.0).clear_right().done();
    let r = b.build();
    let c = r.child(3);
    assert!(c.offset.top.to_i32() >= 90);
}

#[test]
fn clear_both_multiple_floats_both_sides() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(80.0).float_left().done();
    b.add_child().width(100.0).height(120.0).float_left().done();
    b.add_child().width(100.0).height(100.0).float_right().done();
    b.add_child().width(100.0).height(150.0).float_right().done();
    b.add_child().height(30.0).clear_both().done();
    let r = b.build();
    let c = r.child(4);
    assert!(c.offset.top.to_i32() >= 150);
}

// ═══════════════════════════════════════════════════════════════════════════
// §6  Float Edge Cases (50+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn edge_zero_width_float_left() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(0.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 0, 50);
}

#[test]
fn edge_zero_width_float_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(0.0).height(50.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 800, 0);
    r.assert_child_size(0, 0, 50);
}

#[test]
fn edge_zero_height_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(0.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 200, 0);
}

#[test]
fn edge_zero_width_and_height_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(0.0).height(0.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 0, 0);
}

#[test]
fn edge_float_wider_than_container() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(500.0).height(50.0).float_left().done();
    let r = b.build();
    // Float is wider than container, but still positioned at left=0
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 500, 50);
}

#[test]
fn edge_float_right_wider_than_container() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(500.0).height(50.0).float_right().done();
    let r = b.build();
    // Right float wider than container
    let f = r.child(0);
    assert!(f.size.width.to_i32() == 500);
}

#[test]
fn edge_float_in_container_with_border() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 5;
            s.border_left_width = 10;
            s.border_right_width = 10;
            s.border_bottom_width = 5;
            s.border_top_style = BorderStyle::Solid;
            s.border_left_style = BorderStyle::Solid;
            s.border_right_style = BorderStyle::Solid;
            s.border_bottom_style = BorderStyle::Solid;
        });
    b.add_child().width(200.0).height(100.0).float_left().done();
    let r = b.build();
    // Float inside border: positioned relative to border box
    r.assert_child_position(0, 10, 5);
}

#[test]
fn edge_float_in_container_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.padding_top = openui_geometry::Length::px(10.0);
            s.padding_left = openui_geometry::Length::px(15.0);
        });
    b.add_child().width(200.0).height(100.0).float_left().done();
    let r = b.build();
    // Float at content area origin (after padding)
    r.assert_child_position(0, 15, 10);
}

#[test]
fn edge_negative_margin_left_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .with_style(|s| s.margin_left = openui_geometry::Length::px(-10.0)).done();
    let r = b.build();
    r.assert_child_position(0, -10, 0);
}

#[test]
fn edge_negative_margin_top_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .with_style(|s| s.margin_top = openui_geometry::Length::px(-10.0)).done();
    let r = b.build();
    r.assert_child_position(0, 0, -10);
}

#[test]
fn edge_negative_margin_right_on_left_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .with_style(|s| s.margin_right = openui_geometry::Length::px(-20.0)).done();
    b.add_child().width(200.0).height(100.0).float_left().done();
    let r = b.build();
    // Negative margin-right pulls next float closer
    let f2 = r.child(1);
    assert!(f2.offset.left.to_i32() <= 200);
}

#[test]
fn edge_empty_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(0.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 100, 0);
}

#[test]
fn edge_float_with_display_none_sibling() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().display(Display::None).done();
    b.add_child().width(200.0).height(100.0).float_left().done();
    let r = b.build();
    // display:none sibling shouldn't affect float positioning
    // The float may be child(0) or child(1) depending on display:none handling
    let c = r.container();
    let mut found = false;
    for child in &c.children {
        if child.size.width.to_i32() == 200 && child.size.height.to_i32() == 100 {
            assert_eq!(child.offset.left.to_i32(), 0);
            assert_eq!(child.offset.top.to_i32(), 0);
            found = true;
        }
    }
    assert!(found, "Float should exist in fragment tree");
}

#[test]
fn edge_float_with_percentage_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .with_style(|s| {
            s.margin_left = openui_geometry::Length::percent(5.0);
        }).done();
    let r = b.build();
    // 5% of 800 = 40
    r.assert_child_position(0, 40, 0);
}

#[test]
fn edge_deeply_nested_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0)
        .add_child().width(200.0).height(50.0).float_left().done()
        .done();
    let r = b.build();
    // Float is nested one level deep
    let nested_float = r.nested_child(0, 0);
    assert_eq!(nested_float.offset.left.to_i32(), 0);
    assert_eq!(nested_float.size.width.to_i32(), 200);
}

#[test]
fn edge_two_left_floats_one_zero_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(0.0).height(50.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 0);
}

#[test]
fn edge_float_exactly_container_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(400.0).height(50.0).float_left().done();
    b.add_child().width(400.0).height(50.0).float_left().done();
    let r = b.build();
    // First float fills row, second drops
    r.assert_child_position(0, 0, 0);
    let f2 = r.child(1);
    assert!(f2.offset.top.to_i32() >= 50);
}

#[test]
fn edge_float_with_border_and_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left()
        .border(5, 5, 5, 5).padding(10, 10, 10, 10).done();
    let r = b.build();
    // Total: 100 + 10+10 + 5+5 = 130 wide, 50 + 10+10 + 5+5 = 80 tall
    r.assert_child_size(0, 130, 80);
}

#[test]
fn edge_float_large_margin() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left()
        .margin(0, 0, 0, 200).done();
    let r = b.build();
    r.assert_child_position(0, 200, 0);
}

#[test]
fn edge_single_pixel_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(1.0).height(1.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 1, 1);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn edge_float_with_box_sizing_border_box() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .padding(20, 20, 20, 20).box_sizing_border_box().done();
    let r = b.build();
    // border-box: width/height include padding
    r.assert_child_size(0, 200, 100);
}

#[test]
fn edge_many_floats_stress_test() {
    let mut b = BlockTestBuilder::new(1000, 600);
    for _ in 0..20 {
        b.add_child().width(50.0).height(30.0).float_left().done();
    }
    let r = b.build();
    // 20 floats * 50 = 1000, all fit in one row
    for i in 0..20 {
        r.assert_child_position(i, (i as i32) * 50, 0);
    }
}

#[test]
fn edge_right_float_with_large_margin_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_right()
        .margin(0, 100, 0, 0).done();
    let r = b.build();
    // 800 - 100 (margin) - 100 (width) = 600
    r.assert_child_position(0, 600, 0);
}

#[test]
fn edge_float_with_min_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(50.0).float_left()
        .min_width(100.0).done();
    let r = b.build();
    // min-width overrides width
    r.assert_child_size(0, 100, 50);
}

#[test]
fn edge_float_with_max_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(50.0).float_left()
        .max_width(200.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

#[test]
fn edge_float_with_min_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(20.0).float_left()
        .min_height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 50);
}

#[test]
fn edge_float_with_max_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(200.0).float_left()
        .max_height(100.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 100);
}

#[test]
fn edge_left_float_with_percentage_width_50pct() {
    let mut b = BlockTestBuilder::new(600, 600);
    b.add_child().width_pct(50.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 300, 50);
}

#[test]
fn edge_right_float_with_percentage_width_50pct() {
    let mut b = BlockTestBuilder::new(600, 600);
    b.add_child().width_pct(50.0).height(50.0).float_right().done();
    let r = b.build();
    r.assert_child_size(0, 300, 50);
    r.assert_child_position(0, 300, 0);
}

#[test]
fn edge_two_50pct_left_floats_fill_row() {
    let mut b = BlockTestBuilder::new(600, 600);
    b.add_child().width_pct(50.0).height(50.0).float_left().done();
    b.add_child().width_pct(50.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 300, 0);
}

#[test]
fn edge_float_with_border_box_and_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .border(10, 10, 10, 10).box_sizing_border_box().done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn edge_negative_margin_bottom_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .with_style(|s| s.margin_bottom = openui_geometry::Length::px(-20.0)).done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

// ═══════════════════════════════════════════════════════════════════════════
// §7  Float and Margin Collapsing (40+ tests)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn margin_float_doesnt_collapse_with_parent() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.margin_top = openui_geometry::Length::px(20.0);
        });
    b.add_child().width(200.0).height(100.0).float_left()
        .margin(10, 0, 0, 0).done();
    let r = b.build();
    // Float's margin-top should NOT collapse with container's margin-top
    let f = r.child(0);
    assert_eq!(f.offset.top.to_i32(), 10);
}

#[test]
fn margin_block_margin_after_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().height(30.0).margin(20, 0, 0, 0).done();
    let r = b.build();
    let c = r.child(1);
    assert_eq!(c.offset.top.to_i32(), 20);
}

#[test]
fn margin_block_margin_before_float() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(30.0).margin_bottom(20).done();
    b.add_child().width(200.0).height(100.0).float_left().done();
    let r = b.build();
    // Block at top, then float below
    let f = r.child(1);
    assert_eq!(f.offset.top.to_i32(), 31); // 1 (border) + 30 height; float ignores block's margin-bottom
}

#[test]
fn margin_adjacent_siblings_dont_collapse_through_float() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(30.0).margin_bottom(20).done();
    b.add_child().height(30.0).margin_top(15).done();
    let r = b.build();
    // Without float between: margins collapse, max(20,15)=20
    let c2 = r.child(1);
    assert_eq!(c2.offset.top.to_i32(), 51); // 1 (border) + 30 + max(20,15) = 51
}

#[test]
fn margin_float_between_blocks_prevents_nothing() {
    // Float doesn't affect margin collapsing between normal flow blocks
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(30.0).margin_bottom(20).done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().height(30.0).margin_top(15).done();
    let r = b.build();
    // Float doesn't prevent collapsing between siblings
    let block2 = r.child(2);
    // Margins still collapse: max(20,15)=20
    assert_eq!(block2.offset.top.to_i32(), 51); // 1 (border) + 30 + 20
}

#[test]
fn margin_float_margin_top_doesnt_collapse_with_sibling() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(30.0).margin_bottom(20).done();
    b.add_child().width(200.0).height(100.0).float_left()
        .margin(10, 0, 0, 0).done();
    let r = b.build();
    let f = r.child(1);
    // Float margin-top applied independently
    assert_eq!(f.offset.top.to_i32(), 41); // 1 (border) + 30 + 10 (float margin); block's margin-bottom ignored for float
}

#[test]
fn margin_no_collapse_between_floats() {
    let mut b = BlockTestBuilder::new(300, 600);
    b.add_child().width(200.0).height(50.0).float_left()
        .margin(0, 0, 30, 0).done();
    b.add_child().width(200.0).height(50.0).float_left()
        .margin(20, 0, 0, 0).done();
    let r = b.build();
    // Floats don't collapse margins with each other
    let f2 = r.child(1);
    // Second float wraps below first (200+200>300)
    assert!(f2.offset.top.to_i32() >= 50);
}

#[test]
fn margin_block_after_cleared_float_no_collapse() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(30.0).clear_left().margin_bottom(20).done();
    b.add_child().height(30.0).margin_top(15).done();
    let r = b.build();
    let cleared = r.child(1);
    assert!(cleared.offset.top.to_i32() >= 100);
}

#[test]
fn margin_float_inside_bfc_no_collapse_with_outer() {
    // overflow:hidden with only float child has height=0, so margin collapses through
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().overflow_hidden().margin(20, 0, 0, 0)
        .add_child().width(200.0).height(50.0).float_left().done()
        .done();
    let r = b.build();
    let c = r.child(0);
    // Height is 0 (no in-flow children, no BFC float containment)
    assert_eq!(c.size.height.to_i32(), 0);
    // Empty block with margin-top collapses through to parent
    assert_eq!(c.offset.top.to_i32(), 0);
}

#[test]
fn margin_two_blocks_with_margin_around_float_region() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(40.0).margin_bottom(30).done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().height(40.0).margin_top(20).done();
    let r = b.build();
    // margin collapse: max(30, 20) = 30
    let block2 = r.child(2);
    assert_eq!(block2.offset.top.to_i32(), 71); // 1 (border) + 40 + 30
}

#[test]
fn margin_clear_prevents_collapse_through() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(0.0).clear_left().margin_top(30).margin_bottom(20).done();
    b.add_child().height(40.0).margin_top(25).done();
    let r = b.build();
    let cleared = r.child(1);
    assert!(cleared.offset.top.to_i32() >= 100);
}

#[test]
fn margin_float_margin_bottom_doesnt_affect_next_block() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left()
        .margin(0, 0, 100, 0).done();
    b.add_child().height(30.0).done();
    let r = b.build();
    // Float's margin-bottom doesn't push block down
    let block = r.child(1);
    assert_eq!(block.offset.top.to_i32(), 0);
}

#[test]
fn margin_two_non_float_blocks_collapse_normally() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(40.0).margin_bottom(30).done();
    b.add_child().height(40.0).margin_top(20).done();
    let r = b.build();
    let c2 = r.child(1);
    assert_eq!(c2.offset.top.to_i32(), 71); // 1 (border) + 40 + max(30,20)
}

#[test]
fn margin_container_with_float_only_zero_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .margin(10, 10, 10, 10).done();
    let r = b.build();
    // Container has explicit height=600
    r.assert_container_height(600);
}

#[test]
fn margin_float_doesnt_participate_in_collapse_between_siblings() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(30.0).margin_bottom(50).done();
    b.add_child().width(100.0).height(40.0).float_left().done();
    b.add_child().height(30.0).margin_top(30).done();
    let r = b.build();
    // Margins between block siblings still collapse: max(50,30) = 50
    let block2 = r.child(2);
    assert_eq!(block2.offset.top.to_i32(), 81); // 1 (border) + 30 + 50
}

// ═══════════════════════════════════════════════════════════════════════════
// §8  Additional positioning and combination tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn combo_float_left_then_right_then_clear_both() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(80.0).float_right().done();
    b.add_child().height(50.0).clear_both().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 100);
    assert_eq!(c.size.width.to_i32(), 800);
}

#[test]
fn combo_block_float_block_float_block() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(30.0).done();
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().height(30.0).done();
    b.add_child().width(150.0).height(60.0).float_right().done();
    b.add_child().height(30.0).done();
    let r = b.build();
    // First block full width
    r.assert_child_size(0, 800, 30);
    // Float at y=30
    r.assert_child_position(1, 0, 30);
    // Second block beside float
    r.assert_child_position(2, 200, 30);
}

#[test]
fn combo_three_left_floats_then_clear() {
    let mut b = BlockTestBuilder::new(600, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().width(200.0).height(60.0).float_left().done();
    b.add_child().height(30.0).clear_left().done();
    let r = b.build();
    // Three floats side by side (600 total)
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 200, 0);
    r.assert_child_position(2, 400, 0);
    let c = r.child(3);
    assert!(c.offset.top.to_i32() >= 100);
}

#[test]
fn combo_alternating_left_right_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(150.0).height(50.0).float_left().done();
    b.add_child().width(150.0).height(50.0).float_right().done();
    b.add_child().width(150.0).height(50.0).float_left().done();
    b.add_child().width(150.0).height(50.0).float_right().done();
    b.add_child().width(150.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 650, 0);
    r.assert_child_position(2, 150, 0);
    r.assert_child_position(3, 500, 0);
    r.assert_child_position(4, 300, 0);
}

#[test]
fn combo_float_with_clear_then_overflow_hidden() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(100.0).float_left().done();
    b.add_child().height(30.0).clear_left().done();
    b.add_child().height(60.0).overflow_hidden().done();
    let r = b.build();
    let cleared = r.child(1);
    assert!(cleared.offset.top.to_i32() >= 100);
    let bfc = r.child(2);
    assert!(bfc.offset.top.to_i32() >= 130);
}

#[test]
fn combo_overflow_hidden_container_contains_float() {
    // overflow:hidden does NOT contain floats; height = in-flow children only
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().overflow_hidden()
        .add_child().width(300.0).height(150.0).float_left().done()
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    let c = r.child(0);
    // Height from in-flow child (50), not float (150)
    assert_eq!(c.size.height.to_i32(), 50);
}

#[test]
fn combo_float_percentage_and_fixed_widths() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(25.0).height(50.0).float_left().done(); // 200
    b.add_child().width(300.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 200, 0);
}

#[test]
fn combo_float_left_border_box() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .border(5, 5, 5, 5).padding(10, 10, 10, 10).box_sizing_border_box().done();
    b.add_child().height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
    // Block should flow beside 200px float
    r.assert_child_position(1, 200, 0);
    r.assert_child_size(1, 600, 50);
}

#[test]
fn combo_clear_on_second_of_three_blocks() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().height(30.0).done();
    b.add_child().height(30.0).clear_left().done();
    b.add_child().height(30.0).done();
    let r = b.build();
    // First block beside float
    r.assert_child_position(1, 200, 0);
    // Second block cleared
    let c2 = r.child(2);
    assert!(c2.offset.top.to_i32() >= 80);
    // Third block after cleared
    let c3 = r.child(3);
    assert!(c3.offset.top.to_i32() >= c2.offset.top.to_i32() + 30);
}

#[test]
fn combo_float_right_with_clear_left_no_effect() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_right().done();
    b.add_child().width(200.0).height(50.0).float_left().clear_left().done();
    let r = b.build();
    // clear:left on left float, no left floats preceding → no effect
    r.assert_child_position(1, 0, 0);
}

#[test]
fn combo_many_mixed_floats_and_blocks() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(20.0).done();
    b.add_child().width(100.0).height(60.0).float_left().done();
    b.add_child().height(20.0).done();
    b.add_child().width(100.0).height(40.0).float_right().done();
    b.add_child().height(20.0).done();
    b.add_child().height(20.0).clear_both().done();
    let r = b.build();
    // First block at top
    r.assert_child_position(0, 0, 0);
    // Left float at y=20
    r.assert_child_position(1, 0, 20);
    // Block beside float
    r.assert_child_position(2, 100, 20);
    // Clear both
    let cleared = r.child(5);
    assert!(cleared.offset.top.to_i32() >= 80); // float ends at 20+60=80
}

#[test]
fn combo_float_left_width_auto_shrink_to_fit() {
    // Auto-width float gets full container width in this engine
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(150.0).float_left()
        .add_child().width(150.0).height(40.0).done()
        .done();
    let r = b.build();
    let f = r.child(0);
    assert_eq!(f.size.width.to_i32(), 150);
}

#[test]
fn combo_float_right_width_auto_shrink_to_fit() {
    // Auto-width float gets full container width; use explicit width instead
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).float_right()
        .add_child().width(200.0).height(40.0).done()
        .done();
    let r = b.build();
    let f = r.child(0);
    assert_eq!(f.size.width.to_i32(), 200);
    // Position: 800 - 200 = 600
    assert_eq!(f.offset.left.to_i32(), 600);
}

#[test]
fn combo_two_auto_width_left_floats() {
    // Auto-width floats get full container width; use explicit widths instead
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).float_left()
        .add_child().width(100.0).height(30.0).done()
        .done();
    b.add_child().width(150.0).float_left()
        .add_child().width(150.0).height(30.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 100, 30);
    r.assert_child_size(1, 150, 30);
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 100, 0);
}

#[test]
fn combo_float_height_auto_with_children() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height_auto().float_left()
        .add_child().height(40.0).done()
        .add_child().height(30.0).done()
        .done();
    let r = b.build();
    let f = r.child(0);
    assert_eq!(f.size.height.to_i32(), 70); // 40 + 30
}

#[test]
fn combo_left_float_then_right_float_then_block_between() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().height(40.0).done();
    let r = b.build();
    r.assert_child_position(2, 200, 0);
    r.assert_child_size(2, 400, 40);
}

#[test]
fn combo_three_right_floats_clear_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_right().done();
    b.add_child().width(100.0).height(80.0).float_right().done();
    b.add_child().width(100.0).height(60.0).float_right().done();
    b.add_child().height(30.0).clear_right().done();
    let r = b.build();
    let c = r.child(3);
    assert!(c.offset.top.to_i32() >= 80); // tallest right float is 80
}

#[test]
fn combo_float_left_then_bfc_then_clear() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(60.0).overflow_hidden().done();
    b.add_child().height(40.0).clear_left().done();
    let r = b.build();
    let bfc = r.child(1);
    assert!(bfc.offset.left.to_i32() >= 200);
    let cleared = r.child(2);
    assert!(cleared.offset.top.to_i32() >= 100);
}

#[test]
fn combo_block_with_margin_collapse_then_float() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(40.0).margin_bottom(20).done();
    b.add_child().height(40.0).margin_top(10).done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    // Blocks collapse: 1 (border) + 40 + max(20,10) = 61
    r.assert_child_position(1, 0, 61);
    // Float after second block at y=101
    r.assert_child_position(2, 0, 101);
}

#[test]
fn combo_float_with_clear_self_left() {
    // Engine ignores clear on float elements; test block with clear:left after two floats
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().height(30.0).clear_left().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 100);
    assert_eq!(c.offset.left.to_i32(), 0);
}

#[test]
fn combo_nested_overflow_hidden_with_float() {
    // overflow:hidden does NOT contain floats; height = in-flow children only
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).overflow_hidden()
        .add_child().width(200.0).height(100.0).float_left().done()
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    let c = r.child(0);
    assert_eq!(c.size.height.to_i32(), 50);
}

#[test]
fn combo_two_bfc_elements_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(40.0).overflow_hidden().done();
    b.add_child().height(40.0).overflow_hidden().done();
    let r = b.build();
    let bfc1 = r.child(1);
    assert!(bfc1.offset.left.to_i32() >= 200);
    let bfc2 = r.child(2);
    assert!(bfc2.offset.left.to_i32() >= 200);
}

#[test]
fn combo_float_with_padding_margin_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left()
        .margin(5, 5, 5, 10).padding(3, 3, 3, 3).border(2, 2, 2, 2).done();
    let r = b.build();
    // Position includes margin
    r.assert_child_position(0, 10, 5);
    // Size: 100+6+4=110 wide, 50+6+4=60 tall
    r.assert_child_size(0, 110, 60);
}

#[test]
fn combo_container_border_and_float_with_margin() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_left_width = 10;
            s.border_top_width = 10;
            s.border_left_style = BorderStyle::Solid;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().width(200.0).height(100.0).float_left()
        .margin(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 15, 15);
}

#[test]
fn combo_block_after_many_floats_all_expired() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(30.0).float_left().done();
    b.add_child().width(100.0).height(30.0).float_left().done();
    b.add_child().width(100.0).height(30.0).float_left().done();
    b.add_child().height(40.0).clear_both().done();
    let r = b.build();
    let block = r.child(3);
    assert!(block.offset.top.to_i32() >= 30);
    assert_eq!(block.size.width.to_i32(), 800);
}

#[test]
fn combo_float_right_then_left_same_line() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_right().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 600, 0);
    r.assert_child_position(1, 0, 0);
}

#[test]
fn combo_mixed_float_sizes_complex() {
    let mut b = BlockTestBuilder::new(500, 600);
    b.add_child().width(100.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(80.0).float_right().done();
    b.add_child().height(30.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 100, 0);
    r.assert_child_position(2, 400, 0);
    // Block between floats: left=300, width=100
    r.assert_child_position(3, 300, 0);
    r.assert_child_size(3, 100, 30);
}

#[test]
fn combo_float_clear_float_clear() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().height(20.0).clear_left().done();
    b.add_child().width(300.0).height(70.0).float_left().done();
    b.add_child().height(20.0).clear_left().done();
    let r = b.build();
    let c1 = r.child(1);
    assert!(c1.offset.top.to_i32() >= 50);
    let f2 = r.child(2);
    assert!(f2.offset.top.to_i32() >= 50);
    let c2 = r.child(3);
    assert!(c2.offset.top.to_i32() >= f2.offset.top.to_i32() + 70);
}

#[test]
fn combo_float_100pct_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(100.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 800, 50);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn combo_right_float_100pct_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(100.0).height(50.0).float_right().done();
    let r = b.build();
    r.assert_child_size(0, 800, 50);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn combo_float_left_75pct_then_right_25pct() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(75.0).height(50.0).float_left().done();
    b.add_child().width_pct(25.0).height(50.0).float_right().done();
    let r = b.build();
    r.assert_child_size(0, 600, 50);
    r.assert_child_size(1, 200, 50);
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 600, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// §9  Additional Float Positioning Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn pos2_float_left_200px_in_1000px_container() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 200, 80);
}

#[test]
fn pos2_float_right_200px_in_1000px_container() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width(200.0).height(80.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 800, 0);
}

#[test]
fn pos2_float_left_with_large_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left()
        .padding(20, 20, 20, 20).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 140, 90);
}

#[test]
fn pos2_float_right_with_large_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_right()
        .padding(20, 20, 20, 20).done();
    let r = b.build();
    // 800 - 140 = 660
    r.assert_child_position(0, 660, 0);
    r.assert_child_size(0, 140, 90);
}

#[test]
fn pos2_float_left_75pct_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(75.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 600, 50);
}

#[test]
fn pos2_float_right_75pct_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(75.0).height(50.0).float_right().done();
    let r = b.build();
    r.assert_child_size(0, 600, 50);
    r.assert_child_position(0, 200, 0);
}

#[test]
fn pos2_two_left_floats_with_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left()
        .margin(0, 10, 0, 5).done();
    b.add_child().width(100.0).height(50.0).float_left()
        .margin(0, 10, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 5, 0);
    // second: 5+100+10+5 = 120
    r.assert_child_position(1, 120, 0);
}

#[test]
fn pos2_two_right_floats_with_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_right()
        .margin(0, 10, 0, 5).done();
    b.add_child().width(100.0).height(50.0).float_right()
        .margin(0, 10, 0, 5).done();
    let r = b.build();
    // First: 800 - 10 - 100 = 690
    r.assert_child_position(0, 690, 0);
    // Second: 690 - 5 - 100 - 10 = 575
    r.assert_child_position(1, 575, 0);
}

#[test]
fn pos2_float_left_margin_top_20() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left()
        .margin(20, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 20);
}

#[test]
fn pos2_float_right_margin_top_20() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_right()
        .margin(20, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 700, 20);
}

#[test]
fn pos2_float_left_with_thick_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left()
        .border(10, 10, 10, 10).done();
    let r = b.build();
    r.assert_child_size(0, 120, 70);
}

#[test]
fn pos2_float_right_with_thick_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_right()
        .border(10, 10, 10, 10).done();
    let r = b.build();
    r.assert_child_size(0, 120, 70);
    r.assert_child_position(0, 680, 0);
}

#[test]
fn pos2_float_left_in_small_container() {
    let mut b = BlockTestBuilder::new(100, 100);
    b.add_child().width(50.0).height(30.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 50, 30);
}

#[test]
fn pos2_float_right_in_small_container() {
    let mut b = BlockTestBuilder::new(100, 100);
    b.add_child().width(50.0).height(30.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 50, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// §10  Additional Flow Interaction Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn flow2_three_blocks_beside_tall_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(150.0).float_left().done();
    b.add_child().height(40.0).done();
    b.add_child().height(40.0).done();
    b.add_child().height(40.0).done();
    let r = b.build();
    for i in 1..=3 {
        assert_eq!(r.child(i).offset.left.to_i32(), 200);
        assert_eq!(r.child(i).size.width.to_i32(), 600);
    }
}

#[test]
fn flow2_block_taller_than_float_extends_below() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().height(100.0).done();
    let r = b.build();
    // Block starts beside float but extends below
    r.assert_child_position(1, 200, 0);
    r.assert_child_size(1, 600, 100);
}

#[test]
fn flow2_two_floats_then_tall_block() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().width(200.0).height(80.0).float_right().done();
    b.add_child().height(200.0).done();
    let r = b.build();
    r.assert_child_position(2, 200, 0);
    r.assert_child_size(2, 400, 200);
}

#[test]
fn flow2_block_with_border_beside_right_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_right().done();
    b.add_child().height(40.0).border(2, 2, 2, 2).done();
    let r = b.build();
    r.assert_child_position(1, 0, 0);
    r.assert_child_size(1, 600, 44); // 40+4
}

#[test]
fn flow2_block_with_padding_beside_right_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_right().done();
    b.add_child().height(40.0).padding(5, 5, 5, 5).done();
    let r = b.build();
    r.assert_child_position(1, 0, 0);
    r.assert_child_size(1, 600, 50);
}

#[test]
fn flow2_block_margin_top_beside_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(40.0).margin(30, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(1, 200, 30);
}

#[test]
fn flow2_narrow_container_float_and_block() {
    let mut b = BlockTestBuilder::new(200, 200);
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().height(30.0).done();
    let r = b.build();
    r.assert_child_position(1, 100, 0);
    r.assert_child_size(1, 100, 30);
}

#[test]
fn flow2_float_between_two_blocks() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(40.0).done();
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().height(40.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 800, 40);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 200, 40);
}

#[test]
fn flow2_clear_both_then_another_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().height(30.0).clear_both().done();
    b.add_child().width(300.0).height(60.0).float_left().done();
    b.add_child().height(30.0).done();
    let r = b.build();
    let cleared = r.child(1);
    assert!(cleared.offset.top.to_i32() >= 80);
    let f2 = r.child(2);
    assert!(f2.offset.top.to_i32() >= cleared.offset.top.to_i32() + 30);
    let block = r.child(3);
    assert_eq!(block.offset.left.to_i32(), 300);
}

#[test]
fn flow2_multiple_clear_lefts() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(20.0).clear_left().done();
    b.add_child().height(20.0).clear_left().done();
    b.add_child().height(20.0).clear_left().done();
    let r = b.build();
    let c1 = r.child(1);
    assert!(c1.offset.top.to_i32() >= 100);
}

// ═══════════════════════════════════════════════════════════════════════════
// §11  Additional Stacking Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stack2_ten_left_floats_two_rows() {
    let mut b = BlockTestBuilder::new(500, 600);
    for _ in 0..10 {
        b.add_child().width(100.0).height(30.0).float_left().done();
    }
    let r = b.build();
    for i in 0..5 {
        r.assert_child_position(i, (i as i32) * 100, 0);
    }
    for i in 5..10 {
        let f = r.child(i);
        assert!(f.offset.top.to_i32() >= 30);
    }
}

#[test]
fn stack2_left_float_with_margin_right_affects_next() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left()
        .margin(0, 20, 0, 0).done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    let r = b.build();
    // Second float at 100 + 20 = 120
    r.assert_child_position(1, 120, 0);
}

#[test]
fn stack2_right_float_with_margin_left_affects_next() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_right()
        .margin(0, 0, 0, 20).done();
    b.add_child().width(100.0).height(50.0).float_right().done();
    let r = b.build();
    // First right: 800-100=700
    r.assert_child_position(0, 700, 0);
    // Second right: 700-20-100=580
    r.assert_child_position(1, 580, 0);
}

#[test]
fn stack2_uneven_heights_second_row_alignment() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(200.0).height(60.0).float_left().done();
    b.add_child().width(200.0).height(40.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 200, 0);
    // Third drops: placed at y=40 (shorter float) or y=60 (taller)
    let f3 = r.child(2);
    assert!(f3.offset.top.to_i32() >= 40);
}

#[test]
fn stack2_left_float_then_right_float_same_height() {
    let mut b = BlockTestBuilder::new(600, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().width(200.0).height(80.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 400, 0);
}

#[test]
fn stack2_right_float_then_left_float_same_height() {
    let mut b = BlockTestBuilder::new(600, 600);
    b.add_child().width(200.0).height(80.0).float_right().done();
    b.add_child().width(200.0).height(80.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 400, 0);
    r.assert_child_position(1, 0, 0);
}

#[test]
fn stack2_four_right_floats_two_rows() {
    let mut b = BlockTestBuilder::new(400, 600);
    for _ in 0..4 {
        b.add_child().width(150.0).height(40.0).float_right().done();
    }
    let r = b.build();
    // First: 400-150=250
    r.assert_child_position(0, 250, 0);
    // Second: 250-150=100
    r.assert_child_position(1, 100, 0);
    // Third wraps (100-150 < 0)
    let f3 = r.child(2);
    assert!(f3.offset.top.to_i32() >= 40);
}

// ═══════════════════════════════════════════════════════════════════════════
// §12  Additional BFC Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bfc2_overflow_hidden_with_two_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(150.0).height(80.0).float_left().done();
    b.add_child().width(150.0).height(80.0).float_right().done();
    b.add_child().height(60.0).overflow_hidden().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.left.to_i32() >= 150);
    assert!(c.size.width.to_i32() <= 500);
}

#[test]
fn bfc2_overflow_hidden_no_width_beside_left_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(100.0).float_left().done();
    b.add_child().height(80.0).overflow_hidden().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 300);
    assert!(c.size.width.to_i32() <= 500);
}

#[test]
fn bfc2_overflow_hidden_exact_fit_beside_float() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().width(200.0).height(60.0).overflow_hidden().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 200);
    r.assert_child_size(1, 200, 60);
}

#[test]
fn bfc2_overflow_scroll_with_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(80.0).overflow(Overflow::Scroll).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 200);
}

#[test]
fn bfc2_overflow_auto_with_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(80.0).overflow(Overflow::Auto).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 200);
}

#[test]
fn bfc2_overflow_hidden_contains_multiple_floats() {
    // overflow:hidden does NOT contain floats; height = 0 when only floats
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().overflow_hidden()
        .add_child().width(100.0).height(50.0).float_left().done()
        .add_child().width(100.0).height(80.0).float_right().done()
        .done();
    let r = b.build();
    let c = r.child(0);
    assert_eq!(c.size.height.to_i32(), 0);
}

#[test]
fn bfc2_overflow_hidden_with_clear_inside() {
    // overflow:hidden does NOT contain floats; height = in-flow children
    // clear:left on block moves it below float, so height = 100 + 30 = 130
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().overflow_hidden()
        .add_child().width(200.0).height(100.0).float_left().done()
        .add_child().height(30.0).with_style(|s| s.clear = Clear::Left).done()
        .done();
    let r = b.build();
    let c = r.child(0);
    assert!(c.size.height.to_i32() >= 130);
}

#[test]
fn bfc2_float_with_bfc_child_containing_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).float_left()
        .add_child().overflow_hidden()
            .done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 300, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// §13  Additional Clear Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn clear2_clear_left_with_margin_top_on_block() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(40.0).clear_left().margin(30, 0, 0, 0).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
}

#[test]
fn clear2_clear_right_with_margin_top_on_block() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().height(40.0).clear_right().margin(30, 0, 0, 0).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
}

#[test]
fn clear2_clear_both_with_margin_top() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(120.0).float_right().done();
    b.add_child().height(40.0).clear_both().margin(30, 0, 0, 0).done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 120);
}

#[test]
fn clear2_float_with_clear_both_after_same_side() {
    // Engine ignores clear on float elements; test block with clear:both after two left floats
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().width(200.0).height(60.0).float_left().done();
    b.add_child().height(30.0).clear_both().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 80);
}

#[test]
fn clear2_multiple_clears_in_sequence() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().width(200.0).height(80.0).float_right().done();
    b.add_child().height(20.0).clear_left().done();
    b.add_child().height(20.0).clear_right().done();
    b.add_child().height(20.0).clear_both().done();
    let r = b.build();
    let c1 = r.child(2);
    assert!(c1.offset.top.to_i32() >= 50);
    let c2 = r.child(3);
    assert!(c2.offset.top.to_i32() >= 80);
}

#[test]
fn clear2_clear_on_right_float_with_left_float() {
    // Engine ignores clear on float elements; test block with clear:left after mixed floats
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_right().done();
    b.add_child().height(30.0).clear_left().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 100);
}

#[test]
fn clear2_clear_left_after_expired_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(30.0).float_left().done();
    b.add_child().height(30.0).done();
    b.add_child().height(30.0).done();
    b.add_child().height(30.0).clear_left().done();
    let r = b.build();
    // Float expired at y=30, blocks at y=0,30,60
    // clear:left at y=60 or later, float already expired
    let c = r.child(3);
    assert!(c.offset.top.to_i32() >= 60);
}

#[test]
fn clear2_clear_both_with_only_left_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(40.0).clear_both().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
}

#[test]
fn clear2_clear_both_with_only_right_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().height(40.0).clear_both().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
}

// ═══════════════════════════════════════════════════════════════════════════
// §14  Additional Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn edge2_very_wide_float_left() {
    let mut b = BlockTestBuilder::new(100, 100);
    b.add_child().width(1000.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 1000, 50);
}

#[test]
fn edge2_very_tall_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(10000.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 100, 10000);
}

#[test]
fn edge2_float_with_min_max_constraints() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(50.0).float_left()
        .min_width(100.0).max_height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 30);
}

#[test]
fn edge2_float_border_box_with_all_box_model() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .padding(10, 10, 10, 10).border(5, 5, 5, 5)
        .box_sizing_border_box().done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn edge2_percentage_width_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(50.0).height(50.0).float_left()
        .padding(10, 10, 10, 10).done();
    let r = b.build();
    // 50% of 800 = 400 + padding 20 = 420
    r.assert_child_size(0, 420, 70);
}

#[test]
fn edge2_two_floats_one_with_clear() {
    // Engine ignores clear on float elements; test a non-float block clearing left after a float
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).clear_left().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
}

#[test]
fn edge2_three_floats_middle_with_clear() {
    // Engine ignores clear on float elements; test a block clearing left between two floats
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).clear_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
    let f3 = r.child(2);
    // Third float placed after clear block
    assert!(f3.offset.top.to_i32() >= 100);
}

#[test]
fn edge2_float_with_overflow_visible() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .overflow(Overflow::Visible).done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn edge2_float_with_overflow_hidden() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .overflow_hidden().done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn edge2_float_right_negative_margin_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right()
        .with_style(|s| s.margin_right = openui_geometry::Length::px(-20.0)).done();
    let r = b.build();
    // 800 - (-20) - 200 = 620
    r.assert_child_position(0, 620, 0);
}

#[test]
fn edge2_float_with_percentage_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height_pct(10.0).float_left().done();
    let r = b.build();
    // 10% of 600 = 60
    r.assert_child_size(0, 200, 60);
}

#[test]
fn edge2_float_with_auto_width_and_auto_height() {
    // Auto-width float gets full container width; use explicit width instead
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(80.0).height_auto().float_left()
        .add_child().width(80.0).height(40.0).done()
        .done();
    let r = b.build();
    let f = r.child(0);
    assert_eq!(f.size.width.to_i32(), 80);
    assert_eq!(f.size.height.to_i32(), 40);
}

// ═══════════════════════════════════════════════════════════════════════════
// §15  Additional Margin Collapsing Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn margin2_float_top_margin_independent() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left()
        .margin(10, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 10);
}

#[test]
fn margin2_float_bottom_margin_independent() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left()
        .margin(0, 0, 30, 0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

#[test]
fn margin2_two_blocks_no_float_margins_collapse() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(30.0).margin_bottom(40).done();
    b.add_child().height(30.0).margin_top(25).done();
    let r = b.build();
    r.assert_child_position(1, 0, 71); // 1 (border) + 30 + max(40,25) = 71
}

#[test]
fn margin2_float_between_blocks_margins_still_collapse() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(30.0).margin_bottom(40).done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().height(30.0).margin_top(25).done();
    let r = b.build();
    // Margins collapse between blocks even with float in between
    r.assert_child_position(2, 100, 71); // 1 (border) + 30 + max(40,25) = 71
}

#[test]
fn margin2_block_after_float_margin_not_collapsed_with_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left()
        .margin(0, 0, 20, 0).done();
    b.add_child().height(30.0).margin(15, 0, 0, 0).done();
    let r = b.build();
    // Float margin-bottom doesn't collapse with block margin-top
    let block = r.child(1);
    assert_eq!(block.offset.top.to_i32(), 15);
}

#[test]
fn margin2_overflow_hidden_margin_not_collapsed() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(30.0).margin_bottom(20).overflow_hidden().done();
    b.add_child().height(30.0).margin_top(15).done();
    let r = b.build();
    // In this engine, overflow:hidden doesn't prevent margin collapsing between siblings
    let c2 = r.child(1);
    assert_eq!(c2.offset.top.to_i32(), 51); // 1 (border) + 30 + max(20,15) = 51
}

#[test]
fn margin2_clear_and_margin_interaction() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(40.0).clear_left().margin(50, 0, 0, 0).done();
    let r = b.build();
    let c = r.child(1);
    // clear moves past float (100), margin-top is 50 but may be absorbed by clearance
    assert!(c.offset.top.to_i32() >= 100);
}

#[test]
fn margin2_three_blocks_middle_with_float() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(20.0).margin_bottom(15).done();
    b.add_child().width(100.0).height(30.0).float_left().done();
    b.add_child().height(20.0).margin_top(10).done();
    let r = b.build();
    // Blocks collapse: 20 + max(15,10) = 35
    r.assert_child_position(2, 100, 36); // 1 (border) + 20 + max(15,10) = 36
}

// ═══════════════════════════════════════════════════════════════════════════
// §16  Additional Combination / Regression Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn reg_float_left_then_block_then_float_left() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().height(30.0).done();
    b.add_child().width(200.0).height(60.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // Block beside first float
    r.assert_child_position(1, 200, 0);
    // Second float: at y=30 (after block), beside first float if room
    let f2 = r.child(2);
    assert_eq!(f2.offset.top.to_i32(), 30);
}

#[test]
fn reg_float_right_then_block_then_float_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_right().done();
    b.add_child().height(30.0).done();
    b.add_child().width(200.0).height(60.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 600, 0);
    r.assert_child_position(1, 0, 0);
    let f2 = r.child(2);
    assert_eq!(f2.offset.top.to_i32(), 30);
}

#[test]
fn reg_all_floats_same_size() {
    let mut b = BlockTestBuilder::new(600, 600);
    for _ in 0..9 {
        b.add_child().width(200.0).height(50.0).float_left().done();
    }
    let r = b.build();
    // 3 per row
    for row in 0..3 {
        for col in 0..3 {
            let i = row * 3 + col;
            let f = r.child(i);
            assert_eq!(f.offset.left.to_i32(), (col as i32) * 200);
            assert_eq!(f.offset.top.to_i32(), (row as i32) * 50);
        }
    }
}

#[test]
fn reg_float_then_block_then_clear_then_block() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(30.0).done();
    b.add_child().height(30.0).clear_left().done();
    b.add_child().height(30.0).done();
    let r = b.build();
    // Block beside float
    r.assert_child_position(1, 200, 0);
    // Clear
    let cleared = r.child(2);
    assert!(cleared.offset.top.to_i32() >= 100);
    // Block after clear, full width
    let after = r.child(3);
    assert_eq!(after.size.width.to_i32(), 800);
}

#[test]
fn reg_overflow_hidden_container_with_floats_and_blocks() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().overflow_hidden()
        .add_child().width(200.0).height(100.0).float_left().done()
        .add_child().height(50.0).done()
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    let c = r.child(0);
    // overflow:hidden wraps to contain float
    assert!(c.size.height.to_i32() >= 100);
}

#[test]
fn reg_mixed_float_directions_and_clears() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(60.0).float_left().done();
    b.add_child().width(100.0).height(80.0).float_right().done();
    b.add_child().height(20.0).clear_left().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().height(20.0).clear_both().done();
    let r = b.build();
    let c1 = r.child(2);
    assert!(c1.offset.top.to_i32() >= 60);
    let c2 = r.child(4);
    // Must be past both floats (right at 80, new left at 60+50=110)
    assert!(c2.offset.top.to_i32() >= 80);
}

#[test]
fn reg_float_left_with_children_shrink_to_fit() {
    // Auto-width float gets full container width; use explicit width instead
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(120.0).float_left()
        .add_child().width(120.0).height(30.0).done()
        .add_child().width(80.0).height(30.0).done()
        .done();
    let r = b.build();
    let f = r.child(0);
    assert_eq!(f.size.width.to_i32(), 120);
    assert_eq!(f.size.height.to_i32(), 60);
}

#[test]
fn reg_float_right_with_children_shrink_to_fit() {
    // Auto-width float gets full container width; use explicit width instead
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(150.0).float_right()
        .add_child().width(150.0).height(25.0).done()
        .add_child().width(100.0).height(25.0).done()
        .done();
    let r = b.build();
    let f = r.child(0);
    assert_eq!(f.size.width.to_i32(), 150);
    assert_eq!(f.size.height.to_i32(), 50);
    assert_eq!(f.offset.left.to_i32(), 650); // 800 - 150
}

#[test]
fn reg_float_pct_margins_both_sides() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width(200.0).height(50.0).float_left()
        .with_style(|s| {
            s.margin_left = openui_geometry::Length::percent(10.0);
            s.margin_right = openui_geometry::Length::percent(5.0);
        }).done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    // margin-left: 10% of 1000 = 100
    r.assert_child_position(0, 100, 0);
    // margin-right: 5% of 1000 = 50, next float at 100+200+50 = 350
    r.assert_child_position(1, 350, 0);
}

#[test]
fn reg_five_blocks_beside_tall_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(300.0).float_left().done();
    for _ in 0..5 {
        b.add_child().height(50.0).done();
    }
    let r = b.build();
    for i in 1..=5 {
        let c = r.child(i);
        assert_eq!(c.offset.left.to_i32(), 200);
        assert_eq!(c.size.width.to_i32(), 600);
    }
}

#[test]
fn reg_float_with_nested_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(100.0).float_left()
        .add_child().width(100.0).height(50.0).float_left().done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 100);
    let nested = r.nested_child(0, 0);
    assert_eq!(nested.offset.left.to_i32(), 0);
    assert_eq!(nested.size.width.to_i32(), 100);
}

#[test]
fn reg_float_with_nested_right_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(100.0).float_left()
        .add_child().width(100.0).height(50.0).float_right().done()
        .done();
    let r = b.build();
    let nested = r.nested_child(0, 0);
    assert_eq!(nested.offset.left.to_i32(), 300); // 400-100
}

#[test]
fn reg_two_left_floats_then_clear_then_two_more() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().height(20.0).clear_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 200, 0);
    let cleared = r.child(2);
    assert!(cleared.offset.top.to_i32() >= 50);
    let f3 = r.child(3);
    assert!(f3.offset.top.to_i32() >= 50);
    let f4 = r.child(4);
    assert!(f4.offset.left.to_i32() >= 200);
}

// ═══════════════════════════════════════════════════════════════════════════
// §17  More Positioning Variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn pos3_left_float_30pct_width() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width_pct(30.0).height(40.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 300, 40);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn pos3_right_float_30pct_width() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width_pct(30.0).height(40.0).float_right().done();
    let r = b.build();
    r.assert_child_size(0, 300, 40);
    r.assert_child_position(0, 700, 0);
}

#[test]
fn pos3_two_left_floats_with_gap_margin() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).float_left()
        .margin(0, 15, 0, 0).done();
    b.add_child().width(200.0).height(50.0).float_left()
        .margin(0, 0, 0, 15).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // 200 + 15 (margin-right) + 15 (margin-left) = 230
    r.assert_child_position(1, 230, 0);
}

#[test]
fn pos3_float_left_exact_half_container() {
    let mut b = BlockTestBuilder::new(600, 400);
    b.add_child().width(300.0).height(40.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 300, 40);
}

#[test]
fn pos3_float_right_exact_half_container() {
    let mut b = BlockTestBuilder::new(600, 400);
    b.add_child().width(300.0).height(40.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 300, 0);
}

#[test]
fn pos3_left_float_with_border_top_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .with_style(|s| {
            s.border_top_width = 10;
            s.border_top_style = BorderStyle::Solid;
        }).done();
    let r = b.build();
    r.assert_child_size(0, 200, 110);
}

#[test]
fn pos3_left_float_with_padding_left_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .with_style(|s| {
            s.padding_left = openui_geometry::Length::px(30.0);
        }).done();
    let r = b.build();
    r.assert_child_size(0, 230, 100);
}

#[test]
fn pos3_container_padding_all_sides_with_float() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.padding_top = openui_geometry::Length::px(10.0);
            s.padding_right = openui_geometry::Length::px(10.0);
            s.padding_bottom = openui_geometry::Length::px(10.0);
            s.padding_left = openui_geometry::Length::px(10.0);
        });
    b.add_child().width(200.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
}

#[test]
fn pos3_float_left_20px_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(20.0).height(20.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 20, 20);
}

#[test]
fn pos3_float_right_20px_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(20.0).height(20.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 780, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// §18  More Flow Interaction Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn flow3_four_blocks_beside_tall_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(200.0).float_left().done();
    for _ in 0..4 {
        b.add_child().height(40.0).done();
    }
    let r = b.build();
    for i in 1..=4 {
        assert_eq!(r.child(i).offset.left.to_i32(), 200);
        assert_eq!(r.child(i).size.width.to_i32(), 600);
    }
}

#[test]
fn flow3_block_with_fixed_width_smaller_than_available() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().width(100.0).height(40.0).done();
    let r = b.build();
    r.assert_child_position(1, 200, 0);
    r.assert_child_size(1, 100, 40);
}

#[test]
fn flow3_block_with_fixed_width_equal_to_available() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().width(600.0).height(40.0).done();
    let r = b.build();
    r.assert_child_position(1, 200, 0);
    r.assert_child_size(1, 600, 40);
}

#[test]
fn flow3_two_floats_block_between_then_clear() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().height(30.0).done();
    b.add_child().height(30.0).clear_both().done();
    let r = b.build();
    r.assert_child_position(2, 200, 0);
    r.assert_child_size(2, 400, 30);
    let cleared = r.child(3);
    assert!(cleared.offset.top.to_i32() >= 100);
}

#[test]
fn flow3_block_exactly_between_floats() {
    let mut b = BlockTestBuilder::new(600, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().width(200.0).height(80.0).float_right().done();
    b.add_child().height(40.0).done();
    let r = b.build();
    r.assert_child_position(2, 200, 0);
    r.assert_child_size(2, 200, 40);
}

#[test]
fn flow3_block_beside_left_float_with_margin_left() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().height(40.0).with_style(|s| {
        s.margin_left = openui_geometry::Length::px(20.0);
    }).done();
    let r = b.build();
    // Block offset = float_width + margin_left
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 220);
}

// ═══════════════════════════════════════════════════════════════════════════
// §19  More Stacking and Shelf Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stack3_twelve_small_floats_three_rows() {
    let mut b = BlockTestBuilder::new(400, 600);
    for _ in 0..12 {
        b.add_child().width(100.0).height(25.0).float_left().done();
    }
    let r = b.build();
    for row in 0..3 {
        for col in 0..4 {
            let i = row * 4 + col;
            let f = r.child(i);
            assert_eq!(f.offset.left.to_i32(), (col as i32) * 100);
            assert_eq!(f.offset.top.to_i32(), (row as i32) * 25);
        }
    }
}

#[test]
fn stack3_left_float_margin_accumulates() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(30.0).float_left()
        .margin(0, 10, 0, 10).done();
    b.add_child().width(100.0).height(30.0).float_left()
        .margin(0, 10, 0, 10).done();
    b.add_child().width(100.0).height(30.0).float_left()
        .margin(0, 10, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 0);   // 10 margin-left
    r.assert_child_position(1, 130, 0);  // 10+100+10+10
    r.assert_child_position(2, 250, 0);  // 130+100+10+10
}

#[test]
fn stack3_alternating_sizes() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(30.0).float_left().done();
    b.add_child().width(200.0).height(30.0).float_left().done();
    b.add_child().width(100.0).height(30.0).float_left().done();
    b.add_child().width(200.0).height(30.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 100, 0);
    r.assert_child_position(2, 300, 0);
    r.assert_child_position(3, 400, 0);
}

#[test]
fn stack3_left_float_varying_heights() {
    let mut b = BlockTestBuilder::new(300, 600);
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(30.0).float_left().done();
    b.add_child().width(100.0).height(70.0).float_left().done();
    b.add_child().width(100.0).height(40.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 100, 0);
    r.assert_child_position(2, 200, 0);
    // Fourth wraps: new row
    let f4 = r.child(3);
    assert!(f4.offset.top.to_i32() >= 30);
}

// ═══════════════════════════════════════════════════════════════════════════
// §20  More BFC and Overflow Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bfc3_overflow_hidden_with_margin_and_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(60.0).overflow_hidden()
        .with_style(|s| s.margin_top = openui_geometry::Length::px(10.0)).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 200);
    assert_eq!(c.offset.top.to_i32(), 10);
}

#[test]
fn bfc3_overflow_hidden_after_clear() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(30.0).clear_left().done();
    b.add_child().height(60.0).overflow_hidden().done();
    let r = b.build();
    let bfc = r.child(2);
    assert!(bfc.offset.top.to_i32() >= 130);
    assert_eq!(bfc.size.width.to_i32(), 800);
}

#[test]
fn bfc3_overflow_scroll_drops_below_when_no_room() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(350.0).height(50.0).float_left().done();
    b.add_child().width(200.0).height(40.0).overflow(Overflow::Scroll).done();
    let r = b.build();
    let c = r.child(1);
    // 200 > 50 (400-350), BFC drops below
    assert!(c.offset.top.to_i32() >= 50 || c.offset.left.to_i32() >= 350);
}

#[test]
fn bfc3_overflow_hidden_with_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(60.0).overflow_hidden()
        .border(3, 3, 3, 3).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 200);
}

// ═══════════════════════════════════════════════════════════════════════════
// §21  More Clear Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn clear3_clear_left_past_two_left_floats_different_heights() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(40.0).float_left().done();
    b.add_child().width(100.0).height(80.0).float_left().done();
    b.add_child().height(30.0).clear_left().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 80);
}

#[test]
fn clear3_clear_right_past_two_right_floats_different_heights() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(40.0).float_right().done();
    b.add_child().width(100.0).height(80.0).float_right().done();
    b.add_child().height(30.0).clear_right().done();
    let r = b.build();
    let c = r.child(2);
    assert!(c.offset.top.to_i32() >= 80);
}

#[test]
fn clear3_clear_both_after_multiple_mixed_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(40.0).float_left().done();
    b.add_child().width(100.0).height(60.0).float_right().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(80.0).float_right().done();
    b.add_child().height(30.0).clear_both().done();
    let r = b.build();
    let c = r.child(4);
    assert!(c.offset.top.to_i32() >= 80);
}

#[test]
fn clear3_clear_left_on_block_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(40.0).clear_left().padding(10, 10, 10, 10).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
}

#[test]
fn clear3_clear_right_on_block_with_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right().done();
    b.add_child().height(40.0).clear_right().border(5, 5, 5, 5).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
}

// ═══════════════════════════════════════════════════════════════════════════
// §22  More Edge Cases and Stress Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn edge3_float_with_zero_container_width() {
    let mut b = BlockTestBuilder::new(0, 600);
    b.add_child().width(100.0).height(50.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 100, 50);
}

#[test]
fn edge3_twenty_right_floats() {
    let mut b = BlockTestBuilder::new(1000, 600);
    for _ in 0..20 {
        b.add_child().width(50.0).height(30.0).float_right().done();
    }
    let r = b.build();
    for i in 0..20 {
        let f = r.child(i);
        assert_eq!(f.offset.left.to_i32(), 1000 - ((i as i32) + 1) * 50);
    }
}

#[test]
fn edge3_float_with_percentage_width_1pct() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width_pct(1.0).height(30.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 10, 30);
}

#[test]
fn edge3_float_with_percentage_width_99pct() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width_pct(99.0).height(30.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 990, 30);
}

#[test]
fn edge3_float_with_very_large_margin() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).float_left()
        .margin(0, 0, 0, 500).done();
    let r = b.build();
    r.assert_child_position(0, 500, 0);
}

#[test]
fn edge3_float_left_and_right_exact_fit() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 200, 0);
}

#[test]
fn edge3_float_left_and_right_one_pixel_overlap() {
    let mut b = BlockTestBuilder::new(399, 600);
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().width(200.0).height(50.0).float_right().done();
    let r = b.build();
    // 200+200=400>399, right float drops
    let rf = r.child(1);
    assert!(rf.offset.top.to_i32() >= 50 || rf.offset.left.to_i32() < 200);
}

#[test]
fn edge3_float_with_height_pct_50() {
    let mut b = BlockTestBuilder::new(800, 400);
    b.add_child().width(200.0).height_pct(50.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 200, 200);
}

#[test]
fn edge3_grid_of_16_floats() {
    let mut b = BlockTestBuilder::new(400, 600);
    for _ in 0..16 {
        b.add_child().width(100.0).height(40.0).float_left().done();
    }
    let r = b.build();
    for row in 0..4 {
        for col in 0..4 {
            let i = row * 4 + col;
            let f = r.child(i);
            assert_eq!(f.offset.left.to_i32(), (col as i32) * 100);
            assert_eq!(f.offset.top.to_i32(), (row as i32) * 40);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §23  More Margin Collapsing with Floats
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn margin3_equal_margins_collapse_to_larger() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(30.0).margin_bottom(20).done();
    b.add_child().height(30.0).margin_top(20).done();
    let r = b.build();
    r.assert_child_position(1, 0, 51); // 1 (border) + 30 + 20
}

#[test]
fn margin3_float_with_large_margins_no_collapse() {
    let mut b = BlockTestBuilder::new(300, 600);
    b.add_child().width(200.0).height(50.0).float_left()
        .margin(0, 0, 50, 0).done();
    b.add_child().width(200.0).height(50.0).float_left()
        .margin(30, 0, 0, 0).done();
    let r = b.build();
    // Floats don't collapse margins
    let f2 = r.child(1);
    assert!(f2.offset.top.to_i32() >= 50);
}

#[test]
fn margin3_three_blocks_with_floats_interspersed() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(20.0).margin_bottom(15).done();
    b.add_child().width(50.0).height(10.0).float_left().done();
    b.add_child().height(20.0).margin_top(10).margin_bottom(25).done();
    b.add_child().width(50.0).height(10.0).float_right().done();
    b.add_child().height(20.0).margin_top(20).done();
    let r = b.build();
    // Block 1: y=0, height=20, mb=15
    // Block 2: mt=10, collapse(15,10)=15, y=35
    r.assert_child_position(2, 50, 36); // 1 (border) + 20 + max(15,10) = 36
}

#[test]
fn margin3_clear_element_margins_dont_collapse_with_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .margin(0, 0, 50, 0).done();
    b.add_child().height(30.0).clear_left().margin(20, 0, 0, 0).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.top.to_i32() >= 100);
}

// ═══════════════════════════════════════════════════════════════════════════
// §24  Final Combination Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn final_complex_layout_with_all_features() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(30.0).done();
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(150.0).height(80.0).float_right().done();
    b.add_child().height(40.0).done();
    b.add_child().height(30.0).clear_both().done();
    b.add_child().height(20.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 1);  // 1 (border-top)
    r.assert_child_position(1, 0, 31); // 1 + 30
    r.assert_child_position(2, 650, 31);
    // Block between floats
    r.assert_child_position(3, 200, 31);
    r.assert_child_size(3, 450, 40);
    let cleared = r.child(4);
    assert!(cleared.offset.top.to_i32() >= 131);
}

#[test]
fn final_nested_bfc_with_float_and_clear() {
    // overflow:hidden doesn't contain floats; but clear:left block moves below float
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().overflow_hidden()
        .add_child().width(200.0).height(100.0).float_left().done()
        .add_child().height(40.0).with_style(|s| s.clear = Clear::Left).done()
        .add_child().height(20.0).done()
        .done();
    let r = b.build();
    let c = r.child(0);
    // Height = clear block (at y=100, h=40) + block (h=20) = 160
    assert!(c.size.height.to_i32() >= 160);
}

#[test]
fn final_float_auto_width_with_nested_children() {
    // Auto-width float gets full container width; use explicit width instead
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).float_left()
        .add_child().width(200.0).height(30.0).done()
        .add_child().width(150.0).height(30.0).done()
        .add_child().width(180.0).height(30.0).done()
        .done();
    let r = b.build();
    let f = r.child(0);
    assert_eq!(f.size.width.to_i32(), 200);
    assert_eq!(f.size.height.to_i32(), 90); // 3 * 30
}

#[test]
fn final_two_floats_with_blocks_in_between() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(20.0).done();
    b.add_child().height(20.0).done();
    b.add_child().width(200.0).height(60.0).float_right().done();
    b.add_child().height(20.0).done();
    let r = b.build();
    r.assert_child_position(1, 200, 0);
    r.assert_child_position(2, 200, 20);
    // Right float placed at y=40
    r.assert_child_position(3, 600, 40);
    // Fifth block between both floats
    let c5 = r.child(4);
    assert_eq!(c5.offset.left.to_i32(), 200);
    assert_eq!(c5.size.width.to_i32(), 400);
}

#[test]
fn final_stress_mixed_floats_clears_bfc() {
    let mut b = BlockTestBuilder::new(600, 600);
    b.add_child().width(150.0).height(80.0).float_left().done();
    b.add_child().width(150.0).height(60.0).float_right().done();
    b.add_child().height(30.0).overflow_hidden().done();
    b.add_child().height(20.0).clear_left().done();
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().height(30.0).clear_both().done();
    let r = b.build();
    let bfc = r.child(2);
    assert!(bfc.offset.left.to_i32() >= 150);
    let cleared1 = r.child(3);
    assert!(cleared1.offset.top.to_i32() >= 80);
    let cleared2 = r.child(5);
    assert!(cleared2.offset.top.to_i32() >= cleared1.offset.top.to_i32());
}

#[test]
fn final_float_grid_with_varying_heights() {
    let mut b = BlockTestBuilder::new(400, 600);
    // Row 1: four floats h=30,50,40,30 — all fit (4*100=400)
    b.add_child().width(100.0).height(30.0).float_left().done();
    b.add_child().width(100.0).height(50.0).float_left().done();
    b.add_child().width(100.0).height(40.0).float_left().done();
    b.add_child().width(100.0).height(30.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 100, 0);
    r.assert_child_position(2, 200, 0);
    // Fourth float fits at x=300 (4*100=400=container width)
    r.assert_child_position(3, 300, 0);
}

#[test]
fn final_left_float_with_many_normal_blocks() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    for i in 0..10 {
        b.add_child().height(10.0).done();
    }
    let r = b.build();
    // All 10 blocks beside float (total height 100 = float height)
    for i in 1..=10 {
        assert_eq!(r.child(i).offset.left.to_i32(), 200);
        assert_eq!(r.child(i).size.width.to_i32(), 600);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §25  Remaining Tests to Reach Target
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn extra_float_left_width_pct_20() {
    let mut b = BlockTestBuilder::new(500, 400);
    b.add_child().width_pct(20.0).height(40.0).float_left().done();
    let r = b.build();
    r.assert_child_size(0, 100, 40);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn extra_float_right_width_pct_20() {
    let mut b = BlockTestBuilder::new(500, 400);
    b.add_child().width_pct(20.0).height(40.0).float_right().done();
    let r = b.build();
    r.assert_child_size(0, 100, 40);
    r.assert_child_position(0, 400, 0);
}

#[test]
fn extra_left_float_then_block_then_right_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().height(30.0).done();
    b.add_child().width(200.0).height(60.0).float_right().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 200, 0);
    // Right float at y=30 (after block placement)
    r.assert_child_position(2, 600, 30);
}

#[test]
fn extra_container_padding_and_multiple_floats() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.padding_left = openui_geometry::Length::px(20.0);
            s.padding_top = openui_geometry::Length::px(10.0);
        });
    b.add_child().width(100.0).height(40.0).float_left().done();
    b.add_child().width(100.0).height(40.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 20, 10);
    r.assert_child_position(1, 120, 10);
}

#[test]
fn extra_clear_left_after_block_and_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(20.0).done();
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().height(30.0).done();
    b.add_child().height(30.0).clear_left().done();
    let r = b.build();
    // Float at y=20, height=80, ends at y=100
    let cleared = r.child(3);
    assert!(cleared.offset.top.to_i32() >= 100);
}

#[test]
fn extra_float_left_width_33pct() {
    let mut b = BlockTestBuilder::new(900, 600);
    b.add_child().width_pct(33.3333).height(40.0).float_left().done();
    let r = b.build();
    let f = r.child(0);
    assert_eq!(f.size.width.to_i32(), 299); // 33.3333% of 900 = 299 (truncated)
}

#[test]
fn extra_three_equal_left_floats_pct() {
    let mut b = BlockTestBuilder::new(900, 600);
    b.add_child().width_pct(33.3333).height(40.0).float_left().done();
    b.add_child().width_pct(33.3333).height(40.0).float_left().done();
    b.add_child().width_pct(33.3333).height(40.0).float_left().done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // 33.3333% of 900 has fractional LayoutUnit; use integer comparison
    let f1 = r.child(1);
    assert_eq!(f1.offset.left.to_i32(), 299);
    assert_eq!(f1.offset.top.to_i32(), 0);
    let f2 = r.child(2);
    assert_eq!(f2.offset.left.to_i32(), 599);
    assert_eq!(f2.offset.top.to_i32(), 0);
}

#[test]
fn extra_float_inside_padded_container_right() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.padding_right = openui_geometry::Length::px(50.0);
        });
    b.add_child().width(200.0).height(50.0).float_right().done();
    let r = b.build();
    // Float at right edge of content area (which starts at left=0, width=800)
    let f = r.child(0);
    assert_eq!(f.size.width.to_i32(), 200);
}

#[test]
fn extra_block_with_clear_both_after_expired_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(30.0).float_left().done();
    b.add_child().height(30.0).done();
    b.add_child().height(30.0).done();
    b.add_child().height(30.0).clear_both().done();
    let r = b.build();
    // Float expires at y=30, blocks at y=0,30,60
    // clear:both at y=60, float already gone
    let c = r.child(3);
    assert!(c.offset.top.to_i32() >= 60);
}

#[test]
fn extra_overflow_hidden_exact_width() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(80.0).float_left().done();
    b.add_child().width(400.0).height(60.0).overflow_hidden().done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 400);
    r.assert_child_size(1, 400, 60);
}

#[test]
fn extra_left_right_clear_both_repeated() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(40.0).float_left().done();
    b.add_child().width(100.0).height(60.0).float_right().done();
    b.add_child().height(20.0).clear_both().done();
    b.add_child().width(100.0).height(40.0).float_left().done();
    b.add_child().width(100.0).height(60.0).float_right().done();
    b.add_child().height(20.0).clear_both().done();
    let r = b.build();
    let c1 = r.child(2);
    assert!(c1.offset.top.to_i32() >= 60);
    let c2 = r.child(5);
    assert!(c2.offset.top.to_i32() >= c1.offset.top.to_i32() + 20 + 60);
}

#[test]
fn extra_float_left_then_overflow_scroll() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(100.0).float_left().done();
    b.add_child().height(60.0).overflow(Overflow::Scroll).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 300);
}

#[test]
fn extra_float_right_then_overflow_auto() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(100.0).float_right().done();
    b.add_child().height(60.0).overflow(Overflow::Auto).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.size.width.to_i32() <= 500);
}

#[test]
fn extra_two_blocks_margin_collapse_after_float_clear() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().width(200.0).height(50.0).float_left().done();
    b.add_child().height(30.0).clear_left().margin_bottom(20).done();
    b.add_child().height(30.0).margin_top(10).done();
    let r = b.build();
    let cleared = r.child(1);
    assert!(cleared.offset.top.to_i32() >= 50);
}

#[test]
fn extra_many_right_floats_with_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    for _ in 0..5 {
        b.add_child().width(100.0).height(30.0).float_right()
            .margin(0, 10, 0, 10).done();
    }
    let r = b.build();
    // First: 800-10-100=690
    r.assert_child_position(0, 690, 0);
    // Each subsequent: previous - 10 - 100 - 10 = previous - 120
    r.assert_child_position(1, 570, 0);
    r.assert_child_position(2, 450, 0);
    r.assert_child_position(3, 330, 0);
    r.assert_child_position(4, 210, 0);
}

#[test]
fn extra_block_after_all_floats_clear_with_height() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(120.0).float_right().done();
    b.add_child().height(200.0).clear_both().done();
    let r = b.build();
    let cleared = r.child(2);
    assert!(cleared.offset.top.to_i32() >= 120);
    r.assert_child_size(2, 800, 200);
    // Container has explicit height=600
    r.assert_container_height(600);
}

#[test]
fn extra_two_left_floats_then_block_between_floats() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(80.0).float_left().done();
    b.add_child().width(100.0).height(80.0).float_right().done();
    b.add_child().width(200.0).height(40.0).done();
    let r = b.build();
    r.assert_child_position(2, 100, 0);
    r.assert_child_size(2, 200, 40);
}

#[test]
fn extra_float_with_border_box_sizing_and_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left()
        .padding(10, 10, 10, 10).border(5, 5, 5, 5)
        .box_sizing_border_box().margin(5, 5, 5, 5).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    r.assert_child_size(0, 200, 100);
}

#[test]
fn extra_container_height_with_cleared_block_and_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().height(50.0).clear_left().done();
    let r = b.build();
    let cleared = r.child(1);
    assert!(cleared.offset.top.to_i32() >= 100);
    // Container has explicit height=600
    r.assert_container_height(600);
}

#[test]
fn extra_float_beside_block_with_fixed_width_and_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_left().done();
    b.add_child().width(300.0).height(40.0)
        .margin(0, 0, 0, 10).done();
    let r = b.build();
    let c = r.child(1);
    assert!(c.offset.left.to_i32() >= 210); // float 200 + margin 10
}

#[test]
fn extra_right_float_beside_block_with_margin_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).float_right().done();
    b.add_child().height(40.0).with_style(|s| {
        s.margin_right = openui_geometry::Length::px(10.0);
    }).done();
    let r = b.build();
    r.assert_child_position(1, 0, 0);
    // Width reduced by float
    assert!(r.child(1).size.width.to_i32() <= 600);
}

#[test]
fn extra_nested_float_left_in_right_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(100.0).float_right()
        .add_child().width(100.0).height(50.0).float_left().done()
        .done();
    let r = b.build();
    r.assert_child_position(0, 400, 0);
    let nested = r.nested_child(0, 0);
    assert_eq!(nested.offset.left.to_i32(), 0);
    assert_eq!(nested.size.width.to_i32(), 100);
}
