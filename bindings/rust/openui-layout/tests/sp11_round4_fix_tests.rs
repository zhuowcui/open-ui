//! Tests for SP11 Round 4 fixes — atomic inline vertical-align positioning.
//!
//! Verifies that atomic inline elements (display: inline-block) respect
//! vertical-align: top, bottom, and middle when positioned within the line box.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{Display, VerticalAlign};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn collect_box_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    let mut result = Vec::new();
    if fragment.kind == FragmentKind::Box && !fragment.node_id.is_none() {
        result.push(fragment);
    }
    for child in &fragment.children {
        result.extend(collect_box_fragments(child));
    }
    result
}

/// Create a block with text + an atomic inline element with the given style.
fn make_atomic_inline_block(
    atomic_w: f32,
    atomic_h: f32,
    valign: VerticalAlign,
) -> (Document, NodeId, NodeId) {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    // Add text before the atomic to establish baseline and line metrics.
    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("text ".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(atomic_w);
    doc.node_mut(atomic).style.height = Length::px(atomic_h);
    doc.node_mut(atomic).style.vertical_align = valign;
    doc.append_child(block, atomic);

    (doc, block, atomic)
}

fn do_layout(doc: &Document, block: NodeId) -> Fragment {
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    inline_layout(doc, block, &sp)
}

// ═══════════════════════════════════════════════════════════════════════
// ATOMIC INLINE VERTICAL-ALIGN POSITIONING
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn atomic_inline_valign_top_at_line_top() {
    // vertical-align:top → top of atomic aligns with top of line box (offset.top ≈ 0).
    let (doc, block, atomic) = make_atomic_inline_block(50.0, 60.0, VerticalAlign::Top);
    let frag = do_layout(&doc, block);

    let boxes = collect_box_fragments(&frag);
    let atomic_frags: Vec<_> = boxes.iter().filter(|f| f.node_id == atomic).collect();
    assert!(!atomic_frags.is_empty(), "Should have atomic inline fragment");

    let top = atomic_frags[0].offset.top.to_f32();
    assert!(
        top.abs() < 2.0,
        "vertical-align:top atomic should be near top of line, got {top}"
    );
}

#[test]
fn atomic_inline_valign_bottom_at_line_bottom() {
    // vertical-align:bottom → bottom of atomic aligns with bottom of line box.
    let (doc, block, atomic) = make_atomic_inline_block(50.0, 60.0, VerticalAlign::Bottom);
    let frag = do_layout(&doc, block);

    let line_box = &frag.children[0];
    let boxes = collect_box_fragments(&frag);
    let atomic_frags: Vec<_> = boxes.iter().filter(|f| f.node_id == atomic).collect();
    assert!(!atomic_frags.is_empty(), "Should have atomic inline fragment");

    let atomic_bottom = atomic_frags[0].offset.top + atomic_frags[0].size.height;
    let diff = (line_box.size.height - atomic_bottom).to_f32().abs();
    assert!(
        diff < 2.0,
        "vertical-align:bottom atomic bottom ({:?}) should be near line bottom ({:?}), diff={diff}",
        atomic_bottom,
        line_box.size.height
    );
}

#[test]
fn atomic_inline_valign_middle_centered_around_baseline() {
    // vertical-align:middle → item centered around baseline - x_height/2.
    // The top offset should differ from both top-aligned and bottom-aligned.
    let (doc_mid, block_mid, atomic_mid) =
        make_atomic_inline_block(50.0, 60.0, VerticalAlign::Middle);
    let frag_mid = do_layout(&doc_mid, block_mid);
    let boxes_mid = collect_box_fragments(&frag_mid);
    let mid_frag: Vec<_> = boxes_mid.iter().filter(|f| f.node_id == atomic_mid).collect();

    let (doc_bl, block_bl, atomic_bl) =
        make_atomic_inline_block(50.0, 60.0, VerticalAlign::Baseline);
    let frag_bl = do_layout(&doc_bl, block_bl);
    let boxes_bl = collect_box_fragments(&frag_bl);
    let bl_frag: Vec<_> = boxes_bl.iter().filter(|f| f.node_id == atomic_bl).collect();

    assert!(!mid_frag.is_empty() && !bl_frag.is_empty());
    // Middle-aligned top offset should differ from baseline-aligned.
    assert_ne!(
        mid_frag[0].offset.top, bl_frag[0].offset.top,
        "Middle-aligned ({:?}) should differ from baseline-aligned ({:?})",
        mid_frag[0].offset.top, bl_frag[0].offset.top
    );
}
