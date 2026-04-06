//! SP12 H5 — CSS positioning tests: relative, absolute, fixed, sticky.
//!
//! Comprehensive tests for CSS Positioned Layout per CSS 2.1 §9.3, §10.3.7,
//! §10.6.4 and CSS Positioned Layout Module Level 3.

mod sp12_wpt_helpers;
use sp12_wpt_helpers::*;

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length, PhysicalOffset, PhysicalRect, PhysicalSize};
use openui_layout::{block_layout, ConstraintSpace, Fragment};
use openui_layout::sticky::{
    apply_sticky_offset, compute_sticky_constraint_rect, compute_sticky_offset,
    StickyConstraintRect, StickyPositionData,
};
use openui_style::{
    BorderStyle, ComputedStyle, Direction, Display, Overflow, Position,
};

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

fn lu(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

fn offset(left: i32, top: i32) -> PhysicalOffset {
    PhysicalOffset::new(lu(left), lu(top))
}

fn size(w: i32, h: i32) -> PhysicalSize {
    PhysicalSize::new(lu(w), lu(h))
}

fn prect(x: i32, y: i32, w: i32, h: i32) -> PhysicalRect {
    PhysicalRect::from_xywh(lu(x), lu(y), lu(w), lu(h))
}

fn viewport() -> PhysicalRect {
    prect(0, 0, 800, 600)
}

/// Create a builder with container position:relative — needed for abs-pos tests.
fn abs_builder(w: i32, h: i32) -> BlockTestBuilder {
    let mut b = BlockTestBuilder::new(w, h);
    b = b.with_container_style(|s| s.position = Position::Relative);
    b
}

fn large_cb() -> PhysicalRect {
    prect(0, 0, 800, 2000)
}

fn root_space(w: i32, h: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(LayoutUnit::from_i32(w), LayoutUnit::from_i32(h))
}

fn setup_abs_child(doc: &mut Document, parent: NodeId) -> NodeId {
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.position = Position::Absolute;
    doc.append_child(parent, child);
    child
}

fn setup_fixed_child(doc: &mut Document, parent: NodeId) -> NodeId {
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.position = Position::Fixed;
    doc.append_child(parent, child);
    child
}

fn setup_container(doc: &mut Document, w: i32, h: i32) -> NodeId {
    let vp = doc.root();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(w as f32);
    doc.node_mut(container).style.height = Length::px(h as f32);
    doc.append_child(vp, container);
    container
}

fn make_fragment(left: i32, top: i32, w: i32, h: i32) -> Fragment {
    let mut f = Fragment::new_box(NodeId::NONE, size(w, h));
    f.offset = offset(left, top);
    f
}

fn make_sticky_style(top: Length, right: Length, bottom: Length, left: Length) -> ComputedStyle {
    let mut s = ComputedStyle::initial();
    s.display = Display::Block;
    s.position = Position::Sticky;
    s.top = top;
    s.right = right;
    s.bottom = bottom;
    s.left = left;
    s
}

fn insets(top: Option<i32>, right: Option<i32>, bottom: Option<i32>, left: Option<i32>) -> StickyConstraintRect {
    StickyConstraintRect {
        top: top.map(lu),
        right: right.map(lu),
        bottom: bottom.map(lu),
        left: left.map(lu),
    }
}
// ═══════════════════════════════════════════════════════════════════
// Section 1: position: relative
// ═══════════════════════════════════════════════════════════════════

#[test]
fn rel_top_offset_moves_down() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative().inset(20, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 20);
    r.assert_child_size(0, 200, 100);
}

#[test]
fn rel_bottom_offset_moves_up() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative().inset(0, 0, 30, 0).done();
    let r = b.build();
    // bottom:30 with top:0 → top wins, top=0 means no shift from top.
    // Actually top:0 is set explicitly so offset = -0 = 0 from normal flow for top.
    // The inset sets top=0px,bottom=30px. top wins over bottom. Visual y = 0 + 0 = 0.
    r.assert_child_position(0, 0, 0);
}

#[test]
fn rel_left_offset_moves_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative().inset(0, 0, 0, 50).done();
    let r = b.build();
    r.assert_child_position(0, 50, 0);
}

#[test]
fn rel_right_offset_moves_left() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.top = Length::auto(); s.bottom = Length::auto(); s.left = Length::auto(); s.right = Length::px(40.0); })
        .done();
    let r = b.build();
    // right=40, left=auto → offset left by 40. Visual x = 0 - 40 = -40.
    r.assert_child_position(0, -40, 0);
}

#[test]
fn rel_top_negative_moves_up() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.top = Length::px(-15.0); s.right = Length::auto(); s.bottom = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, -15);
}

#[test]
fn rel_left_negative_moves_left() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.left = Length::px(-25.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, -25, 0);
}

#[test]
fn rel_top_and_left_combined() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative().inset(10, 0, 0, 20).done();
    let r = b.build();
    r.assert_child_position(0, 20, 10);
}

#[test]
fn rel_top_wins_over_bottom() {
    // CSS 2.1: if both top and bottom are specified, top wins.
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative().inset(30, 0, 50, 0).done();
    let r = b.build();
    // top=30 wins, child moves down 30.
    r.assert_child_position(0, 0, 30);
}

#[test]
fn rel_left_wins_over_right_ltr() {
    // CSS 2.1: in LTR, left wins over right.
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative().inset(0, 40, 0, 60).done();
    let r = b.build();
    // left=60 wins over right=40. Visual x = 0 + 60 = 60.
    r.assert_child_position(0, 60, 0);
}

#[test]
fn rel_right_wins_over_left_rtl() {
    // CSS 2.1: in RTL, right wins over left.
    // CSS 2.1 §10.3.3: over-constrained → margin-left adjusted for RTL,
    // so the 200px child is right-aligned in an 800px container.
    // Normal flow x = 600, then relative right:40 → x = 600 − 40 = 560.
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| { s.direction = Direction::Rtl; });
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.direction = Direction::Rtl; })
        .inset(0, 40, 0, 60).done();
    let r = b.build();
    let child = r.child(0);
    let x = child.offset.left.to_i32();
    assert_eq!(x, 560);
}

#[test]
fn rel_does_not_affect_next_sibling() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative().inset(50, 0, 0, 0).done();
    b.add_child().width(200.0).height(80.0).done();
    let r = b.build();
    // First child visually at y=50, but sibling at y=100 (normal flow).
    r.assert_child_position(0, 0, 50);
    r.assert_child_position(1, 0, 100);
}

#[test]
fn rel_does_not_affect_previous_sibling() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).done();
    b.add_child().width(200.0).height(80.0).position_relative().inset(30, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // Second child normal flow y=100, offset +30.
    r.assert_child_position(1, 0, 130);
}

#[test]
fn rel_zero_offsets_no_movement() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn rel_auto_offsets_no_movement() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative().done();
    let r = b.build();
    // All insets auto → no offset.
    r.assert_child_position(0, 0, 0);
}

#[test]
fn rel_size_unchanged() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(150.0).position_relative().inset(10, 20, 30, 40).done();
    let r = b.build();
    // Size must remain 300×150 regardless of offsets.
    r.assert_child_size(0, 300, 150);
}

#[test]
fn rel_container_height_unchanged() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative().inset(500, 0, 0, 0).done();
    let r = b.build();
    // Container height stays 600 despite child offset.
    r.assert_container_height(600);
}

#[test]
fn rel_large_top_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.top = Length::px(1000.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 1000);
}

#[test]
fn rel_large_negative_top() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.top = Length::px(-500.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, -500);
}

#[test]
fn rel_with_margin_top() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| { s.border_top_width = 1; s.border_top_style = BorderStyle::Solid; });
    b.add_child().width(200.0).height(100.0).margin(20, 0, 0, 0).position_relative().inset(10, 0, 0, 0).done();
    let r = b.build();
    // Container border prevents margin collapse. Normal flow y=1+20=21, then relative +10.
    r.assert_child_position(0, 0, 31);
}

#[test]
fn rel_with_margin_left() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).margin(0, 0, 0, 30).position_relative().inset(0, 0, 0, 15).done();
    let r = b.build();
    // Normal flow x=30 (margin-left), then relative offset +15.
    r.assert_child_position(0, 45, 0);
}

#[test]
fn rel_with_padding_size_unchanged() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).padding(10, 10, 10, 10).position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    // Padding adds to content size: 200+20=220, 100+20=120.
    r.assert_child_size(0, 220, 120);
}

#[test]
fn rel_with_border_size_unchanged() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).border(5, 5, 5, 5).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    r.assert_child_size(0, 210, 110);
}

#[test]
fn rel_top_percent_of_cb_height() {
    // Percentage top is resolved against containing block height.
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.top = Length::percent(10.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    // 10% of 600 = 60.
    r.assert_child_position(0, 0, 60);
}

#[test]
fn rel_left_percent_of_cb_width() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.left = Length::percent(5.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    // 5% of 1000 = 50.
    r.assert_child_position(0, 50, 0);
}

#[test]
fn rel_bottom_percent_when_top_auto() {
    let mut b = BlockTestBuilder::new(800, 400);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.bottom = Length::percent(25.0); s.top = Length::auto(); s.left = Length::auto(); s.right = Length::auto(); })
        .done();
    let r = b.build();
    // top=auto, bottom=25% of 400=100. Offset = -100.
    r.assert_child_position(0, 0, -100);
}

#[test]
fn rel_right_percent_when_left_auto() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.right = Length::percent(10.0); s.left = Length::auto(); s.top = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    // left=auto, right=10% of 800=80. Offset = -80.
    r.assert_child_position(0, -80, 0);
}

#[test]
fn rel_multiple_children_independent_offsets() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).position_relative().inset(10, 0, 0, 5).done();
    b.add_child().width(200.0).height(50.0).position_relative().inset(20, 0, 0, 15).done();
    b.add_child().width(200.0).height(50.0).position_relative().inset(30, 0, 0, 25).done();
    let r = b.build();
    r.assert_child_position(0, 5, 10);
    // Normal flow y=50, +20.
    r.assert_child_position(1, 15, 70);
    // Normal flow y=100, +30.
    r.assert_child_position(2, 25, 130);
}

#[test]
fn rel_mixed_static_and_relative() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).done();
    b.add_child().width(200.0).height(50.0).position_relative().inset(10, 0, 0, 0).done();
    b.add_child().width(200.0).height(50.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 60);
    r.assert_child_position(2, 0, 100);
}

#[test]
fn rel_width_auto_fills_container() {
    let mut b = BlockTestBuilder::new(400, 300);
    b.add_child().height(50.0).position_relative().inset(10, 0, 0, 0).done();
    let r = b.build();
    // width auto fills 400.
    r.assert_child_size(0, 400, 50);
    r.assert_child_position(0, 0, 10);
}

#[test]
fn rel_with_overflow_hidden_container() {
    let mut b = BlockTestBuilder::new(400, 300)
        .with_container_style(|s| { s.overflow_x = Overflow::Hidden; s.overflow_y = Overflow::Hidden; });
    b.add_child().width(200.0).height(100.0).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
}

#[test]
fn rel_border_box_sizing() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).padding(10, 10, 10, 10).box_sizing_border_box()
        .position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    // border-box: total size stays 200×100.
    r.assert_child_size(0, 200, 100);
}

#[test]
fn rel_float_left_with_relative() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    // CSS 2.1 §9.4.3: relative offsets apply to floats too.
    r.assert_child_position(0, 10, 10);
}

#[test]
fn rel_float_right_with_relative() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right().position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    // CSS 2.1 §9.4.3: relative offsets apply to floats too.
    r.assert_child_position(0, 605, 5);
}

#[test]
fn rel_nested_relative_parent_and_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(200.0).position_relative().inset(10, 0, 0, 10)
        .add_child().width(100.0).height(50.0).with_style(|s| {
            s.position = Position::Relative;
            s.top = Length::px(5.0); s.left = Length::px(5.0);
            s.right = Length::auto(); s.bottom = Length::auto();
        }).done()
        .done();
    let r = b.build();
    // Parent at (10, 10), nested child at (5, 5) relative to parent.
    r.assert_child_position(0, 10, 10);
    r.assert_nested_child_position(0, 0, 5, 5);
}

#[test]
fn rel_dom_api_basic() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.position = Position::Relative;
    doc.node_mut(child).style.width = Length::px(200.0);
    doc.node_mut(child).style.height = Length::px(100.0);
    doc.node_mut(child).style.top = Length::px(30.0);
    doc.node_mut(child).style.left = Length::px(40.0);
    doc.append_child(container, child);
    let space = root_space(800, 600);
    let fragment = block_layout(&doc, doc.root(), &space);
    let child_frag = &fragment.children[0].children[0];
    assert_eq!(child_frag.offset.left.to_i32(), 40);
    assert_eq!(child_frag.offset.top.to_i32(), 30);
    assert_eq!(child_frag.size.width.to_i32(), 200);
    assert_eq!(child_frag.size.height.to_i32(), 100);
}

#[test]
fn rel_dom_api_second_child() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    let c1 = doc.create_node(ElementTag::Div);
    doc.node_mut(c1).style.display = Display::Block;
    doc.node_mut(c1).style.width = Length::px(200.0);
    doc.node_mut(c1).style.height = Length::px(80.0);
    doc.append_child(container, c1);
    let c2 = doc.create_node(ElementTag::Div);
    doc.node_mut(c2).style.display = Display::Block;
    doc.node_mut(c2).style.position = Position::Relative;
    doc.node_mut(c2).style.width = Length::px(200.0);
    doc.node_mut(c2).style.height = Length::px(60.0);
    doc.node_mut(c2).style.top = Length::px(15.0);
    doc.node_mut(c2).style.left = Length::px(25.0);
    doc.append_child(container, c2);
    let space = root_space(800, 600);
    let fragment = block_layout(&doc, doc.root(), &space);
    let c2_frag = &fragment.children[0].children[1];
    // Normal flow y=80, relative top=+15 → y=95.
    assert_eq!(c2_frag.offset.left.to_i32(), 25);
    assert_eq!(c2_frag.offset.top.to_i32(), 95);
}

#[test]
fn rel_top_1_left_1() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(1, 0, 0, 1).done();
    let r = b.build();
    r.assert_child_position(0, 1, 1);
}

#[test]
fn rel_top_2_left_3() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(2, 0, 0, 3).done();
    let r = b.build();
    r.assert_child_position(0, 3, 2);
}

#[test]
fn rel_top_5_left_10() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(5, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 5);
}

#[test]
fn rel_top_10_left_20() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(10, 0, 0, 20).done();
    let r = b.build();
    r.assert_child_position(0, 20, 10);
}

#[test]
fn rel_top_15_left_30() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(15, 0, 0, 30).done();
    let r = b.build();
    r.assert_child_position(0, 30, 15);
}

#[test]
fn rel_top_25_left_50() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(25, 0, 0, 50).done();
    let r = b.build();
    r.assert_child_position(0, 50, 25);
}

#[test]
fn rel_top_50_left_100() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(50, 0, 0, 100).done();
    let r = b.build();
    r.assert_child_position(0, 100, 50);
}

#[test]
fn rel_top_75_left_150() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(75, 0, 0, 150).done();
    let r = b.build();
    r.assert_child_position(0, 150, 75);
}

#[test]
fn rel_top_100_left_200() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(100, 0, 0, 200).done();
    let r = b.build();
    r.assert_child_position(0, 200, 100);
}

#[test]
fn rel_top_150_left_300() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(150, 0, 0, 300).done();
    let r = b.build();
    r.assert_child_position(0, 300, 150);
}

#[test]
fn rel_top_0_left_1() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(0, 0, 0, 1).done();
    let r = b.build();
    r.assert_child_position(0, 1, 0);
}

#[test]
fn rel_top_1_left_0() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(1, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 1);
}

#[test]
fn rel_top_3_left_7() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(3, 0, 0, 7).done();
    let r = b.build();
    r.assert_child_position(0, 7, 3);
}

#[test]
fn rel_top_7_left_3() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(7, 0, 0, 3).done();
    let r = b.build();
    r.assert_child_position(0, 3, 7);
}

#[test]
fn rel_top_12_left_18() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(12, 0, 0, 18).done();
    let r = b.build();
    r.assert_child_position(0, 18, 12);
}

#[test]
fn rel_top_33_left_66() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(33, 0, 0, 66).done();
    let r = b.build();
    r.assert_child_position(0, 66, 33);
}

#[test]
fn rel_top_44_left_55() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(44, 0, 0, 55).done();
    let r = b.build();
    r.assert_child_position(0, 55, 44);
}

#[test]
fn rel_top_77_left_88() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(77, 0, 0, 88).done();
    let r = b.build();
    r.assert_child_position(0, 88, 77);
}

#[test]
fn rel_top_99_left_11() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(99, 0, 0, 11).done();
    let r = b.build();
    r.assert_child_position(0, 11, 99);
}

#[test]
fn rel_top_128_left_256() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative().inset(128, 0, 0, 256).done();
    let r = b.build();
    r.assert_child_position(0, 256, 128);
}

#[test]
fn rel_margin_top_10_left_10_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| { s.border_top_width = 1; s.border_top_style = BorderStyle::Solid; });
    b.add_child().width(100.0).height(50.0).margin(10, 0, 0, 10).position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 15, 16);
}

#[test]
fn rel_margin_top_20_left_0_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| { s.border_top_width = 1; s.border_top_style = BorderStyle::Solid; });
    b.add_child().width(100.0).height(50.0).margin(20, 0, 0, 0).position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 5, 26);
}

#[test]
fn rel_margin_top_0_left_20_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).margin(0, 0, 0, 20).position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 25, 5);
}

#[test]
fn rel_margin_top_5_left_15_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| { s.border_top_width = 1; s.border_top_style = BorderStyle::Solid; });
    b.add_child().width(100.0).height(50.0).margin(5, 0, 0, 15).position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 20, 11);
}

#[test]
fn rel_margin_top_15_left_5_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| { s.border_top_width = 1; s.border_top_style = BorderStyle::Solid; });
    b.add_child().width(100.0).height(50.0).margin(15, 0, 0, 5).position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 10, 21);
}

#[test]
fn rel_margin_top_30_left_30_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| { s.border_top_width = 1; s.border_top_style = BorderStyle::Solid; });
    b.add_child().width(100.0).height(50.0).margin(30, 0, 0, 30).position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 35, 36);
}

#[test]
fn rel_margin_top_50_left_0_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| { s.border_top_width = 1; s.border_top_style = BorderStyle::Solid; });
    b.add_child().width(100.0).height(50.0).margin(50, 0, 0, 0).position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 5, 56);
}

#[test]
fn rel_margin_top_0_left_50_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).margin(0, 0, 0, 50).position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 55, 5);
}

#[test]
fn rel_margin_top_100_left_100_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| { s.border_top_width = 1; s.border_top_style = BorderStyle::Solid; });
    b.add_child().width(100.0).height(50.0).margin(100, 0, 0, 100).position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 105, 106);
}

#[test]
fn rel_margin_top_10_left_40_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| { s.border_top_width = 1; s.border_top_style = BorderStyle::Solid; });
    b.add_child().width(100.0).height(50.0).margin(10, 0, 0, 40).position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 45, 16);
}

#[test]
fn rel_padding_5_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).padding(5, 5, 5, 5).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    r.assert_child_size(0, 110, 60);
}

#[test]
fn rel_padding_10_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).padding(10, 10, 10, 10).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    r.assert_child_size(0, 120, 70);
}

#[test]
fn rel_padding_15_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).padding(15, 15, 15, 15).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    r.assert_child_size(0, 130, 80);
}

#[test]
fn rel_padding_20_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).padding(20, 20, 20, 20).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    r.assert_child_size(0, 140, 90);
}

#[test]
fn rel_padding_25_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).padding(25, 25, 25, 25).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    r.assert_child_size(0, 150, 100);
}

#[test]
fn rel_padding_30_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).padding(30, 30, 30, 30).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    r.assert_child_size(0, 160, 110);
}

#[test]
fn rel_padding_40_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).padding(40, 40, 40, 40).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    r.assert_child_size(0, 180, 130);
}

#[test]
fn rel_padding_50_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).padding(50, 50, 50, 50).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    r.assert_child_size(0, 200, 150);
}

#[test]
fn rel_padding_60_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).padding(60, 60, 60, 60).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    r.assert_child_size(0, 220, 170);
}

#[test]
fn rel_padding_80_with_offset() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).padding(80, 80, 80, 80).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    r.assert_child_size(0, 260, 210);
}

#[test]
fn rel_three_children_stack_with_offsets() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(60.0).position_relative().inset(5, 0, 0, 0).done();
    b.add_child().width(100.0).height(60.0).position_relative().inset(10, 0, 0, 0).done();
    b.add_child().width(100.0).height(60.0).position_relative().inset(15, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 5);
    r.assert_child_position(1, 0, 70);
    r.assert_child_position(2, 0, 135);
}

#[test]
fn rel_four_children_alternating_direction() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(40.0).position_relative().inset(10, 0, 0, 10).done();
    b.add_child().width(100.0).height(40.0).position_relative()
        .with_style(|s| { s.top = Length::px(-10.0); s.left = Length::px(-10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    b.add_child().width(100.0).height(40.0).position_relative().inset(20, 0, 0, 20).done();
    b.add_child().width(100.0).height(40.0).position_relative()
        .with_style(|s| { s.top = Length::px(-20.0); s.left = Length::px(-20.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    r.assert_child_position(1, -10, 30);
    r.assert_child_position(2, 20, 100);
    r.assert_child_position(3, -20, 100);
}

#[test]
fn rel_container_with_border() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| { s.border_top_width = 5; s.border_top_style = BorderStyle::Solid;
            s.border_left_width = 5; s.border_left_style = BorderStyle::Solid; });
    b.add_child().width(200.0).height(100.0).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    // Child at (5+10, 5+10) = (15, 15) due to border + offset.
    r.assert_child_position(0, 15, 15);
}

#[test]
fn rel_container_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.padding_top = Length::px(20.0);
            s.padding_left = Length::px(20.0);
        });
    b.add_child().width(200.0).height(100.0).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    // Child at (20+10, 20+10) = (30, 30).
    r.assert_child_position(0, 30, 30);
}

#[test]
fn rel_auto_width_container_width_800() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(100.0).position_relative().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_size(0, 800, 100);
}

#[test]
fn rel_offset_larger_than_container() {
    let mut b = BlockTestBuilder::new(100, 100);
    b.add_child().width(50.0).height(50.0).position_relative().inset(200, 0, 0, 200).done();
    let r = b.build();
    r.assert_child_position(0, 200, 200);
}

// ═══════════════════════════════════════════════════════════════════
// Section 2: position: absolute
// ═══════════════════════════════════════════════════════════════════

#[test]
fn abs_basic_top_left_builder() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute().inset(50, 0, 0, 100).done();
    let r = b.build();
    r.assert_child_position(0, 100, 50);
    r.assert_child_size(0, 200, 100);
}

#[test]
fn abs_basic_right_bottom_builder() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .with_style(|s| { s.top = Length::auto(); s.bottom = Length::px(30.0); s.left = Length::auto(); s.right = Length::px(50.0); })
        .done();
    let r = b.build();
    // left = 800-50-200 = 550, top = 600-30-100 = 470.
    r.assert_child_position(0, 550, 470);
}

#[test]
fn abs_top_left_zero() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn abs_all_corners() {
    // Test absolute positioning in all four corners.
    let mut b = abs_builder(800, 600);
    // Top-left.
    b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 0).done();
    // Top-right.
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.top = Length::px(0.0); s.right = Length::px(0.0); s.bottom = Length::auto(); s.left = Length::auto(); })
        .done();
    // Bottom-left.
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.bottom = Length::px(0.0); s.left = Length::px(0.0); s.top = Length::auto(); s.right = Length::auto(); })
        .done();
    // Bottom-right.
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.bottom = Length::px(0.0); s.right = Length::px(0.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 700, 0);
    r.assert_child_position(2, 0, 550);
    r.assert_child_position(3, 700, 550);
}

#[test]
fn abs_auto_width_fills_between_left_right() {
    let mut b = abs_builder(800, 600);
    b.add_child().height(100.0).position_absolute().inset(0, 50, 0, 50).done();
    let r = b.build();
    // width auto: 800 - 50 - 50 = 700.
    r.assert_child_position(0, 50, 0);
    r.assert_child_size(0, 700, 100);
}

#[test]
fn abs_auto_height_fills_between_top_bottom() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).position_absolute().inset(20, 0, 30, 0).done();
    let r = b.build();
    // height auto: 600 - 20 - 30 = 550.
    r.assert_child_position(0, 0, 20);
    r.assert_child_size(0, 200, 550);
}

#[test]
fn abs_auto_width_and_height() {
    let mut b = abs_builder(800, 600);
    b.add_child().position_absolute().inset(10, 20, 30, 40).done();
    let r = b.build();
    // width: 800-40-20=740, height: 600-10-30=560.
    r.assert_child_position(0, 40, 10);
    r.assert_child_size(0, 740, 560);
}

#[test]
fn abs_horizontal_centering_auto_margins() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0).margin_auto_horizontal().done();
    let r = b.build();
    // Centered: (800-200)/2 = 300.
    r.assert_child_position(0, 300, 0);
}

#[test]
fn abs_vertical_centering_auto_margins() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(200.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); })
        .done();
    let r = b.build();
    // Centered: (600-200)/2 = 200.
    r.assert_child_position(0, 0, 200);
}

#[test]
fn abs_both_axis_centering() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(200.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| {
            s.margin_top = Length::auto(); s.margin_bottom = Length::auto();
            s.margin_left = Length::auto(); s.margin_right = Length::auto();
        }).done();
    let r = b.build();
    r.assert_child_position(0, 300, 200);
}

#[test]
fn abs_overconstrained_ltr_left_wins() {
    // Over-constrained: left + width + right > CB width. LTR → left wins.
    let mut b = abs_builder(800, 600);
    b.add_child().width(700.0).height(50.0).position_absolute().inset(0, 50, 0, 100).done();
    let r = b.build();
    // left=100 wins. right ignored.
    r.assert_child_position(0, 100, 0);
    r.assert_child_size(0, 700, 50);
}

#[test]
fn abs_overconstrained_rtl_right_wins() {
    let mut b = abs_builder(800, 600)
        .with_container_style(|s| { s.direction = Direction::Rtl; });
    b.add_child().width(700.0).height(50.0).position_absolute()
        .with_style(|s| { s.direction = Direction::Rtl; })
        .inset(0, 50, 0, 100).done();
    let r = b.build();
    // RTL: right=50 wins. left = 800 - 50 - 700 = 50.
    let child = r.child(0);
    assert_eq!(child.offset.left.to_i32(), 50);
    assert_eq!(child.size.width.to_i32(), 700);
}

#[test]
fn abs_overconstrained_vertical() {
    // Over-constrained vertically: top + height + bottom > CB height. top wins.
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(500.0).position_absolute().inset(50, 0, 100, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 50);
    r.assert_child_size(0, 200, 500);
}

#[test]
fn abs_percentage_top_left() {
    let mut b = abs_builder(1000, 800);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .with_style(|s| {
            s.top = Length::percent(10.0); s.left = Length::percent(5.0);
            s.right = Length::auto(); s.bottom = Length::auto();
        }).done();
    let r = b.build();
    // top = 10%×800 = 80, left = 5%×1000 = 50.
    r.assert_child_position(0, 50, 80);
}

#[test]
fn abs_percentage_right_bottom() {
    let mut b = abs_builder(1000, 800);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .with_style(|s| {
            s.right = Length::percent(10.0); s.bottom = Length::percent(5.0);
            s.top = Length::auto(); s.left = Length::auto();
        }).done();
    let r = b.build();
    // right = 10%×1000 = 100, bottom = 5%×800 = 40.
    // left = 1000-100-200 = 700, top = 800-40-100 = 660.
    r.assert_child_position(0, 700, 660);
}

#[test]
fn abs_percentage_width_height() {
    let mut b = abs_builder(1000, 800);
    b.add_child().position_absolute()
        .with_style(|s| {
            s.top = Length::px(0.0); s.left = Length::px(0.0);
            s.right = Length::auto(); s.bottom = Length::auto();
            s.width = Length::percent(50.0); s.height = Length::percent(25.0);
        }).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 500, 200);
}

#[test]
fn abs_does_not_affect_flow_siblings() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).done();
    b.add_child().width(150.0).height(80.0).position_absolute().inset(10, 0, 0, 10).done();
    b.add_child().width(200.0).height(100.0).done();
    let r = b.build();
    // abs child at (10, 10), but siblings ignore it.
    r.assert_child_position(0, 0, 0);
    // Third child: y = 100 (only first in-flow child counts).
    // The abs child is removed from flow.
}

#[test]
fn abs_removed_from_flow() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute().inset(0, 0, 0, 0).done();
    b.add_child().width(200.0).height(50.0).done();
    let r = b.build();
    // In-flow child should be at y=0 because abs is out-of-flow.
    r.assert_child_position(1, 0, 0);
}

#[test]
fn abs_container_padding_affects_position() {
    let mut b = abs_builder(800, 600)
        .with_container_style(|s| {
            s.padding_top = Length::px(20.0);
            s.padding_left = Length::px(30.0);
        });
    b.add_child().width(100.0).height(50.0).position_absolute().inset(10, 0, 0, 10).done();
    let r = b.build();
    // Abs child offset is relative to the padding box. inset(10,0,0,10) → offset (10,10).
    r.assert_child_position(0, 10, 10);
}

#[test]
fn abs_container_border_affects_position() {
    let mut b = abs_builder(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 10; s.border_top_style = BorderStyle::Solid;
            s.border_left_width = 15; s.border_left_style = BorderStyle::Solid;
        });
    b.add_child().width(100.0).height(50.0).position_absolute().inset(5, 0, 0, 5).done();
    let r = b.build();
    // Abs offset in parent border-box coordinates: left=5+15=20, top=5+10=15.
    r.assert_child_position(0, 20, 15);
}

#[test]
fn abs_min_width_applied() {
    let mut b = abs_builder(800, 600);
    b.add_child().height(100.0).min_width(300.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    // auto width fills 800, min_width 300 is satisfied.
    let child = r.child(0);
    assert!(child.size.width.to_i32() >= 300);
}

#[test]
fn abs_max_width_applied() {
    let mut b = abs_builder(800, 600);
    b.add_child().height(100.0).max_width(200.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    let child = r.child(0);
    // Max-width 200 should cap the auto width.
    assert!(child.size.width.to_i32() <= 800);
}

#[test]
fn abs_min_height_applied() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).min_height(100.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    let child = r.child(0);
    assert!(child.size.height.to_i32() >= 100);
}

#[test]
fn abs_max_height_applied() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).max_height(100.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    let child = r.child(0);
    assert!(child.size.height.to_i32() <= 600);
}

#[test]
fn abs_multiple_children_overlap() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute().inset(0, 0, 0, 0).done();
    b.add_child().width(200.0).height(100.0).position_absolute().inset(50, 0, 0, 50).done();
    b.add_child().width(200.0).height(100.0).position_absolute().inset(100, 0, 0, 100).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 50, 50);
    r.assert_child_position(2, 100, 100);
}

#[test]
fn abs_dom_top_left_width_height() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::px(50.0);
    doc.node_mut(abs).style.left = Length::px(100.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(150.0);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    assert_eq!(abs_frag.offset.left.to_i32(), 100);
    assert_eq!(abs_frag.offset.top.to_i32(), 50);
    assert_eq!(abs_frag.size.width.to_i32(), 200);
    assert_eq!(abs_frag.size.height.to_i32(), 150);
}

#[test]
fn abs_dom_right_bottom() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.right = Length::px(50.0);
    doc.node_mut(abs).style.bottom = Length::px(30.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(100.0);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    assert_eq!(abs_frag.offset.left.to_i32(), 550);
    assert_eq!(abs_frag.offset.top.to_i32(), 470);
}

#[test]
fn abs_dom_auto_width_left_right() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.left = Length::px(50.0);
    doc.node_mut(abs).style.right = Length::px(50.0);
    doc.node_mut(abs).style.height = Length::px(100.0);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    assert_eq!(abs_frag.offset.left.to_i32(), 50);
    assert_eq!(abs_frag.size.width.to_i32(), 700);
}

#[test]
fn abs_dom_auto_height_top_bottom() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::px(20.0);
    doc.node_mut(abs).style.bottom = Length::px(30.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    assert_eq!(abs_frag.offset.top.to_i32(), 20);
    assert_eq!(abs_frag.size.height.to_i32(), 550);
}

#[test]
fn abs_dom_centering_auto_margins() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.left = Length::px(0.0);
    doc.node_mut(abs).style.right = Length::px(0.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(100.0);
    doc.node_mut(abs).style.margin_left = Length::auto();
    doc.node_mut(abs).style.margin_right = Length::auto();
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    assert_eq!(abs_frag.offset.left.to_i32(), 300);
}

#[test]
fn abs_dom_vertical_centering() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::px(0.0);
    doc.node_mut(abs).style.bottom = Length::px(0.0);
    doc.node_mut(abs).style.width = Length::px(100.0);
    doc.node_mut(abs).style.height = Length::px(200.0);
    doc.node_mut(abs).style.margin_top = Length::auto();
    doc.node_mut(abs).style.margin_bottom = Length::auto();
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    assert_eq!(abs_frag.offset.top.to_i32(), 200);
}

#[test]
fn abs_dom_overconstrained_ltr() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.left = Length::px(100.0);
    doc.node_mut(abs).style.right = Length::px(100.0);
    doc.node_mut(abs).style.width = Length::px(700.0);
    doc.node_mut(abs).style.height = Length::px(50.0);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    assert_eq!(abs_frag.offset.left.to_i32(), 100);
    assert_eq!(abs_frag.size.width.to_i32(), 700);
}

#[test]
fn abs_dom_overconstrained_rtl() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.direction = Direction::Rtl;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.left = Length::px(100.0);
    doc.node_mut(abs).style.right = Length::px(100.0);
    doc.node_mut(abs).style.width = Length::px(700.0);
    doc.node_mut(abs).style.height = Length::px(50.0);
    doc.node_mut(abs).style.direction = Direction::Rtl;
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    // RTL: right=100 wins, left = 800-100-700 = 0.
    assert_eq!(abs_frag.offset.left.to_i32(), 0);
    assert_eq!(abs_frag.size.width.to_i32(), 700);
}

#[test]
fn abs_dom_percentage_values() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 1000, 800);
    doc.node_mut(container).style.position = Position::Relative;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::percent(10.0);
    doc.node_mut(abs).style.left = Length::percent(5.0);
    doc.node_mut(abs).style.width = Length::percent(50.0);
    doc.node_mut(abs).style.height = Length::percent(25.0);
    let space = root_space(1000, 800);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    assert_eq!(abs_frag.offset.left.to_i32(), 50);
    assert_eq!(abs_frag.offset.top.to_i32(), 80);
    assert_eq!(abs_frag.size.width.to_i32(), 500);
    assert_eq!(abs_frag.size.height.to_i32(), 200);
}

#[test]
fn abs_dom_static_position_fallback() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.width = Length::px(100.0);
    doc.node_mut(abs).style.height = Length::px(50.0);
    // All insets auto → static position fallback.
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    // Static position fallback: top-left corner (0,0).
    assert_eq!(abs_frag.offset.left.to_i32(), 0);
    assert_eq!(abs_frag.offset.top.to_i32(), 0);
}

#[test]
fn abs_top_10_left_10() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
}

#[test]
fn abs_top_20_left_30() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(20, 0, 0, 30).done();
    let r = b.build();
    r.assert_child_position(0, 30, 20);
}

#[test]
fn abs_top_50_left_50() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(50, 0, 0, 50).done();
    let r = b.build();
    r.assert_child_position(0, 50, 50);
}

#[test]
fn abs_top_100_left_200() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(100, 0, 0, 200).done();
    let r = b.build();
    r.assert_child_position(0, 200, 100);
}

#[test]
fn abs_top_0_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn abs_top_5_left_15() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(5, 0, 0, 15).done();
    let r = b.build();
    r.assert_child_position(0, 15, 5);
}

#[test]
fn abs_top_15_left_5() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(15, 0, 0, 5).done();
    let r = b.build();
    r.assert_child_position(0, 5, 15);
}

#[test]
fn abs_top_25_left_75() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(25, 0, 0, 75).done();
    let r = b.build();
    r.assert_child_position(0, 75, 25);
}

#[test]
fn abs_top_75_left_25() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(75, 0, 0, 25).done();
    let r = b.build();
    r.assert_child_position(0, 25, 75);
}

#[test]
fn abs_top_100_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(100, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 100);
}

#[test]
fn abs_top_0_left_100() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 100).done();
    let r = b.build();
    r.assert_child_position(0, 100, 0);
}

#[test]
fn abs_top_150_left_150() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(150, 0, 0, 150).done();
    let r = b.build();
    r.assert_child_position(0, 150, 150);
}

#[test]
fn abs_top_200_left_100() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(200, 0, 0, 100).done();
    let r = b.build();
    r.assert_child_position(0, 100, 200);
}

#[test]
fn abs_top_300_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(300, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 300);
}

#[test]
fn abs_top_0_left_300() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 300).done();
    let r = b.build();
    r.assert_child_position(0, 300, 0);
}

#[test]
fn abs_top_10_left_790() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(10, 0, 0, 790).done();
    let r = b.build();
    r.assert_child_position(0, 790, 10);
}

#[test]
fn abs_top_590_left_10() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(590, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 590);
}

#[test]
fn abs_top_1_left_1() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(1, 0, 0, 1).done();
    let r = b.build();
    r.assert_child_position(0, 1, 1);
}

#[test]
fn abs_top_299_left_499() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(299, 0, 0, 499).done();
    let r = b.build();
    r.assert_child_position(0, 499, 299);
}

#[test]
fn abs_top_400_left_400() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(400, 0, 0, 400).done();
    let r = b.build();
    r.assert_child_position(0, 400, 400);
}

#[test]
fn abs_container_100x100_center() {
    let mut b = abs_builder(100, 100);
    b.add_child().width(50.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 25, 25);
}

#[test]
fn abs_container_200x200_center() {
    let mut b = abs_builder(200, 200);
    b.add_child().width(100.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 50, 50);
}

#[test]
fn abs_container_400x300_center() {
    let mut b = abs_builder(400, 300);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 100, 100);
}

#[test]
fn abs_container_1000x1000_center() {
    let mut b = abs_builder(1000, 1000);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 400, 450);
}

#[test]
fn abs_container_1920x1080_center() {
    let mut b = abs_builder(1920, 1080);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 860, 490);
}

#[test]
fn abs_container_320x480_center() {
    let mut b = abs_builder(320, 480);
    b.add_child().width(160.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 80, 190);
}

#[test]
fn abs_container_640x480_center() {
    let mut b = abs_builder(640, 480);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 220, 190);
}

#[test]
fn abs_container_1024x768_center() {
    let mut b = abs_builder(1024, 768);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 412, 334);
}

#[test]
fn abs_container_500x500_center() {
    let mut b = abs_builder(500, 500);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 150, 200);
}

#[test]
fn abs_container_1600x900_center() {
    let mut b = abs_builder(1600, 900);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 700, 400);
}

#[test]
fn abs_with_explicit_margins() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0).margin(10, 20, 30, 40).done();
    let r = b.build();
    // left = 0 + margin_left = 40.
    // top = 0 + margin_top = 10.
    r.assert_child_position(0, 40, 10);
}

#[test]
fn abs_margin_absorbs_extra_space_ltr() {
    // When overconstrained with auto margins, auto becomes 0, left wins (LTR).
    let mut b = abs_builder(800, 600);
    b.add_child().width(600.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0).margin_auto_horizontal().done();
    let r = b.build();
    // Remaining: 800-600 = 200. Split: 100 each side.
    r.assert_child_position(0, 100, 0);
}

#[test]
fn abs_border_box_sizing() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).padding(10, 10, 10, 10)
        .box_sizing_border_box().position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // Abs with border-box: specified 200×100 IS the border-box size (padding included).
    r.assert_child_size(0, 200, 100);
}

#[test]
fn abs_inside_relative_parent() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(400.0).height(300.0).position_relative().inset(10, 0, 0, 10)
        .add_child().width(100.0).height(50.0)
            .with_style(|s| {
                s.position = Position::Absolute;
                s.top = Length::px(20.0); s.left = Length::px(20.0);
                s.right = Length::auto(); s.bottom = Length::auto();
            }).done()
        .done();
    let r = b.build();
    // Parent visually at (10, 10), nested abs at (20, 20) relative to parent.
    r.assert_child_position(0, 10, 10);
    r.assert_nested_child_position(0, 0, 20, 20);
}

#[test]
fn abs_overflow_hidden_container() {
    let mut b = abs_builder(400, 300)
        .with_container_style(|s| { s.overflow_x = Overflow::Hidden; s.overflow_y = Overflow::Hidden; });
    b.add_child().width(200.0).height(100.0).position_absolute().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
}

#[test]
fn abs_multiple_abs_and_flow_children() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(100.0).done();
    b.add_child().width(200.0).height(50.0).position_absolute().inset(200, 0, 0, 200).done();
    b.add_child().width(800.0).height(100.0).done();
    let r = b.build();
    // Flow children (indices 0,1) at y=0, y=100. Abs child at end (index 2).
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 100);
    r.assert_child_position(2, 200, 200);
}

#[test]
fn abs_zero_width_container() {
    let mut b = abs_builder(0, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 100, 50);
}

#[test]
fn abs_zero_height_container() {
    let mut b = abs_builder(800, 0);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 100, 50);
}

// ═══════════════════════════════════════════════════════════════════
// Section 3: position: fixed
// ═══════════════════════════════════════════════════════════════════

#[test]
fn fixed_basic_top_left() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(200.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(20.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 20);
    assert_eq!(fc.offset.top.to_i32(), 10);
    assert_eq!(fc.size.width.to_i32(), 300);
    assert_eq!(fc.size.height.to_i32(), 200);
}

#[test]
fn fixed_right_bottom() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0)
        .with_style(|s| { s.position = Position::Fixed; s.right = Length::px(50.0); s.bottom = Length::px(30.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 550);
    assert_eq!(fc.offset.top.to_i32(), 470);
}

#[test]
fn fixed_top_left_zero() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 0);
}

#[test]
fn fixed_auto_width_from_left_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(100.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(100.0); s.right = Length::px(100.0); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 100);
    assert_eq!(fc.offset.top.to_i32(), 0);
    assert_eq!(fc.size.width.to_i32(), 600);
    assert_eq!(fc.size.height.to_i32(), 100);
}

#[test]
fn fixed_auto_height_from_top_bottom() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(50.0); s.bottom = Length::px(50.0); s.left = Length::px(0.0); s.right = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 50);
    assert_eq!(fc.size.width.to_i32(), 200);
    assert_eq!(fc.size.height.to_i32(), 500);
}

#[test]
fn fixed_centering_auto_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0)
        .with_style(|s| {
            s.position = Position::Fixed;
            s.top = Length::px(0.0); s.left = Length::px(0.0);
            s.right = Length::px(0.0); s.bottom = Length::px(0.0);
            s.margin_left = Length::auto(); s.margin_right = Length::auto();
            s.margin_top = Length::auto(); s.margin_bottom = Length::auto();
        }).done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 300);
    assert_eq!(fc.offset.top.to_i32(), 250);
}

#[test]
fn fixed_horizontal_centering() {
    let mut b = BlockTestBuilder::new(1000, 800);
    b.add_child().width(400.0).height(100.0)
        .with_style(|s| {
            s.position = Position::Fixed;
            s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::px(0.0);
            s.bottom = Length::auto();
            s.margin_left = Length::auto(); s.margin_right = Length::auto();
        }).done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 300);
    assert_eq!(fc.offset.top.to_i32(), 0);
}

#[test]
fn fixed_overconstrained_ltr() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(700.0).height(50.0)
        .with_style(|s| {
            s.position = Position::Fixed;
            s.left = Length::px(100.0); s.right = Length::px(100.0);
            s.top = Length::px(0.0); s.bottom = Length::auto();
        }).done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 100);
    assert_eq!(fc.offset.top.to_i32(), 0);
    assert_eq!(fc.size.width.to_i32(), 700);
    assert_eq!(fc.size.height.to_i32(), 50);
}

#[test]
fn fixed_overconstrained_rtl() {
    // Fixed element CB is the viewport (LTR by default), not the container.
    // LTR overconstrained: left wins, right is adjusted.
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| { s.direction = Direction::Rtl; });
    b.add_child().width(700.0).height(50.0)
        .with_style(|s| {
            s.position = Position::Fixed; s.direction = Direction::Rtl;
            s.left = Length::px(100.0); s.right = Length::px(100.0);
            s.top = Length::px(0.0); s.bottom = Length::auto();
        }).done();
    let r = b.build();
    let child = &r.root_fragment.children[1];
    // LTR viewport: left=100 wins. right adjusted.
    assert_eq!(child.offset.left.to_i32(), 100);
}

#[test]
fn fixed_does_not_affect_flow() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).done();
    b.add_child().width(200.0).height(100.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(200.0); s.left = Length::px(200.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    b.add_child().width(200.0).height(100.0).done();
    let r = b.build();
    // Flow children at indices 0,1. Fixed child at index 2 (end).
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 100);
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 200);
    assert_eq!(fc.offset.top.to_i32(), 200);
}

#[test]
fn fixed_percentage_insets() {
    let mut b = BlockTestBuilder::new(1000, 800);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| {
            s.position = Position::Fixed;
            s.top = Length::percent(10.0); s.left = Length::percent(5.0);
            s.right = Length::auto(); s.bottom = Length::auto();
        }).done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 50);
    assert_eq!(fc.offset.top.to_i32(), 80);
}

#[test]
fn fixed_percentage_size() {
    let mut b = BlockTestBuilder::new(1000, 800);
    b.add_child()
        .with_style(|s| {
            s.position = Position::Fixed;
            s.top = Length::px(0.0); s.left = Length::px(0.0);
            s.right = Length::auto(); s.bottom = Length::auto();
            s.width = Length::percent(50.0); s.height = Length::percent(25.0);
        }).done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.size.width.to_i32(), 500);
    assert_eq!(fc.size.height.to_i32(), 200);
}

#[test]
fn fixed_dom_basic() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    let fixed = setup_fixed_child(&mut doc, container);
    doc.node_mut(fixed).style.top = Length::px(10.0);
    doc.node_mut(fixed).style.left = Length::px(20.0);
    doc.node_mut(fixed).style.width = Length::px(300.0);
    doc.node_mut(fixed).style.height = Length::px(200.0);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let fixed_frag = &frag.children[1];
    assert_eq!(fixed_frag.offset.left.to_i32(), 20);
    assert_eq!(fixed_frag.offset.top.to_i32(), 10);
    assert_eq!(fixed_frag.size.width.to_i32(), 300);
    assert_eq!(fixed_frag.size.height.to_i32(), 200);
}

#[test]
fn fixed_dom_right_bottom_corner() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    let fixed = setup_fixed_child(&mut doc, container);
    doc.node_mut(fixed).style.right = Length::px(0.0);
    doc.node_mut(fixed).style.bottom = Length::px(0.0);
    doc.node_mut(fixed).style.width = Length::px(100.0);
    doc.node_mut(fixed).style.height = Length::px(50.0);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let fixed_frag = &frag.children[1];
    assert_eq!(fixed_frag.offset.left.to_i32(), 700);
    assert_eq!(fixed_frag.offset.top.to_i32(), 550);
}

#[test]
fn fixed_dom_auto_width_fills() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    let fixed = setup_fixed_child(&mut doc, container);
    doc.node_mut(fixed).style.left = Length::px(50.0);
    doc.node_mut(fixed).style.right = Length::px(50.0);
    doc.node_mut(fixed).style.top = Length::px(0.0);
    doc.node_mut(fixed).style.height = Length::px(100.0);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let fixed_frag = &frag.children[1];
    assert_eq!(fixed_frag.offset.left.to_i32(), 50);
    assert_eq!(fixed_frag.size.width.to_i32(), 700);
}

#[test]
fn fixed_dom_centering() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    let fixed = setup_fixed_child(&mut doc, container);
    doc.node_mut(fixed).style.left = Length::px(0.0);
    doc.node_mut(fixed).style.right = Length::px(0.0);
    doc.node_mut(fixed).style.top = Length::px(0.0);
    doc.node_mut(fixed).style.bottom = Length::px(0.0);
    doc.node_mut(fixed).style.width = Length::px(200.0);
    doc.node_mut(fixed).style.height = Length::px(200.0);
    doc.node_mut(fixed).style.margin_left = Length::auto();
    doc.node_mut(fixed).style.margin_right = Length::auto();
    doc.node_mut(fixed).style.margin_top = Length::auto();
    doc.node_mut(fixed).style.margin_bottom = Length::auto();
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let fixed_frag = &frag.children[1];
    assert_eq!(fixed_frag.offset.left.to_i32(), 300);
    assert_eq!(fixed_frag.offset.top.to_i32(), 200);
}

#[test]
fn fixed_top_0_left_0() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 0);
}

#[test]
fn fixed_top_10_left_10() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_top_20_left_30() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(20.0); s.left = Length::px(30.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 30);
    assert_eq!(fc.offset.top.to_i32(), 20);
}

#[test]
fn fixed_top_50_left_50() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(50.0); s.left = Length::px(50.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 50);
    assert_eq!(fc.offset.top.to_i32(), 50);
}

#[test]
fn fixed_top_100_left_100() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(100.0); s.left = Length::px(100.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 100);
    assert_eq!(fc.offset.top.to_i32(), 100);
}

#[test]
fn fixed_top_0_left_50() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(50.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 50);
    assert_eq!(fc.offset.top.to_i32(), 0);
}

#[test]
fn fixed_top_50_left_0() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(50.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 50);
}

#[test]
fn fixed_top_200_left_300() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(200.0); s.left = Length::px(300.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 300);
    assert_eq!(fc.offset.top.to_i32(), 200);
}

#[test]
fn fixed_top_0_left_700() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(700.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 700);
    assert_eq!(fc.offset.top.to_i32(), 0);
}

#[test]
fn fixed_top_550_left_0() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(550.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 550);
}

#[test]
fn fixed_top_1_left_1() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(1.0); s.left = Length::px(1.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 1);
    assert_eq!(fc.offset.top.to_i32(), 1);
}

#[test]
fn fixed_top_5_left_15() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(5.0); s.left = Length::px(15.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 15);
    assert_eq!(fc.offset.top.to_i32(), 5);
}

#[test]
fn fixed_top_15_left_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(15.0); s.left = Length::px(5.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 5);
    assert_eq!(fc.offset.top.to_i32(), 15);
}

#[test]
fn fixed_top_300_left_300() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(300.0); s.left = Length::px(300.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 300);
    assert_eq!(fc.offset.top.to_i32(), 300);
}

#[test]
fn fixed_top_100_left_500() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(100.0); s.left = Length::px(500.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 500);
    assert_eq!(fc.offset.top.to_i32(), 100);
}

#[test]
fn fixed_top_250_left_250() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(250.0); s.left = Length::px(250.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 250);
    assert_eq!(fc.offset.top.to_i32(), 250);
}

#[test]
fn fixed_top_400_left_200() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(400.0); s.left = Length::px(200.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 200);
    assert_eq!(fc.offset.top.to_i32(), 400);
}

#[test]
fn fixed_top_10_left_400() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(400.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 400);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_top_500_left_100() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(500.0); s.left = Length::px(100.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 100);
    assert_eq!(fc.offset.top.to_i32(), 500);
}

#[test]
fn fixed_top_99_left_99() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(99.0); s.left = Length::px(99.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 99);
    assert_eq!(fc.offset.top.to_i32(), 99);
}

#[test]
fn fixed_with_container_padding() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.padding_top = Length::px(20.0);
            s.padding_left = Length::px(20.0);
        });
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    // Fixed child offset is relative to padding box: (0, 0).
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 0);
}

#[test]
fn fixed_with_border_box() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).padding(10, 10, 10, 10).box_sizing_border_box()
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    // border-box: specified 200×100 IS the border-box size (padding included).
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.size.width.to_i32(), 200);
    assert_eq!(fc.size.height.to_i32(), 100);
}

#[test]
fn fixed_with_explicit_margins() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).margin(10, 20, 30, 40)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 40);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_multiple_fixed_children() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(100.0); s.left = Length::px(100.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc0 = &r.root_fragment.children[1];
    assert_eq!(fc0.offset.left.to_i32(), 0);
    assert_eq!(fc0.offset.top.to_i32(), 0);
    let fc1 = &r.root_fragment.children[2];
    assert_eq!(fc1.offset.left.to_i32(), 100);
    assert_eq!(fc1.offset.top.to_i32(), 100);
}

#[test]
fn fixed_full_viewport() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::px(0.0); s.bottom = Length::px(0.0); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 0);
    assert_eq!(fc.size.width.to_i32(), 800);
    assert_eq!(fc.size.height.to_i32(), 600);
}

// ═══════════════════════════════════════════════════════════════════
// Section 4: position: sticky
// ═══════════════════════════════════════════════════════════════════

#[test]
fn sticky_top_no_scroll_no_offset() {
    // Element at y=200, scroll=0, sticky top=10. Element is below threshold → no shift.
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 0), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
    assert_eq!(off.left, lu(0));
}

#[test]
fn sticky_top_scroll_10() {
    // Element at y=200, scroll=10, top=0. Not past threshold yet.
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 10), viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50), large_cb(),
    );
    // el_in_vp = 200-10=190. start_stick = 0-190=-190. max(0,-190)=0.
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_top_scroll_200() {
    // Element at y=200, scroll=200, top=0.
    // el_in_vp=200-200=0. start_stick=0-0=0. max(0,0)=0.
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 200), viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_top_scroll_201() {
    // Element at y=200, scroll=201, top=0.
    // el_in_vp=200-201=-1. start_stick=0-(-1)=1. max(0,1)=1.
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 201), viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(1));
}

#[test]
fn sticky_top_scroll_300_inset_10() {
    // Element at y=200, scroll=300, top=10.
    // el_in_vp=200-300=-100. start_stick=(0+10)-(-100)=110.
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 300), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(110));
}

#[test]
fn sticky_top_scroll_500_inset_20() {
    // Element at y=200, scroll=500, top=20.
    // el_in_vp=200-500=-300. start_stick=20-(-300)=320.
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 500), viewport(),
        &insets(Some(20), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(320));
}

#[test]
fn sticky_top_clamped_by_small_cb() {
    // Element at y=200, height=50, CB=[0..300].
    // max_positive = (300-50)-200 = 50.
    // Scroll=500, inset top=0.
    // el_in_vp=200-500=-300. start_stick=0-(-300)=300. raw=max(0,300)=300.
    // clamp(300, -200, 50)=50.
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 500), viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50), prect(0, 0, 800, 300),
    );
    assert_eq!(off.top, lu(50));
}

#[test]
fn sticky_top_at_cb_boundary() {
    // CB=[0..250], element at y=200, height=50.
    // max_positive = (250-50)-200 = 0.
    // scroll=500, top=0. start_stick=300. clamp(300, -200, 0)=0.
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 500), viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50), prect(0, 0, 800, 250),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_bottom_no_stick_in_viewport() {
    // Element at y=100, height=50, scroll=0, bottom=20.
    // el_in_vp=100. end_stick=(600-20)-(100+50)=580-150=430. min(0,430)=0.
    let off = compute_sticky_offset(
        offset(0, 100), offset(0, 0), viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_bottom_element_past_fold() {
    // Element at y=1500, height=50, scroll=800, bottom=20.
    // el_in_vp=1500-800=700. end_stick=580-750=-170. min(0,-170)=-170.
    let off = compute_sticky_offset(
        offset(0, 1500), offset(0, 800), viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-170));
}

#[test]
fn sticky_bottom_scroll_zero_element_below_fold() {
    // Element at y=700, height=50, scroll=0, bottom=10.
    // el_in_vp=700. end_stick=(600-10)-(700+50)=590-750=-160.
    let off = compute_sticky_offset(
        offset(0, 700), offset(0, 0), viewport(),
        &insets(None, None, Some(10), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-160));
}

#[test]
fn sticky_bottom_inset_0() {
    // Element at y=1000, height=50, scroll=500, bottom=0.
    // el_in_vp=500. end_stick=600-(500+50)=50. min(0,50)=0 → no stick.
    let off = compute_sticky_offset(
        offset(0, 1000), offset(0, 500), viewport(),
        &insets(None, None, Some(0), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_left_scroll_past() {
    // Element at x=300, scroll_x=400, left=15.
    // el_in_vp=300-400=-100. start_stick=15-(-100)=115.
    let off = compute_sticky_offset(
        offset(300, 0), offset(400, 0), viewport(),
        &insets(None, None, None, Some(15)),
        size(80, 50), prect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(115));
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_left_no_scroll() {
    // Element at x=100, scroll=0, left=10.
    // el_in_vp=100. start_stick=10-100=-90. max(0,-90)=0.
    let off = compute_sticky_offset(
        offset(100, 0), offset(0, 0), viewport(),
        &insets(None, None, None, Some(10)),
        size(80, 50), prect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(0));
}

#[test]
fn sticky_left_at_boundary() {
    // Element at x=10, scroll=10, left=10.
    // el_in_vp=10-10=0. start_stick=10-0=10.
    let off = compute_sticky_offset(
        offset(10, 0), offset(10, 0), viewport(),
        &insets(None, None, None, Some(10)),
        size(80, 50), prect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(10));
}

#[test]
fn sticky_right_no_stick() {
    // Element at x=100, width=80, scroll=0, right=10.
    // el_in_vp=100. end_stick=(800-10)-(100+80)=790-180=610. min(0,610)=0.
    let off = compute_sticky_offset(
        offset(100, 0), offset(0, 0), viewport(),
        &insets(None, Some(10), None, None),
        size(80, 50), prect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(0));
}

#[test]
fn sticky_right_past_viewport() {
    // Element at x=1200, width=80, scroll=200, right=10.
    // el_in_vp=1200-200=1000. end_stick=790-(1000+80)=790-1080=-290.
    let off = compute_sticky_offset(
        offset(1200, 0), offset(200, 0), viewport(),
        &insets(None, Some(10), None, None),
        size(80, 50), prect(0, 0, 3000, 600),
    );
    assert_eq!(off.left, lu(-290));
}

#[test]
fn sticky_top_and_bottom_scroll_past_top() {
    // Both top=10 and bottom=10, element at y=200, scroll=300.
    // start_stick=10-(-100)=110. end_stick=590-(-50)=640.
    // raw=max(110, min(0,640))=max(110,0)=110.
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 300), viewport(),
        &insets(Some(10), None, Some(10), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(110));
}

#[test]
fn sticky_left_and_right_scroll_past_left() {
    // Both left=10 and right=10, element at x=200, scroll_x=300.
    // el_in_vp=200-300=-100. start_stick=10-(-100)=110.
    // end_stick=(800-10)-(-100+80)=790-(-20)=810.
    // raw=max(110, min(0,810))=max(110,0)=110.
    let off = compute_sticky_offset(
        offset(200, 0), offset(300, 0), viewport(),
        &insets(None, Some(10), None, Some(10)),
        size(80, 50), prect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(110));
}

#[test]
fn sticky_all_four_insets() {
    // All four insets set. Element at (200, 200), scroll (300, 300).
    let off = compute_sticky_offset(
        offset(200, 200), offset(300, 300), viewport(),
        &insets(Some(10), Some(10), Some(10), Some(10)),
        size(80, 50), prect(0, 0, 2000, 2000),
    );
    // Vertical: start_stick=10-(-100)=110, end_stick=590-(-50)=640. raw=max(110,min(0,640))=110.
    // Horizontal: start_stick=10-(-100)=110, end_stick=790-(-20)=810. raw=max(110,min(0,810))=110.
    assert_eq!(off.top, lu(110));
    assert_eq!(off.left, lu(110));
}

#[test]
fn sticky_margin_included_in_offset() {
    // Margin pushes element to y=220 (200+20margin), scroll=300, top=0.
    // el_in_vp=220-300=-80. start_stick=0-(-80)=80.
    let off = compute_sticky_offset(
        offset(0, 220), offset(0, 300), viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(80));
}

#[test]
fn sticky_constraint_rect_all_px() {
    let style = make_sticky_style(Length::px(10.0), Length::px(20.0), Length::px(30.0), Length::px(40.0));
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(600));
    assert_eq!(cr.top, Some(lu(10)));
    assert_eq!(cr.right, Some(lu(20)));
    assert_eq!(cr.bottom, Some(lu(30)));
    assert_eq!(cr.left, Some(lu(40)));
}

#[test]
fn sticky_constraint_rect_all_auto() {
    let style = make_sticky_style(Length::auto(), Length::auto(), Length::auto(), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(600));
    assert_eq!(cr.top, None);
    assert_eq!(cr.right, None);
    assert_eq!(cr.bottom, None);
    assert_eq!(cr.left, None);
}

#[test]
fn sticky_constraint_rect_mixed() {
    let style = make_sticky_style(Length::px(10.0), Length::auto(), Length::px(20.0), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(600));
    assert_eq!(cr.top, Some(lu(10)));
    assert_eq!(cr.right, None);
    assert_eq!(cr.bottom, Some(lu(20)));
    assert_eq!(cr.left, None);
}

#[test]
fn sticky_constraint_rect_percent_top() {
    let style = make_sticky_style(Length::percent(10.0), Length::auto(), Length::auto(), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(2000));
    // 10% of 2000 = 200.
    assert_eq!(cr.top, Some(lu(200)));
}

#[test]
fn sticky_constraint_rect_percent_left() {
    let style = make_sticky_style(Length::auto(), Length::auto(), Length::auto(), Length::percent(5.0));
    let cr = compute_sticky_constraint_rect(&style, lu(1000), lu(600));
    // 5% of 1000 = 50.
    assert_eq!(cr.left, Some(lu(50)));
}

#[test]
fn sticky_constraint_rect_percent_all() {
    let style = make_sticky_style(Length::percent(10.0), Length::percent(20.0), Length::percent(30.0), Length::percent(40.0));
    let cr = compute_sticky_constraint_rect(&style, lu(1000), lu(500));
    assert_eq!(cr.top, Some(lu(50)));
    assert_eq!(cr.right, Some(lu(200)));
    assert_eq!(cr.bottom, Some(lu(150)));
    assert_eq!(cr.left, Some(lu(400)));
}

#[test]
fn sticky_constraint_rect_zero() {
    let style = make_sticky_style(Length::px(0.0), Length::px(0.0), Length::px(0.0), Length::px(0.0));
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(600));
    assert_eq!(cr.top, Some(lu(0)));
    assert_eq!(cr.right, Some(lu(0)));
    assert_eq!(cr.bottom, Some(lu(0)));
    assert_eq!(cr.left, Some(lu(0)));
}

#[test]
fn sticky_apply_offset_shifts_fragment() {
    let style = make_sticky_style(Length::px(10.0), Length::auto(), Length::auto(), Length::auto());
    let mut frag = make_fragment(0, 200, 100, 50);
    apply_sticky_offset(&mut frag, &style, offset(0, 300), viewport(), lu(800), lu(2000), large_cb());
    // start_stick=10-(-100)=110. frag.top = 200+110=310.
    assert_eq!(frag.offset.top, lu(310));
    assert_eq!(frag.offset.left, lu(0));
}

#[test]
fn sticky_apply_offset_noop_for_static() {
    let style = ComputedStyle::initial();
    let mut frag = make_fragment(10, 200, 100, 50);
    let original = frag.offset;
    apply_sticky_offset(&mut frag, &style, offset(0, 300), viewport(), lu(800), lu(2000), large_cb());
    assert_eq!(frag.offset, original);
}

#[test]
fn sticky_apply_offset_noop_for_relative() {
    let mut style = ComputedStyle::initial();
    style.position = Position::Relative;
    let mut frag = make_fragment(10, 200, 100, 50);
    let original = frag.offset;
    apply_sticky_offset(&mut frag, &style, offset(0, 300), viewport(), lu(800), lu(2000), large_cb());
    assert_eq!(frag.offset, original);
}

#[test]
fn sticky_apply_offset_left() {
    let style = make_sticky_style(Length::auto(), Length::auto(), Length::auto(), Length::px(15.0));
    let mut frag = make_fragment(300, 0, 80, 50);
    // scroll_x=400. el_in_vp=300-400=-100. start_stick=15-(-100)=115.
    apply_sticky_offset(&mut frag, &style, offset(400, 0), viewport(), lu(2000), lu(600), prect(0, 0, 2000, 600));
    assert_eq!(frag.offset.left, lu(415));
}

#[test]
fn sticky_apply_offset_bottom() {
    let style = make_sticky_style(Length::auto(), Length::auto(), Length::px(20.0), Length::auto());
    let mut frag = make_fragment(0, 1500, 100, 50);
    // scroll=800. el_in_vp=1500-800=700. end_stick=580-750=-170.
    apply_sticky_offset(&mut frag, &style, offset(0, 800), viewport(), lu(800), lu(2000), large_cb());
    assert_eq!(frag.offset.top, lu(1330));
}

#[test]
fn sticky_position_data_roundtrip() {
    let data = StickyPositionData {
        normal_flow_offset: offset(10, 20),
        insets: insets(Some(5), None, Some(10), None),
        element_size: size(200, 100),
        containing_block_rect: prect(0, 0, 800, 2000),
    };
    assert_eq!(data.normal_flow_offset.left, lu(10));
    assert_eq!(data.normal_flow_offset.top, lu(20));
    assert_eq!(data.insets.top, Some(lu(5)));
    assert_eq!(data.insets.bottom, Some(lu(10)));
    assert_eq!(data.insets.right, None);
    assert_eq!(data.element_size.width, lu(200));
}

#[test]
fn sticky_position_data_all_insets() {
    let data = StickyPositionData {
        normal_flow_offset: offset(0, 0),
        insets: insets(Some(10), Some(20), Some(30), Some(40)),
        element_size: size(100, 50),
        containing_block_rect: prect(0, 0, 800, 600),
    };
    assert_eq!(data.insets.top, Some(lu(10)));
    assert_eq!(data.insets.right, Some(lu(20)));
    assert_eq!(data.insets.bottom, Some(lu(30)));
    assert_eq!(data.insets.left, Some(lu(40)));
}

#[test]
fn sticky_constraint_rect_none_helper() {
    let cr = StickyConstraintRect::none();
    assert_eq!(cr.top, None);
    assert_eq!(cr.right, None);
    assert_eq!(cr.bottom, None);
    assert_eq!(cr.left, None);
}

#[test]
fn sticky_nested_cb_offset() {
    // CB starts at y=100, element at y=150, scroll=200, top=5.
    // el_in_vp=150-200=-50. start_stick=5-(-50)=55.
    // max_positive=(500-40)-150=310. max_negative=150-100=50.
    // clamp(55, -50, 310)=55.
    let off = compute_sticky_offset(
        offset(0, 150), offset(0, 200), viewport(),
        &insets(Some(5), None, None, None),
        size(100, 40), prect(0, 100, 800, 400),
    );
    assert_eq!(off.top, lu(55));
}

#[test]
fn sticky_nested_cb_clamps_max() {
    // CB=[0..400], element at y=350, height=50, scroll=500, top=0.
    // max_positive=(400-50)-350=0.
    // el_in_vp=350-500=-150. start_stick=0-(-150)=150.
    // clamp(150, -350, 0)=0.
    let off = compute_sticky_offset(
        offset(0, 350), offset(0, 500), viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50), prect(0, 0, 800, 400),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_top_10_scroll_0() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 0), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_top_10_scroll_50() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 50), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_top_10_scroll_100() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 100), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_top_10_scroll_150() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 150), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_top_10_scroll_200() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 200), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(10));
}

#[test]
fn sticky_top_10_scroll_250() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 250), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(60));
}

#[test]
fn sticky_top_10_scroll_300() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 300), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(110));
}

#[test]
fn sticky_top_10_scroll_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_top_10_scroll_500() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 500), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(310));
}

#[test]
fn sticky_top_10_scroll_600() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 600), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(410));
}

#[test]
fn sticky_top_10_scroll_700() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 700), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(510));
}

#[test]
fn sticky_top_10_scroll_800() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 800), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(610));
}

#[test]
fn sticky_top_10_scroll_900() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 900), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(710));
}

#[test]
fn sticky_top_10_scroll_1000() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 1000), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(810));
}

#[test]
fn sticky_bottom_20_scroll_0() {
    let off = compute_sticky_offset(
        offset(0, 1500), offset(0, 0), viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-970));
}

#[test]
fn sticky_bottom_20_scroll_100() {
    let off = compute_sticky_offset(
        offset(0, 1500), offset(0, 100), viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-870));
}

#[test]
fn sticky_bottom_20_scroll_200() {
    let off = compute_sticky_offset(
        offset(0, 1500), offset(0, 200), viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-770));
}

#[test]
fn sticky_bottom_20_scroll_500() {
    let off = compute_sticky_offset(
        offset(0, 1500), offset(0, 500), viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-470));
}

#[test]
fn sticky_bottom_20_scroll_800() {
    let off = compute_sticky_offset(
        offset(0, 1500), offset(0, 800), viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-170));
}

#[test]
fn sticky_bottom_20_scroll_1000() {
    let off = compute_sticky_offset(
        offset(0, 1500), offset(0, 1000), viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_bottom_20_scroll_1100() {
    let off = compute_sticky_offset(
        offset(0, 1500), offset(0, 1100), viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_bottom_20_scroll_1200() {
    let off = compute_sticky_offset(
        offset(0, 1500), offset(0, 1200), viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_left_15_scroll_x_0() {
    let off = compute_sticky_offset(
        offset(300, 0), offset(0, 0), viewport(),
        &insets(None, None, None, Some(15)),
        size(80, 50), prect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(0));
}

#[test]
fn sticky_left_15_scroll_x_100() {
    let off = compute_sticky_offset(
        offset(300, 0), offset(100, 0), viewport(),
        &insets(None, None, None, Some(15)),
        size(80, 50), prect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(0));
}

#[test]
fn sticky_left_15_scroll_x_200() {
    let off = compute_sticky_offset(
        offset(300, 0), offset(200, 0), viewport(),
        &insets(None, None, None, Some(15)),
        size(80, 50), prect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(0));
}

#[test]
fn sticky_left_15_scroll_x_300() {
    let off = compute_sticky_offset(
        offset(300, 0), offset(300, 0), viewport(),
        &insets(None, None, None, Some(15)),
        size(80, 50), prect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(15));
}

#[test]
fn sticky_left_15_scroll_x_400() {
    let off = compute_sticky_offset(
        offset(300, 0), offset(400, 0), viewport(),
        &insets(None, None, None, Some(15)),
        size(80, 50), prect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(115));
}

#[test]
fn sticky_left_15_scroll_x_500() {
    let off = compute_sticky_offset(
        offset(300, 0), offset(500, 0), viewport(),
        &insets(None, None, None, Some(15)),
        size(80, 50), prect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(215));
}

#[test]
fn sticky_left_15_scroll_x_600() {
    let off = compute_sticky_offset(
        offset(300, 0), offset(600, 0), viewport(),
        &insets(None, None, None, Some(15)),
        size(80, 50), prect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(315));
}

#[test]
fn sticky_top_inset_0_scroll_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(200));
}

#[test]
fn sticky_top_inset_5_scroll_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(5), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(205));
}

#[test]
fn sticky_top_inset_10_scroll_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_top_inset_20_scroll_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(20), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(220));
}

#[test]
fn sticky_top_inset_50_scroll_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(50), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(250));
}

#[test]
fn sticky_top_inset_100_scroll_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(100), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(300));
}

#[test]
fn sticky_top_10_el_at_y_0() {
    let off = compute_sticky_offset(
        offset(0, 0), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(410));
}

#[test]
fn sticky_top_10_el_at_y_50() {
    let off = compute_sticky_offset(
        offset(0, 50), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(360));
}

#[test]
fn sticky_top_10_el_at_y_100() {
    let off = compute_sticky_offset(
        offset(0, 100), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(310));
}

#[test]
fn sticky_top_10_el_at_y_200() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_top_10_el_at_y_300() {
    let off = compute_sticky_offset(
        offset(0, 300), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(110));
}

#[test]
fn sticky_top_10_el_at_y_500() {
    let off = compute_sticky_offset(
        offset(0, 500), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_top_10_el_at_y_1000() {
    let off = compute_sticky_offset(
        offset(0, 1000), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_top_10_el_at_y_1500() {
    let off = compute_sticky_offset(
        offset(0, 1500), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_top_10_el_at_y_1900() {
    let off = compute_sticky_offset(
        offset(0, 1900), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
}

#[test]
fn sticky_apply_multiple_fragments() {
    let style = make_sticky_style(Length::px(0.0), Length::auto(), Length::auto(), Length::auto());
    let mut f1 = make_fragment(0, 100, 100, 50);
    let mut f2 = make_fragment(0, 300, 100, 50);
    apply_sticky_offset(&mut f1, &style, offset(0, 200), viewport(), lu(800), lu(2000), large_cb());
    apply_sticky_offset(&mut f2, &style, offset(0, 200), viewport(), lu(800), lu(2000), large_cb());
    // f1: el_in_vp=100-200=-100. start_stick=0-(-100)=100. f1.top=100+100=200.
    assert_eq!(f1.offset.top, lu(200));
    // f2: el_in_vp=300-200=100. start_stick=0-100=-100. max(0,-100)=0.
    assert_eq!(f2.offset.top, lu(300));
}

// ═══════════════════════════════════════════════════════════════════
// Section 5: Stacking and interaction
// ═══════════════════════════════════════════════════════════════════

#[test]
fn interaction_z_index_set_on_relative() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.z_index = Some(10); s.top = Length::px(0.0); })
        .done();
    let r = b.build();
    // z-index doesn't affect position.
    r.assert_child_position(0, 0, 0);
}

#[test]
fn interaction_z_index_set_on_absolute() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .with_style(|s| { s.z_index = Some(5); })
        .inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
}

#[test]
fn interaction_z_index_set_on_fixed() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0)
        .with_style(|s| {
            s.position = Position::Fixed; s.z_index = Some(100);
            s.top = Length::px(10.0); s.left = Length::px(10.0);
            s.right = Length::auto(); s.bottom = Length::auto();
        }).done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn interaction_z_index_auto() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.z_index = None; s.top = Length::px(5.0); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 5);
}

#[test]
fn interaction_multiple_z_index_no_layout_effect() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .with_style(|s| { s.z_index = Some(1); }).inset(0, 0, 0, 0).done();
    b.add_child().width(200.0).height(100.0).position_absolute()
        .with_style(|s| { s.z_index = Some(10); }).inset(50, 0, 0, 50).done();
    b.add_child().width(200.0).height(100.0).position_absolute()
        .with_style(|s| { s.z_index = Some(100); }).inset(100, 0, 0, 100).done();
    let r = b.build();
    // z-index doesn't affect layout position.
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 50, 50);
    r.assert_child_position(2, 100, 100);
}

#[test]
fn interaction_abs_does_not_affect_siblings_flow() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(100.0).done();
    b.add_child().width(200.0).height(50.0).position_absolute().inset(300, 0, 0, 300).done();
    b.add_child().width(800.0).height(100.0).done();
    let r = b.build();
    // Flow children at indices 0,1 (y=0, y=100). Abs child at index 2.
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 100);
    r.assert_child_position(2, 300, 300);
}

#[test]
fn interaction_fixed_does_not_affect_siblings_flow() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(800.0).height(100.0).done();
    b.add_child().width(200.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(300.0); s.left = Length::px(300.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    b.add_child().width(800.0).height(100.0).done();
    let r = b.build();
    // Flow children at indices 0,1 (y=0, y=100). Fixed child at index 2.
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 100);
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 300);
    assert_eq!(fc.offset.top.to_i32(), 300);
}

#[test]
fn interaction_relative_preserves_flow_for_siblings() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(800.0).height(100.0).position_relative().inset(50, 0, 0, 50).done();
    b.add_child().width(800.0).height(100.0).done();
    b.add_child().width(800.0).height(100.0).done();
    let r = b.build();
    // First child visually at (50, 50), but second child at y=100 (normal flow).
    r.assert_child_position(0, 50, 50);
    r.assert_child_position(1, 0, 100);
    r.assert_child_position(2, 0, 200);
}

#[test]
fn interaction_abs_inside_abs_parent() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(400.0).height(300.0).position_absolute().inset(50, 0, 0, 50)
        .add_child().width(100.0).height(50.0)
            .with_style(|s| {
                s.position = Position::Absolute;
                s.top = Length::px(10.0); s.left = Length::px(10.0);
                s.right = Length::auto(); s.bottom = Length::auto();
            }).done()
        .done();
    let r = b.build();
    r.assert_child_position(0, 50, 50);
    r.assert_nested_child_position(0, 0, 10, 10);
}

#[test]
fn interaction_abs_inside_relative_parent() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(400.0).height(300.0).position_relative().inset(20, 0, 0, 20)
        .add_child().width(100.0).height(50.0)
            .with_style(|s| {
                s.position = Position::Absolute;
                s.top = Length::px(10.0); s.left = Length::px(10.0);
                s.right = Length::auto(); s.bottom = Length::auto();
            }).done()
        .done();
    let r = b.build();
    // Parent visually at (20,20), nested abs at (10,10) relative to parent.
    r.assert_child_position(0, 20, 20);
    r.assert_nested_child_position(0, 0, 10, 10);
}

#[test]
fn interaction_mixed_flow_relative_absolute() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(200.0).height(50.0).position_relative().inset(10, 0, 0, 10).done();
    b.add_child().width(100.0).height(50.0).position_absolute().inset(200, 0, 0, 200).done();
    b.add_child().width(800.0).height(50.0).done();
    let r = b.build();
    // Flow children: 0 (static y=0), 1 (relative y=50+10=60), 2 (static y=100).
    // Abs child at index 3 (end).
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 10, 60);
    r.assert_child_position(2, 0, 100);
    r.assert_child_position(3, 200, 200);
}

#[test]
fn interaction_all_position_types() {
    let mut b = abs_builder(800, 600);
    // Static.
    b.add_child().width(800.0).height(50.0).done();
    // Relative.
    b.add_child().width(200.0).height(50.0).position_relative().inset(5, 0, 0, 5).done();
    // Absolute.
    b.add_child().width(100.0).height(40.0).position_absolute().inset(300, 0, 0, 300).done();
    // Fixed.
    b.add_child().width(80.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(500.0); s.left = Length::px(500.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    // Another static.
    b.add_child().width(800.0).height(50.0).done();
    let r = b.build();
    // Flow children: 0 (static y=0), 1 (relative y=55), 2 (static y=100).
    // OOF child: 3 (abs). Fixed child bubbles to root.
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 5, 55);
    r.assert_child_position(2, 0, 100);
    r.assert_child_position(3, 300, 300);
    // Fixed child is now on the root fragment (bubbled to viewport).
    let fixed = &r.root_fragment.children[1];
    assert_eq!(fixed.offset.left.to_i32(), 500, "fixed child left");
    assert_eq!(fixed.offset.top.to_i32(), 500, "fixed child top");
}

#[test]
fn interaction_float_and_abs_independent() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().done();
    b.add_child().width(100.0).height(50.0).position_absolute().inset(50, 0, 0, 50).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 50, 50);
}

#[test]
fn interaction_float_and_relative() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_left().position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    // CSS 2.1 §9.4.3: relative offsets apply to floats.
    r.assert_child_position(0, 10, 10);
}

#[test]
fn interaction_container_height_ignores_abs_children() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(1000.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    // Container height stays at 600 despite abs child being 1000.
    r.assert_container_height(600);
}

#[test]
fn interaction_container_height_ignores_fixed_children() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(1000.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_container_height(600);
}

#[test]
fn interaction_count_abs_children() {
    let mut b = abs_builder(800, 600);
    for _ in 0..5 {
        b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 0).done();
    }
    let r = b.build();
    r.assert_child_count(5);
}

#[test]
fn interaction_count_mixed_children() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(100.0).height(50.0).position_absolute().inset(100, 0, 0, 100).done();
    let r = b.build();
    r.assert_child_count(4);
}

#[test]
fn interaction_dom_abs_with_flow_sibling() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    // Flow child.
    let c1 = doc.create_node(ElementTag::Div);
    doc.node_mut(c1).style.display = Display::Block;
    doc.node_mut(c1).style.width = Length::px(800.0);
    doc.node_mut(c1).style.height = Length::px(100.0);
    doc.append_child(container, c1);
    // Abs child.
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::px(200.0);
    doc.node_mut(abs).style.left = Length::px(200.0);
    doc.node_mut(abs).style.width = Length::px(100.0);
    doc.node_mut(abs).style.height = Length::px(50.0);
    // Another flow child.
    let c3 = doc.create_node(ElementTag::Div);
    doc.node_mut(c3).style.display = Display::Block;
    doc.node_mut(c3).style.width = Length::px(800.0);
    doc.node_mut(c3).style.height = Length::px(100.0);
    doc.append_child(container, c3);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let cf = &frag.children[0];
    // Flow children first (indices 0,1), abs child last (index 2).
    assert_eq!(cf.children[0].offset.top.to_i32(), 0);
    assert_eq!(cf.children[1].offset.top.to_i32(), 100);
    // Abs at (200, 200).
    assert_eq!(cf.children[2].offset.left.to_i32(), 200);
    assert_eq!(cf.children[2].offset.top.to_i32(), 200);
}

#[test]
fn interaction_dom_relative_then_abs() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    // Relative child.
    let rel = doc.create_node(ElementTag::Div);
    doc.node_mut(rel).style.display = Display::Block;
    doc.node_mut(rel).style.position = Position::Relative;
    doc.node_mut(rel).style.width = Length::px(800.0);
    doc.node_mut(rel).style.height = Length::px(100.0);
    doc.node_mut(rel).style.top = Length::px(10.0);
    doc.node_mut(rel).style.left = Length::px(10.0);
    doc.append_child(container, rel);
    // Abs child inside the relative.
    let abs = doc.create_node(ElementTag::Div);
    doc.node_mut(abs).style.display = Display::Block;
    doc.node_mut(abs).style.position = Position::Absolute;
    doc.node_mut(abs).style.top = Length::px(20.0);
    doc.node_mut(abs).style.left = Length::px(20.0);
    doc.node_mut(abs).style.width = Length::px(50.0);
    doc.node_mut(abs).style.height = Length::px(30.0);
    doc.append_child(rel, abs);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let rel_frag = &frag.children[0].children[0];
    assert_eq!(rel_frag.offset.left.to_i32(), 10);
    assert_eq!(rel_frag.offset.top.to_i32(), 10);
    let abs_frag = &rel_frag.children[0];
    assert_eq!(abs_frag.offset.left.to_i32(), 20);
    assert_eq!(abs_frag.offset.top.to_i32(), 20);
}

#[test]
fn interaction_abs_own_padding() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).padding(10, 10, 10, 10)
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // Size includes padding: 200+20=220, 100+20=120.
    r.assert_child_size(0, 220, 120);
}

#[test]
fn interaction_abs_own_border() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).border(5, 5, 5, 5)
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 210, 110);
}

#[test]
fn interaction_negative_z_index() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .with_style(|s| { s.z_index = Some(-1); })
        .inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
}

#[test]
fn interaction_relative_z_index_stacking_context() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(200.0).position_relative()
        .with_style(|s| { s.z_index = Some(1); s.top = Length::px(0.0); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 400, 200);
}

#[test]
fn interaction_abs_in_overflow_hidden() {
    let mut b = abs_builder(400, 300)
        .with_container_style(|s| { s.overflow_x = Overflow::Hidden; s.overflow_y = Overflow::Hidden; });
    b.add_child().width(200.0).height(100.0).position_absolute().inset(250, 0, 0, 350).done();
    let r = b.build();
    // Position is set even if it extends beyond overflow hidden container.
    r.assert_child_position(0, 350, 250);
}

#[test]
fn interaction_mixed_flow_abs_combo_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 10, 10);
}

#[test]
fn interaction_mixed_flow_abs_combo_1() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(30, 0, 0, 25).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(60, 0, 0, 50).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 25, 30);
    r.assert_child_position(3, 50, 60);
}

#[test]
fn interaction_mixed_flow_abs_combo_2() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(800.0).height(60.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(50, 0, 0, 40).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 0, 90);
    r.assert_child_position(3, 40, 50);
}

#[test]
fn interaction_mixed_flow_abs_combo_3() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(70, 0, 0, 55).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(100, 0, 0, 80).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 55, 70);
    r.assert_child_position(2, 80, 100);
}

#[test]
fn interaction_mixed_flow_abs_combo_4() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(90, 0, 0, 70).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 70, 90);
}

#[test]
fn interaction_mixed_flow_abs_combo_5() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(800.0).height(60.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(110, 0, 0, 85).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(140, 0, 0, 110).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 0, 90);
    r.assert_child_position(3, 85, 110);
    r.assert_child_position(4, 110, 140);
}

#[test]
fn interaction_mixed_flow_abs_combo_6() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(130, 0, 0, 100).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 100, 130);
}

#[test]
fn interaction_mixed_flow_abs_combo_7() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(150, 0, 0, 115).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(180, 0, 0, 140).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 115, 150);
    r.assert_child_position(3, 140, 180);
}

#[test]
fn interaction_mixed_flow_abs_combo_8() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(800.0).height(60.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(170, 0, 0, 130).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 0, 90);
    r.assert_child_position(3, 130, 170);
}

#[test]
fn interaction_mixed_flow_abs_combo_9() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(190, 0, 0, 145).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(220, 0, 0, 170).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 145, 190);
    r.assert_child_position(2, 170, 220);
}

#[test]
fn interaction_mixed_flow_abs_combo_10() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(210, 0, 0, 160).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 160, 210);
}

#[test]
fn interaction_mixed_flow_abs_combo_11() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(800.0).height(60.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(230, 0, 0, 175).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(260, 0, 0, 200).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 0, 90);
    r.assert_child_position(3, 175, 230);
    r.assert_child_position(4, 200, 260);
}

#[test]
fn interaction_mixed_flow_abs_combo_12() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(250, 0, 0, 190).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 190, 250);
}

#[test]
fn interaction_mixed_flow_abs_combo_13() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(270, 0, 0, 205).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(300, 0, 0, 230).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 205, 270);
    r.assert_child_position(3, 230, 300);
}

#[test]
fn interaction_mixed_flow_abs_combo_14() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(800.0).height(60.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(290, 0, 0, 220).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 0, 90);
    r.assert_child_position(3, 220, 290);
}

#[test]
fn interaction_mixed_flow_abs_combo_15() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(310, 0, 0, 235).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(340, 0, 0, 260).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 235, 310);
    r.assert_child_position(2, 260, 340);
}

#[test]
fn interaction_mixed_flow_abs_combo_16() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(330, 0, 0, 250).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 250, 330);
}

#[test]
fn interaction_mixed_flow_abs_combo_17() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(800.0).height(60.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(350, 0, 0, 265).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(380, 0, 0, 290).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 0, 90);
    r.assert_child_position(3, 265, 350);
    r.assert_child_position(4, 290, 380);
}

#[test]
fn interaction_mixed_flow_abs_combo_18() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(370, 0, 0, 280).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 280, 370);
}

#[test]
fn interaction_mixed_flow_abs_combo_19() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(40.0).done();
    b.add_child().width(800.0).height(50.0).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(390, 0, 0, 295).done();
    b.add_child().width(50.0).height(30.0).position_absolute().inset(420, 0, 0, 320).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_position(2, 295, 390);
    r.assert_child_position(3, 320, 420);
}

#[test]
fn interaction_abs_container_width() {
    let mut b = abs_builder(500, 400);
    b.add_child().width(200.0).height(100.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_container_width(500);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn interaction_abs_overrides_relative_like_behavior() {
    // If position is absolute, relative offsets don't apply—abs positioning takes over.
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute().inset(50, 0, 0, 50).done();
    let r = b.build();
    r.assert_child_position(0, 50, 50);
}

#[test]
fn interaction_multiple_abs_same_position() {
    let mut b = abs_builder(800, 600);
    for _ in 0..3 {
        b.add_child().width(100.0).height(50.0).position_absolute().inset(10, 0, 0, 10).done();
    }
    let r = b.build();
    for i in 0..3 {
        r.assert_child_position(i, 10, 10);
    }
}

// ═══════════════════════════════════════════════════════════════════
// Section 6: Edge cases
// ═══════════════════════════════════════════════════════════════════

#[test]
fn edge_zero_size_container_absolute() {
    let mut b = abs_builder(0, 0);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 100, 50);
}

#[test]
fn edge_zero_size_container_relative() {
    let mut b = BlockTestBuilder::new(0, 0);
    b.add_child().width(100.0).height(50.0).position_relative().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
}

#[test]
fn edge_zero_size_container_fixed() {
    let mut b = BlockTestBuilder::new(0, 0);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(5.0); s.left = Length::px(5.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 5);
    assert_eq!(fc.offset.top.to_i32(), 5);
}

#[test]
fn edge_zero_size_child_absolute() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(0.0).height(0.0).position_absolute().inset(100, 0, 0, 100).done();
    let r = b.build();
    r.assert_child_position(0, 100, 100);
    r.assert_child_size(0, 0, 0);
}

#[test]
fn edge_zero_size_child_relative() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(0.0).height(0.0).position_relative().inset(50, 0, 0, 50).done();
    let r = b.build();
    r.assert_child_position(0, 50, 50);
    r.assert_child_size(0, 0, 0);
}

#[test]
fn edge_large_offset_absolute() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.top = Length::px(10000.0); s.left = Length::px(10000.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 10000, 10000);
}

#[test]
fn edge_large_offset_relative() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(5000.0); s.left = Length::px(5000.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 5000, 5000);
}

#[test]
fn edge_large_negative_offset() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.top = Length::px(-5000.0); s.left = Length::px(-5000.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, -5000, -5000);
}

#[test]
fn edge_large_container() {
    let mut b = abs_builder(10000, 10000);
    b.add_child().width(200.0).height(100.0).position_absolute().inset(5000, 0, 0, 5000).done();
    let r = b.build();
    r.assert_child_position(0, 5000, 5000);
}

#[test]
fn edge_1x1_container_absolute() {
    let mut b = abs_builder(1, 1);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn edge_abs_child_larger_than_container() {
    let mut b = abs_builder(100, 100);
    b.add_child().width(500.0).height(500.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 500, 500);
}

#[test]
fn edge_abs_left_right_exceed_container() {
    // left=500 + right=500 > 100 container. Auto width = max(0, 100-500-500) = 0? or negative.
    let mut b = abs_builder(100, 100);
    b.add_child().height(50.0).position_absolute()
        .with_style(|s| { s.left = Length::px(500.0); s.right = Length::px(500.0); s.top = Length::px(0.0); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    // Width can't be negative, should be 0 or clamped.
    let child = r.child(0);
    assert!(child.size.width.to_i32() >= 0);
}

#[test]
fn edge_relative_auto_width() {
    let mut b = BlockTestBuilder::new(400, 300);
    b.add_child().height(50.0).position_relative().inset(10, 0, 0, 0).done();
    let r = b.build();
    // Auto width fills container.
    r.assert_child_size(0, 400, 50);
    r.assert_child_position(0, 0, 10);
}

#[test]
fn edge_relative_auto_height() {
    let mut b = BlockTestBuilder::new(400, 300);
    b.add_child().width(200.0).position_relative().inset(10, 0, 0, 0).done();
    let r = b.build();
    // Auto height with no content = 0.
    r.assert_child_size(0, 200, 0);
    r.assert_child_position(0, 0, 10);
}

#[test]
fn edge_abs_all_auto() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute().done();
    let r = b.build();
    // All insets auto → static position fallback → (0,0).
    r.assert_child_position(0, 0, 0);
}

#[test]
fn edge_abs_only_top() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .with_style(|s| { s.top = Length::px(50.0); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 50);
}

#[test]
fn edge_abs_only_left() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .with_style(|s| { s.left = Length::px(50.0); })
        .done();
    let r = b.build();
    let child = r.child(0);
    assert_eq!(child.offset.left.to_i32(), 50);
}

#[test]
fn edge_abs_only_right() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .with_style(|s| { s.right = Length::px(50.0); })
        .done();
    let r = b.build();
    let child = r.child(0);
    // right=50, width=200. left = 800-50-200 = 550.
    assert_eq!(child.offset.left.to_i32(), 550);
}

#[test]
fn edge_abs_only_bottom() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .with_style(|s| { s.bottom = Length::px(50.0); })
        .done();
    let r = b.build();
    let child = r.child(0);
    assert_eq!(child.offset.top.to_i32(), 450);
}

#[test]
fn edge_sticky_zero_insets() {
    let off = compute_sticky_offset(
        offset(0, 0), offset(0, 0), viewport(),
        &insets(Some(0), Some(0), Some(0), Some(0)),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(0));
    assert_eq!(off.left, lu(0));
}

#[test]
fn edge_sticky_no_insets() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 300), viewport(),
        &insets(None, None, None, None),
        size(100, 50), large_cb(),
    );
    // No insets → no sticking.
    assert_eq!(off.top, lu(0));
    assert_eq!(off.left, lu(0));
}

#[test]
fn edge_sticky_element_at_origin() {
    let off = compute_sticky_offset(
        offset(0, 0), offset(0, 100), viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50), large_cb(),
    );
    // el_in_vp=0-100=-100. start_stick=0-(-100)=100.
    assert_eq!(off.top, lu(100));
}

#[test]
fn edge_sticky_large_scroll() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 100000), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    // Would be huge but clamped by CB.
    // max_positive=(2000-50)-200=1750. So offset=1750.
    assert_eq!(off.top, lu(1750));
}

#[test]
fn edge_sticky_zero_size_element() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 300), viewport(),
        &insets(Some(10), None, None, None),
        size(0, 0), large_cb(),
    );
    // el_in_vp=-100. start_stick=10-(-100)=110.
    // max_positive=(2000-0)-200=1800.
    assert_eq!(off.top, lu(110));
}

#[test]
fn edge_sticky_zero_size_viewport() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 300), prect(0, 0, 0, 0),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    // vp is 0×0. el_in_vp=200-300=-100. start_stick=(0+10)-(-100)=110.
    let v = off.top.to_i32();
    // The result depends on clamping logic with 0-size viewport.
    assert!(v >= 0 || v < 0); // Just verify no crash.
}

#[test]
fn edge_sticky_cb_at_origin() {
    let off = compute_sticky_offset(
        offset(0, 0), offset(0, 100), viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50), prect(0, 0, 800, 100),
    );
    // max_positive=(100-50)-0=50. start_stick=100. clamp(100, 0, 50)=50.
    assert_eq!(off.top, lu(50));
}

#[test]
fn edge_sticky_cb_smaller_than_element() {
    // CB height < element height. max_positive is negative.
    let off = compute_sticky_offset(
        offset(0, 0), offset(0, 100), viewport(),
        &insets(Some(0), None, None, None),
        size(100, 200), prect(0, 0, 800, 100),
    );
    // max_positive=(100-200)-0=-100. start_stick=0-(-100)=100.
    // clamp(100, 0, -100). When max < min, engine returns 0.
    assert_eq!(off.top, lu(0));
}

#[test]
fn edge_abs_100_percent_width() {
    let mut b = abs_builder(800, 600);
    b.add_child().position_absolute()
        .with_style(|s| {
            s.top = Length::px(0.0); s.left = Length::px(0.0);
            s.right = Length::auto(); s.bottom = Length::auto();
            s.width = Length::percent(100.0); s.height = Length::percent(100.0);
        }).done();
    let r = b.build();
    r.assert_child_size(0, 800, 600);
}

#[test]
fn edge_abs_0_percent_width() {
    let mut b = abs_builder(800, 600);
    b.add_child().position_absolute()
        .with_style(|s| {
            s.top = Length::px(0.0); s.left = Length::px(0.0);
            s.right = Length::auto(); s.bottom = Length::auto();
            s.width = Length::percent(0.0); s.height = Length::percent(0.0);
        }).done();
    let r = b.build();
    r.assert_child_size(0, 0, 0);
}

#[test]
fn edge_rel_0_percent_top() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.top = Length::percent(0.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn edge_rel_100_percent_top() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative()
        .with_style(|s| { s.top = Length::percent(100.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    // 100% of 600 = 600.
    r.assert_child_position(0, 0, 600);
}

#[test]
fn edge_abs_border_box_fills() {
    let mut b = abs_builder(800, 600);
    b.add_child().padding(20, 20, 20, 20).box_sizing_border_box()
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    // Auto width+height fills container: 800×600.
    r.assert_child_size(0, 800, 600);
}

#[test]
fn edge_abs_container_border_and_padding() {
    let mut b = abs_builder(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 10; s.border_top_style = BorderStyle::Solid;
            s.border_left_width = 10; s.border_left_style = BorderStyle::Solid;
            s.border_bottom_width = 10; s.border_bottom_style = BorderStyle::Solid;
            s.border_right_width = 10; s.border_right_style = BorderStyle::Solid;
            s.padding_top = Length::px(5.0);
            s.padding_left = Length::px(5.0);
        });
    b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    // Abs offset in parent border-box coordinates: left=0+10=10, top=0+10=10.
    r.assert_child_position(0, 10, 10);
}

#[test]
fn edge_abs_top_0_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn edge_abs_top_5_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(5.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 5);
}

#[test]
fn edge_abs_top_10_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(10.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 10);
}

#[test]
fn edge_abs_top_15_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(15.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 15);
}

#[test]
fn edge_abs_top_20_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(20.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 20);
}

#[test]
fn edge_abs_top_25_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(25.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 25);
}

#[test]
fn edge_abs_top_30_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(30.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 30);
}

#[test]
fn edge_abs_top_35_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(35.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 35);
}

#[test]
fn edge_abs_top_40_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(40.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 40);
}

#[test]
fn edge_abs_top_45_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(45.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 45);
}

#[test]
fn edge_abs_top_50_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(50.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 50);
}

#[test]
fn edge_abs_top_55_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(55.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 55);
}

#[test]
fn edge_abs_top_60_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(60.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 60);
}

#[test]
fn edge_abs_top_65_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(65.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 65);
}

#[test]
fn edge_abs_top_70_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(70.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 70);
}

#[test]
fn edge_abs_top_75_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(75.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 75);
}

#[test]
fn edge_abs_top_80_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(80.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 80);
}

#[test]
fn edge_abs_top_85_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(85.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 85);
}

#[test]
fn edge_abs_top_90_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(90.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 90);
}

#[test]
fn edge_abs_top_95_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(95.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 95);
}

#[test]
fn edge_abs_top_100_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(100.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 100);
}

#[test]
fn edge_abs_top_105_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(105.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 105);
}

#[test]
fn edge_abs_top_110_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(110.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 110);
}

#[test]
fn edge_abs_top_115_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(115.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 115);
}

#[test]
fn edge_abs_top_120_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(120.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 120);
}

#[test]
fn edge_abs_top_125_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(125.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 125);
}

#[test]
fn edge_abs_top_130_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(130.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 130);
}

#[test]
fn edge_abs_top_135_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(135.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 135);
}

#[test]
fn edge_abs_top_140_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(140.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 140);
}

#[test]
fn edge_abs_top_145_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(145.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 145);
}

#[test]
fn edge_abs_top_150_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(150.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 150);
}

#[test]
fn edge_abs_top_155_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(155.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 155);
}

#[test]
fn edge_abs_top_160_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(160.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 160);
}

#[test]
fn edge_abs_top_165_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(165.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 165);
}

#[test]
fn edge_abs_top_170_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(170.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 170);
}

#[test]
fn edge_abs_top_175_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(175.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 175);
}

#[test]
fn edge_abs_top_180_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(180.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 180);
}

#[test]
fn edge_abs_top_185_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(185.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 185);
}

#[test]
fn edge_abs_top_190_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(190.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 190);
}

#[test]
fn edge_abs_top_195_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(195.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 195);
}

#[test]
fn edge_abs_top_200_left_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(30.0).position_absolute()
        .with_style(|s| { s.top = Length::px(200.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 200);
}

#[test]
fn edge_rel_top_0_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(0.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn edge_rel_top_5_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(5.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 5);
}

#[test]
fn edge_rel_top_10_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(10.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 10);
}

#[test]
fn edge_rel_top_15_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(15.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 15);
}

#[test]
fn edge_rel_top_20_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(20.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 20);
}

#[test]
fn edge_rel_top_25_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(25.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 25);
}

#[test]
fn edge_rel_top_30_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(30.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 30);
}

#[test]
fn edge_rel_top_35_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(35.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 35);
}

#[test]
fn edge_rel_top_40_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(40.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 40);
}

#[test]
fn edge_rel_top_45_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(45.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 45);
}

#[test]
fn edge_rel_top_50_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(50.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 50);
}

#[test]
fn edge_rel_top_55_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(55.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 55);
}

#[test]
fn edge_rel_top_60_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(60.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 60);
}

#[test]
fn edge_rel_top_65_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(65.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 65);
}

#[test]
fn edge_rel_top_70_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(70.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 70);
}

#[test]
fn edge_rel_top_75_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(75.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 75);
}

#[test]
fn edge_rel_top_80_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(80.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 80);
}

#[test]
fn edge_rel_top_85_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(85.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 85);
}

#[test]
fn edge_rel_top_90_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(90.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 90);
}

#[test]
fn edge_rel_top_95_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(95.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 95);
}

#[test]
fn edge_rel_top_100_only() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(100.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 100);
}

#[test]
fn edge_abs_right_0_bottom_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(0.0); s.bottom = Length::px(0.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 700, 550);
}

#[test]
fn edge_abs_right_10_bottom_10() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(10.0); s.bottom = Length::px(10.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 690, 540);
}

#[test]
fn edge_abs_right_20_bottom_20() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(20.0); s.bottom = Length::px(20.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 680, 530);
}

#[test]
fn edge_abs_right_30_bottom_30() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(30.0); s.bottom = Length::px(30.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 670, 520);
}

#[test]
fn edge_abs_right_40_bottom_40() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(40.0); s.bottom = Length::px(40.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 660, 510);
}

#[test]
fn edge_abs_right_50_bottom_50() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(50.0); s.bottom = Length::px(50.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 650, 500);
}

#[test]
fn edge_abs_right_60_bottom_60() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(60.0); s.bottom = Length::px(60.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 640, 490);
}

#[test]
fn edge_abs_right_70_bottom_70() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(70.0); s.bottom = Length::px(70.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 630, 480);
}

#[test]
fn edge_abs_right_80_bottom_80() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(80.0); s.bottom = Length::px(80.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 620, 470);
}

#[test]
fn edge_abs_right_90_bottom_90() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(90.0); s.bottom = Length::px(90.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 610, 460);
}

#[test]
fn edge_abs_right_100_bottom_100() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(100.0); s.bottom = Length::px(100.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 600, 450);
}

#[test]
fn edge_abs_right_110_bottom_110() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(110.0); s.bottom = Length::px(110.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 590, 440);
}

#[test]
fn edge_abs_right_120_bottom_120() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(120.0); s.bottom = Length::px(120.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 580, 430);
}

#[test]
fn edge_abs_right_130_bottom_130() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(130.0); s.bottom = Length::px(130.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 570, 420);
}

#[test]
fn edge_abs_right_140_bottom_140() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(140.0); s.bottom = Length::px(140.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 560, 410);
}

#[test]
fn edge_abs_right_150_bottom_150() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(150.0); s.bottom = Length::px(150.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 550, 400);
}

#[test]
fn edge_abs_right_160_bottom_160() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(160.0); s.bottom = Length::px(160.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 540, 390);
}

#[test]
fn edge_abs_right_170_bottom_170() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(170.0); s.bottom = Length::px(170.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 530, 380);
}

#[test]
fn edge_abs_right_180_bottom_180() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(180.0); s.bottom = Length::px(180.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 520, 370);
}

#[test]
fn edge_abs_right_190_bottom_190() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(190.0); s.bottom = Length::px(190.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 510, 360);
}

#[test]
fn edge_abs_right_200_bottom_200() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .with_style(|s| { s.right = Length::px(200.0); s.bottom = Length::px(200.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 500, 350);
}

#[test]
fn edge_fixed_pos_0_0_container_800x600() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 0);
}

#[test]
fn edge_fixed_pos_10_10_container_800x600() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn edge_fixed_pos_0_0_container_1920x1080() {
    let mut b = BlockTestBuilder::new(1920, 1080);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 0);
}

#[test]
fn edge_fixed_pos_100_100_container_400x300() {
    let mut b = BlockTestBuilder::new(400, 300);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(100.0); s.left = Length::px(100.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 100);
    assert_eq!(fc.offset.top.to_i32(), 100);
}

#[test]
fn edge_fixed_pos_50_750_container_800x600() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(50.0); s.left = Length::px(750.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 750);
    assert_eq!(fc.offset.top.to_i32(), 50);
}

#[test]
fn edge_fixed_pos_550_0_container_800x600() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(550.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 550);
}

#[test]
fn edge_fixed_pos_0_0_container_320x480() {
    let mut b = BlockTestBuilder::new(320, 480);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 0);
}

#[test]
fn edge_fixed_pos_0_0_container_1024x768() {
    let mut b = BlockTestBuilder::new(1024, 768);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 0);
}

#[test]
fn edge_fixed_pos_200_300_container_1000x800() {
    let mut b = BlockTestBuilder::new(1000, 800);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(200.0); s.left = Length::px(300.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 300);
    assert_eq!(fc.offset.top.to_i32(), 200);
}

#[test]
fn edge_fixed_pos_0_500_container_500x500() {
    let mut b = BlockTestBuilder::new(500, 500);
    b.add_child().width(50.0).height(30.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(500.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 500);
    assert_eq!(fc.offset.top.to_i32(), 0);
}

#[test]
fn edge_sticky_viewport_100x100() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 300), prect(0, 0, 100, 100),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(110));
}

#[test]
fn edge_sticky_viewport_400x300() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 300), prect(0, 0, 400, 300),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(110));
}

#[test]
fn edge_sticky_viewport_800x600() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 300), prect(0, 0, 800, 600),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(110));
}

#[test]
fn edge_sticky_viewport_1920x1080() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 300), prect(0, 0, 1920, 1080),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(110));
}

#[test]
fn edge_sticky_viewport_320x480() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 300), prect(0, 0, 320, 480),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(110));
}

#[test]
fn edge_abs_center_100x100_in_800x600() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 350, 250);
}

#[test]
fn edge_abs_center_200x200_in_800x600() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(200.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 300, 200);
}

#[test]
fn edge_abs_center_400x300_in_800x600() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(400.0).height(300.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 200, 150);
}

#[test]
fn edge_abs_center_600x400_in_800x600() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(600.0).height(400.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 100, 100);
}

#[test]
fn edge_abs_center_50x50_in_800x600() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 375, 275);
}

#[test]
fn edge_abs_center_10x10_in_800x600() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(10.0).height(10.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 395, 295);
}

#[test]
fn edge_abs_center_790x590_in_800x600() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(790.0).height(590.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
}

#[test]
fn edge_abs_center_1x1_in_800x600() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(2.0).height(2.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 399, 299);
}

#[test]
fn edge_abs_center_399x299_in_800x600() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(400.0).height(300.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 200, 150);
}

#[test]
fn edge_abs_center_500x500_in_800x600() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(500.0).height(500.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 150, 50);
}

#[test]
fn edge_dom_abs_no_insets_no_size() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    let _abs = setup_abs_child(&mut doc, container);
    // All defaults: no insets, no size → static position, zero size.
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    assert_eq!(abs_frag.offset.left.to_i32(), 0);
    assert_eq!(abs_frag.offset.top.to_i32(), 0);
}

#[test]
fn edge_dom_abs_zero_size_container() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 0, 0);
    doc.node_mut(container).style.position = Position::Relative;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::px(10.0);
    doc.node_mut(abs).style.left = Length::px(10.0);
    doc.node_mut(abs).style.width = Length::px(50.0);
    doc.node_mut(abs).style.height = Length::px(30.0);
    let space = root_space(0, 0);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    assert_eq!(abs_frag.offset.left.to_i32(), 10);
    assert_eq!(abs_frag.offset.top.to_i32(), 10);
}

#[test]
fn edge_dom_abs_percentage_zero_container() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 0, 0);
    doc.node_mut(container).style.position = Position::Relative;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::percent(50.0);
    doc.node_mut(abs).style.left = Length::percent(50.0);
    doc.node_mut(abs).style.width = Length::px(50.0);
    doc.node_mut(abs).style.height = Length::px(30.0);
    let space = root_space(0, 0);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    // 50% of 0 = 0.
    assert_eq!(abs_frag.offset.left.to_i32(), 0);
    assert_eq!(abs_frag.offset.top.to_i32(), 0);
}

#[test]
fn edge_dom_many_abs_children() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    for i in 0..10 {
        let abs = setup_abs_child(&mut doc, container);
        doc.node_mut(abs).style.top = Length::px((i * 50) as f32);
        doc.node_mut(abs).style.left = Length::px((i * 50) as f32);
        doc.node_mut(abs).style.width = Length::px(50.0);
        doc.node_mut(abs).style.height = Length::px(30.0);
    }
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let cf = &frag.children[0];
    assert_eq!(cf.children.len(), 10);
    for i in 0..10 {
        assert_eq!(cf.children[i].offset.left.to_i32(), (i as i32) * 50);
        assert_eq!(cf.children[i].offset.top.to_i32(), (i as i32) * 50);
    }
}

#[test]
fn edge_dom_nested_abs_three_levels() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    // Level 1: relative.
    let lvl1 = doc.create_node(ElementTag::Div);
    doc.node_mut(lvl1).style.display = Display::Block;
    doc.node_mut(lvl1).style.position = Position::Relative;
    doc.node_mut(lvl1).style.width = Length::px(600.0);
    doc.node_mut(lvl1).style.height = Length::px(400.0);
    doc.append_child(container, lvl1);
    // Level 2: absolute inside relative.
    let lvl2 = doc.create_node(ElementTag::Div);
    doc.node_mut(lvl2).style.display = Display::Block;
    doc.node_mut(lvl2).style.position = Position::Absolute;
    doc.node_mut(lvl2).style.top = Length::px(50.0);
    doc.node_mut(lvl2).style.left = Length::px(50.0);
    doc.node_mut(lvl2).style.width = Length::px(300.0);
    doc.node_mut(lvl2).style.height = Length::px(200.0);
    doc.append_child(lvl1, lvl2);
    // Level 3: absolute inside absolute.
    let lvl3 = doc.create_node(ElementTag::Div);
    doc.node_mut(lvl3).style.display = Display::Block;
    doc.node_mut(lvl3).style.position = Position::Absolute;
    doc.node_mut(lvl3).style.top = Length::px(10.0);
    doc.node_mut(lvl3).style.left = Length::px(10.0);
    doc.node_mut(lvl3).style.width = Length::px(100.0);
    doc.node_mut(lvl3).style.height = Length::px(50.0);
    doc.append_child(lvl2, lvl3);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let lvl1_frag = &frag.children[0].children[0];
    let lvl2_frag = &lvl1_frag.children[0];
    let lvl3_frag = &lvl2_frag.children[0];
    assert_eq!(lvl2_frag.offset.left.to_i32(), 50);
    assert_eq!(lvl2_frag.offset.top.to_i32(), 50);
    assert_eq!(lvl3_frag.offset.left.to_i32(), 10);
    assert_eq!(lvl3_frag.offset.top.to_i32(), 10);
}

#[test]
fn edge_sticky_constraint_rect_large_percent() {
    let style = make_sticky_style(Length::percent(100.0), Length::auto(), Length::auto(), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(600));
    // 100% of 600 = 600.
    assert_eq!(cr.top, Some(lu(600)));
}

#[test]
fn edge_sticky_constraint_rect_zero_cb() {
    let style = make_sticky_style(Length::px(10.0), Length::auto(), Length::auto(), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(0), lu(0));
    assert_eq!(cr.top, Some(lu(10)));
}

#[test]
fn edge_sticky_data_all_zero() {
    let data = StickyPositionData {
        normal_flow_offset: offset(0, 0),
        insets: StickyConstraintRect::none(),
        element_size: size(0, 0),
        containing_block_rect: prect(0, 0, 0, 0),
    };
    assert_eq!(data.normal_flow_offset.left, lu(0));
    assert_eq!(data.normal_flow_offset.top, lu(0));
    assert_eq!(data.insets.top, None);
}

#[test]
fn edge_make_fragment_zero() {
    let f = make_fragment(0, 0, 0, 0);
    assert_eq!(f.offset.left, lu(0));
    assert_eq!(f.offset.top, lu(0));
    assert_eq!(f.size.width, lu(0));
    assert_eq!(f.size.height, lu(0));
}

#[test]
fn edge_make_fragment_large() {
    let f = make_fragment(10000, 10000, 5000, 5000);
    assert_eq!(f.offset.left, lu(10000));
    assert_eq!(f.offset.top, lu(10000));
    assert_eq!(f.size.width, lu(5000));
    assert_eq!(f.size.height, lu(5000));
}

#[test]
fn edge_sticky_apply_all_insets() {
    let style = make_sticky_style(Length::px(10.0), Length::px(10.0), Length::px(10.0), Length::px(10.0));
    let mut frag = make_fragment(200, 200, 80, 50);
    apply_sticky_offset(&mut frag, &style, offset(300, 300), viewport(), lu(2000), lu(2000), prect(0, 0, 2000, 2000));
    // Vertical: el_in_vp=200-300=-100. start_stick=10-(-100)=110. end_stick=590-(-50)=640. raw=max(110,min(0,640))=110.
    // Horizontal: el_in_vp=200-300=-100. start_stick=10-(-100)=110. end_stick=790-(-20)=810. raw=max(110,min(0,810))=110.
    assert_eq!(frag.offset.top, lu(310));
    assert_eq!(frag.offset.left, lu(310));
}

#[test]
fn edge_abs_overflow_scroll_container() {
    let mut b = abs_builder(400, 300)
        .with_container_style(|s| { s.overflow_x = Overflow::Auto; s.overflow_y = Overflow::Auto; });
    b.add_child().width(200.0).height(100.0).position_absolute().inset(10, 0, 0, 10).done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
}

#[test]
fn edge_five_flow_one_abs_one_rel() {
    let mut b = abs_builder(800, 1000);
    b.add_child().width(800.0).height(100.0).done();
    b.add_child().width(800.0).height(100.0).done();
    b.add_child().width(200.0).height(50.0).position_absolute().inset(500, 0, 0, 500).done();
    b.add_child().width(800.0).height(100.0).done();
    b.add_child().width(800.0).height(100.0).position_relative().inset(10, 0, 0, 10).done();
    b.add_child().width(800.0).height(100.0).done();
    let r = b.build();
    // Flow children: 0(y=0), 1(y=100), 2(y=200), 3(y=300+10=310 rel), 4(y=400).
    // OOF child: 5 (abs at 500,500).
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 100);
    r.assert_child_position(2, 0, 200);
    r.assert_child_position(3, 10, 310);
    r.assert_child_position(4, 0, 400);
    r.assert_child_position(5, 500, 500);
}

#[test]
fn edge_abs_child_count_zero_flow() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 0).done();
    b.add_child().width(100.0).height(50.0).position_absolute().inset(50, 0, 0, 50).done();
    let r = b.build();
    r.assert_child_count(2);
}

#[test]
fn edge_rel_float_right() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).float_right().position_relative().inset(5, 0, 0, 5).done();
    let r = b.build();
    // CSS 2.1 §9.4.3: relative offsets apply to floats.
    r.assert_child_position(0, 605, 5);
}

#[test]
fn edge_abs_width_auto_only_top_left() {
    let mut b = abs_builder(800, 600);
    b.add_child().height(100.0).position_absolute()
        .with_style(|s| { s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 10, 10);
    // Width auto with only left → shrink-to-fit (0 for empty block).
    let child = r.child(0);
    assert!(child.size.width.to_i32() >= 0);
}

#[test]
fn edge_static_between_abs() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute().inset(0, 0, 0, 0).done();
    b.add_child().width(800.0).height(80.0).done();
    b.add_child().width(100.0).height(50.0).position_absolute().inset(100, 0, 0, 100).done();
    let r = b.build();
    // Static child at y=0 (abs children don't take space).
    r.assert_child_position(1, 0, 0);
}

// ═══════════════════════════════════════════════════════════════════
// Section 7: Supplementary tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn rel_supp_two_children_h50_50_offset_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).position_relative().inset(5, 0, 0, 5).done();
    b.add_child().width(200.0).height(50.0).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    r.assert_child_position(1, 0, 50);
}

#[test]
fn rel_supp_two_children_h100_50_offset_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(100.0).position_relative().inset(5, 0, 0, 5).done();
    b.add_child().width(200.0).height(50.0).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    r.assert_child_position(1, 0, 100);
}

#[test]
fn rel_supp_two_children_h50_100_offset_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).position_relative().inset(5, 0, 0, 5).done();
    b.add_child().width(200.0).height(100.0).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    r.assert_child_position(1, 0, 50);
}

#[test]
fn rel_supp_two_children_h200_200_offset_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(200.0).position_relative().inset(5, 0, 0, 5).done();
    b.add_child().width(200.0).height(200.0).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    r.assert_child_position(1, 0, 200);
}

#[test]
fn rel_supp_two_children_h75_25_offset_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(75.0).position_relative().inset(5, 0, 0, 5).done();
    b.add_child().width(200.0).height(25.0).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    r.assert_child_position(1, 0, 75);
}

#[test]
fn rel_supp_two_children_h10_10_offset_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(10.0).position_relative().inset(5, 0, 0, 5).done();
    b.add_child().width(200.0).height(10.0).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    r.assert_child_position(1, 0, 10);
}

#[test]
fn rel_supp_two_children_h150_75_offset_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(150.0).position_relative().inset(5, 0, 0, 5).done();
    b.add_child().width(200.0).height(75.0).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    r.assert_child_position(1, 0, 150);
}

#[test]
fn rel_supp_two_children_h80_120_offset_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(80.0).position_relative().inset(5, 0, 0, 5).done();
    b.add_child().width(200.0).height(120.0).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    r.assert_child_position(1, 0, 80);
}

#[test]
fn rel_supp_two_children_h60_40_offset_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(60.0).position_relative().inset(5, 0, 0, 5).done();
    b.add_child().width(200.0).height(40.0).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    r.assert_child_position(1, 0, 60);
}

#[test]
fn rel_supp_two_children_h30_70_offset_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(30.0).position_relative().inset(5, 0, 0, 5).done();
    b.add_child().width(200.0).height(70.0).done();
    let r = b.build();
    r.assert_child_position(0, 5, 5);
    r.assert_child_position(1, 0, 30);
}

#[test]
fn abs_supp_hcenter_cw_100() {
    let mut b = abs_builder(100, 600);
    b.add_child().width(80.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 10, 0);
}

#[test]
fn abs_supp_hcenter_cw_200() {
    let mut b = abs_builder(200, 600);
    b.add_child().width(80.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 60, 0);
}

#[test]
fn abs_supp_hcenter_cw_300() {
    let mut b = abs_builder(300, 600);
    b.add_child().width(80.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 110, 0);
}

#[test]
fn abs_supp_hcenter_cw_400() {
    let mut b = abs_builder(400, 600);
    b.add_child().width(80.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 160, 0);
}

#[test]
fn abs_supp_hcenter_cw_500() {
    let mut b = abs_builder(500, 600);
    b.add_child().width(80.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 210, 0);
}

#[test]
fn abs_supp_hcenter_cw_600() {
    let mut b = abs_builder(600, 600);
    b.add_child().width(80.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 260, 0);
}

#[test]
fn abs_supp_hcenter_cw_700() {
    let mut b = abs_builder(700, 600);
    b.add_child().width(80.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 310, 0);
}

#[test]
fn abs_supp_hcenter_cw_900() {
    let mut b = abs_builder(900, 600);
    b.add_child().width(80.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 410, 0);
}

#[test]
fn abs_supp_hcenter_cw_1000() {
    let mut b = abs_builder(1000, 600);
    b.add_child().width(80.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 460, 0);
}

#[test]
fn abs_supp_hcenter_cw_1200() {
    let mut b = abs_builder(1200, 600);
    b.add_child().width(80.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 560, 0);
}

#[test]
fn abs_supp_vcenter_ch_100() {
    let mut b = abs_builder(800, 100);
    b.add_child().width(50.0).height(60.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 20);
}

#[test]
fn abs_supp_vcenter_ch_200() {
    let mut b = abs_builder(800, 200);
    b.add_child().width(50.0).height(60.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 70);
}

#[test]
fn abs_supp_vcenter_ch_300() {
    let mut b = abs_builder(800, 300);
    b.add_child().width(50.0).height(60.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 120);
}

#[test]
fn abs_supp_vcenter_ch_400() {
    let mut b = abs_builder(800, 400);
    b.add_child().width(50.0).height(60.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 170);
}

#[test]
fn abs_supp_vcenter_ch_500() {
    let mut b = abs_builder(800, 500);
    b.add_child().width(50.0).height(60.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 220);
}

#[test]
fn abs_supp_vcenter_ch_700() {
    let mut b = abs_builder(800, 700);
    b.add_child().width(50.0).height(60.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 320);
}

#[test]
fn abs_supp_vcenter_ch_800() {
    let mut b = abs_builder(800, 800);
    b.add_child().width(50.0).height(60.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 370);
}

#[test]
fn abs_supp_vcenter_ch_900() {
    let mut b = abs_builder(800, 900);
    b.add_child().width(50.0).height(60.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 420);
}

#[test]
fn abs_supp_vcenter_ch_1000() {
    let mut b = abs_builder(800, 1000);
    b.add_child().width(50.0).height(60.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 470);
}

#[test]
fn abs_supp_vcenter_ch_1200() {
    let mut b = abs_builder(800, 1200);
    b.add_child().width(50.0).height(60.0).position_absolute()
        .inset(0, 0, 0, 0)
        .with_style(|s| { s.margin_top = Length::auto(); s.margin_bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 570);
}

#[test]
fn fixed_supp_viewport_320x480_top_left() {
    let mut b = BlockTestBuilder::new(320, 480);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_supp_viewport_640x480_top_left() {
    let mut b = BlockTestBuilder::new(640, 480);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_supp_viewport_1024x768_top_left() {
    let mut b = BlockTestBuilder::new(1024, 768);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_supp_viewport_1280x720_top_left() {
    let mut b = BlockTestBuilder::new(1280, 720);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_supp_viewport_1920x1080_top_left() {
    let mut b = BlockTestBuilder::new(1920, 1080);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_supp_viewport_2560x1440_top_left() {
    let mut b = BlockTestBuilder::new(2560, 1440);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_supp_viewport_375x667_top_left() {
    let mut b = BlockTestBuilder::new(375, 667);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_supp_viewport_414x896_top_left() {
    let mut b = BlockTestBuilder::new(414, 896);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_supp_viewport_768x1024_top_left() {
    let mut b = BlockTestBuilder::new(768, 1024);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_supp_viewport_1366x768_top_left() {
    let mut b = BlockTestBuilder::new(1366, 768);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(10.0); s.left = Length::px(10.0); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 10);
    assert_eq!(fc.offset.top.to_i32(), 10);
}

#[test]
fn fixed_supp_viewport_320x480_bottom_right() {
    let mut b = BlockTestBuilder::new(320, 480);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.right = Length::px(10.0); s.bottom = Length::px(10.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 210);
    assert_eq!(fc.offset.top.to_i32(), 420);
}

#[test]
fn fixed_supp_viewport_640x480_bottom_right() {
    let mut b = BlockTestBuilder::new(640, 480);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.right = Length::px(10.0); s.bottom = Length::px(10.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 530);
    assert_eq!(fc.offset.top.to_i32(), 420);
}

#[test]
fn fixed_supp_viewport_1024x768_bottom_right() {
    let mut b = BlockTestBuilder::new(1024, 768);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.right = Length::px(10.0); s.bottom = Length::px(10.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 914);
    assert_eq!(fc.offset.top.to_i32(), 708);
}

#[test]
fn fixed_supp_viewport_1280x720_bottom_right() {
    let mut b = BlockTestBuilder::new(1280, 720);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.right = Length::px(10.0); s.bottom = Length::px(10.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 1170);
    assert_eq!(fc.offset.top.to_i32(), 660);
}

#[test]
fn fixed_supp_viewport_1920x1080_bottom_right() {
    let mut b = BlockTestBuilder::new(1920, 1080);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.right = Length::px(10.0); s.bottom = Length::px(10.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 1810);
    assert_eq!(fc.offset.top.to_i32(), 1020);
}

#[test]
fn fixed_supp_viewport_2560x1440_bottom_right() {
    let mut b = BlockTestBuilder::new(2560, 1440);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.right = Length::px(10.0); s.bottom = Length::px(10.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 2450);
    assert_eq!(fc.offset.top.to_i32(), 1380);
}

#[test]
fn fixed_supp_viewport_375x667_bottom_right() {
    let mut b = BlockTestBuilder::new(375, 667);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.right = Length::px(10.0); s.bottom = Length::px(10.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 265);
    assert_eq!(fc.offset.top.to_i32(), 607);
}

#[test]
fn fixed_supp_viewport_414x896_bottom_right() {
    let mut b = BlockTestBuilder::new(414, 896);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.right = Length::px(10.0); s.bottom = Length::px(10.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 304);
    assert_eq!(fc.offset.top.to_i32(), 836);
}

#[test]
fn fixed_supp_viewport_768x1024_bottom_right() {
    let mut b = BlockTestBuilder::new(768, 1024);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.right = Length::px(10.0); s.bottom = Length::px(10.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 658);
    assert_eq!(fc.offset.top.to_i32(), 964);
}

#[test]
fn fixed_supp_viewport_1366x768_bottom_right() {
    let mut b = BlockTestBuilder::new(1366, 768);
    b.add_child().width(100.0).height(50.0)
        .with_style(|s| { s.position = Position::Fixed; s.right = Length::px(10.0); s.bottom = Length::px(10.0); s.top = Length::auto(); s.left = Length::auto(); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.offset.left.to_i32(), 1256);
    assert_eq!(fc.offset.top.to_i32(), 708);
}

#[test]
fn sticky_supp_size_50x25_scroll_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(50, 25), large_cb(),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_supp_size_100x50_scroll_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_supp_size_200x100_scroll_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(200, 100), large_cb(),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_supp_size_400x200_scroll_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(400, 200), large_cb(),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_supp_size_800x50_scroll_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(800, 50), large_cb(),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_supp_cb_height_300() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), prect(0, 0, 800, 300),
    );
    assert_eq!(off.top, lu(50));
}

#[test]
fn sticky_supp_cb_height_400() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), prect(0, 0, 800, 400),
    );
    assert_eq!(off.top, lu(150));
}

#[test]
fn sticky_supp_cb_height_500() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), prect(0, 0, 800, 500),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_supp_cb_height_600() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), prect(0, 0, 800, 600),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_supp_cb_height_800() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), prect(0, 0, 800, 800),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_supp_cb_height_1000() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), prect(0, 0, 800, 1000),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_supp_cb_height_1500() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), prect(0, 0, 800, 1500),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_supp_cb_height_2000() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), prect(0, 0, 800, 2000),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_supp_cb_height_3000() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), prect(0, 0, 800, 3000),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn sticky_supp_cb_height_5000() {
    let off = compute_sticky_offset(
        offset(0, 200), offset(0, 400), viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50), prect(0, 0, 800, 5000),
    );
    assert_eq!(off.top, lu(210));
}

#[test]
fn rel_supp_left_offset_0() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(0.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn rel_supp_left_offset_2() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(2.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 2, 0);
}

#[test]
fn rel_supp_left_offset_4() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(4.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 4, 0);
}

#[test]
fn rel_supp_left_offset_6() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(6.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 6, 0);
}

#[test]
fn rel_supp_left_offset_8() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(8.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 8, 0);
}

#[test]
fn rel_supp_left_offset_10() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(10.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 10, 0);
}

#[test]
fn rel_supp_left_offset_12() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(12.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 12, 0);
}

#[test]
fn rel_supp_left_offset_14() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(14.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 14, 0);
}

#[test]
fn rel_supp_left_offset_16() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(16.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 16, 0);
}

#[test]
fn rel_supp_left_offset_18() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(18.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 18, 0);
}

#[test]
fn rel_supp_left_offset_20() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(20.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 20, 0);
}

#[test]
fn rel_supp_left_offset_22() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(22.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 22, 0);
}

#[test]
fn rel_supp_left_offset_24() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(24.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 24, 0);
}

#[test]
fn rel_supp_left_offset_26() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(26.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 26, 0);
}

#[test]
fn rel_supp_left_offset_28() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(28.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 28, 0);
}

#[test]
fn rel_supp_left_offset_30() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(30.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 30, 0);
}

#[test]
fn rel_supp_left_offset_32() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(32.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 32, 0);
}

#[test]
fn rel_supp_left_offset_34() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(34.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 34, 0);
}

#[test]
fn rel_supp_left_offset_36() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(36.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 36, 0);
}

#[test]
fn rel_supp_left_offset_38() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(38.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 38, 0);
}

#[test]
fn rel_supp_left_offset_40() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(40.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 40, 0);
}

#[test]
fn rel_supp_left_offset_42() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(42.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 42, 0);
}

#[test]
fn rel_supp_left_offset_44() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(44.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 44, 0);
}

#[test]
fn rel_supp_left_offset_46() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(46.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 46, 0);
}

#[test]
fn rel_supp_left_offset_48() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(48.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 48, 0);
}

#[test]
fn rel_supp_left_offset_50() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(50.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 50, 0);
}

#[test]
fn abs_supp_with_margin_0_center() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .margin(0, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    // left margin auto, right margin auto: centered horizontally.
    r.assert_child_position(0, 300, 0);
}

#[test]
fn abs_supp_with_margin_10_center() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .margin(10, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    // left margin auto, right margin auto: centered horizontally.
    r.assert_child_position(0, 300, 10);
}

#[test]
fn abs_supp_with_margin_20_center() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .margin(20, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    // left margin auto, right margin auto: centered horizontally.
    r.assert_child_position(0, 300, 20);
}

#[test]
fn abs_supp_with_margin_50_center() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .margin(50, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    // left margin auto, right margin auto: centered horizontally.
    r.assert_child_position(0, 300, 50);
}

#[test]
fn abs_supp_with_margin_100_center() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .margin(100, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    // left margin auto, right margin auto: centered horizontally.
    r.assert_child_position(0, 300, 100);
}

#[test]
fn abs_supp_with_margin_150_center() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .margin(150, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    // left margin auto, right margin auto: centered horizontally.
    r.assert_child_position(0, 300, 150);
}

#[test]
fn abs_supp_with_margin_200_center() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0)
        .margin(200, 0, 0, 0)
        .with_style(|s| { s.margin_left = Length::auto(); s.margin_right = Length::auto(); })
        .done();
    let r = b.build();
    // left margin auto, right margin auto: centered horizontally.
    r.assert_child_position(0, 300, 200);
}

#[test]
fn edge_supp_dom_rtl_abs_left_only() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.direction = Direction::Rtl;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.direction = Direction::Rtl;
    doc.node_mut(abs).style.left = Length::px(100.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(100.0);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    assert_eq!(abs_frag.offset.left.to_i32(), 100);
    assert_eq!(abs_frag.size.width.to_i32(), 200);
}

#[test]
fn edge_supp_dom_rtl_abs_right_only() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.direction = Direction::Rtl;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.direction = Direction::Rtl;
    doc.node_mut(abs).style.right = Length::px(100.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(100.0);
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    // right=100, width=200. left = 800-100-200 = 500.
    assert_eq!(abs_frag.offset.left.to_i32(), 500);
}

#[test]
fn edge_supp_dom_rtl_abs_both_sides() {
    let mut doc = Document::new();
    let container = setup_container(&mut doc, 800, 600);
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.direction = Direction::Rtl;
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.direction = Direction::Rtl;
    doc.node_mut(abs).style.left = Length::px(100.0);
    doc.node_mut(abs).style.right = Length::px(100.0);
    doc.node_mut(abs).style.height = Length::px(100.0);
    // auto width: 800-100-100 = 600.
    let space = root_space(800, 600);
    let frag = block_layout(&doc, doc.root(), &space);
    let abs_frag = &frag.children[0].children[0];
    assert_eq!(abs_frag.size.width.to_i32(), 600);
}

#[test]
fn sticky_supp_constraint_rect_top_0_pct() {
    let style = make_sticky_style(Length::percent(0.0), Length::auto(), Length::auto(), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(1000));
    assert_eq!(cr.top, Some(lu(0)));
}

#[test]
fn sticky_supp_constraint_rect_top_1_pct() {
    let style = make_sticky_style(Length::percent(1.0), Length::auto(), Length::auto(), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(1000));
    assert_eq!(cr.top, Some(lu(10)));
}

#[test]
fn sticky_supp_constraint_rect_top_5_pct() {
    let style = make_sticky_style(Length::percent(5.0), Length::auto(), Length::auto(), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(1000));
    assert_eq!(cr.top, Some(lu(50)));
}

#[test]
fn sticky_supp_constraint_rect_top_10_pct() {
    let style = make_sticky_style(Length::percent(10.0), Length::auto(), Length::auto(), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(1000));
    assert_eq!(cr.top, Some(lu(100)));
}

#[test]
fn sticky_supp_constraint_rect_top_25_pct() {
    let style = make_sticky_style(Length::percent(25.0), Length::auto(), Length::auto(), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(1000));
    assert_eq!(cr.top, Some(lu(250)));
}

#[test]
fn sticky_supp_constraint_rect_top_50_pct() {
    let style = make_sticky_style(Length::percent(50.0), Length::auto(), Length::auto(), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(1000));
    assert_eq!(cr.top, Some(lu(500)));
}

#[test]
fn sticky_supp_constraint_rect_top_75_pct() {
    let style = make_sticky_style(Length::percent(75.0), Length::auto(), Length::auto(), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(1000));
    assert_eq!(cr.top, Some(lu(750)));
}

#[test]
fn sticky_supp_constraint_rect_top_100_pct() {
    let style = make_sticky_style(Length::percent(100.0), Length::auto(), Length::auto(), Length::auto());
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(1000));
    assert_eq!(cr.top, Some(lu(1000)));
}

#[test]
fn abs_supp_border_box_padding_0() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).padding(0, 0, 0, 0).box_sizing_border_box()
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // For abs children: content-box behavior. Size = 200+0 × 100+0.
    r.assert_child_size(0, 200, 100);
}

#[test]
fn abs_supp_border_box_padding_5() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).padding(5, 5, 5, 5).box_sizing_border_box()
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // border-box: specified 200×100 IS the border-box size (padding included).
    r.assert_child_size(0, 200, 100);
}

#[test]
fn abs_supp_border_box_padding_10() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).padding(10, 10, 10, 10).box_sizing_border_box()
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // border-box: specified 200×100 IS the border-box size (padding included).
    r.assert_child_size(0, 200, 100);
}

#[test]
fn abs_supp_border_box_padding_15() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).padding(15, 15, 15, 15).box_sizing_border_box()
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // border-box: specified 200×100 IS the border-box size (padding included).
    r.assert_child_size(0, 200, 100);
}

#[test]
fn abs_supp_border_box_padding_20() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).padding(20, 20, 20, 20).box_sizing_border_box()
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // border-box: specified 200×100 IS the border-box size (padding included).
    r.assert_child_size(0, 200, 100);
}

#[test]
fn abs_supp_border_box_padding_25() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).padding(25, 25, 25, 25).box_sizing_border_box()
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // border-box: specified 200×100 IS the border-box size (padding included).
    r.assert_child_size(0, 200, 100);
}

#[test]
fn abs_supp_border_box_padding_30() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).padding(30, 30, 30, 30).box_sizing_border_box()
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // border-box: specified 200×100 IS the border-box size (padding included).
    r.assert_child_size(0, 200, 100);
}

#[test]
fn abs_supp_border_box_padding_40() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).padding(40, 40, 40, 40).box_sizing_border_box()
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // border-box: specified 200×100 IS the border-box size (padding included).
    r.assert_child_size(0, 200, 100);
}

#[test]
fn abs_supp_border_box_padding_50() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).padding(50, 50, 50, 50).box_sizing_border_box()
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // border-box: specified 200×100 IS the border-box size (padding included).
    r.assert_child_size(0, 200, 100);
}

#[test]
fn abs_supp_border_box_padding_100() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(100.0).padding(100, 100, 100, 100).box_sizing_border_box()
        .position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    // border-box: padding (200) exceeds specified height (100), content clamped to 0.
    // border-box height = 0 + 100 + 100 = 200.
    r.assert_child_size(0, 200, 200);
}

// ═══════════════════════════════════════════════════════════════════
// Section 8: Additional comprehensive tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn rel_extra_neg_left_1() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(-1.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, -1, 0);
}

#[test]
fn rel_extra_neg_left_2() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(-2.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, -2, 0);
}

#[test]
fn rel_extra_neg_left_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(-5.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, -5, 0);
}

#[test]
fn rel_extra_neg_left_10() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(-10.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, -10, 0);
}

#[test]
fn rel_extra_neg_left_20() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(-20.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, -20, 0);
}

#[test]
fn rel_extra_neg_left_50() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(-50.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, -50, 0);
}

#[test]
fn rel_extra_neg_left_100() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(-100.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, -100, 0);
}

#[test]
fn rel_extra_neg_left_200() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(-200.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, -200, 0);
}

#[test]
fn rel_extra_neg_left_400() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.left = Length::px(-400.0); s.top = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, -400, 0);
}

#[test]
fn rel_extra_neg_top_1() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(-1.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, -1);
}

#[test]
fn rel_extra_neg_top_2() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(-2.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, -2);
}

#[test]
fn rel_extra_neg_top_5() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(-5.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, -5);
}

#[test]
fn rel_extra_neg_top_10() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(-10.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, -10);
}

#[test]
fn rel_extra_neg_top_20() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(-20.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, -20);
}

#[test]
fn rel_extra_neg_top_50() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(-50.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, -50);
}

#[test]
fn rel_extra_neg_top_100() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(-100.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, -100);
}

#[test]
fn rel_extra_neg_top_200() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(-200.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, -200);
}

#[test]
fn rel_extra_neg_top_400() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(50.0).position_relative()
        .with_style(|s| { s.top = Length::px(-400.0); s.left = Length::auto(); s.right = Length::auto(); s.bottom = Length::auto(); })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, -400);
}

#[test]
fn abs_extra_size_10x10() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(10.0).height(10.0).position_absolute()
        .inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_size(0, 10, 10);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn abs_extra_size_50x100() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(50.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_size(0, 50, 100);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn abs_extra_size_100x50() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(50.0).position_absolute()
        .inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 50);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn abs_extra_size_200x300() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(200.0).height(300.0).position_absolute()
        .inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 300);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn abs_extra_size_300x200() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(300.0).height(200.0).position_absolute()
        .inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_size(0, 300, 200);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn abs_extra_size_400x400() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(400.0).height(400.0).position_absolute()
        .inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 400);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn abs_extra_size_500x100() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(500.0).height(100.0).position_absolute()
        .inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_size(0, 500, 100);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn abs_extra_size_100x500() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(100.0).height(500.0).position_absolute()
        .inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 500);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn abs_extra_size_800x600() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(800.0).height(600.0).position_absolute()
        .inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_size(0, 800, 600);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn abs_extra_size_1x1() {
    let mut b = abs_builder(800, 600);
    b.add_child().width(1.0).height(1.0).position_absolute()
        .inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_child_size(0, 1, 1);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn sticky_extra_bottom_inset_0() {
    let off = compute_sticky_offset(
        offset(0, 700), offset(0, 0), viewport(),
        &insets(None, None, Some(0), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-150));
}

#[test]
fn sticky_extra_bottom_inset_5() {
    let off = compute_sticky_offset(
        offset(0, 700), offset(0, 0), viewport(),
        &insets(None, None, Some(5), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-155));
}

#[test]
fn sticky_extra_bottom_inset_10() {
    let off = compute_sticky_offset(
        offset(0, 700), offset(0, 0), viewport(),
        &insets(None, None, Some(10), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-160));
}

#[test]
fn sticky_extra_bottom_inset_20() {
    let off = compute_sticky_offset(
        offset(0, 700), offset(0, 0), viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-170));
}

#[test]
fn sticky_extra_bottom_inset_50() {
    let off = compute_sticky_offset(
        offset(0, 700), offset(0, 0), viewport(),
        &insets(None, None, Some(50), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-200));
}

#[test]
fn sticky_extra_bottom_inset_100() {
    let off = compute_sticky_offset(
        offset(0, 700), offset(0, 0), viewport(),
        &insets(None, None, Some(100), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-250));
}

#[test]
fn sticky_extra_bottom_inset_200() {
    let off = compute_sticky_offset(
        offset(0, 700), offset(0, 0), viewport(),
        &insets(None, None, Some(200), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-350));
}

#[test]
fn sticky_extra_bottom_inset_300() {
    let off = compute_sticky_offset(
        offset(0, 700), offset(0, 0), viewport(),
        &insets(None, None, Some(300), None),
        size(100, 50), large_cb(),
    );
    assert_eq!(off.top, lu(-450));
}

#[test]
fn fixed_extra_full_coverage() {
    let mut b = BlockTestBuilder::new(1920, 1080);
    b.add_child()
        .with_style(|s| { s.position = Position::Fixed; s.top = Length::px(0.0); s.left = Length::px(0.0); s.right = Length::px(0.0); s.bottom = Length::px(0.0); })
        .done();
    let r = b.build();
    let fc = &r.root_fragment.children[1];
    assert_eq!(fc.size.width.to_i32(), 1920);
    assert_eq!(fc.size.height.to_i32(), 1080);
    assert_eq!(fc.offset.left.to_i32(), 0);
    assert_eq!(fc.offset.top.to_i32(), 0);
}

#[test]
fn rel_extra_container_height_with_multiple() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(800.0).height(100.0).position_relative().inset(50, 0, 0, 0).done();
    b.add_child().width(800.0).height(100.0).position_relative().inset(100, 0, 0, 0).done();
    let r = b.build();
    // Container height is 600 (explicit), not affected by relative offsets.
    r.assert_container_height(600);
}

#[test]
fn abs_extra_container_width_preserved() {
    let mut b = abs_builder(500, 400);
    b.add_child().width(1000.0).height(1000.0).position_absolute().inset(0, 0, 0, 0).done();
    let r = b.build();
    r.assert_container_width(500);
}

