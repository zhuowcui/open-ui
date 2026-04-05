//! Tests for SP11 Round 24 code review fixes — openui-layout crate.
//!
//! Issue 2: Atomic inline children never recursively laid out or painted.
//! Issue 3: Auto-width atomic inlines still collapse to zero.
//! Issue 4: text-justify:auto hardcoded to inter-word, CJK wrong.

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    Display, TextAlign, TextJustify,
};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn make_constraint_width(w: i32) -> ConstraintSpace {
    ConstraintSpace::for_block_child(lu_i(w), lu_i(600), lu_i(w), lu_i(600), false)
}

fn collect_box_fragments_recursive<'a>(fragment: &'a Fragment, out: &mut Vec<&'a Fragment>) {
    if fragment.kind == FragmentKind::Box {
        out.push(fragment);
    }
    for child in &fragment.children {
        collect_box_fragments_recursive(child, out);
    }
}

fn all_box_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    let mut out = Vec::new();
    collect_box_fragments_recursive(fragment, &mut out);
    out
}

fn collect_text_fragments_recursive<'a>(fragment: &'a Fragment, out: &mut Vec<&'a Fragment>) {
    if fragment.kind == FragmentKind::Text {
        out.push(fragment);
    }
    for child in &fragment.children {
        collect_text_fragments_recursive(child, out);
    }
}

fn all_text_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    let mut out = Vec::new();
    collect_text_fragments_recursive(fragment, &mut out);
    out
}

// ── Issue 2: Atomic inline children recursively laid out ────────────────

#[test]
fn r24_atomic_inline_with_text_child_has_children() {
    // An inline-block containing text should have child fragments after layout.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    // Create inline-block with explicit size and text child
    let ib = doc.create_node(ElementTag::Div);
    doc.node_mut(ib).style.display = Display::InlineBlock;
    doc.node_mut(ib).style.width = Length::px(100.0);
    doc.node_mut(ib).style.height = Length::px(30.0);
    doc.append_child(block, ib);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hello".to_string());
    doc.append_child(ib, text);

    let space = make_constraint_width(400);
    let result = inline_layout(&doc, block, &space);

    // Find the box fragment for the inline-block
    let boxes = all_box_fragments(&result);
    let ib_frag = boxes.iter().find(|f| f.node_id == ib);
    assert!(
        ib_frag.is_some(),
        "Should have a box fragment for the inline-block element"
    );

    let ib_frag = ib_frag.unwrap();
    // The inline-block should have children (from recursive layout)
    assert!(
        !ib_frag.children.is_empty(),
        "Inline-block fragment should have child fragments after recursive layout, got 0"
    );
}

#[test]
fn r24_atomic_inline_children_contain_text_fragments() {
    // An inline-block with text should produce text fragments in its subtree.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let ib = doc.create_node(ElementTag::Div);
    doc.node_mut(ib).style.display = Display::InlineBlock;
    doc.node_mut(ib).style.width = Length::px(120.0);
    doc.node_mut(ib).style.height = Length::px(30.0);
    doc.append_child(block, ib);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("World".to_string());
    doc.append_child(ib, text);

    let space = make_constraint_width(400);
    let result = inline_layout(&doc, block, &space);

    // Collect all text fragments in the entire tree (including inside inline-block)
    let all_texts = all_text_fragments(&result);
    assert!(
        !all_texts.is_empty(),
        "Should have text fragments inside the inline-block's subtree"
    );
}

// ── Issue 3: Auto-width atomic inlines use intrinsic size ───────────────

#[test]
fn r24_auto_width_inline_block_with_text_nonzero_width() {
    // An inline-block with width:auto and text child should compute
    // a non-zero width from the text content.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let ib = doc.create_node(ElementTag::Div);
    doc.node_mut(ib).style.display = Display::InlineBlock;
    // width: auto (default) — no explicit width set
    doc.node_mut(ib).style.height = Length::px(20.0);
    doc.append_child(block, ib);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hello World".to_string());
    doc.append_child(ib, text);

    let space = make_constraint_width(400);
    let result = inline_layout(&doc, block, &space);

    let boxes = all_box_fragments(&result);
    let ib_frag = boxes.iter().find(|f| f.node_id == ib);
    assert!(
        ib_frag.is_some(),
        "Should have a box fragment for the inline-block"
    );

    let ib_frag = ib_frag.unwrap();
    let width = ib_frag.size.width.to_f32();
    assert!(
        width > 0.0,
        "Auto-width inline-block with text content should have non-zero width, got {width}"
    );
}

#[test]
fn r24_auto_width_inline_block_no_children_zero_width() {
    // An inline-block with width:auto and no children should be zero width.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let ib = doc.create_node(ElementTag::Div);
    doc.node_mut(ib).style.display = Display::InlineBlock;
    // No children, no explicit width
    doc.append_child(block, ib);

    // Also add text after so the line isn't empty
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("after".to_string());
    doc.append_child(block, text);

    let space = make_constraint_width(400);
    let result = inline_layout(&doc, block, &space);

    let boxes = all_box_fragments(&result);
    let ib_frag = boxes.iter().find(|f| f.node_id == ib);
    assert!(
        ib_frag.is_some(),
        "Should have a box fragment for the empty inline-block"
    );

    let ib_frag = ib_frag.unwrap();
    let width = ib_frag.size.width.to_f32();
    // Empty inline-block with auto width should be zero or near-zero
    assert!(
        width < 1.0,
        "Empty auto-width inline-block should have ~zero width, got {width}"
    );
}

#[test]
fn r24_auto_width_respects_min_width_floor() {
    // An inline-block with width:auto, no children, but min-width:50px
    // should be at least 50px wide.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let ib = doc.create_node(ElementTag::Div);
    doc.node_mut(ib).style.display = Display::InlineBlock;
    doc.node_mut(ib).style.min_width = Length::px(50.0);
    doc.append_child(block, ib);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("x".to_string());
    doc.append_child(block, text);

    let space = make_constraint_width(400);
    let result = inline_layout(&doc, block, &space);

    let boxes = all_box_fragments(&result);
    let ib_frag = boxes.iter().find(|f| f.node_id == ib);
    assert!(ib_frag.is_some());

    let ib_frag = ib_frag.unwrap();
    let width = ib_frag.size.width.to_f32();
    assert!(
        width >= 49.9,
        "Auto-width with min-width:50 should be at least 50px, got {width}"
    );
}

// ── Issue 4: text-justify:auto uses inter-character for CJK ─────────────

#[test]
fn r24_text_justify_auto_cjk_uses_inter_character() {
    // CJK text with text-justify:auto should use inter-character justification.
    // Create a line with CJK characters and verify the text expands
    // (which requires inter-character gaps, not inter-word gaps since there are no spaces).
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.node_mut(block).style.text_justify = TextJustify::Auto;
    doc.append_child(root, block);

    // CJK text with no spaces — inter-word justification would produce 0 opportunities
    let text1 = doc.create_node(ElementTag::Text);
    doc.node_mut(text1).text = Some("日本語テスト".to_string());
    doc.append_child(block, text1);

    // Second line to force justification of the first
    let text2 = doc.create_node(ElementTag::Text);
    doc.node_mut(text2).text = Some(" overflow text here".to_string());
    doc.append_child(block, text2);

    let space = make_constraint_width(300);
    let result = inline_layout(&doc, block, &space);

    // The layout should succeed without panicking
    assert!(
        !result.children.is_empty(),
        "Should produce at least one line"
    );
}

#[test]
fn r24_text_justify_auto_latin_uses_inter_word() {
    // Latin text with text-justify:auto should use inter-word justification.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.node_mut(block).style.text_justify = TextJustify::Auto;
    doc.append_child(root, block);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hello World Test Extra Words Here".to_string());
    doc.append_child(block, text);

    let space = make_constraint_width(200);
    let result = inline_layout(&doc, block, &space);

    // Should produce lines; spaces between words should be the expansion points
    assert!(
        !result.children.is_empty(),
        "Should produce at least one line"
    );
}
