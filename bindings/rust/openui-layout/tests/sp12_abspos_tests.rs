//! SP12 D2 — Absolute and fixed positioning tests.
//!
//! Tests for `out_of_flow.rs` and the block.rs OOF integration.
//! Verifies CSS 2.1 §10.3.7 (horizontal) and §10.6.4 (vertical).

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{block_layout, ConstraintSpace};
use openui_style::{Display, Direction, Position, BorderStyle};

// ── Helpers ──────────────────────────────────────────────────────────

fn root_space(w: i32, h: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(LayoutUnit::from_i32(w), LayoutUnit::from_i32(h))
}

fn setup_abs_child(doc: &mut Document, parent: openui_dom::NodeId) -> openui_dom::NodeId {
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.position = Position::Absolute;
    doc.append_child(parent, child);
    child
}

// ── Test 1: basic absolute positioning with top/left ─────────────────

#[test]
fn abs_top_left() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::px(50.0);
    doc.node_mut(abs).style.left = Length::px(100.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(150.0);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    assert_eq!(container_frag.children.len(), 1);

    let abs_frag = &container_frag.children[0];
    assert_eq!(abs_frag.offset.left.to_i32(), 100);
    assert_eq!(abs_frag.offset.top.to_i32(), 50);
    assert_eq!(abs_frag.size.width.to_i32(), 200);
    assert_eq!(abs_frag.size.height.to_i32(), 150);
}

// ── Test 2: absolute with right/bottom ───────────────────────────────

#[test]
fn abs_right_bottom() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.right = Length::px(50.0);
    doc.node_mut(abs).style.bottom = Length::px(30.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(100.0);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    // left = CB_width - right - width = 800 - 50 - 200 = 550
    assert_eq!(abs_frag.offset.left.to_i32(), 550);
    // top = CB_height - bottom - height = 600 - 30 - 100 = 470
    assert_eq!(abs_frag.offset.top.to_i32(), 470);
    assert_eq!(abs_frag.size.width.to_i32(), 200);
    assert_eq!(abs_frag.size.height.to_i32(), 100);
}

// ── Test 3: absolute centering with auto margins ─────────────────────

#[test]
fn abs_centering_auto_margins() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.left = Length::px(0.0);
    doc.node_mut(abs).style.right = Length::px(0.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(100.0);
    doc.node_mut(abs).style.margin_left = Length::auto();
    doc.node_mut(abs).style.margin_right = Length::auto();

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    // Centered: (800 - 200) / 2 = 300
    assert_eq!(abs_frag.offset.left.to_i32(), 300);
    assert_eq!(abs_frag.size.width.to_i32(), 200);
}

// ── Test 4: vertical centering with auto margins ─────────────────────

#[test]
fn abs_vertical_centering_auto_margins() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::px(0.0);
    doc.node_mut(abs).style.bottom = Length::px(0.0);
    doc.node_mut(abs).style.height = Length::px(200.0);
    doc.node_mut(abs).style.width = Length::px(100.0);
    doc.node_mut(abs).style.margin_top = Length::auto();
    doc.node_mut(abs).style.margin_bottom = Length::auto();

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    // Vertically centered: (600 - 200) / 2 = 200
    assert_eq!(abs_frag.offset.top.to_i32(), 200);
    assert_eq!(abs_frag.size.height.to_i32(), 200);
}

// ── Test 5: over-constrained horizontal (LTR) ───────────────────────

#[test]
fn abs_overconstrained_ltr() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.left = Length::px(100.0);
    doc.node_mut(abs).style.right = Length::px(100.0);
    doc.node_mut(abs).style.width = Length::px(700.0);
    doc.node_mut(abs).style.height = Length::px(50.0);
    // Over-constrained: 100 + 700 + 100 = 900 > 800
    // LTR: right is ignored, left wins.

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    assert_eq!(abs_frag.offset.left.to_i32(), 100);
    assert_eq!(abs_frag.size.width.to_i32(), 700);
}

// ── Test 6: over-constrained horizontal (RTL) ───────────────────────

#[test]
fn abs_overconstrained_rtl() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.node_mut(container).style.direction = Direction::Rtl;
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.left = Length::px(100.0);
    doc.node_mut(abs).style.right = Length::px(100.0);
    doc.node_mut(abs).style.width = Length::px(700.0);
    doc.node_mut(abs).style.height = Length::px(50.0);
    doc.node_mut(abs).style.direction = Direction::Rtl;
    // Over-constrained RTL: left is ignored, right wins.
    // new_left = 800 - 100 - 700 = 0

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    // RTL: right=100, width=700 → left = 800 - 100 - 700 = 0
    assert_eq!(abs_frag.offset.left.to_i32(), 0);
    assert_eq!(abs_frag.size.width.to_i32(), 700);
}

// ── Test 7: auto width (shrink-to-fit) ──────────────────────────────

#[test]
fn abs_auto_width() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.left = Length::px(50.0);
    doc.node_mut(abs).style.right = Length::px(50.0);
    // width is auto → compute from constraint: 800 - 50 - 50 = 700
    doc.node_mut(abs).style.height = Length::px(100.0);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    assert_eq!(abs_frag.offset.left.to_i32(), 50);
    // Auto width with left+right specified → fills: 800 - 50 - 50 = 700
    assert_eq!(abs_frag.size.width.to_i32(), 700);
}

// ── Test 8: auto height ──────────────────────────────────────────────

#[test]
fn abs_auto_height() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::px(20.0);
    doc.node_mut(abs).style.bottom = Length::px(30.0);
    // height is auto → compute from constraint: 600 - 20 - 30 = 550
    doc.node_mut(abs).style.width = Length::px(200.0);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    assert_eq!(abs_frag.offset.top.to_i32(), 20);
    assert_eq!(abs_frag.size.height.to_i32(), 550);
}

// ── Test 9: percentage top/left/width/height ─────────────────────────

#[test]
fn abs_percentage_values() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(1000.0);
    doc.node_mut(container).style.height = Length::px(800.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::percent(10.0);     // 10% of 800 = 80
    doc.node_mut(abs).style.left = Length::percent(5.0);     // 5% of 1000 = 50
    doc.node_mut(abs).style.width = Length::percent(50.0);   // 50% of 1000 = 500
    doc.node_mut(abs).style.height = Length::percent(25.0);  // 25% of 800 = 200

    let space = root_space(1000, 800);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    assert_eq!(abs_frag.offset.left.to_i32(), 50);
    assert_eq!(abs_frag.offset.top.to_i32(), 80);
    assert_eq!(abs_frag.size.width.to_i32(), 500);
    assert_eq!(abs_frag.size.height.to_i32(), 200);
}

// ── Test 10: fixed positioning (same algorithm) ──────────────────────

#[test]
fn fixed_positioning_basic() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let fixed = doc.create_node(ElementTag::Div);
    doc.node_mut(fixed).style.display = Display::Block;
    doc.node_mut(fixed).style.position = Position::Fixed;
    doc.node_mut(fixed).style.top = Length::px(10.0);
    doc.node_mut(fixed).style.left = Length::px(20.0);
    doc.node_mut(fixed).style.width = Length::px(300.0);
    doc.node_mut(fixed).style.height = Length::px(200.0);
    doc.append_child(container, fixed);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);

    // Fixed positioning uses viewport as containing block, not the parent.
    // The fixed child bubbles up to the root fragment.
    let container_frag = &fragment.children[0];
    assert_eq!(container_frag.children.len(), 0);

    let fixed_frag = &fragment.children[1];
    assert_eq!(fixed_frag.offset.left.to_i32(), 20);
    assert_eq!(fixed_frag.offset.top.to_i32(), 10);
    assert_eq!(fixed_frag.size.width.to_i32(), 300);
    assert_eq!(fixed_frag.size.height.to_i32(), 200);
}

// ── Test 11: static position fallback ────────────────────────────────

#[test]
fn abs_static_position_fallback() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    // All insets auto → static position fallback
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.width = Length::px(100.0);
    doc.node_mut(abs).style.height = Length::px(50.0);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    // Static position: where it would be in normal flow (top of content area)
    assert_eq!(abs_frag.size.width.to_i32(), 100);
    assert_eq!(abs_frag.size.height.to_i32(), 50);
}

// ── Test 12: absolute with all four sides specified ──────────────────

#[test]
fn abs_all_sides_specified() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::px(50.0);
    doc.node_mut(abs).style.right = Length::px(100.0);
    doc.node_mut(abs).style.bottom = Length::px(100.0);
    doc.node_mut(abs).style.left = Length::px(50.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(150.0);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    // Over-constrained LTR: left wins (right ignored)
    assert_eq!(abs_frag.offset.left.to_i32(), 50);
    // Over-constrained vertical: top wins (bottom ignored)
    assert_eq!(abs_frag.offset.top.to_i32(), 50);
    assert_eq!(abs_frag.size.width.to_i32(), 200);
    assert_eq!(abs_frag.size.height.to_i32(), 150);
}

// ── Test 13: negative offsets ────────────────────────────────────────

#[test]
fn abs_negative_offsets() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::px(-20.0);
    doc.node_mut(abs).style.left = Length::px(-30.0);
    doc.node_mut(abs).style.width = Length::px(100.0);
    doc.node_mut(abs).style.height = Length::px(50.0);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    assert_eq!(abs_frag.offset.left.to_i32(), -30);
    assert_eq!(abs_frag.offset.top.to_i32(), -20);
}

// ── Test 14: nested absolute positioning ─────────────────────────────

#[test]
fn abs_nested() {
    let mut doc = Document::new();
    let vp = doc.root();

    // Outer container is the initial containing block
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(800.0);
    doc.node_mut(outer).style.height = Length::px(600.0);
    doc.node_mut(outer).style.position = Position::Relative; // establishes CB
    doc.append_child(vp, outer);

    // Absolute child inside outer
    let abs1 = setup_abs_child(&mut doc, outer);
    doc.node_mut(abs1).style.top = Length::px(50.0);
    doc.node_mut(abs1).style.left = Length::px(50.0);
    doc.node_mut(abs1).style.width = Length::px(400.0);
    doc.node_mut(abs1).style.height = Length::px(300.0);

    // Nested absolute inside abs1
    let abs2 = doc.create_node(ElementTag::Div);
    doc.node_mut(abs2).style.display = Display::Block;
    doc.node_mut(abs2).style.position = Position::Absolute;
    doc.node_mut(abs2).style.top = Length::px(10.0);
    doc.node_mut(abs2).style.left = Length::px(20.0);
    doc.node_mut(abs2).style.width = Length::px(100.0);
    doc.node_mut(abs2).style.height = Length::px(80.0);
    doc.append_child(abs1, abs2);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let outer_frag = &fragment.children[0];
    let abs1_frag = &outer_frag.children[0];

    assert_eq!(abs1_frag.offset.left.to_i32(), 50);
    assert_eq!(abs1_frag.offset.top.to_i32(), 50);
    assert_eq!(abs1_frag.size.width.to_i32(), 400);
    assert_eq!(abs1_frag.size.height.to_i32(), 300);

    // Nested absolute — positioned relative to abs1's content area
    assert_eq!(abs1_frag.children.len(), 1);
    let abs2_frag = &abs1_frag.children[0];
    assert_eq!(abs2_frag.offset.left.to_i32(), 20);
    assert_eq!(abs2_frag.offset.top.to_i32(), 10);
    assert_eq!(abs2_frag.size.width.to_i32(), 100);
    assert_eq!(abs2_frag.size.height.to_i32(), 80);
}

// ── Test 15: mixed absolute + normal flow children ───────────────────

#[test]
fn abs_mixed_with_normal_flow() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    // Normal flow child
    let normal = doc.create_node(ElementTag::Div);
    doc.node_mut(normal).style.display = Display::Block;
    doc.node_mut(normal).style.height = Length::px(100.0);
    doc.append_child(container, normal);

    // Absolute child
    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::px(200.0);
    doc.node_mut(abs).style.left = Length::px(300.0);
    doc.node_mut(abs).style.width = Length::px(150.0);
    doc.node_mut(abs).style.height = Length::px(75.0);

    // Another normal flow child
    let normal2 = doc.create_node(ElementTag::Div);
    doc.node_mut(normal2).style.display = Display::Block;
    doc.node_mut(normal2).style.height = Length::px(50.0);
    doc.append_child(container, normal2);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];

    // Should have: normal1 + normal2 + abs = 3
    assert_eq!(container_frag.children.len(), 3);

    // Normal flow children stack vertically
    let n1 = &container_frag.children[0];
    assert_eq!(n1.size.height.to_i32(), 100);
    assert_eq!(n1.offset.top.to_i32(), 0);

    let n2 = &container_frag.children[1];
    assert_eq!(n2.size.height.to_i32(), 50);
    assert_eq!(n2.offset.top.to_i32(), 100);

    // Absolute child doesn't affect normal flow
    let abs_frag = &container_frag.children[2];
    assert_eq!(abs_frag.offset.left.to_i32(), 300);
    assert_eq!(abs_frag.offset.top.to_i32(), 200);
    assert_eq!(abs_frag.size.width.to_i32(), 150);
    assert_eq!(abs_frag.size.height.to_i32(), 75);
}

// ── Test 16: auto left with right and width specified ────────────────

#[test]
fn abs_auto_left_with_right_width() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    // left is auto, right=100, width=200
    doc.node_mut(abs).style.right = Length::px(100.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(50.0);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    // left = 800 - 100 - 200 = 500
    assert_eq!(abs_frag.offset.left.to_i32(), 500);
    assert_eq!(abs_frag.size.width.to_i32(), 200);
}

// ── Test 17: auto top with bottom and height specified ───────────────

#[test]
fn abs_auto_top_with_bottom_height() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    // top is auto, bottom=50, height=200
    doc.node_mut(abs).style.bottom = Length::px(50.0);
    doc.node_mut(abs).style.height = Length::px(200.0);
    doc.node_mut(abs).style.width = Length::px(100.0);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    // top = 600 - 50 - 200 = 350
    assert_eq!(abs_frag.offset.top.to_i32(), 350);
    assert_eq!(abs_frag.size.height.to_i32(), 200);
}

// ── Test 18: display:none OOF should not produce fragment ────────────

#[test]
fn abs_display_none_excluded() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = doc.create_node(ElementTag::Div);
    doc.node_mut(abs).style.display = Display::None;
    doc.node_mut(abs).style.position = Position::Absolute;
    doc.node_mut(abs).style.top = Length::px(10.0);
    doc.node_mut(abs).style.left = Length::px(20.0);
    doc.append_child(container, abs);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];

    // display:none should produce no fragment even if position:absolute
    assert_eq!(container_frag.children.len(), 0);
}

// ── Test 19: absolute with border and padding ────────────────────────

#[test]
fn abs_with_border_padding() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.top = Length::px(10.0);
    doc.node_mut(abs).style.left = Length::px(20.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(100.0);
    doc.node_mut(abs).style.padding_top = Length::px(5.0);
    doc.node_mut(abs).style.padding_left = Length::px(10.0);
    doc.node_mut(abs).style.border_top_width = 2;
    doc.node_mut(abs).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(abs).style.border_left_width = 3;
    doc.node_mut(abs).style.border_left_style = BorderStyle::Solid;

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    assert_eq!(abs_frag.offset.left.to_i32(), 20);
    assert_eq!(abs_frag.offset.top.to_i32(), 10);
    // width includes content-box width + border + padding
    // Content: 200, padding-left:10, padding-right:0, border-left:3, border-right:0
    assert_eq!(abs_frag.size.width.to_i32(), 200 + 10 + 3);
    // Content: 100, padding-top:5, padding-bottom:0, border-top:2, border-bottom:0
    assert_eq!(abs_frag.size.height.to_i32(), 100 + 5 + 2);
}

// ── Test 20: auto margin with auto left absorbs remaining space ──────

#[test]
fn abs_auto_margin_left_absorbs() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.height = Length::px(600.0);
    doc.append_child(vp, container);

    let abs = setup_abs_child(&mut doc, container);
    doc.node_mut(abs).style.left = Length::px(0.0);
    doc.node_mut(abs).style.right = Length::px(0.0);
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(50.0);
    doc.node_mut(abs).style.margin_left = Length::auto();
    doc.node_mut(abs).style.margin_right = Length::px(100.0);

    let space = root_space(800, 600);
    let fragment = block_layout(&doc, vp, &space);
    let container_frag = &fragment.children[0];
    let abs_frag = &container_frag.children[0];

    // margin-left absorbs: 800 - 0 - 0 - 200 - 100 = 500
    assert_eq!(abs_frag.offset.left.to_i32(), 500);
    assert_eq!(abs_frag.margin.left.to_i32(), 500);
    assert_eq!(abs_frag.margin.right.to_i32(), 100);
}
