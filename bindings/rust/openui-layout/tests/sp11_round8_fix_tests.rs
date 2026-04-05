//! Tests for SP11 Round 8 fixes.
//!
//! Issue 1: Atomic inline vertical-align percentage should use the element's
//!          own line-height as the percentage basis, not the box height.
//! Issue 3: Atomic inline vertical-align middle should include x_height offset
//!          in line metrics so the item doesn't protrude above the line box.

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{Display, LineHeight, VerticalAlign};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu(px: f32) -> LayoutUnit {
    LayoutUnit::from_f32(px)
}

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn collect_box_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    let mut result = Vec::new();
    for child in &fragment.children {
        if child.kind == FragmentKind::Box {
            result.push(child);
        }
        result.extend(collect_box_fragments(child));
    }
    result
}

// ── Issue 1: Atomic inline vertical-align percentage uses line-height ───

#[test]
fn atomic_inline_valign_percentage_uses_line_height_not_box_height() {
    // An atomic inline with height:100px and line-height:20px should use 20px
    // (not 100px) as the percentage basis for vertical-align: 50%.
    // Shift = 20 * 50 / 100 = 10px (not 50px).
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    // Atomic inline: 100px tall, line-height: 20px, vertical-align: 50%
    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(50.0);
    doc.node_mut(atomic).style.height = Length::px(100.0);
    doc.node_mut(atomic).style.line_height = LineHeight::Length(20.0);
    doc.node_mut(atomic).style.vertical_align = VerticalAlign::Percentage(50.0);
    doc.append_child(block, atomic);

    let sp = ConstraintSpace::for_block_child(lu_i(400), lu_i(600), lu_i(400), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    assert!(!frag.children.is_empty(), "Should have a line box");
    let line_box = &frag.children[0];

    // If the bug were present (using item_height=100 as basis), shift = 50,
    // and line box height would need to be >= 150px.
    // With the fix (using line-height=20 as basis), shift = 10,
    // and line box height should be around 110px.
    assert!(
        line_box.size.height < lu(140.0),
        "Line box height {:?} should reflect line-height-based percentage, not box-height-based",
        line_box.size.height
    );
    // But it should still be at least the item height + the small shift.
    assert!(
        line_box.size.height >= lu(100.0),
        "Line box height {:?} should be at least item height (100px)",
        line_box.size.height
    );
}

#[test]
fn atomic_inline_valign_percentage_different_line_heights() {
    // Two atomic inlines, same height, different line-heights.
    // With line-height as the percentage basis, they should produce different shifts.
    let mut doc = Document::new();
    let root = doc.root();

    // Block A: atomic with line-height: 10px
    let block_a = doc.create_node(ElementTag::Div);
    doc.node_mut(block_a).style.display = Display::Block;
    doc.append_child(root, block_a);

    let atomic_a = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic_a).style.display = Display::InlineBlock;
    doc.node_mut(atomic_a).style.width = Length::px(40.0);
    doc.node_mut(atomic_a).style.height = Length::px(60.0);
    doc.node_mut(atomic_a).style.line_height = LineHeight::Length(10.0);
    doc.node_mut(atomic_a).style.vertical_align = VerticalAlign::Percentage(100.0);
    doc.append_child(block_a, atomic_a);

    // Block B: atomic with line-height: 80px
    let block_b = doc.create_node(ElementTag::Div);
    doc.node_mut(block_b).style.display = Display::Block;
    doc.append_child(root, block_b);

    let atomic_b = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic_b).style.display = Display::InlineBlock;
    doc.node_mut(atomic_b).style.width = Length::px(40.0);
    doc.node_mut(atomic_b).style.height = Length::px(60.0);
    doc.node_mut(atomic_b).style.line_height = LineHeight::Length(80.0);
    doc.node_mut(atomic_b).style.vertical_align = VerticalAlign::Percentage(100.0);
    doc.append_child(block_b, atomic_b);

    let sp = ConstraintSpace::for_block_child(lu_i(400), lu_i(600), lu_i(400), lu_i(600), false);
    let frag_a = inline_layout(&doc, block_a, &sp);
    let frag_b = inline_layout(&doc, block_b, &sp);

    let line_a = &frag_a.children[0];
    let line_b = &frag_b.children[0];

    // Block A: shift = 10 * 100/100 = 10px → line height ≈ 70px
    // Block B: shift = 80 * 100/100 = 80px → line height ≈ 140px
    // They should differ significantly.
    assert!(
        line_b.size.height > line_a.size.height + lu(20.0),
        "Larger line-height ({:?}) should produce larger line box than smaller line-height ({:?})",
        line_b.size.height,
        line_a.size.height,
    );
}

// ── Issue 3: Atomic inline vertical-align middle includes x_height ──────

#[test]
fn atomic_inline_middle_with_text_no_overflow_above() {
    // A 200px-tall image + 16px text. With correct middle alignment
    // (including x_height), the image should not overflow above the line box.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    // Normal text
    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("text".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    // Large atomic inline with middle alignment
    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(50.0);
    doc.node_mut(atomic).style.height = Length::px(200.0);
    doc.node_mut(atomic).style.vertical_align = VerticalAlign::Middle;
    doc.append_child(block, atomic);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    assert!(!frag.children.is_empty(), "Should have a line box");
    let line_box = &frag.children[0];

    // Find the atomic inline fragment in the line box.
    let boxes = collect_box_fragments(line_box);
    assert!(!boxes.is_empty(), "Should have atomic inline box fragment");

    let atomic_frag = boxes[0];
    // The atomic inline's top must not be above the line box top (offset >= 0).
    assert!(
        atomic_frag.offset.top >= lu(0.0),
        "Atomic inline top {:?} should not overflow above line box (should be >= 0)",
        atomic_frag.offset.top,
    );

    // The line box must be tall enough for the full 200px item.
    assert!(
        line_box.size.height >= lu(200.0),
        "Line box {:?} should be >= 200px to fit the middle-aligned atomic inline",
        line_box.size.height,
    );
}

#[test]
fn atomic_inline_middle_small_centered() {
    // A small inline-block (20px) centered with vertical-align:middle.
    // The line box ascent should be larger than descent (x_height shifts upward).
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    // Normal text to establish strut metrics
    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hx".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let atomic = doc.create_node(ElementTag::Div);
    doc.node_mut(atomic).style.display = Display::InlineBlock;
    doc.node_mut(atomic).style.width = Length::px(20.0);
    doc.node_mut(atomic).style.height = Length::px(20.0);
    doc.node_mut(atomic).style.vertical_align = VerticalAlign::Middle;
    doc.append_child(block, atomic);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    assert!(!frag.children.is_empty(), "Should have a line box");
    let line_box = &frag.children[0];

    let boxes = collect_box_fragments(line_box);
    assert!(!boxes.is_empty(), "Should have atomic inline box fragment");

    let atomic_frag = boxes[0];
    // The atomic inline should be positioned within the line box.
    assert!(
        atomic_frag.offset.top >= lu(0.0),
        "Atomic inline should not overflow above line box, top={:?}",
        atomic_frag.offset.top,
    );
    let bottom = atomic_frag.offset.top + atomic_frag.size.height;
    assert!(
        bottom <= line_box.size.height,
        "Atomic inline bottom {:?} should be within line box height {:?}",
        bottom,
        line_box.size.height,
    );
}
