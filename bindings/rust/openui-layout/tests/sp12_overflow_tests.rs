//! Tests for SP12 F1: Overflow Handling.
//!
//! Covers fragment overflow fields/methods, block layout overflow computation,
//! overflow clipping in paint, and the `establishes_new_fc` helper.

use std::sync::Arc;

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{BoxStrut, LayoutUnit, Length, PhysicalOffset, PhysicalRect, PhysicalSize};
use openui_layout::{
    block_layout, establishes_new_fc, ConstraintSpace, Fragment, FragmentKind,
};
use openui_style::*;

// ── Helpers ──────────────────────────────────────────────────────────────

fn make_root_space(w: i32, h: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(LayoutUnit::from_i32(w), LayoutUnit::from_i32(h))
}

fn lu(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

/// Build a simple document with a container and N children of given heights.
/// Returns (doc, container_id, child_ids).
fn build_container_with_children(
    container_width: i32,
    container_height: Option<i32>,
    overflow: Overflow,
    child_heights: &[i32],
) -> (Document, NodeId, Vec<NodeId>) {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(container_width as f32);
    if let Some(h) = container_height {
        doc.node_mut(container).style.height = Length::px(h as f32);
    }
    doc.node_mut(container).style.overflow_x = overflow;
    doc.node_mut(container).style.overflow_y = overflow;
    doc.append_child(vp, container);

    let mut child_ids = Vec::new();
    for &ch in child_heights {
        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.width = Length::px(container_width as f32);
        doc.node_mut(child).style.height = Length::px(ch as f32);
        doc.append_child(container, child);
        child_ids.push(child);
    }

    (doc, container, child_ids)
}

// ═══════════════════════════════════════════════════════════════════════
// Test 1: Fragment overflow fields default to correct initial values
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn fragment_overflow_fields_initial_values() {
    let frag = Fragment::new_box(
        NodeId::NONE,
        PhysicalSize::new(lu(100), lu(50)),
    );
    assert!(frag.overflow_rect.is_none(), "overflow_rect should default to None");
    assert!(!frag.has_overflow_clip, "has_overflow_clip should default to false");
}

// ═══════════════════════════════════════════════════════════════════════
// Test 2: set_overflow_clip method
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn set_overflow_clip_toggles_flag() {
    let mut frag = Fragment::new_box(
        NodeId::NONE,
        PhysicalSize::new(lu(100), lu(50)),
    );
    assert!(!frag.has_overflow_clip);

    frag.set_overflow_clip(true);
    assert!(frag.has_overflow_clip);

    frag.set_overflow_clip(false);
    assert!(!frag.has_overflow_clip);
}

// ═══════════════════════════════════════════════════════════════════════
// Test 3: scrollable_overflow returns border-box when no overflow_rect
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn scrollable_overflow_returns_border_box_when_no_overflow() {
    let frag = Fragment::new_box(
        NodeId::NONE,
        PhysicalSize::new(lu(200), lu(100)),
    );
    let overflow = frag.scrollable_overflow();
    assert_eq!(overflow.x(), lu(0));
    assert_eq!(overflow.y(), lu(0));
    assert_eq!(overflow.width(), lu(200));
    assert_eq!(overflow.height(), lu(100));
}

// ═══════════════════════════════════════════════════════════════════════
// Test 4: scrollable_overflow returns overflow_rect when set
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn scrollable_overflow_returns_overflow_rect_when_set() {
    let mut frag = Fragment::new_box(
        NodeId::NONE,
        PhysicalSize::new(lu(200), lu(100)),
    );
    let big_rect = PhysicalRect::from_xywh(lu(0), lu(0), lu(300), lu(200));
    frag.overflow_rect = Some(big_rect);

    let overflow = frag.scrollable_overflow();
    assert_eq!(overflow.width(), lu(300));
    assert_eq!(overflow.height(), lu(200));
}

// ═══════════════════════════════════════════════════════════════════════
// Test 5: overflow:hidden sets has_overflow_clip in block layout
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_hidden_sets_clip_flag() {
    let (doc, container, _) = build_container_with_children(
        200, Some(100), Overflow::Hidden, &[50],
    );
    let space = make_root_space(400, 600);
    let frag = block_layout(&doc, doc.root(), &space);

    // The container is the first child of the viewport fragment.
    let container_frag = &frag.children[0];
    assert_eq!(container_frag.node_id, container);
    assert!(
        container_frag.has_overflow_clip,
        "overflow:hidden should set has_overflow_clip"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 6: overflow:visible does NOT set has_overflow_clip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_visible_no_clip_flag() {
    let (doc, container, _) = build_container_with_children(
        200, Some(100), Overflow::Visible, &[50],
    );
    let space = make_root_space(400, 600);
    let frag = block_layout(&doc, doc.root(), &space);

    let container_frag = &frag.children[0];
    assert_eq!(container_frag.node_id, container);
    assert!(
        !container_frag.has_overflow_clip,
        "overflow:visible should NOT set has_overflow_clip"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 7: overflow:scroll sets has_overflow_clip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_scroll_sets_clip_flag() {
    let (doc, container, _) = build_container_with_children(
        200, Some(100), Overflow::Scroll, &[50],
    );
    let space = make_root_space(400, 600);
    let frag = block_layout(&doc, doc.root(), &space);

    let container_frag = &frag.children[0];
    assert_eq!(container_frag.node_id, container);
    assert!(
        container_frag.has_overflow_clip,
        "overflow:scroll should set has_overflow_clip"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 8: overflow:clip sets has_overflow_clip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_clip_sets_clip_flag() {
    let (doc, container, _) = build_container_with_children(
        200, Some(100), Overflow::Clip, &[50],
    );
    let space = make_root_space(400, 600);
    let frag = block_layout(&doc, doc.root(), &space);

    let container_frag = &frag.children[0];
    assert_eq!(container_frag.node_id, container);
    assert!(
        container_frag.has_overflow_clip,
        "overflow:clip should set has_overflow_clip"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 9: overflow:auto sets has_overflow_clip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_auto_sets_clip_flag() {
    let (doc, container, _) = build_container_with_children(
        200, Some(100), Overflow::Auto, &[50],
    );
    let space = make_root_space(400, 600);
    let frag = block_layout(&doc, doc.root(), &space);

    let container_frag = &frag.children[0];
    assert_eq!(container_frag.node_id, container);
    assert!(
        container_frag.has_overflow_clip,
        "overflow:auto should set has_overflow_clip"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 10: overflow rect computed when children exceed container height
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_rect_computed_when_children_exceed_container() {
    // Container: 200×100, child: 200×200 → child overflows by 100px vertically.
    let (doc, container, _) = build_container_with_children(
        200, Some(100), Overflow::Hidden, &[200],
    );
    let space = make_root_space(400, 600);
    let frag = block_layout(&doc, doc.root(), &space);

    let container_frag = &frag.children[0];
    assert_eq!(container_frag.node_id, container);

    // The overflow rect should extend beyond the border-box.
    let overflow = container_frag.scrollable_overflow();
    assert!(
        overflow.height() > container_frag.size.height,
        "overflow rect height ({}) should exceed container height ({})",
        overflow.height().to_i32(),
        container_frag.size.height.to_i32(),
    );
    assert_eq!(
        overflow.height(),
        lu(200),
        "overflow rect should encompass the full child height"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 11: no overflow rect when children fit within container
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn no_overflow_rect_when_children_fit() {
    // Container: 200×auto, child: 200×50 → no overflow.
    let (doc, container, _) = build_container_with_children(
        200, None, Overflow::Visible, &[50],
    );
    let space = make_root_space(400, 600);
    let frag = block_layout(&doc, doc.root(), &space);

    let container_frag = &frag.children[0];
    assert_eq!(container_frag.node_id, container);
    assert!(
        container_frag.overflow_rect.is_none(),
        "no overflow rect when children fit (auto height expands to contain)"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 12: nested overflow containers — inner clips propagation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn nested_overflow_containers() {
    let mut doc = Document::new();
    let vp = doc.root();

    // Outer container: 200×100, overflow:visible
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(200.0);
    doc.node_mut(outer).style.height = Length::px(100.0);
    doc.append_child(vp, outer);

    // Inner container: 200×50, overflow:hidden
    let inner = doc.create_node(ElementTag::Div);
    doc.node_mut(inner).style.display = Display::Block;
    doc.node_mut(inner).style.width = Length::px(200.0);
    doc.node_mut(inner).style.height = Length::px(50.0);
    doc.node_mut(inner).style.overflow_x = Overflow::Hidden;
    doc.node_mut(inner).style.overflow_y = Overflow::Hidden;
    doc.append_child(outer, inner);

    // Child inside inner: 200×200 (overflows inner)
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(200.0);
    doc.node_mut(child).style.height = Length::px(200.0);
    doc.append_child(inner, child);

    let space = make_root_space(400, 600);
    let frag = block_layout(&doc, doc.root(), &space);

    // Outer is first child of viewport
    let outer_frag = &frag.children[0];
    assert_eq!(outer_frag.node_id, outer);
    assert!(!outer_frag.has_overflow_clip, "outer overflow:visible → no clip");

    // Inner is first child of outer
    let inner_frag = &outer_frag.children[0];
    assert_eq!(inner_frag.node_id, inner);
    assert!(inner_frag.has_overflow_clip, "inner overflow:hidden → clip");

    // Inner's overflow rect should extend beyond its border-box.
    let inner_overflow = inner_frag.scrollable_overflow();
    assert_eq!(inner_overflow.height(), lu(200));

    // Outer should NOT have overflow from inner's clipped children,
    // because inner clips them. Outer's overflow should not extend
    // beyond its border-box (inner itself fits in outer at 50px < 100px).
    assert!(
        outer_frag.overflow_rect.is_none(),
        "outer should not inherit clipped overflow from inner"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 13: overflow with padding and border
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_with_padding_and_border() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(200.0);
    doc.node_mut(container).style.height = Length::px(100.0);
    doc.node_mut(container).style.overflow_x = Overflow::Hidden;
    doc.node_mut(container).style.overflow_y = Overflow::Hidden;
    doc.node_mut(container).style.padding_top = Length::px(10.0);
    doc.node_mut(container).style.padding_bottom = Length::px(10.0);
    doc.node_mut(container).style.border_top_width = 5;
    doc.node_mut(container).style.border_bottom_width = 5;
    doc.node_mut(container).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(container).style.border_bottom_style = BorderStyle::Solid;
    doc.append_child(vp, container);

    // Child that overflows the content box but fits the border-box
    // Container border-box height = 100px, content area = 100 - 10 - 10 - 5 - 5 = 70px
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(180.0);
    doc.node_mut(child).style.height = Length::px(60.0);
    doc.append_child(container, child);

    let space = make_root_space(400, 600);
    let frag = block_layout(&doc, doc.root(), &space);

    let container_frag = &frag.children[0];
    assert_eq!(container_frag.node_id, container);
    assert!(container_frag.has_overflow_clip, "should clip with overflow:hidden + padding/border");

    // Child fits within border-box (60px child < 70px content area).
    // No overflow rect needed.
    assert!(
        container_frag.overflow_rect.is_none()
            || container_frag.scrollable_overflow().height() <= container_frag.size.height,
        "child should fit within the container border-box"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 14: establishes_new_fc — various style configurations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn establishes_new_fc_overflow_hidden() {
    let mut s = ComputedStyle::initial();
    assert!(!establishes_new_fc(&s), "default should not establish FC");

    s.overflow_x = Overflow::Hidden;
    assert!(establishes_new_fc(&s), "overflow:hidden should establish FC");
}

#[test]
fn establishes_new_fc_overflow_scroll() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    assert!(establishes_new_fc(&s), "overflow:scroll should establish FC");
}

#[test]
fn establishes_new_fc_flex() {
    let mut s = ComputedStyle::initial();
    s.display = Display::Flex;
    assert!(establishes_new_fc(&s), "display:flex should establish FC");
}

#[test]
fn establishes_new_fc_float() {
    let mut s = ComputedStyle::initial();
    s.float = Float::Left;
    assert!(establishes_new_fc(&s), "float:left should establish FC");
}

#[test]
fn establishes_new_fc_absolute_position() {
    let mut s = ComputedStyle::initial();
    s.position = Position::Absolute;
    assert!(establishes_new_fc(&s), "position:absolute should establish FC");
}

// ═══════════════════════════════════════════════════════════════════════
// Test 15: PhysicalRect::unite computes correct union
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn physical_rect_unite() {
    let a = PhysicalRect::from_xywh(lu(0), lu(0), lu(100), lu(50));
    let b = PhysicalRect::from_xywh(lu(50), lu(30), lu(100), lu(80));
    let u = a.unite(&b);

    assert_eq!(u.x(), lu(0));
    assert_eq!(u.y(), lu(0));
    assert_eq!(u.width(), lu(150));
    assert_eq!(u.height(), lu(110));
}

#[test]
fn physical_rect_unite_empty() {
    let a = PhysicalRect::from_xywh(lu(10), lu(20), lu(100), lu(50));
    let empty = PhysicalRect::default();

    assert_eq!(a.unite(&empty), a, "unite with empty should return self");
    assert_eq!(empty.unite(&a), a, "empty united with other should return other");
}

// ═══════════════════════════════════════════════════════════════════════
// Test 16: border_box_rect method
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn border_box_rect_method() {
    let frag = Fragment::new_box(
        NodeId::NONE,
        PhysicalSize::new(lu(300), lu(150)),
    );
    let rect = frag.border_box_rect();
    assert_eq!(rect.x(), lu(0));
    assert_eq!(rect.y(), lu(0));
    assert_eq!(rect.width(), lu(300));
    assert_eq!(rect.height(), lu(150));
}

// ═══════════════════════════════════════════════════════════════════════
// Test 17: multiple children overflow accumulation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_rect_multiple_children() {
    // Container: 200×100, two children: 200×80 each → total 160px, overflows by 60px.
    let (doc, container, _) = build_container_with_children(
        200, Some(100), Overflow::Hidden, &[80, 80],
    );
    let space = make_root_space(400, 600);
    let frag = block_layout(&doc, doc.root(), &space);

    let container_frag = &frag.children[0];
    assert_eq!(container_frag.node_id, container);

    let overflow = container_frag.scrollable_overflow();
    assert_eq!(
        overflow.height(),
        lu(160),
        "overflow rect should span all children (80+80=160)"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 18: overflow_x only (asymmetric overflow)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_x_only_sets_clip() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(200.0);
    doc.node_mut(container).style.height = Length::px(100.0);
    doc.node_mut(container).style.overflow_x = Overflow::Hidden;
    // overflow_y stays visible
    doc.append_child(vp, container);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(200.0);
    doc.node_mut(child).style.height = Length::px(50.0);
    doc.append_child(container, child);

    let space = make_root_space(400, 600);
    let frag = block_layout(&doc, doc.root(), &space);

    let container_frag = &frag.children[0];
    assert!(
        container_frag.has_overflow_clip,
        "overflow_x:hidden alone should set has_overflow_clip"
    );
}
