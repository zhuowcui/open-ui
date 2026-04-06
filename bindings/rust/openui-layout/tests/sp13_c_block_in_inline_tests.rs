//! SP13 Phase C integration tests: block-in-inline layout.
//!
//! CSS 2.2 §9.2.1.1: When a block-level element appears inside inline
//! content, the inline formatting context is split into anonymous block
//! boxes around the block element.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::block::block_layout;
use openui_layout::ConstraintSpace;
use openui_style::Display;

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu(px: f32) -> LayoutUnit {
    LayoutUnit::from_f32(px)
}

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn space(width: i32, height: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(lu_i(width), lu_i(height))
}

fn add_text(doc: &mut Document, parent: NodeId, content: &str) -> NodeId {
    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some(content.to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(parent, t);
    t
}

// ── Tests ───────────────────────────────────────────────────────────────

#[test]
fn simple_block_in_inline() {
    let mut doc = Document::new();
    let root = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(container, span);

    add_text(&mut doc, span, "Hello ");

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(50.0);
    doc.append_child(span, block);

    add_text(&mut doc, span, " World");

    let frag = block_layout(&doc, container, &space(500, 500));

    assert!(
        frag.children.len() >= 3,
        "Expected at least 3 children for block-in-inline, got {}",
        frag.children.len()
    );
    assert_eq!(frag.children[1].size.height, lu(50.0), "Block child should be 50px tall");
    let first_line_h = frag.children[0].size.height;
    let last_line_h = frag.children[2].size.height;
    let expected_total = first_line_h + lu(50.0) + last_line_h;
    assert_eq!(frag.size.height, expected_total, "Total height mismatch");
}

#[test]
fn multiple_blocks_in_inline() {
    let mut doc = Document::new();
    let root = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(container, span);

    add_text(&mut doc, span, "A");

    let block_b = doc.create_node(ElementTag::Div);
    doc.node_mut(block_b).style.display = Display::Block;
    doc.node_mut(block_b).style.height = Length::px(30.0);
    doc.append_child(span, block_b);

    add_text(&mut doc, span, "C");

    let block_d = doc.create_node(ElementTag::Div);
    doc.node_mut(block_d).style.display = Display::Block;
    doc.node_mut(block_d).style.height = Length::px(40.0);
    doc.append_child(span, block_d);

    add_text(&mut doc, span, "E");

    let frag = block_layout(&doc, container, &space(500, 500));

    assert!(frag.children.len() >= 5, "Expected at least 5 children, got {}", frag.children.len());
    assert_eq!(frag.children[1].size.height, lu(30.0));
    assert_eq!(frag.children[3].size.height, lu(40.0));
}

#[test]
fn nested_inline_with_block() {
    let mut doc = Document::new();
    let root = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);

    let outer_span = doc.create_node(ElementTag::Span);
    doc.node_mut(outer_span).style.display = Display::Inline;
    doc.append_child(container, outer_span);

    let inner_span = doc.create_node(ElementTag::Span);
    doc.node_mut(inner_span).style.display = Display::Inline;
    doc.append_child(outer_span, inner_span);

    add_text(&mut doc, inner_span, "text");

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(60.0);
    doc.append_child(inner_span, block);

    let frag = block_layout(&doc, container, &space(500, 500));

    assert!(frag.children.len() >= 2, "Expected at least 2 children, got {}", frag.children.len());
    let has_60px = frag.children.iter().any(|c| c.size.height == lu(60.0));
    assert!(has_60px, "Should contain 60px block fragment");
}

#[test]
fn empty_before_block() {
    let mut doc = Document::new();
    let root = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(container, span);

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(50.0);
    doc.append_child(span, block);

    add_text(&mut doc, span, "text");

    let frag = block_layout(&doc, container, &space(500, 500));

    let has_50px = frag.children.iter().any(|c| c.size.height == lu(50.0));
    assert!(has_50px, "Should contain 50px block fragment");
    assert!(frag.size.height >= lu(50.0));
}

#[test]
fn empty_after_block() {
    let mut doc = Document::new();
    let root = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(container, span);

    add_text(&mut doc, span, "text");

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(50.0);
    doc.append_child(span, block);

    let frag = block_layout(&doc, container, &space(500, 500));

    let has_50px = frag.children.iter().any(|c| c.size.height == lu(50.0));
    assert!(has_50px, "Should contain 50px block fragment");
    assert!(frag.children.len() >= 2);
}

#[test]
fn block_with_height() {
    let mut doc = Document::new();
    let root = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(container, span);

    add_text(&mut doc, span, "X");

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(100.0);
    doc.append_child(span, block);

    add_text(&mut doc, span, "Y");

    let frag = block_layout(&doc, container, &space(500, 500));

    assert!(frag.size.height > lu(100.0), "Total height should exceed 100px, got {:?}", frag.size.height);
    for i in 1..frag.children.len() {
        let prev_bottom = frag.children[i - 1].offset.top + frag.children[i - 1].size.height;
        let curr_top = frag.children[i].offset.top;
        assert!(curr_top >= prev_bottom, "Children must not overlap at index {}", i);
    }
}

#[test]
fn block_in_inline_stacking_order() {
    let mut doc = Document::new();
    let root = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(container, span);

    add_text(&mut doc, span, "A");

    let b1 = doc.create_node(ElementTag::Div);
    doc.node_mut(b1).style.display = Display::Block;
    doc.node_mut(b1).style.height = Length::px(20.0);
    doc.append_child(span, b1);

    add_text(&mut doc, span, "B");

    let b2 = doc.create_node(ElementTag::Div);
    doc.node_mut(b2).style.display = Display::Block;
    doc.node_mut(b2).style.height = Length::px(30.0);
    doc.append_child(span, b2);

    add_text(&mut doc, span, "C");

    let frag = block_layout(&doc, container, &space(500, 500));

    for i in 1..frag.children.len() {
        let prev_end = frag.children[i - 1].offset.top + frag.children[i - 1].size.height;
        let curr_start = frag.children[i].offset.top;
        assert!(curr_start >= prev_end, "Children must not overlap at index {}", i);
    }
    assert!(frag.size.height >= lu(50.0), "Total height should be at least 50px");
}

#[test]
fn block_in_inline_with_padding() {
    let mut doc = Document::new();
    let root = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.padding_left = Length::px(10.0);
    doc.node_mut(span).style.padding_right = Length::px(10.0);
    doc.append_child(container, span);

    add_text(&mut doc, span, "before");

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(50.0);
    doc.append_child(span, block);

    add_text(&mut doc, span, "after");

    let frag = block_layout(&doc, container, &space(500, 500));

    assert!(frag.children.len() >= 3, "Expected at least 3 children, got {}", frag.children.len());
    let has_50px = frag.children.iter().any(|c| c.size.height == lu(50.0));
    assert!(has_50px, "Should contain 50px block");
    assert!(frag.size.height > lu(50.0), "Total height should exceed 50px");
}
