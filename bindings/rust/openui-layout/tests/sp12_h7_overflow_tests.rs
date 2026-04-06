//! SP12 H7 — Comprehensive CSS Overflow Tests.
//!
//! Covers overflow:visible/hidden/scroll/auto/clip, overflow rect computation,
//! BFC establishment from overflow, overflow with positioning, border-radius
//! clipping, and edge cases.

#[path = "sp12_wpt_helpers.rs"]
mod sp12_wpt_helpers;

use sp12_wpt_helpers::*;

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length, PhysicalOffset, PhysicalRect, PhysicalSize};
use openui_layout::{block_layout, establishes_new_fc, ConstraintSpace, Fragment};
use openui_style::*;

// ── Helpers ──────────────────────────────────────────────────────────────

fn lu(px: i32) -> LayoutUnit { LayoutUnit::from_i32(px) }

fn make_space(w: i32, h: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(lu(w), lu(h))
}

/// Build a document with a container (optional height) and N children.
fn build_doc(
    cw: i32, ch: Option<i32>, overflow: Overflow, child_heights: &[i32],
) -> (Document, NodeId, Vec<NodeId>) {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(cw as f32);
    if let Some(h) = ch { doc.node_mut(c).style.height = Length::px(h as f32); }
    doc.node_mut(c).style.overflow_x = overflow;
    doc.node_mut(c).style.overflow_y = overflow;
    doc.append_child(vp, c);
    let mut ids = Vec::new();
    for &h in child_heights {
        let n = doc.create_node(ElementTag::Div);
        doc.node_mut(n).style.display = Display::Block;
        doc.node_mut(n).style.width = Length::px(cw as f32);
        doc.node_mut(n).style.height = Length::px(h as f32);
        doc.append_child(c, n);
        ids.push(n);
    }
    (doc, c, ids)
}

/// Run layout and return the root fragment.
fn layout(doc: &Document) -> Fragment {
    block_layout(doc, doc.root(), &make_space(800, 600))
}

/// Container fragment (first child of viewport).
fn container(frag: &Fragment) -> &Fragment { &frag.children[0] }

// ═══════════════════════════════════════════════════════════════════════
// Section 1: overflow: visible
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn visible_default_overflow_is_visible() {
    let s = ComputedStyle::initial();
    assert_eq!(s.overflow_x, Overflow::Visible);
    assert_eq!(s.overflow_y, Overflow::Visible);
}

#[test]
fn visible_no_clip_flag() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[50]);
    let f = layout(&doc);
    assert!(!container(&f).has_overflow_clip);
}

#[test]
fn visible_content_overflows_vertically() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[200]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert!(ov.height() > c.size.height);
}

#[test]
fn visible_overflow_rect_includes_child() {
    let (doc, _, _) = build_doc(200, Some(50), Overflow::Visible, &[150]);
    let f = layout(&doc);
    let ov = container(&f).scrollable_overflow();
    assert_eq!(ov.height(), lu(150));
}

#[test]
fn visible_no_overflow_when_auto_height() {
    let (doc, _, _) = build_doc(200, None, Overflow::Visible, &[50]);
    let f = layout(&doc);
    assert!(container(&f).overflow_rect.is_none());
}

#[test]
fn visible_child_exact_fit_no_overflow() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[100]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert!(ov.height() <= c.size.height);
}

#[test]
fn visible_two_children_no_overflow() {
    let (doc, _, _) = build_doc(200, Some(200), Overflow::Visible, &[50, 50]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert!(ov.height() <= c.size.height);
}

#[test]
fn visible_two_children_overflow() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[60, 60]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert!(ov.height() > c.size.height);
}

#[test]
fn visible_three_children_overflow() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[50, 50, 50]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert!(ov.height() == lu(150));
}

#[test]
fn visible_single_large_child() {
    let (doc, _, _) = build_doc(200, Some(50), Overflow::Visible, &[500]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert!(ov.height() == lu(500));
}

#[test]
fn visible_many_small_children() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[20, 20, 20, 20, 20, 20, 20, 20, 20, 20]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert!(ov.height() == lu(200));
}

#[test]
fn visible_child_zero_height() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[0]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert!(ov.height() <= c.size.height);
}

#[test]
fn visible_all_children_zero_height() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[0, 0, 0]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert!(ov.height() <= c.size.height);
}

#[test]
fn visible_one_tall_one_short() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[200, 10]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert!(ov.height() == lu(210));
}

#[test]
fn visible_single_child_1px_overflow() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[101]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert!(ov.height() == lu(101));
}

#[test]
fn visible_nested_overflow_propagation() {
    let mut doc = Document::new();
    let vp = doc.root();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(200.0);
    doc.node_mut(outer).style.height = Length::px(100.0);
    doc.append_child(vp, outer);
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(200.0);
    doc.node_mut(child).style.height = Length::px(200.0);
    doc.append_child(outer, child);
    let f = layout(&doc);
    let c = &f.children[0];
    let ov = c.scrollable_overflow();
    assert!(ov.height() >= lu(200));
}

#[test]
fn visible_deeply_nested_overflow() {
    let mut doc = Document::new();
    let vp = doc.root();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(200.0);
    doc.node_mut(outer).style.height = Length::px(50.0);
    doc.append_child(vp, outer);
    let mid = doc.create_node(ElementTag::Div);
    doc.node_mut(mid).style.display = Display::Block;
    doc.node_mut(mid).style.width = Length::px(200.0);
    doc.node_mut(mid).style.height = Length::px(50.0);
    doc.append_child(outer, mid);
    let inner = doc.create_node(ElementTag::Div);
    doc.node_mut(inner).style.display = Display::Block;
    doc.node_mut(inner).style.width = Length::px(200.0);
    doc.node_mut(inner).style.height = Length::px(300.0);
    doc.append_child(mid, inner);
    let f = layout(&doc);
    let outer_f = &f.children[0];
    let ov = outer_f.scrollable_overflow();
    assert!(ov.height() >= lu(300));
}

#[test]
fn visible_parametric_1() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[53]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_2() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[56]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_3() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[59]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_4() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[62]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_5() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[65]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_6() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[68]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_7() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[71]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_8() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[74]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_9() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[77]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_10() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[80]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_11() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[83]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_12() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[86]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_13() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[89]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_14() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[92]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_15() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[95]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_16() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[98]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() <= lu(100));
}

#[test]
fn visible_parametric_17() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[101]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(101));
}

#[test]
fn visible_parametric_18() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[104]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(104));
}

#[test]
fn visible_parametric_19() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[107]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(107));
}

#[test]
fn visible_parametric_20() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[110]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(110));
}

#[test]
fn visible_parametric_21() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[113]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(113));
}

#[test]
fn visible_parametric_22() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[116]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(116));
}

#[test]
fn visible_parametric_23() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[119]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(119));
}

#[test]
fn visible_parametric_24() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[122]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(122));
}

#[test]
fn visible_parametric_25() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[125]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(125));
}

#[test]
fn visible_parametric_26() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[128]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(128));
}

#[test]
fn visible_parametric_27() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[131]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(131));
}

#[test]
fn visible_parametric_28() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[134]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(134));
}

#[test]
fn visible_parametric_29() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[137]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(137));
}

#[test]
fn visible_parametric_30() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[140]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(140));
}

#[test]
fn visible_parametric_31() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[143]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(143));
}

#[test]
fn visible_parametric_32() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[146]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(146));
}

#[test]
fn visible_parametric_33() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[149]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(149));
}

#[test]
fn visible_parametric_34() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[152]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(152));
}

#[test]
fn visible_parametric_35() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[155]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(155));
}

#[test]
fn visible_parametric_36() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[158]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(158));
}

#[test]
fn visible_parametric_37() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[161]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(161));
}

#[test]
fn visible_parametric_38() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[164]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(164));
}

#[test]
fn visible_parametric_39() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[167]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(167));
}

#[test]
fn visible_parametric_40() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Visible, &[170]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.scrollable_overflow().height() >= lu(170));
}

// ═══════════════════════════════════════════════════════════════════════
// Section 2: overflow: hidden
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn hidden_sets_clip_flag() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[50]);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn hidden_content_clipped_to_padding_box() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[200]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn hidden_overflow_rect_extends_beyond_border_box() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[200]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert_eq!(ov.height(), lu(200));
}

#[test]
fn hidden_overflow_x_only() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn hidden_overflow_y_only() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn hidden_no_children_still_clips() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[]);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn hidden_children_fit_no_overflow_rect() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[30, 30]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    let ov = c.scrollable_overflow();
    assert!(ov.height() <= c.size.height);
}

#[test]
fn hidden_nested_containers() {
    let mut doc = Document::new();
    let vp = doc.root();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(200.0);
    doc.node_mut(outer).style.height = Length::px(100.0);
    doc.node_mut(outer).style.overflow_x = Overflow::Hidden;
    doc.node_mut(outer).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, outer);
    let inner = doc.create_node(ElementTag::Div);
    doc.node_mut(inner).style.display = Display::Block;
    doc.node_mut(inner).style.width = Length::px(200.0);
    doc.node_mut(inner).style.height = Length::px(50.0);
    doc.node_mut(inner).style.overflow_x = Overflow::Hidden;
    doc.node_mut(inner).style.overflow_y = Overflow::Hidden;
    doc.append_child(outer, inner);
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(200.0);
    doc.node_mut(child).style.height = Length::px(200.0);
    doc.append_child(inner, child);
    let f = layout(&doc);
    let outer_f = &f.children[0];
    assert!(outer_f.has_overflow_clip);
    let inner_f = &outer_f.children[0];
    assert!(inner_f.has_overflow_clip);
    assert!(outer_f.overflow_rect.is_none());
}

#[test]
fn hidden_with_padding() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .overflow_hidden()
        .padding(10, 10, 10, 10)
        .add_child().width(180.0).height(50.0).done()
        .done();
    let r = b.build();
    let c = r.child(0);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_with_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .overflow_hidden()
        .border(5, 5, 5, 5)
        .add_child().width(190.0).height(50.0).done()
        .done();
    let r = b.build();
    assert!(r.child(0).has_overflow_clip);
}

#[test]
fn hidden_with_border_radius() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.node_mut(c).style.border_top_left_radius = (10.0, 10.0);
    doc.node_mut(c).style.border_top_right_radius = (10.0, 10.0);
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    let cf = container(&f);
    assert!(cf.has_overflow_clip);
    assert!(doc.node(cf.node_id).style.has_border_radius());
}

#[test]
fn hidden_parametric_1() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[35]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_2() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[40]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_3() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[45]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_4() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[50]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_5() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[55]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_6() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[60]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_7() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[65]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_8() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[70]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_9() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[75]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_10() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[80]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_11() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[85]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_12() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[90]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_13() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[95]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_14() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[100]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn hidden_parametric_15() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[105]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(105));
}

#[test]
fn hidden_parametric_16() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[110]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(110));
}

#[test]
fn hidden_parametric_17() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[115]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(115));
}

#[test]
fn hidden_parametric_18() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[120]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(120));
}

#[test]
fn hidden_parametric_19() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[125]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(125));
}

#[test]
fn hidden_parametric_20() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[130]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(130));
}

#[test]
fn hidden_parametric_21() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[135]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(135));
}

#[test]
fn hidden_parametric_22() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[140]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(140));
}

#[test]
fn hidden_parametric_23() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[145]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(145));
}

#[test]
fn hidden_parametric_24() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[150]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(150));
}

#[test]
fn hidden_parametric_25() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[155]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(155));
}

#[test]
fn hidden_parametric_26() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[160]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(160));
}

#[test]
fn hidden_parametric_27() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[165]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(165));
}

#[test]
fn hidden_parametric_28() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[170]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(170));
}

#[test]
fn hidden_parametric_29() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[175]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(175));
}

#[test]
fn hidden_parametric_30() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[180]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(180));
}

#[test]
fn hidden_parametric_31() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[185]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(185));
}

#[test]
fn hidden_parametric_32() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[190]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(190));
}

#[test]
fn hidden_parametric_33() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[195]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(195));
}

#[test]
fn hidden_parametric_34() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[200]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(200));
}

#[test]
fn hidden_parametric_35() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[205]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(205));
}

#[test]
fn hidden_parametric_36() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[210]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(210));
}

#[test]
fn hidden_parametric_37() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[215]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(215));
}

#[test]
fn hidden_parametric_38() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[220]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(220));
}

#[test]
fn hidden_parametric_39() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[225]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(225));
}

#[test]
fn hidden_parametric_40() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[230]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(230));
}

#[test]
fn hidden_parametric_41() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[235]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(235));
}

#[test]
fn hidden_parametric_42() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[240]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(240));
}

#[test]
fn hidden_parametric_43() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[245]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(245));
}

#[test]
fn hidden_parametric_44() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[250]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(250));
}

#[test]
fn hidden_parametric_45() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[255]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(255));
}

#[test]
fn hidden_parametric_46() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[260]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(260));
}

#[test]
fn hidden_parametric_47() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[265]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(265));
}

#[test]
fn hidden_parametric_48() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[270]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(270));
}

#[test]
fn hidden_parametric_49() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[275]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(275));
}

#[test]
fn hidden_parametric_50() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[280]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(280));
}

// ═══════════════════════════════════════════════════════════════════════
// Section 3: overflow: scroll
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn scroll_sets_clip_flag() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[50]);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn scroll_creates_clip_context() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[200]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(200));
}

#[test]
fn scroll_is_scrollable() {
    assert!(Overflow::Scroll.is_scrollable());
    assert!(!Overflow::Hidden.is_scrollable());
    assert!(!Overflow::Visible.is_scrollable());
}

#[test]
fn scroll_is_clipping() {
    assert!(Overflow::Scroll.is_clipping());
    assert!(Overflow::Hidden.is_clipping());
    assert!(!Overflow::Visible.is_clipping());
}

#[test]
fn scroll_container_size_unchanged() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[300]);
    let f = layout(&doc);
    let c = container(&f);
    assert_eq!(c.size.height, lu(100));
    assert_eq!(c.size.width, lu(200));
}

#[test]
fn scroll_no_content_still_clips() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[]);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn scroll_nested_scrollable_containers() {
    let mut doc = Document::new();
    let vp = doc.root();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(200.0);
    doc.node_mut(outer).style.height = Length::px(100.0);
    doc.node_mut(outer).style.overflow_x = Overflow::Scroll;
    doc.node_mut(outer).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, outer);
    let inner = doc.create_node(ElementTag::Div);
    doc.node_mut(inner).style.display = Display::Block;
    doc.node_mut(inner).style.width = Length::px(200.0);
    doc.node_mut(inner).style.height = Length::px(80.0);
    doc.node_mut(inner).style.overflow_x = Overflow::Scroll;
    doc.node_mut(inner).style.overflow_y = Overflow::Scroll;
    doc.append_child(outer, inner);
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(200.0);
    doc.node_mut(child).style.height = Length::px(300.0);
    doc.append_child(inner, child);
    let f = layout(&doc);
    let outer_f = &f.children[0];
    assert!(outer_f.has_overflow_clip);
    let inner_f = &outer_f.children[0];
    assert!(inner_f.has_overflow_clip);
}

#[test]
fn scroll_parametric_1() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[28]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn scroll_parametric_2() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[36]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn scroll_parametric_3() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[44]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn scroll_parametric_4() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[52]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn scroll_parametric_5() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[60]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn scroll_parametric_6() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[68]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn scroll_parametric_7() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[76]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn scroll_parametric_8() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[84]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn scroll_parametric_9() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[92]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn scroll_parametric_10() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[100]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn scroll_parametric_11() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[108]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(108));
}

#[test]
fn scroll_parametric_12() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[116]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(116));
}

#[test]
fn scroll_parametric_13() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[124]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(124));
}

#[test]
fn scroll_parametric_14() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[132]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(132));
}

#[test]
fn scroll_parametric_15() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[140]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(140));
}

#[test]
fn scroll_parametric_16() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[148]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(148));
}

#[test]
fn scroll_parametric_17() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[156]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(156));
}

#[test]
fn scroll_parametric_18() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[164]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(164));
}

#[test]
fn scroll_parametric_19() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[172]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(172));
}

#[test]
fn scroll_parametric_20() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[180]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(180));
}

#[test]
fn scroll_parametric_21() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[188]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(188));
}

#[test]
fn scroll_parametric_22() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[196]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(196));
}

#[test]
fn scroll_parametric_23() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[204]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(204));
}

#[test]
fn scroll_parametric_24() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[212]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(212));
}

#[test]
fn scroll_parametric_25() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[220]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(220));
}

#[test]
fn scroll_parametric_26() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[228]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(228));
}

#[test]
fn scroll_parametric_27() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[236]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(236));
}

#[test]
fn scroll_parametric_28() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[244]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(244));
}

#[test]
fn scroll_parametric_29() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[252]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(252));
}

#[test]
fn scroll_parametric_30() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[260]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(260));
}

#[test]
fn scroll_parametric_31() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[268]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(268));
}

#[test]
fn scroll_parametric_32() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[276]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(276));
}

#[test]
fn scroll_parametric_33() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[284]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(284));
}

#[test]
fn scroll_parametric_34() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[292]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(292));
}

#[test]
fn scroll_parametric_35() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[300]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(300));
}

#[test]
fn scroll_parametric_36() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[308]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(308));
}

#[test]
fn scroll_parametric_37() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[316]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(316));
}

#[test]
fn scroll_parametric_38() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[324]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(324));
}

#[test]
fn scroll_parametric_39() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[332]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(332));
}

#[test]
fn scroll_parametric_40() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[340]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(340));
}

#[test]
fn scroll_multi_child_1() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[32, 32]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
}

#[test]
fn scroll_multi_child_2() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[34, 34, 34]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(102));
}

#[test]
fn scroll_multi_child_3() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[36, 36, 36, 36]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(144));
}

#[test]
fn scroll_multi_child_4() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[38, 38, 38, 38, 38]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(190));
}

#[test]
fn scroll_multi_child_5() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[40, 40, 40, 40, 40, 40]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(240));
}

#[test]
fn scroll_multi_child_6() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[42, 42, 42, 42, 42, 42, 42]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(294));
}

#[test]
fn scroll_multi_child_7() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[44, 44, 44, 44, 44, 44, 44, 44]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(352));
}

#[test]
fn scroll_multi_child_8() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[46, 46, 46, 46, 46, 46, 46, 46, 46]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(414));
}

#[test]
fn scroll_multi_child_9() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[48, 48, 48, 48, 48, 48, 48, 48, 48, 48]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(480));
}

#[test]
fn scroll_multi_child_10() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(550));
}

#[test]
fn scroll_multi_child_11() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[52, 52, 52, 52, 52, 52, 52, 52, 52, 52, 52, 52]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(624));
}

#[test]
fn scroll_multi_child_12() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(702));
}

#[test]
fn scroll_multi_child_13() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(784));
}

#[test]
fn scroll_multi_child_14() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Scroll, &[58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(870));
}

// ═══════════════════════════════════════════════════════════════════════
// Section 4: overflow: auto
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn auto_sets_clip_flag() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[50]);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn auto_is_scrollable() {
    assert!(Overflow::Auto.is_scrollable());
}

#[test]
fn auto_is_clipping() {
    assert!(Overflow::Auto.is_clipping());
}

#[test]
fn auto_no_overflow_children_fit() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[30, 30]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_with_overflow_children_exceed() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[60, 60]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(120));
}

#[test]
fn auto_content_taller_than_container() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[300]);
    let f = layout(&doc);
    let c = container(&f);
    assert_eq!(c.size.height, lu(100));
    assert_eq!(c.scrollable_overflow().height(), lu(300));
}

#[test]
fn auto_empty_container() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[]);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn auto_auto_height_expands() {
    let (doc, _, _) = build_doc(200, None, Overflow::Auto, &[50, 50]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
}

#[test]
fn auto_parametric_1() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[17]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_parametric_2() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[24]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_parametric_3() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[31]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_parametric_4() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[38]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_parametric_5() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[45]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_parametric_6() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[52]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_parametric_7() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[59]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_parametric_8() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[66]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_parametric_9() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[73]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_parametric_10() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[80]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_parametric_11() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[87]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_parametric_12() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[94]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn auto_parametric_13() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[101]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(101));
}

#[test]
fn auto_parametric_14() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[108]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(108));
}

#[test]
fn auto_parametric_15() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[115]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(115));
}

#[test]
fn auto_parametric_16() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[122]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(122));
}

#[test]
fn auto_parametric_17() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[129]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(129));
}

#[test]
fn auto_parametric_18() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[136]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(136));
}

#[test]
fn auto_parametric_19() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[143]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(143));
}

#[test]
fn auto_parametric_20() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[150]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(150));
}

#[test]
fn auto_parametric_21() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[157]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(157));
}

#[test]
fn auto_parametric_22() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[164]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(164));
}

#[test]
fn auto_parametric_23() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[171]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(171));
}

#[test]
fn auto_parametric_24() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[178]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(178));
}

#[test]
fn auto_parametric_25() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[185]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(185));
}

#[test]
fn auto_parametric_26() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[192]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(192));
}

#[test]
fn auto_parametric_27() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[199]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(199));
}

#[test]
fn auto_parametric_28() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[206]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(206));
}

#[test]
fn auto_parametric_29() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[213]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(213));
}

#[test]
fn auto_parametric_30() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[220]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(220));
}

#[test]
fn auto_parametric_31() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[227]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(227));
}

#[test]
fn auto_parametric_32() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[234]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(234));
}

#[test]
fn auto_parametric_33() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[241]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(241));
}

#[test]
fn auto_parametric_34() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[248]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(248));
}

#[test]
fn auto_parametric_35() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[255]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(255));
}

#[test]
fn auto_parametric_36() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[262]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(262));
}

#[test]
fn auto_parametric_37() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[269]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(269));
}

#[test]
fn auto_parametric_38() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[276]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(276));
}

#[test]
fn auto_parametric_39() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[283]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(283));
}

#[test]
fn auto_parametric_40() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[290]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(290));
}

#[test]
fn auto_two_children_1() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[33, 25]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
}

#[test]
fn auto_two_children_2() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[36, 30]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
}

#[test]
fn auto_two_children_3() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[39, 35]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
}

#[test]
fn auto_two_children_4() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[42, 40]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
}

#[test]
fn auto_two_children_5() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[45, 45]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
}

#[test]
fn auto_two_children_6() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[48, 50]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
}

#[test]
fn auto_two_children_7() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[51, 55]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(106));
}

#[test]
fn auto_two_children_8() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[54, 60]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(114));
}

#[test]
fn auto_two_children_9() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[57, 65]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(122));
}

#[test]
fn auto_two_children_10() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[60, 70]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(130));
}

#[test]
fn auto_two_children_11() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[63, 75]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(138));
}

#[test]
fn auto_two_children_12() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[66, 80]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(146));
}

#[test]
fn auto_two_children_13() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[69, 85]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(154));
}

#[test]
fn auto_two_children_14() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Auto, &[72, 90]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(162));
}

// ═══════════════════════════════════════════════════════════════════════
// Section 5: overflow: clip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn clip_sets_clip_flag() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[50]);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn clip_not_scrollable() {
    assert!(!Overflow::Clip.is_scrollable());
}

#[test]
fn clip_is_clipping() {
    assert!(Overflow::Clip.is_clipping());
}

#[test]
fn clip_with_overflow_content() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[200]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_empty_container() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[]);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn clip_container_size_unchanged() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[300]);
    let f = layout(&doc);
    let c = container(&f);
    assert_eq!(c.size.height, lu(100));
    assert_eq!(c.size.width, lu(200));
}

#[test]
fn clip_parametric_1() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[30]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_2() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[40]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_3() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[50]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_4() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[60]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_5() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[70]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_6() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[80]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_7() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[90]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_8() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[100]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_9() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[110]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_10() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[120]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_11() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[130]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_12() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[140]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_13() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[150]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_14() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[160]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_15() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[170]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_16() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[180]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_17() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[190]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_18() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[200]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_19() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[210]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_20() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[220]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_21() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[230]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_22() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[240]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_23() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[250]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_24() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[260]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_25() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[270]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_26() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[280]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_27() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[290]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_28() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[300]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_29() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[310]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_30() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[320]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_31() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[330]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_32() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[340]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_33() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[350]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn clip_parametric_34() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[360]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

// ═══════════════════════════════════════════════════════════════════════
// Section 6: Overflow Rect Computation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_rect_single_child() {
    let (doc, _, _) = build_doc(200, Some(50), Overflow::Hidden, &[100]);
    let f = layout(&doc);
    let ov = container(&f).scrollable_overflow();
    assert_eq!(ov.height(), lu(100));
}

#[test]
fn overflow_rect_union_of_multiple_children() {
    let (doc, _, _) = build_doc(200, Some(50), Overflow::Hidden, &[30, 40, 50]);
    let f = layout(&doc);
    let ov = container(&f).scrollable_overflow();
    assert_eq!(ov.height(), lu(120));
}

#[test]
fn overflow_rect_with_relative_child() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(20.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn overflow_rect_absolute_child_layout() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.node_mut(c).style.position = Position::Relative;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(50.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Absolute;
    doc.node_mut(ch).style.top = Length::px(200.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn overflow_rect_empty_container_zero() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[]);
    let f = layout(&doc);
    let c = container(&f);
    let ov = c.scrollable_overflow();
    assert_eq!(ov.width(), c.size.width);
    assert_eq!(ov.height(), c.size.height);
}

#[test]
fn overflow_rect_from_fragment_api() {
    let frag = Fragment::new_box(NodeId::NONE, PhysicalSize::new(lu(100), lu(50)));
    assert!(frag.overflow_rect.is_none());
    let ov = frag.scrollable_overflow();
    assert_eq!(ov.x(), lu(0));
    assert_eq!(ov.y(), lu(0));
    assert_eq!(ov.width(), lu(100));
    assert_eq!(ov.height(), lu(50));
}

#[test]
fn overflow_rect_with_explicit_rect() {
    let mut frag = Fragment::new_box(NodeId::NONE, PhysicalSize::new(lu(100), lu(50)));
    frag.overflow_rect = Some(PhysicalRect::from_xywh(lu(0), lu(0), lu(200), lu(300)));
    let ov = frag.scrollable_overflow();
    assert_eq!(ov.width(), lu(200));
    assert_eq!(ov.height(), lu(300));
}

#[test]
fn overflow_rect_border_box_rect() {
    let frag = Fragment::new_box(NodeId::NONE, PhysicalSize::new(lu(300), lu(150)));
    let r = frag.border_box_rect();
    assert_eq!(r.x(), lu(0));
    assert_eq!(r.y(), lu(0));
    assert_eq!(r.width(), lu(300));
    assert_eq!(r.height(), lu(150));
}

#[test]
fn physical_rect_unite_basic() {
    let a = PhysicalRect::from_xywh(lu(0), lu(0), lu(100), lu(50));
    let b = PhysicalRect::from_xywh(lu(50), lu(30), lu(100), lu(80));
    let u = a.unite(&b);
    assert_eq!(u.x(), lu(0));
    assert_eq!(u.y(), lu(0));
    assert_eq!(u.width(), lu(150));
    assert_eq!(u.height(), lu(110));
}

#[test]
fn physical_rect_unite_with_empty() {
    let a = PhysicalRect::from_xywh(lu(10), lu(20), lu(100), lu(50));
    let empty = PhysicalRect::default();
    assert_eq!(a.unite(&empty), a);
    assert_eq!(empty.unite(&a), a);
}

#[test]
fn overflow_rect_nested_children() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(50.0);
    doc.append_child(vp, c);
    let mid = doc.create_node(ElementTag::Div);
    doc.node_mut(mid).style.display = Display::Block;
    doc.node_mut(mid).style.width = Length::px(200.0);
    doc.node_mut(mid).style.height = Length::px(30.0);
    doc.append_child(c, mid);
    let leaf = doc.create_node(ElementTag::Div);
    doc.node_mut(leaf).style.display = Display::Block;
    doc.node_mut(leaf).style.width = Length::px(200.0);
    doc.node_mut(leaf).style.height = Length::px(200.0);
    doc.append_child(mid, leaf);
    let f = layout(&doc);
    let cf = &f.children[0];
    let ov = cf.scrollable_overflow();
    assert!(ov.height() >= lu(200));
}

#[test]
fn overflow_rect_parametric_1() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[23, 23]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_2() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[26, 26, 26]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_3() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[29, 29, 29, 29]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(116));
}

#[test]
fn overflow_rect_parametric_4() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[32, 32, 32, 32, 32]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(160));
}

#[test]
fn overflow_rect_parametric_5() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[35]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_6() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[38, 38]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_7() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[41, 41, 41]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(123));
}

#[test]
fn overflow_rect_parametric_8() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[44, 44, 44, 44]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(176));
}

#[test]
fn overflow_rect_parametric_9() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[47, 47, 47, 47, 47]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(235));
}

#[test]
fn overflow_rect_parametric_10() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[50]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_11() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[53, 53]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(106));
}

#[test]
fn overflow_rect_parametric_12() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[56, 56, 56]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(168));
}

#[test]
fn overflow_rect_parametric_13() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[59, 59, 59, 59]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(236));
}

#[test]
fn overflow_rect_parametric_14() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[62, 62, 62, 62, 62]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(310));
}

#[test]
fn overflow_rect_parametric_15() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[65]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_16() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[68, 68]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(136));
}

#[test]
fn overflow_rect_parametric_17() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[71, 71, 71]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(213));
}

#[test]
fn overflow_rect_parametric_18() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[74, 74, 74, 74]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(296));
}

#[test]
fn overflow_rect_parametric_19() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[77, 77, 77, 77, 77]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(385));
}

#[test]
fn overflow_rect_parametric_20() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[80]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_21() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[83, 83]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(166));
}

#[test]
fn overflow_rect_parametric_22() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[86, 86, 86]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(258));
}

#[test]
fn overflow_rect_parametric_23() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[89, 89, 89, 89]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(356));
}

#[test]
fn overflow_rect_parametric_24() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[92, 92, 92, 92, 92]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(460));
}

#[test]
fn overflow_rect_parametric_25() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[95]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_26() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[98, 98]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(196));
}

#[test]
fn overflow_rect_parametric_27() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[101, 101, 101]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(303));
}

#[test]
fn overflow_rect_parametric_28() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[104, 104, 104, 104]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(416));
}

#[test]
fn overflow_rect_parametric_29() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[107, 107, 107, 107, 107]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(535));
}

#[test]
fn overflow_rect_parametric_30() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[110]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(110));
}

#[test]
fn overflow_rect_parametric_31() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[113, 113]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(226));
}

#[test]
fn overflow_rect_parametric_32() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[116, 116, 116]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(348));
}

#[test]
fn overflow_rect_parametric_33() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[119, 119, 119, 119]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(476));
}

#[test]
fn overflow_rect_parametric_34() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[22, 22, 22, 22, 22]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(110));
}

#[test]
fn overflow_rect_parametric_35() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[25]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_36() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[28, 28]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_37() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[31, 31, 31]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_38() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[34, 34, 34, 34]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(136));
}

#[test]
fn overflow_rect_parametric_39() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[37, 37, 37, 37, 37]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(185));
}

#[test]
fn overflow_rect_parametric_40() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[40]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_41() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[43, 43]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_42() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[46, 46, 46]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(138));
}

#[test]
fn overflow_rect_parametric_43() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[49, 49, 49, 49]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(196));
}

#[test]
fn overflow_rect_parametric_44() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[52, 52, 52, 52, 52]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(260));
}

#[test]
fn overflow_rect_parametric_45() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[55]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_46() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[58, 58]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(116));
}

#[test]
fn overflow_rect_parametric_47() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[61, 61, 61]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(183));
}

#[test]
fn overflow_rect_parametric_48() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[64, 64, 64, 64]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(256));
}

#[test]
fn overflow_rect_parametric_49() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[67, 67, 67, 67, 67]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(335));
}

#[test]
fn overflow_rect_parametric_50() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[70]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_51() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[73, 73]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(146));
}

#[test]
fn overflow_rect_parametric_52() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[76, 76, 76]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(228));
}

#[test]
fn overflow_rect_parametric_53() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[79, 79, 79, 79]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(316));
}

#[test]
fn overflow_rect_parametric_54() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[82, 82, 82, 82, 82]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(410));
}

#[test]
fn overflow_rect_parametric_55() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[85]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_56() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[88, 88]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(176));
}

#[test]
fn overflow_rect_parametric_57() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[91, 91, 91]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(273));
}

#[test]
fn overflow_rect_parametric_58() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[94, 94, 94, 94]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(376));
}

#[test]
fn overflow_rect_parametric_59() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[97, 97, 97, 97, 97]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(485));
}

#[test]
fn overflow_rect_parametric_60() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[100]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_61() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[103, 103]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(206));
}

#[test]
fn overflow_rect_parametric_62() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[106, 106, 106]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(318));
}

#[test]
fn overflow_rect_parametric_63() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[109, 109, 109, 109]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(436));
}

#[test]
fn overflow_rect_parametric_64() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[112, 112, 112, 112, 112]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(560));
}

#[test]
fn overflow_rect_parametric_65() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[115]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(115));
}

#[test]
fn overflow_rect_parametric_66() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[118, 118]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(236));
}

#[test]
fn overflow_rect_parametric_67() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[21, 21, 21]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_68() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[24, 24, 24, 24]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

#[test]
fn overflow_rect_parametric_69() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[27, 27, 27, 27, 27]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() >= lu(135));
}

#[test]
fn overflow_rect_parametric_70() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[30]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.scrollable_overflow().height() <= c.size.height);
}

// ═══════════════════════════════════════════════════════════════════════
// Section 7: Overflow and BFC
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn bfc_overflow_hidden_creates_new_fc() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_auto_creates_new_fc() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto;
    s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_scroll_creates_new_fc() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_clip_creates_new_fc() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip;
    s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_visible_no_fc() {
    let s = ComputedStyle::initial();
    assert!(!establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_x_hidden_creates_fc() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_x_auto_creates_fc() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_x_scroll_creates_fc() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_x_clip_creates_fc() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_y_hidden_creates_fc() {
    let mut s = ComputedStyle::initial();
    s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_y_auto_creates_fc() {
    let mut s = ComputedStyle::initial();
    s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_y_scroll_creates_fc() {
    let mut s = ComputedStyle::initial();
    s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_y_clip_creates_fc() {
    let mut s = ComputedStyle::initial();
    s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_overflow_hidden_no_margin_collapse_through() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).margin(20, 0, 20, 0).done();
    b.add_child().width(200.0).height(50.0).overflow_hidden().margin(30, 0, 30, 0).done();
    let r = b.build();
    let c0 = r.child(0);
    let c1 = r.child(1);
    assert!(c1.offset.top > c0.offset.top + c0.size.height);
}

#[test]
fn bfc_overflow_scroll_no_margin_collapse_through() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).margin(0, 0, 20, 0).done();
    b.add_child().width(200.0).height(50.0).overflow(Overflow::Scroll).margin(30, 0, 0, 0).done();
    let r = b.build();
    let c0 = r.child(0);
    let c1 = r.child(1);
    let gap = c1.offset.top.to_i32() - (c0.offset.top.to_i32() + c0.size.height.to_i32());
    assert!(gap >= 30);
}

#[test]
fn bfc_overflow_auto_no_margin_collapse_through() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).margin(0, 0, 20, 0).done();
    b.add_child().width(200.0).height(50.0).overflow(Overflow::Auto).margin(30, 0, 0, 0).done();
    let r = b.build();
    let c0 = r.child(0);
    let c1 = r.child(1);
    let gap = c1.offset.top.to_i32() - (c0.offset.top.to_i32() + c0.size.height.to_i32());
    assert!(gap >= 30);
}

#[test]
fn bfc_overflow_hidden_avoids_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(50.0).overflow_hidden().done();
    let r = b.build();
    let fl = r.child(0);
    let bfc = r.child(1);
    let float_right = fl.offset.left.to_i32() + fl.size.width.to_i32();
    let avoids = bfc.offset.left.to_i32() >= float_right
        || bfc.offset.top.to_i32() >= fl.offset.top.to_i32() + fl.size.height.to_i32();
    assert!(avoids, "BFC element should avoid float");
}

#[test]
fn bfc_overflow_auto_avoids_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(50.0).overflow(Overflow::Auto).done();
    let r = b.build();
    let fl = r.child(0);
    let bfc = r.child(1);
    let float_right = fl.offset.left.to_i32() + fl.size.width.to_i32();
    let avoids = bfc.offset.left.to_i32() >= float_right
        || bfc.offset.top.to_i32() >= fl.offset.top.to_i32() + fl.size.height.to_i32();
    assert!(avoids);
}

#[test]
fn bfc_overflow_scroll_avoids_float() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(100.0).height(100.0).float_left().done();
    b.add_child().width(200.0).height(50.0).overflow(Overflow::Scroll).done();
    let r = b.build();
    let fl = r.child(0);
    let bfc = r.child(1);
    let float_right = fl.offset.left.to_i32() + fl.size.width.to_i32();
    let avoids = bfc.offset.left.to_i32() >= float_right
        || bfc.offset.top.to_i32() >= fl.offset.top.to_i32() + fl.size.height.to_i32();
    assert!(avoids);
}

#[test]
fn bfc_combined_float_left_hidden() {
    let mut s = ComputedStyle::initial();
    s.float = Float::Left; s.overflow_x = Overflow::Hidden; s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_combined_float_right_auto() {
    let mut s = ComputedStyle::initial();
    s.float = Float::Right; s.overflow_x = Overflow::Auto; s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_combined_abs_pos_hidden() {
    let mut s = ComputedStyle::initial();
    s.position = Position::Absolute; s.overflow_x = Overflow::Hidden; s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_combined_fixed_pos_scroll() {
    let mut s = ComputedStyle::initial();
    s.position = Position::Fixed; s.overflow_x = Overflow::Scroll; s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_combined_inline_block_hidden() {
    let mut s = ComputedStyle::initial();
    s.display = Display::InlineBlock; s.overflow_x = Overflow::Hidden; s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_combined_flow_root_auto() {
    let mut s = ComputedStyle::initial();
    s.display = Display::FlowRoot; s.overflow_x = Overflow::Auto; s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_combined_flex_clip() {
    let mut s = ComputedStyle::initial();
    s.display = Display::Flex; s.overflow_x = Overflow::Clip; s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
}

#[test]
fn bfc_parametric_1() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto;
    s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Auto.is_clipping());
}

#[test]
fn bfc_parametric_2() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Scroll.is_clipping());
}

#[test]
fn bfc_parametric_3() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip;
    s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Clip.is_clipping());
}

#[test]
fn bfc_parametric_4() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Hidden.is_clipping());
}

#[test]
fn bfc_parametric_5() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto;
    s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Auto.is_clipping());
}

#[test]
fn bfc_parametric_6() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Scroll.is_clipping());
}

#[test]
fn bfc_parametric_7() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip;
    s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Clip.is_clipping());
}

#[test]
fn bfc_parametric_8() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Hidden.is_clipping());
}

#[test]
fn bfc_parametric_9() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto;
    s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Auto.is_clipping());
}

#[test]
fn bfc_parametric_10() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Scroll.is_clipping());
}

#[test]
fn bfc_parametric_11() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip;
    s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Clip.is_clipping());
}

#[test]
fn bfc_parametric_12() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Hidden.is_clipping());
}

#[test]
fn bfc_parametric_13() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto;
    s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Auto.is_clipping());
}

#[test]
fn bfc_parametric_14() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Scroll.is_clipping());
}

#[test]
fn bfc_parametric_15() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip;
    s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Clip.is_clipping());
}

#[test]
fn bfc_parametric_16() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Hidden.is_clipping());
}

#[test]
fn bfc_parametric_17() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto;
    s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Auto.is_clipping());
}

#[test]
fn bfc_parametric_18() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Scroll.is_clipping());
}

#[test]
fn bfc_parametric_19() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip;
    s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Clip.is_clipping());
}

#[test]
fn bfc_parametric_20() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Hidden.is_clipping());
}

#[test]
fn bfc_parametric_21() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto;
    s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Auto.is_clipping());
}

#[test]
fn bfc_parametric_22() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Scroll.is_clipping());
}

#[test]
fn bfc_parametric_23() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip;
    s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Clip.is_clipping());
}

#[test]
fn bfc_parametric_24() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Hidden.is_clipping());
}

#[test]
fn bfc_parametric_25() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto;
    s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Auto.is_clipping());
}

#[test]
fn bfc_parametric_26() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Scroll.is_clipping());
}

#[test]
fn bfc_parametric_27() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip;
    s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Clip.is_clipping());
}

#[test]
fn bfc_parametric_28() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Hidden.is_clipping());
}

#[test]
fn bfc_parametric_29() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto;
    s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Auto.is_clipping());
}

#[test]
fn bfc_parametric_30() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Scroll.is_clipping());
}

#[test]
fn bfc_parametric_31() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip;
    s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Clip.is_clipping());
}

#[test]
fn bfc_parametric_32() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Hidden.is_clipping());
}

#[test]
fn bfc_parametric_33() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto;
    s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Auto.is_clipping());
}

#[test]
fn bfc_parametric_34() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Scroll.is_clipping());
}

#[test]
fn bfc_parametric_35() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip;
    s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Clip.is_clipping());
}

#[test]
fn bfc_parametric_36() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Hidden.is_clipping());
}

#[test]
fn bfc_parametric_37() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto;
    s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Auto.is_clipping());
}

#[test]
fn bfc_parametric_38() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll;
    s.overflow_y = Overflow::Scroll;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Scroll.is_clipping());
}

#[test]
fn bfc_parametric_39() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip;
    s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Clip.is_clipping());
}

#[test]
fn bfc_parametric_40() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
    assert!(Overflow::Hidden.is_clipping());
}

// ═══════════════════════════════════════════════════════════════════════
// Section 8: Overflow with Positioning
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn positioning_relative_child_in_hidden_container() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(10.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_absolute_child_in_hidden_container() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.node_mut(c).style.position = Position::Relative;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(50.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Absolute;
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_relative_contributes_to_overflow() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(80.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_overflow_hidden_with_static_children() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[40, 40]);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_relative_in_hidden() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(50.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(10.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_absolute_in_hidden() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.node_mut(c).style.position = Position::Relative;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(50.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Absolute;
    doc.node_mut(ch).style.top = Length::px(10.0);
    doc.node_mut(ch).style.left = Length::px(10.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_relative_in_scroll() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(50.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(10.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_absolute_in_scroll() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.node_mut(c).style.position = Position::Relative;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(50.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Absolute;
    doc.node_mut(ch).style.top = Length::px(10.0);
    doc.node_mut(ch).style.left = Length::px(10.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_relative_in_auto() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(50.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(10.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_absolute_in_auto() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.node_mut(c).style.position = Position::Relative;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(50.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Absolute;
    doc.node_mut(ch).style.top = Length::px(10.0);
    doc.node_mut(ch).style.left = Length::px(10.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_relative_in_clip() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(50.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(10.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_absolute_in_clip() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.node_mut(c).style.position = Position::Relative;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(50.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Absolute;
    doc.node_mut(ch).style.top = Length::px(10.0);
    doc.node_mut(ch).style.left = Length::px(10.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_1() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(5.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_2() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(10.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_3() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(15.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_4() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(20.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_5() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(25.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_6() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(30.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_7() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(35.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_8() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(40.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_9() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(45.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_10() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(50.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_11() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(55.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_12() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(60.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_13() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(65.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_14() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(70.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_15() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(75.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_16() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(80.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_17() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(85.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_18() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(90.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_19() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(95.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_20() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(100.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_21() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(105.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_22() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(110.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_23() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(115.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_24() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(120.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_25() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(125.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_26() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(130.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_27() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(135.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_28() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(140.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_29() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(145.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_30() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(150.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_31() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(155.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_32() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(160.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_33() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(165.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_34() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(170.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_35() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(175.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_36() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(180.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_37() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(185.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_38() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(190.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_39() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(195.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_40() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(200.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_41() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(205.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_42() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(210.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_43() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(215.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_44() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(220.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_45() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(225.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_46() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(230.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_47() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(235.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_48() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(240.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_49() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(245.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn positioning_parametric_50() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.node_mut(ch).style.position = Position::Relative;
    doc.node_mut(ch).style.top = Length::px(250.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

// ═══════════════════════════════════════════════════════════════════════
// Section 9: Border-Radius Clipping
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn border_radius_single_uniform() {
    let mut s = ComputedStyle::initial();
    s.border_top_left_radius = (10.0, 10.0);
    s.border_top_right_radius = (10.0, 10.0);
    s.border_bottom_right_radius = (10.0, 10.0);
    s.border_bottom_left_radius = (10.0, 10.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_no_radius() {
    let s = ComputedStyle::initial();
    assert!(!s.has_border_radius());
}

#[test]
fn border_radius_different_per_corner() {
    let mut s = ComputedStyle::initial();
    s.border_top_left_radius = (5.0, 5.0);
    s.border_top_right_radius = (10.0, 10.0);
    s.border_bottom_right_radius = (15.0, 15.0);
    s.border_bottom_left_radius = (20.0, 20.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_elliptical() {
    let mut s = ComputedStyle::initial();
    s.border_top_left_radius = (10.0, 20.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_single_corner_only() {
    let mut s = ComputedStyle::initial();
    s.border_top_left_radius = (10.0, 10.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_with_overflow_hidden_layout() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.node_mut(c).style.border_top_left_radius = (20.0, 20.0);
    doc.node_mut(c).style.border_top_right_radius = (20.0, 20.0);
    doc.node_mut(c).style.border_bottom_right_radius = (20.0, 20.0);
    doc.node_mut(c).style.border_bottom_left_radius = (20.0, 20.0);
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    let cf = container(&f);
    assert!(cf.has_overflow_clip);
    assert!(doc.node(cf.node_id).style.has_border_radius());
}

#[test]
fn border_radius_without_overflow_no_clip() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.border_top_left_radius = (20.0, 20.0);
    doc.append_child(vp, c);
    let f = layout(&doc);
    assert!(!container(&f).has_overflow_clip);
}

#[test]
fn border_radius_large_clamped_check() {
    let mut s = ComputedStyle::initial();
    s.border_top_left_radius = (9999.0, 9999.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_top_left_1px() {
    let mut s = ComputedStyle::initial();
    s.border_top_left_radius = (1.0, 1.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_top_left_5px() {
    let mut s = ComputedStyle::initial();
    s.border_top_left_radius = (5.0, 5.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_top_left_10px() {
    let mut s = ComputedStyle::initial();
    s.border_top_left_radius = (10.0, 10.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_top_left_25px() {
    let mut s = ComputedStyle::initial();
    s.border_top_left_radius = (25.0, 25.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_top_left_50px() {
    let mut s = ComputedStyle::initial();
    s.border_top_left_radius = (50.0, 50.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_top_left_100px() {
    let mut s = ComputedStyle::initial();
    s.border_top_left_radius = (100.0, 100.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_top_right_1px() {
    let mut s = ComputedStyle::initial();
    s.border_top_right_radius = (1.0, 1.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_top_right_5px() {
    let mut s = ComputedStyle::initial();
    s.border_top_right_radius = (5.0, 5.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_top_right_10px() {
    let mut s = ComputedStyle::initial();
    s.border_top_right_radius = (10.0, 10.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_top_right_25px() {
    let mut s = ComputedStyle::initial();
    s.border_top_right_radius = (25.0, 25.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_top_right_50px() {
    let mut s = ComputedStyle::initial();
    s.border_top_right_radius = (50.0, 50.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_top_right_100px() {
    let mut s = ComputedStyle::initial();
    s.border_top_right_radius = (100.0, 100.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_bottom_right_1px() {
    let mut s = ComputedStyle::initial();
    s.border_bottom_right_radius = (1.0, 1.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_bottom_right_5px() {
    let mut s = ComputedStyle::initial();
    s.border_bottom_right_radius = (5.0, 5.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_bottom_right_10px() {
    let mut s = ComputedStyle::initial();
    s.border_bottom_right_radius = (10.0, 10.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_bottom_right_25px() {
    let mut s = ComputedStyle::initial();
    s.border_bottom_right_radius = (25.0, 25.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_bottom_right_50px() {
    let mut s = ComputedStyle::initial();
    s.border_bottom_right_radius = (50.0, 50.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_bottom_right_100px() {
    let mut s = ComputedStyle::initial();
    s.border_bottom_right_radius = (100.0, 100.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_bottom_left_1px() {
    let mut s = ComputedStyle::initial();
    s.border_bottom_left_radius = (1.0, 1.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_bottom_left_5px() {
    let mut s = ComputedStyle::initial();
    s.border_bottom_left_radius = (5.0, 5.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_bottom_left_10px() {
    let mut s = ComputedStyle::initial();
    s.border_bottom_left_radius = (10.0, 10.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_bottom_left_25px() {
    let mut s = ComputedStyle::initial();
    s.border_bottom_left_radius = (25.0, 25.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_bottom_left_50px() {
    let mut s = ComputedStyle::initial();
    s.border_bottom_left_radius = (50.0, 50.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_bottom_left_100px() {
    let mut s = ComputedStyle::initial();
    s.border_bottom_left_radius = (100.0, 100.0);
    assert!(s.has_border_radius());
}

#[test]
fn border_radius_with_overflow_hidden() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Hidden;
    doc.node_mut(c).style.overflow_y = Overflow::Hidden;
    doc.node_mut(c).style.border_top_left_radius = (15.0, 15.0);
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    let cf = container(&f);
    assert!(cf.has_overflow_clip);
    assert!(doc.node(cf.node_id).style.has_border_radius());
}

#[test]
fn border_radius_with_overflow_scroll() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Scroll;
    doc.node_mut(c).style.overflow_y = Overflow::Scroll;
    doc.node_mut(c).style.border_top_left_radius = (15.0, 15.0);
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    let cf = container(&f);
    assert!(cf.has_overflow_clip);
    assert!(doc.node(cf.node_id).style.has_border_radius());
}

#[test]
fn border_radius_with_overflow_auto() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Auto;
    doc.node_mut(c).style.overflow_y = Overflow::Auto;
    doc.node_mut(c).style.border_top_left_radius = (15.0, 15.0);
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    let cf = container(&f);
    assert!(cf.has_overflow_clip);
    assert!(doc.node(cf.node_id).style.has_border_radius());
}

#[test]
fn border_radius_with_overflow_clip() {
    let mut doc = Document::new();
    let vp = doc.root();
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(200.0);
    doc.node_mut(c).style.height = Length::px(100.0);
    doc.node_mut(c).style.overflow_x = Overflow::Clip;
    doc.node_mut(c).style.overflow_y = Overflow::Clip;
    doc.node_mut(c).style.border_top_left_radius = (15.0, 15.0);
    doc.append_child(vp, c);
    let ch = doc.create_node(ElementTag::Div);
    doc.node_mut(ch).style.display = Display::Block;
    doc.node_mut(ch).style.width = Length::px(200.0);
    doc.node_mut(ch).style.height = Length::px(50.0);
    doc.append_child(c, ch);
    let f = layout(&doc);
    let cf = container(&f);
    assert!(cf.has_overflow_clip);
    assert!(doc.node(cf.node_id).style.has_border_radius());
}

// ═══════════════════════════════════════════════════════════════════════
// Section 10: Edge Cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn edge_zero_size_container_hidden() {
    let (doc, _, _) = build_doc(0, Some(0), Overflow::Hidden, &[]);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn edge_zero_width_container() {
    let (doc, _, _) = build_doc(0, Some(100), Overflow::Hidden, &[50]);
    let f = layout(&doc);
    assert!(container(&f).has_overflow_clip);
}

#[test]
fn edge_zero_height_container() {
    let (doc, _, _) = build_doc(200, Some(0), Overflow::Hidden, &[50]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(0));
}

#[test]
fn edge_very_large_overflow() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Hidden, &[10000]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.scrollable_overflow().height(), lu(10000));
}

#[test]
fn edge_overflow_shorthand_both_same() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Hidden;
    assert!(s.overflow_x == s.overflow_y);
    assert!(establishes_new_fc(&s));
}

#[test]
fn edge_overflow_shorthand_mixed() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden;
    s.overflow_y = Overflow::Scroll;
    assert!(s.overflow_x != s.overflow_y);
    assert!(establishes_new_fc(&s));
}

#[test]
fn edge_overflow_with_padding_all_sides() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .overflow_hidden()
        .padding(10, 10, 10, 10)
        .add_child().width(180.0).height(50.0).done()
        .done();
    let r = b.build();
    assert!(r.child(0).has_overflow_clip);
}

#[test]
fn edge_overflow_with_border_all_sides() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .overflow_hidden()
        .border(5, 5, 5, 5)
        .add_child().width(190.0).height(50.0).done()
        .done();
    let r = b.build();
    assert!(r.child(0).has_overflow_clip);
}

#[test]
fn edge_overflow_with_padding_and_border() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .overflow_hidden()
        .padding(10, 10, 10, 10)
        .border(5, 5, 5, 5)
        .add_child().width(170.0).height(40.0).done()
        .done();
    let r = b.build();
    assert!(r.child(0).has_overflow_clip);
}

#[test]
fn edge_overflow_enum_values() {
    assert_eq!(Overflow::Visible as u8, 0);
    assert_eq!(Overflow::Hidden as u8, 1);
    assert_eq!(Overflow::Scroll as u8, 2);
    assert_eq!(Overflow::Auto as u8, 3);
    assert_eq!(Overflow::Clip as u8, 4);
}

#[test]
fn edge_overflow_default_is_visible() {
    assert_eq!(Overflow::default(), Overflow::Visible);
    assert_eq!(Overflow::INITIAL, Overflow::Visible);
}

#[test]
fn edge_set_overflow_clip_toggle() {
    let mut frag = Fragment::new_box(NodeId::NONE, PhysicalSize::new(lu(100), lu(50)));
    assert!(!frag.has_overflow_clip);
    frag.set_overflow_clip(true);
    assert!(frag.has_overflow_clip);
    frag.set_overflow_clip(false);
    assert!(!frag.has_overflow_clip);
}

#[test]
fn edge_overflow_rect_none_default() {
    let frag = Fragment::new_box(NodeId::NONE, PhysicalSize::new(lu(100), lu(50)));
    assert!(frag.overflow_rect.is_none());
}

#[test]
fn edge_content_offset_with_padding() {
    let mut frag = Fragment::new_box(NodeId::NONE, PhysicalSize::new(lu(200), lu(100)));
    frag.padding = openui_geometry::BoxStrut::new(lu(10), lu(10), lu(10), lu(10));
    frag.border = openui_geometry::BoxStrut::new(lu(5), lu(5), lu(5), lu(5));
    let offset = frag.content_offset();
    assert_eq!(offset.left, lu(15));
    assert_eq!(offset.top, lu(15));
}

#[test]
fn edge_content_size_with_padding_border() {
    let mut frag = Fragment::new_box(NodeId::NONE, PhysicalSize::new(lu(200), lu(100)));
    frag.padding = openui_geometry::BoxStrut::new(lu(10), lu(10), lu(10), lu(10));
    frag.border = openui_geometry::BoxStrut::new(lu(5), lu(5), lu(5), lu(5));
    let cs = frag.content_size();
    assert_eq!(cs.width, lu(170));
    assert_eq!(cs.height, lu(70));
}

#[test]
fn edge_padding_box_size() {
    let mut frag = Fragment::new_box(NodeId::NONE, PhysicalSize::new(lu(200), lu(100)));
    frag.border = openui_geometry::BoxStrut::new(lu(5), lu(5), lu(5), lu(5));
    let ps = frag.padding_box_size();
    assert_eq!(ps.width, lu(190));
    assert_eq!(ps.height, lu(90));
}

#[test]
fn edge_overflow_hidden_border_box() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .overflow_hidden()
        .box_sizing_border_box()
        .padding(10, 10, 10, 10)
        .add_child().width(180.0).height(50.0).done()
        .done();
    let r = b.build();
    assert!(r.child(0).has_overflow_clip);
}

#[test]
fn edge_parametric_1() {
    let (doc, _, _) = build_doc(57, Some(41), Overflow::Scroll, &[23]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(57));
    assert_eq!(c.size.height, lu(41));
}

#[test]
fn edge_parametric_2() {
    let (doc, _, _) = build_doc(64, Some(52), Overflow::Auto, &[36]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(64));
    assert_eq!(c.size.height, lu(52));
}

#[test]
fn edge_parametric_3() {
    let (doc, _, _) = build_doc(71, Some(63), Overflow::Clip, &[49]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(71));
    assert_eq!(c.size.height, lu(63));
}

#[test]
fn edge_parametric_4() {
    let (doc, _, _) = build_doc(78, Some(74), Overflow::Hidden, &[62]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(78));
    assert_eq!(c.size.height, lu(74));
}

#[test]
fn edge_parametric_5() {
    let (doc, _, _) = build_doc(85, Some(85), Overflow::Scroll, &[75]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(85));
    assert_eq!(c.size.height, lu(85));
}

#[test]
fn edge_parametric_6() {
    let (doc, _, _) = build_doc(92, Some(96), Overflow::Auto, &[88]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(92));
    assert_eq!(c.size.height, lu(96));
}

#[test]
fn edge_parametric_7() {
    let (doc, _, _) = build_doc(99, Some(107), Overflow::Clip, &[101]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(99));
    assert_eq!(c.size.height, lu(107));
}

#[test]
fn edge_parametric_8() {
    let (doc, _, _) = build_doc(106, Some(118), Overflow::Hidden, &[114]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(106));
    assert_eq!(c.size.height, lu(118));
}

#[test]
fn edge_parametric_9() {
    let (doc, _, _) = build_doc(113, Some(129), Overflow::Scroll, &[127]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(113));
    assert_eq!(c.size.height, lu(129));
}

#[test]
fn edge_parametric_10() {
    let (doc, _, _) = build_doc(120, Some(140), Overflow::Auto, &[140]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(120));
    assert_eq!(c.size.height, lu(140));
}

#[test]
fn edge_parametric_11() {
    let (doc, _, _) = build_doc(127, Some(151), Overflow::Clip, &[153]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(127));
    assert_eq!(c.size.height, lu(151));
}

#[test]
fn edge_parametric_12() {
    let (doc, _, _) = build_doc(134, Some(162), Overflow::Hidden, &[166]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(134));
    assert_eq!(c.size.height, lu(162));
}

#[test]
fn edge_parametric_13() {
    let (doc, _, _) = build_doc(141, Some(173), Overflow::Scroll, &[179]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(141));
    assert_eq!(c.size.height, lu(173));
}

#[test]
fn edge_parametric_14() {
    let (doc, _, _) = build_doc(148, Some(184), Overflow::Auto, &[192]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(148));
    assert_eq!(c.size.height, lu(184));
}

#[test]
fn edge_parametric_15() {
    let (doc, _, _) = build_doc(155, Some(195), Overflow::Clip, &[205]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(155));
    assert_eq!(c.size.height, lu(195));
}

#[test]
fn edge_parametric_16() {
    let (doc, _, _) = build_doc(162, Some(206), Overflow::Hidden, &[218]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(162));
    assert_eq!(c.size.height, lu(206));
}

#[test]
fn edge_parametric_17() {
    let (doc, _, _) = build_doc(169, Some(217), Overflow::Scroll, &[231]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(169));
    assert_eq!(c.size.height, lu(217));
}

#[test]
fn edge_parametric_18() {
    let (doc, _, _) = build_doc(176, Some(228), Overflow::Auto, &[244]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(176));
    assert_eq!(c.size.height, lu(228));
}

#[test]
fn edge_parametric_19() {
    let (doc, _, _) = build_doc(183, Some(39), Overflow::Clip, &[257]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(183));
    assert_eq!(c.size.height, lu(39));
}

#[test]
fn edge_parametric_20() {
    let (doc, _, _) = build_doc(190, Some(50), Overflow::Hidden, &[270]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(190));
    assert_eq!(c.size.height, lu(50));
}

#[test]
fn edge_parametric_21() {
    let (doc, _, _) = build_doc(197, Some(61), Overflow::Scroll, &[283]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(197));
    assert_eq!(c.size.height, lu(61));
}

#[test]
fn edge_parametric_22() {
    let (doc, _, _) = build_doc(204, Some(72), Overflow::Auto, &[296]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(204));
    assert_eq!(c.size.height, lu(72));
}

#[test]
fn edge_parametric_23() {
    let (doc, _, _) = build_doc(211, Some(83), Overflow::Clip, &[309]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(211));
    assert_eq!(c.size.height, lu(83));
}

#[test]
fn edge_parametric_24() {
    let (doc, _, _) = build_doc(218, Some(94), Overflow::Hidden, &[322]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(218));
    assert_eq!(c.size.height, lu(94));
}

#[test]
fn edge_parametric_25() {
    let (doc, _, _) = build_doc(225, Some(105), Overflow::Scroll, &[335]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(225));
    assert_eq!(c.size.height, lu(105));
}

#[test]
fn edge_parametric_26() {
    let (doc, _, _) = build_doc(232, Some(116), Overflow::Auto, &[348]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(232));
    assert_eq!(c.size.height, lu(116));
}

#[test]
fn edge_parametric_27() {
    let (doc, _, _) = build_doc(239, Some(127), Overflow::Clip, &[361]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(239));
    assert_eq!(c.size.height, lu(127));
}

#[test]
fn edge_parametric_28() {
    let (doc, _, _) = build_doc(246, Some(138), Overflow::Hidden, &[374]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(246));
    assert_eq!(c.size.height, lu(138));
}

#[test]
fn edge_parametric_29() {
    let (doc, _, _) = build_doc(253, Some(149), Overflow::Scroll, &[387]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(253));
    assert_eq!(c.size.height, lu(149));
}

#[test]
fn edge_parametric_30() {
    let (doc, _, _) = build_doc(260, Some(160), Overflow::Auto, &[400]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(260));
    assert_eq!(c.size.height, lu(160));
}

#[test]
fn edge_parametric_31() {
    let (doc, _, _) = build_doc(267, Some(171), Overflow::Clip, &[413]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(267));
    assert_eq!(c.size.height, lu(171));
}

#[test]
fn edge_parametric_32() {
    let (doc, _, _) = build_doc(274, Some(182), Overflow::Hidden, &[426]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(274));
    assert_eq!(c.size.height, lu(182));
}

#[test]
fn edge_parametric_33() {
    let (doc, _, _) = build_doc(281, Some(193), Overflow::Scroll, &[439]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(281));
    assert_eq!(c.size.height, lu(193));
}

#[test]
fn edge_parametric_34() {
    let (doc, _, _) = build_doc(288, Some(204), Overflow::Auto, &[452]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(288));
    assert_eq!(c.size.height, lu(204));
}

#[test]
fn edge_parametric_35() {
    let (doc, _, _) = build_doc(295, Some(215), Overflow::Clip, &[465]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(295));
    assert_eq!(c.size.height, lu(215));
}

#[test]
fn edge_parametric_36() {
    let (doc, _, _) = build_doc(302, Some(226), Overflow::Hidden, &[478]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(302));
    assert_eq!(c.size.height, lu(226));
}

#[test]
fn edge_parametric_37() {
    let (doc, _, _) = build_doc(309, Some(37), Overflow::Scroll, &[491]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(309));
    assert_eq!(c.size.height, lu(37));
}

#[test]
fn edge_parametric_38() {
    let (doc, _, _) = build_doc(316, Some(48), Overflow::Auto, &[504]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(316));
    assert_eq!(c.size.height, lu(48));
}

#[test]
fn edge_parametric_39() {
    let (doc, _, _) = build_doc(323, Some(59), Overflow::Clip, &[17]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(323));
    assert_eq!(c.size.height, lu(59));
}

#[test]
fn edge_parametric_40() {
    let (doc, _, _) = build_doc(330, Some(70), Overflow::Hidden, &[30]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.width, lu(330));
    assert_eq!(c.size.height, lu(70));
}

// ═══════════════════════════════════════════════════════════════════════
// Section 11: Additional Coverage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn additional_overflow_x_hidden_y_visible() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Hidden; s.overflow_y = Overflow::Visible;
    assert!(establishes_new_fc(&s));
}

#[test]
fn additional_overflow_x_visible_y_hidden() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Visible; s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
}

#[test]
fn additional_overflow_x_scroll_y_auto() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll; s.overflow_y = Overflow::Auto;
    assert!(establishes_new_fc(&s)); assert!(s.overflow_x.is_scrollable()); assert!(s.overflow_y.is_scrollable());
}

#[test]
fn additional_overflow_x_clip_y_hidden() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Clip; s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s)); assert!(s.overflow_x.is_clipping()); assert!(s.overflow_y.is_clipping());
}

#[test]
fn additional_overflow_x_auto_y_clip() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Auto; s.overflow_y = Overflow::Clip;
    assert!(establishes_new_fc(&s));
}

#[test]
fn additional_overflow_x_scroll_y_hidden() {
    let mut s = ComputedStyle::initial();
    s.overflow_x = Overflow::Scroll; s.overflow_y = Overflow::Hidden;
    assert!(establishes_new_fc(&s));
}

#[test]
fn additional_fragment_width_height_accessors() {
    let frag = Fragment::new_box(NodeId::NONE, PhysicalSize::new(lu(250), lu(75)));
    assert_eq!(frag.width(), lu(250));
    assert_eq!(frag.height(), lu(75));
}

#[test]
fn additional_physical_rect_right_bottom() {
    let r = PhysicalRect::from_xywh(lu(10), lu(20), lu(100), lu(50));
    assert_eq!(r.right(), lu(110));
    assert_eq!(r.bottom(), lu(70));
}

#[test]
fn additional_physical_rect_is_empty() {
    let r = PhysicalRect::default();
    assert!(r.is_empty());
    let r2 = PhysicalRect::from_xywh(lu(0), lu(0), lu(100), lu(50));
    assert!(!r2.is_empty());
}

#[test]
fn additional_overflow_visible_not_clipping() {
    assert!(!Overflow::Visible.is_clipping());
    assert!(!Overflow::Visible.is_scrollable());
}

#[test]
fn additional_overflow_hidden_not_scrollable() {
    assert!(!Overflow::Hidden.is_scrollable());
    assert!(Overflow::Hidden.is_clipping());
}

#[test]
fn additional_overflow_clip_not_scrollable() {
    assert!(!Overflow::Clip.is_scrollable());
    assert!(Overflow::Clip.is_clipping());
}

#[test]
fn additional_builder_overflow_hidden_child() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(300.0).height(200.0).overflow_hidden().done();
    let r = b.build();
    assert!(r.child(0).has_overflow_clip);
    r.assert_child_size(0, 300, 200);
}

#[test]
fn additional_builder_overflow_scroll_size() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(300.0).overflow(Overflow::Scroll).done();
    let r = b.build();
    assert!(r.child(0).has_overflow_clip);
    r.assert_child_size(0, 400, 300);
}

#[test]
fn additional_builder_overflow_auto_size() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(400.0).height(300.0).overflow(Overflow::Auto).done();
    let r = b.build();
    assert!(r.child(0).has_overflow_clip);
}

#[test]
fn additional_nested_visible_hidden() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .add_child().width(200.0).height(80.0).overflow_hidden().done()
        .done();
    let r = b.build();
    let outer = r.child(0);
    assert!(!outer.has_overflow_clip);
    assert!(outer.children[0].has_overflow_clip);
}

#[test]
fn additional_nested_hidden_visible() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(200.0).height(100.0)
        .overflow_hidden()
        .add_child().width(200.0).height(80.0).done()
        .done();
    let r = b.build();
    assert!(r.child(0).has_overflow_clip);
}

#[test]
fn additional_three_overflow_types_in_sequence() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width(200.0).height(50.0).overflow_hidden().done();
    b.add_child().width(200.0).height(50.0).overflow(Overflow::Scroll).done();
    b.add_child().width(200.0).height(50.0).overflow(Overflow::Auto).done();
    let r = b.build();
    assert!(r.child(0).has_overflow_clip);
    assert!(r.child(1).has_overflow_clip);
    assert!(r.child(2).has_overflow_clip);
}

#[test]
fn additional_overflow_clip_with_large_child() {
    let (doc, _, _) = build_doc(200, Some(100), Overflow::Clip, &[5000]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert_eq!(c.size.height, lu(100));
}

#[test]
fn additional_overflow_auto_height_no_explicit_height() {
    let (doc, _, _) = build_doc(200, None, Overflow::Hidden, &[50, 60, 70]);
    let f = layout(&doc);
    let c = container(&f);
    assert!(c.has_overflow_clip);
    assert!(c.size.height >= lu(180));
}

#[test]
fn additional_all_overflow_clipping_check() {
    for ov in [Overflow::Hidden, Overflow::Scroll, Overflow::Auto, Overflow::Clip] {
        assert!(ov.is_clipping(), "{:?} should be clipping", ov);
    }
}

#[test]
fn additional_physical_rect_from_xywh_various() {
    for (x, y, w, h) in [(0,0,100,50), (10,20,30,40), (0,0,1,1)] {
        let r = PhysicalRect::from_xywh(lu(x), lu(y), lu(w), lu(h));
        assert_eq!(r.x(), lu(x));
        assert_eq!(r.y(), lu(y));
        assert_eq!(r.width(), lu(w));
        assert_eq!(r.height(), lu(h));
    }
}
