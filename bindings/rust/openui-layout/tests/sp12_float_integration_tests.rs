//! SP12 B3: Float integration in block algorithm tests.
//!
//! Verifies that CSS floats are properly integrated into the block layout
//! algorithm, including positioning, exclusion tracking, clearance, and
//! content wrapping around floats.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{block_layout, ConstraintSpace};
use openui_style::*;

// ── Helpers ──────────────────────────────────────────────────────────────

fn lu(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

fn root_space(w: i32, h: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(lu(w), lu(h))
}

fn add_float(doc: &mut Document, parent: NodeId, width: f32, height: f32, float: Float) -> NodeId {
    let node = doc.create_node(ElementTag::Div);
    doc.node_mut(node).style.display = Display::Block;
    doc.node_mut(node).style.width = Length::px(width);
    doc.node_mut(node).style.height = Length::px(height);
    doc.node_mut(node).style.float = float;
    doc.append_child(parent, node);
    node
}

fn add_block(doc: &mut Document, parent: NodeId, height: f32) -> NodeId {
    let node = doc.create_node(ElementTag::Div);
    doc.node_mut(node).style.display = Display::Block;
    doc.node_mut(node).style.height = Length::px(height);
    doc.append_child(parent, node);
    node
}

fn add_block_with_width(doc: &mut Document, parent: NodeId, width: f32, height: f32) -> NodeId {
    let node = doc.create_node(ElementTag::Div);
    doc.node_mut(node).style.display = Display::Block;
    doc.node_mut(node).style.width = Length::px(width);
    doc.node_mut(node).style.height = Length::px(height);
    doc.append_child(parent, node);
    node
}

fn add_container(doc: &mut Document, parent: NodeId, width: f32) -> NodeId {
    let node = doc.create_node(ElementTag::Div);
    doc.node_mut(node).style.display = Display::Block;
    doc.node_mut(node).style.width = Length::px(width);
    doc.append_child(parent, node);
    node
}

// ── Test 1: Float left basic ─────────────────────────────────────────

#[test]
fn float_left_basic() {
    // A left-floated child is positioned at the left edge of the content area.
    // It does not advance the block offset.
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    assert_eq!(cont.children.len(), 1);
    let float_frag = &cont.children[0];

    // Positioned at top-left of content area.
    assert_eq!(float_frag.offset.left, lu(0));
    assert_eq!(float_frag.offset.top, lu(0));
    assert_eq!(float_frag.size.width, lu(200));
    assert_eq!(float_frag.size.height, lu(100));

    // Container height is 0 because floats don't contribute to intrinsic size.
    assert_eq!(cont.size.height, lu(0));
}

// ── Test 2: Float right basic ────────────────────────────────────────

#[test]
fn float_right_basic() {
    // A right-floated child is positioned at the right edge.
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Right);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let float_frag = &cont.children[0];
    // Right float: 800 - 200 = 600
    assert_eq!(float_frag.offset.left, lu(600));
    assert_eq!(float_frag.offset.top, lu(0));
    assert_eq!(float_frag.size.width, lu(200));
}

// ── Test 3: Float does not advance block offset ──────────────────────

#[test]
fn float_does_not_advance_block_offset() {
    // A block child after a float starts at the same block offset as
    // if the float weren't there (top of container).
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);
    add_block(&mut doc, container, 50.0);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    // The block child starts at block offset 0, same as the float.
    let block_child = &cont.children[1];
    assert_eq!(block_child.offset.top, lu(0));
}

// ── Test 4: Multiple left floats stacking ────────────────────────────

#[test]
fn multiple_left_floats_stack() {
    // Two left floats should stack horizontally.
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);
    add_float(&mut doc, container, 150.0, 80.0, Float::Left);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    assert_eq!(cont.children.len(), 2);
    let f1 = &cont.children[0];
    let f2 = &cont.children[1];

    assert_eq!(f1.offset.left, lu(0));
    // Second float placed after first: left=200
    assert_eq!(f2.offset.left, lu(200));
    assert_eq!(f2.size.width, lu(150));
}

// ── Test 5: Left and right float ─────────────────────────────────────

#[test]
fn left_and_right_float() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);
    add_float(&mut doc, container, 200.0, 80.0, Float::Right);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let left_float = &cont.children[0];
    let right_float = &cont.children[1];

    assert_eq!(left_float.offset.left, lu(0));
    assert_eq!(right_float.offset.left, lu(600)); // 800 - 200
}

// ── Test 6: Clear left ───────────────────────────────────────────────

#[test]
fn clear_left() {
    // A block child with clear:left should move below the left float.
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);

    let block = add_block(&mut doc, container, 50.0);
    doc.node_mut(block).style.clear = Clear::Left;

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let block_child = &cont.children[1];
    // Block should be below the float (at offset 100 or more).
    assert!(
        block_child.offset.top >= lu(100),
        "Block with clear:left should be below the left float, got top={}",
        block_child.offset.top.to_i32()
    );
}

// ── Test 7: Clear right ──────────────────────────────────────────────

#[test]
fn clear_right() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 150.0, Float::Right);

    let block = add_block(&mut doc, container, 50.0);
    doc.node_mut(block).style.clear = Clear::Right;

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let block_child = &cont.children[1];
    assert!(
        block_child.offset.top >= lu(150),
        "Block with clear:right should be below the right float, got top={}",
        block_child.offset.top.to_i32()
    );
}

// ── Test 8: Clear both ───────────────────────────────────────────────

#[test]
fn clear_both() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);
    add_float(&mut doc, container, 200.0, 150.0, Float::Right);

    let block = add_block(&mut doc, container, 50.0);
    doc.node_mut(block).style.clear = Clear::Both;

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let block_child = &cont.children[2];
    // Should clear past both floats — the tallest is 150.
    assert!(
        block_child.offset.top >= lu(150),
        "Block with clear:both should be below both floats, got top={}",
        block_child.offset.top.to_i32()
    );
}

// ── Test 9: Content wrapping — block shrinks beside left float ───────

#[test]
fn content_wraps_around_left_float() {
    // A non-float block child should have its available inline size reduced
    // by a left float, and be offset to the right.
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);
    add_block(&mut doc, container, 50.0);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let block_child = &cont.children[1];
    // The block child should be offset to the right by 200px.
    assert_eq!(block_child.offset.left, lu(200));
    // Its width should be reduced to 600px (800 - 200).
    assert_eq!(block_child.size.width, lu(600));
}

// ── Test 10: Content wrapping — block shrinks beside right float ─────

#[test]
fn content_wraps_around_right_float() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Right);
    add_block(&mut doc, container, 50.0);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let block_child = &cont.children[1];
    // Block child starts at left edge (no left float).
    assert_eq!(block_child.offset.left, lu(0));
    // Width reduced by right float: 800 - 200 = 600.
    assert_eq!(block_child.size.width, lu(600));
}

// ── Test 11: Content wrapping with both floats ───────────────────────

#[test]
fn content_wraps_between_both_floats() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);
    add_float(&mut doc, container, 150.0, 100.0, Float::Right);
    add_block(&mut doc, container, 50.0);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let block_child = &cont.children[2];
    // Offset by left float width.
    assert_eq!(block_child.offset.left, lu(200));
    // Width: 800 - 200 (left) - 150 (right) = 450.
    assert_eq!(block_child.size.width, lu(450));
}

// ── Test 12: Float + clear combination ───────────────────────────────

#[test]
fn float_then_clear_then_float() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);

    let cleared_block = add_block(&mut doc, container, 30.0);
    doc.node_mut(cleared_block).style.clear = Clear::Left;

    add_float(&mut doc, container, 300.0, 80.0, Float::Left);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    // Cleared block should be at or below 100.
    let cb = &cont.children[1];
    assert!(cb.offset.top >= lu(100));

    // The second float should be below the cleared block.
    let f2 = &cont.children[2];
    assert!(
        f2.offset.top >= lu(100),
        "Second float should be at or below the first float's bottom"
    );
}

// ── Test 13: Mixed float and non-float children ──────────────────────

#[test]
fn mixed_float_and_non_float_children() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_block(&mut doc, container, 40.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);
    add_block(&mut doc, container, 60.0);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    // First block at top, full width.
    let b1 = &cont.children[0];
    assert_eq!(b1.offset.top, lu(0));
    assert_eq!(b1.size.width, lu(800));

    // Float placed at block offset 40 (after first block).
    let f = &cont.children[1];
    assert_eq!(f.offset.top, lu(40));
    assert_eq!(f.offset.left, lu(0));

    // Second block also at block offset 40, wrapping around float.
    let b2 = &cont.children[2];
    assert_eq!(b2.offset.top, lu(40));
    assert_eq!(b2.offset.left, lu(200));
    assert_eq!(b2.size.width, lu(600));
}

// ── Test 14: Float with margins ──────────────────────────────────────

#[test]
fn float_left_with_margins() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);

    let float_node = doc.create_node(ElementTag::Div);
    doc.node_mut(float_node).style.display = Display::Block;
    doc.node_mut(float_node).style.width = Length::px(200.0);
    doc.node_mut(float_node).style.height = Length::px(100.0);
    doc.node_mut(float_node).style.float = Float::Left;
    doc.node_mut(float_node).style.margin_left = Length::px(10.0);
    doc.node_mut(float_node).style.margin_right = Length::px(20.0);
    doc.node_mut(float_node).style.margin_top = Length::px(5.0);
    doc.append_child(container, float_node);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];
    let f = &cont.children[0];

    // Border box positioned after left margin.
    assert_eq!(f.offset.left, lu(10));
    assert_eq!(f.offset.top, lu(5));
}

// ── Test 15: Block child below expired float gets full width ─────────

#[test]
fn block_after_expired_float_gets_full_width() {
    // Once a block child is positioned below a float's bottom edge,
    // it should get the full container width.
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 50.0, Float::Left);

    let cleared = add_block(&mut doc, container, 40.0);
    doc.node_mut(cleared).style.clear = Clear::Left;

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let block_child = &cont.children[1];
    // Below the float, should have full width.
    assert_eq!(block_child.offset.left, lu(0));
    assert_eq!(block_child.size.width, lu(800));
}

// ── Test 16: Float right with margins ────────────────────────────────

#[test]
fn float_right_with_margins() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);

    let float_node = doc.create_node(ElementTag::Div);
    doc.node_mut(float_node).style.display = Display::Block;
    doc.node_mut(float_node).style.width = Length::px(200.0);
    doc.node_mut(float_node).style.height = Length::px(100.0);
    doc.node_mut(float_node).style.float = Float::Right;
    doc.node_mut(float_node).style.margin_right = Length::px(30.0);
    doc.append_child(container, float_node);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];
    let f = &cont.children[0];

    // Right float: 800 - 30(right margin) - 200(width) = 570
    assert_eq!(f.offset.left, lu(570));
}

// ── Test 17: Container with only floats has zero height ──────────────

#[test]
fn only_floats_zero_height() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);
    add_float(&mut doc, container, 200.0, 150.0, Float::Right);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    // Floats don't contribute to intrinsic block size.
    assert_eq!(cont.size.height, lu(0));
}

// ── Test 18: Clear:none has no effect ────────────────────────────────

#[test]
fn clear_none_no_effect() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);

    let block = add_block(&mut doc, container, 50.0);
    doc.node_mut(block).style.clear = Clear::None;

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let block_child = &cont.children[1];
    // No clearance, block starts at top alongside the float.
    assert_eq!(block_child.offset.top, lu(0));
}

// ── Test 19: Clear:left doesn't clear right float ────────────────────

#[test]
fn clear_left_ignores_right_float() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Right);

    let block = add_block(&mut doc, container, 50.0);
    doc.node_mut(block).style.clear = Clear::Left;

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let block_child = &cont.children[1];
    // clear:left shouldn't move below a right float.
    assert_eq!(block_child.offset.top, lu(0));
}

// ── Test 20: Float in container with border+padding ──────────────────

#[test]
fn float_in_container_with_border_padding() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(800.0);
    doc.node_mut(container).style.padding_left = Length::px(20.0);
    doc.node_mut(container).style.padding_top = Length::px(10.0);
    doc.node_mut(container).style.border_left_width = 5;
    doc.node_mut(container).style.border_top_width = 5;
    doc.node_mut(container).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(container).style.border_top_style = BorderStyle::Solid;
    doc.append_child(vp, container);

    add_float(&mut doc, container, 200.0, 100.0, Float::Left);

    let space = root_space(1000, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];
    let f = &cont.children[0];

    // Float should be inside the content area: offset by border+padding.
    assert_eq!(f.offset.left, lu(25)); // 5 border + 20 padding
    assert_eq!(f.offset.top, lu(15));  // 5 border + 10 padding
}

// ── Test 21: Float drops below when no room beside existing float ────

#[test]
fn float_drops_below_when_no_room() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 400.0);
    add_float(&mut doc, container, 300.0, 100.0, Float::Left);
    // Second float needs 300px but only 100px available beside first.
    add_float(&mut doc, container, 300.0, 80.0, Float::Left);

    let space = root_space(400, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let f2 = &cont.children[1];
    // Should drop below the first float.
    assert!(
        f2.offset.top >= lu(100),
        "Second float should drop below first, got top={}",
        f2.offset.top.to_i32()
    );
    // And be at the left edge.
    assert_eq!(f2.offset.left, lu(0));
}

// ── Test 22: Multiple blocks wrapping around same float ──────────────

#[test]
fn multiple_blocks_wrap_around_same_float() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 200.0, Float::Left);
    add_block(&mut doc, container, 50.0);
    add_block(&mut doc, container, 50.0);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let b1 = &cont.children[1];
    let b2 = &cont.children[2];

    // Both blocks should be offset by the float.
    assert_eq!(b1.offset.left, lu(200));
    assert_eq!(b1.size.width, lu(600));

    assert_eq!(b2.offset.left, lu(200));
    assert_eq!(b2.size.width, lu(600));
}

// ── Test 23: Block child with fixed width beside float ───────────────

#[test]
fn fixed_width_block_beside_float() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 200.0, 100.0, Float::Left);
    add_block_with_width(&mut doc, container, 300.0, 50.0);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let block_child = &cont.children[1];
    // Fixed-width block keeps its width, but is offset past the float.
    assert_eq!(block_child.offset.left, lu(200));
    assert_eq!(block_child.size.width, lu(300));
}

// ── Test 24: Clear right with multiple right floats ──────────────────

#[test]
fn clear_right_clears_tallest_right_float() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);
    add_float(&mut doc, container, 100.0, 50.0, Float::Right);
    add_float(&mut doc, container, 100.0, 200.0, Float::Right);

    let block = add_block(&mut doc, container, 30.0);
    doc.node_mut(block).style.clear = Clear::Right;

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let block_child = &cont.children[2];
    // Should clear past the tallest right float (200).
    assert!(
        block_child.offset.top >= lu(200),
        "Should clear past tallest right float, got top={}",
        block_child.offset.top.to_i32()
    );
}

// ── Test 25: Float sizes are determined by layout ────────────────────

#[test]
fn float_child_laid_out_with_correct_size() {
    // Verify the float's fragment has the expected size from layout.
    let mut doc = Document::new();
    let vp = doc.root();

    let container = add_container(&mut doc, vp, 800.0);

    let float_node = doc.create_node(ElementTag::Div);
    doc.node_mut(float_node).style.display = Display::Block;
    doc.node_mut(float_node).style.width = Length::px(250.0);
    doc.node_mut(float_node).style.height = Length::px(75.0);
    doc.node_mut(float_node).style.float = Float::Left;
    doc.append_child(container, float_node);

    let space = root_space(800, 600);
    let frag = block_layout(&doc, vp, &space);
    let cont = &frag.children[0];

    let f = &cont.children[0];
    assert_eq!(f.size.width, lu(250));
    assert_eq!(f.size.height, lu(75));
}
