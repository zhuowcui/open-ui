//! SP12 E1 — Intrinsic block sizing tests.
//!
//! Tests for min-content, max-content, shrink-to-fit, replaced elements,
//! and block size from content.

use openui_geometry::{LayoutUnit, Length};
use openui_dom::{Document, ElementTag, NodeId};
use openui_style::Display;
use openui_layout::intrinsic_sizing::{
    IntrinsicSizes, compute_intrinsic_block_sizes, compute_intrinsic_inline_sizes,
    compute_block_size_from_content, shrink_to_fit_inline_size,
    compute_replaced_intrinsic_sizes,
};

fn lu(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

/// Create a simple document with a block parent and one block child
/// whose width/height are set to the given pixel values.
fn doc_with_one_child(child_width: f32, child_height: f32) -> (Document, NodeId) {
    let mut doc = Document::new();
    let root = doc.root();

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(child_width);
    doc.node_mut(child).style.height = Length::px(child_height);
    doc.append_child(parent, child);

    (doc, parent)
}

/// Create a document with a block parent and multiple block children
/// with the given (width, height) pairs.
fn doc_with_children(sizes: &[(f32, f32)]) -> (Document, NodeId) {
    let mut doc = Document::new();
    let root = doc.root();

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);

    for &(w, h) in sizes {
        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.width = Length::px(w);
        doc.node_mut(child).style.height = Length::px(h);
        doc.append_child(parent, child);
    }

    (doc, parent)
}

// ── 1. IntrinsicSizes struct construction ────────────────────────────────

#[test]
fn intrinsic_sizes_struct_construction() {
    let sizes = IntrinsicSizes {
        min_content_inline_size: lu(100),
        max_content_inline_size: lu(200),
        min_content_block_size: lu(50),
        max_content_block_size: lu(80),
    };
    assert_eq!(sizes.min_content_inline_size, lu(100));
    assert_eq!(sizes.max_content_inline_size, lu(200));
    assert_eq!(sizes.min_content_block_size, lu(50));
    assert_eq!(sizes.max_content_block_size, lu(80));
}

// ── 2. Zero-content element ─────────────────────────────────────────────

#[test]
fn zero_content_element() {
    let mut doc = Document::new();
    let root = doc.root();

    let empty = doc.create_node(ElementTag::Div);
    doc.node_mut(empty).style.display = Display::Block;
    doc.append_child(root, empty);

    let sizes = compute_intrinsic_block_sizes(&doc, empty);
    assert_eq!(sizes.min_content_inline_size, lu(0));
    assert_eq!(sizes.max_content_inline_size, lu(0));
    assert_eq!(sizes.min_content_block_size, lu(0));
    assert_eq!(sizes.max_content_block_size, lu(0));
}

// ── 3. Min-content of single child block ────────────────────────────────

#[test]
fn min_content_single_child() {
    let (doc, parent) = doc_with_one_child(150.0, 50.0);

    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    // Single child with width=150 → min-content inline = 150
    assert_eq!(sizes.min_content_inline_size, LayoutUnit::from_f32(150.0));
}

// ── 4. Min-content of multiple children (takes maximum) ─────────────────

#[test]
fn min_content_multiple_children_takes_max() {
    let (doc, parent) = doc_with_children(&[
        (100.0, 30.0),
        (200.0, 40.0),
        (150.0, 20.0),
    ]);

    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    // Min-content inline = max(100, 200, 150) = 200
    assert_eq!(sizes.min_content_inline_size, LayoutUnit::from_f32(200.0));
}

// ── 5. Max-content of single child ──────────────────────────────────────

#[test]
fn max_content_single_child() {
    let (doc, parent) = doc_with_one_child(250.0, 80.0);

    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    // Max-content inline = 250
    assert_eq!(sizes.max_content_inline_size, LayoutUnit::from_f32(250.0));
}

// ── 6. Max-content with padding and border ──────────────────────────────

#[test]
fn max_content_with_padding_border() {
    let mut doc = Document::new();
    let root = doc.root();

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    // Add padding: 10px all around
    doc.node_mut(parent).style.padding_top = Length::px(10.0);
    doc.node_mut(parent).style.padding_right = Length::px(10.0);
    doc.node_mut(parent).style.padding_bottom = Length::px(10.0);
    doc.node_mut(parent).style.padding_left = Length::px(10.0);
    // Add border: 5px all around
    doc.node_mut(parent).style.border_top_width = 5;
    doc.node_mut(parent).style.border_right_width = 5;
    doc.node_mut(parent).style.border_bottom_width = 5;
    doc.node_mut(parent).style.border_left_width = 5;
    doc.node_mut(parent).style.border_top_style = openui_style::BorderStyle::Solid;
    doc.node_mut(parent).style.border_right_style = openui_style::BorderStyle::Solid;
    doc.node_mut(parent).style.border_bottom_style = openui_style::BorderStyle::Solid;
    doc.node_mut(parent).style.border_left_style = openui_style::BorderStyle::Solid;
    doc.append_child(root, parent);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(100.0);
    doc.node_mut(child).style.height = Length::px(40.0);
    doc.append_child(parent, child);

    let sizes = compute_intrinsic_block_sizes(&doc, parent);

    // Child contributes 100 inline, plus parent padding (10+10) + border (5+5) = 130
    let expected_inline = LayoutUnit::from_f32(100.0)
        + LayoutUnit::from_f32(20.0)  // padding inline
        + LayoutUnit::from_i32(10);    // border inline
    assert_eq!(sizes.max_content_inline_size, expected_inline);

    // Block: child 40 + parent padding (10+10) + border (5+5) = 70
    let expected_block = LayoutUnit::from_f32(40.0)
        + LayoutUnit::from_f32(20.0)  // padding block
        + LayoutUnit::from_i32(10);    // border block
    assert_eq!(sizes.max_content_block_size, expected_block);
}

// ── 7. Shrink-to-fit: available > max-content (use max) ─────────────────

#[test]
fn shrink_to_fit_available_exceeds_max() {
    let min = lu(50);
    let max = lu(200);
    let available = lu(300);
    // min(max(50, 300), 200) = min(300, 200) = 200
    assert_eq!(shrink_to_fit_inline_size(min, max, available), lu(200));
}

// ── 8. Shrink-to-fit: min-content < available < max-content (use available) ──

#[test]
fn shrink_to_fit_available_between_min_max() {
    let min = lu(80);
    let max = lu(400);
    let available = lu(250);
    // min(max(80, 250), 400) = min(250, 400) = 250
    assert_eq!(shrink_to_fit_inline_size(min, max, available), lu(250));
}

// ── 9. Shrink-to-fit: available < min-content (use min) ─────────────────

#[test]
fn shrink_to_fit_available_below_min() {
    let min = lu(120);
    let max = lu(300);
    let available = lu(60);
    // min(max(120, 60), 300) = min(120, 300) = 120
    assert_eq!(shrink_to_fit_inline_size(min, max, available), lu(120));
}

// ── 10. Block size from content (sum of children) ────────────────────────

#[test]
fn block_size_from_content_sum() {
    let mut doc = Document::new();
    let root = doc.root();

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);

    // Children margin boxes: 50, 30, 40
    let child_boxes = [lu(50), lu(30), lu(40)];
    let result = compute_block_size_from_content(&doc, parent, &child_boxes);
    // Sum = 50 + 30 + 40 = 120
    assert_eq!(result, lu(120));
}

// ── 11. Replaced element intrinsic sizes ─────────────────────────────────

#[test]
fn replaced_element_default_size() {
    use openui_style::ComputedStyle;

    // No explicit width/height → default 300×150
    let style = ComputedStyle::initial();
    let sizes = compute_replaced_intrinsic_sizes(&style);

    assert_eq!(sizes.min_content_inline_size, lu(300));
    assert_eq!(sizes.max_content_inline_size, lu(300));
    assert_eq!(sizes.min_content_block_size, lu(150));
    assert_eq!(sizes.max_content_block_size, lu(150));
}

#[test]
fn replaced_element_explicit_both() {
    use openui_style::ComputedStyle;

    let mut style = ComputedStyle::initial();
    style.width = Length::px(640.0);
    style.height = Length::px(480.0);

    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, LayoutUnit::from_f32(640.0));
    assert_eq!(sizes.min_content_block_size, LayoutUnit::from_f32(480.0));
}

// ── 12. Replaced with aspect ratio ──────────────────────────────────────

#[test]
fn replaced_with_width_only_applies_aspect_ratio() {
    use openui_style::ComputedStyle;

    let mut style = ComputedStyle::initial();
    style.width = Length::px(600.0);
    // height auto → derived from default 2:1 aspect ratio (300:150)
    // height = 600 * 150 / 300 = 300

    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, LayoutUnit::from_f32(600.0));
    assert_eq!(sizes.min_content_block_size, LayoutUnit::from_f32(300.0));
}

#[test]
fn replaced_with_height_only_applies_aspect_ratio() {
    use openui_style::ComputedStyle;

    let mut style = ComputedStyle::initial();
    style.height = Length::px(300.0);
    // width auto → derived from default 2:1 aspect ratio (300:150)
    // width = 300 * 300 / 150 = 600

    let sizes = compute_replaced_intrinsic_sizes(&style);
    assert_eq!(sizes.min_content_inline_size, LayoutUnit::from_f32(600.0));
    assert_eq!(sizes.min_content_block_size, LayoutUnit::from_f32(300.0));
}

// ── 13. Auto block size with margins ─────────────────────────────────────

#[test]
fn auto_block_size_with_margin_collapsing() {
    let mut doc = Document::new();
    let root = doc.root();

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);

    // Two children with adjacent margins that should collapse.
    let child1 = doc.create_node(ElementTag::Div);
    doc.node_mut(child1).style.display = Display::Block;
    doc.node_mut(child1).style.height = Length::px(50.0);
    doc.node_mut(child1).style.margin_bottom = Length::px(20.0);
    doc.append_child(parent, child1);

    let child2 = doc.create_node(ElementTag::Div);
    doc.node_mut(child2).style.display = Display::Block;
    doc.node_mut(child2).style.height = Length::px(30.0);
    doc.node_mut(child2).style.margin_top = Length::px(30.0);
    doc.append_child(parent, child2);

    // Margin boxes: child1 = 50 + 20(margin-bottom), child2 = 30(margin-top) + 30
    // Before collapse: total = (50+20) + (30+30) = 130
    // After collapse: adjacent margins 20 and 30 → collapse by min(20,30) = 20
    // Collapsed total = 130 - 20 = 110
    let child_boxes = [
        LayoutUnit::from_f32(70.0),  // child1: 50 + 20 margin-bottom
        LayoutUnit::from_f32(60.0),  // child2: 30 margin-top + 30
    ];
    let result = compute_block_size_from_content(&doc, parent, &child_boxes);
    assert_eq!(result, lu(110));
}

// ── 14. Nested block intrinsic sizing ────────────────────────────────────

#[test]
fn nested_block_intrinsic_sizing() {
    let mut doc = Document::new();
    let root = doc.root();

    // grandparent → parent → child(width=200, height=60)
    let grandparent = doc.create_node(ElementTag::Div);
    doc.node_mut(grandparent).style.display = Display::Block;
    doc.append_child(root, grandparent);

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(grandparent, parent);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(200.0);
    doc.node_mut(child).style.height = Length::px(60.0);
    doc.append_child(parent, child);

    let sizes = compute_intrinsic_block_sizes(&doc, grandparent);
    // The width propagates up: grandparent → parent → child(200)
    assert_eq!(sizes.min_content_inline_size, LayoutUnit::from_f32(200.0));
    assert_eq!(sizes.max_content_inline_size, LayoutUnit::from_f32(200.0));
    // Block size propagates: 60
    assert_eq!(sizes.max_content_block_size, LayoutUnit::from_f32(60.0));
}

// ── 15. Inline intrinsic sizes for text ──────────────────────────────────

#[test]
fn inline_intrinsic_sizes_text() {
    let mut doc = Document::new();
    let root = doc.root();

    let text_node = doc.create_node(ElementTag::Text);
    doc.node_mut(text_node).text = Some("hello world".to_string());
    doc.append_child(root, text_node);

    let sizes = compute_intrinsic_inline_sizes(&doc, text_node);
    // min-content: widest word "hello" or "world" = 5 * 8 = 40
    assert_eq!(sizes.min, LayoutUnit::from_f32(40.0));
    // max-content: "hello world" = 11 * 8 = 88
    assert_eq!(sizes.max, LayoutUnit::from_f32(88.0));
}

// ── 16. Multiple children block sizes sum in block axis ──────────────────

#[test]
fn multiple_children_block_size_sums() {
    let (doc, parent) = doc_with_children(&[
        (100.0, 30.0),
        (200.0, 40.0),
        (150.0, 20.0),
    ]);

    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    // Block size = sum of children heights: 30 + 40 + 20 = 90
    assert_eq!(
        sizes.max_content_block_size,
        LayoutUnit::from_f32(30.0) + LayoutUnit::from_f32(40.0) + LayoutUnit::from_f32(20.0)
    );
}

// ── 17. Shrink-to-fit boundary: min == max == available ──────────────────

#[test]
fn shrink_to_fit_all_equal() {
    let v = lu(100);
    assert_eq!(shrink_to_fit_inline_size(v, v, v), v);
}

// ── 18. Display:none children are skipped ────────────────────────────────

#[test]
fn display_none_children_skipped() {
    let mut doc = Document::new();
    let root = doc.root();

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);

    // Visible child: 100×50
    let visible = doc.create_node(ElementTag::Div);
    doc.node_mut(visible).style.display = Display::Block;
    doc.node_mut(visible).style.width = Length::px(100.0);
    doc.node_mut(visible).style.height = Length::px(50.0);
    doc.append_child(parent, visible);

    // Hidden child: 300×200 (should not contribute)
    let hidden = doc.create_node(ElementTag::Div);
    doc.node_mut(hidden).style.display = Display::None;
    doc.node_mut(hidden).style.width = Length::px(300.0);
    doc.node_mut(hidden).style.height = Length::px(200.0);
    doc.append_child(parent, hidden);

    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    // Only visible child contributes
    assert_eq!(sizes.min_content_inline_size, LayoutUnit::from_f32(100.0));
    assert_eq!(sizes.max_content_inline_size, LayoutUnit::from_f32(100.0));
    assert_eq!(sizes.max_content_block_size, LayoutUnit::from_f32(50.0));
}

// ── 19. Child with min-width constraint ──────────────────────────────────

#[test]
fn child_with_min_width_constraint() {
    let mut doc = Document::new();
    let root = doc.root();

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(root, parent);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(80.0);
    doc.node_mut(child).style.min_width = Length::px(120.0);
    doc.node_mut(child).style.height = Length::px(30.0);
    doc.append_child(parent, child);

    let sizes = compute_intrinsic_block_sizes(&doc, parent);
    // Child width=80 but min-width=120 → clamped to 120
    assert_eq!(sizes.min_content_inline_size, LayoutUnit::from_f32(120.0));
    assert_eq!(sizes.max_content_inline_size, LayoutUnit::from_f32(120.0));
}

// ── 20. Block size from content with min-height ──────────────────────────

#[test]
fn block_size_from_content_clamped_by_min_height() {
    let mut doc = Document::new();
    let root = doc.root();

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.min_height = Length::px(200.0);
    doc.append_child(root, parent);

    // Children total only 50px
    let child_boxes = [lu(50)];
    let result = compute_block_size_from_content(&doc, parent, &child_boxes);
    // Clamped by min-height: max(50, 200) = 200
    assert_eq!(result, lu(200));
}

// ── 21. Block size from content with max-height ──────────────────────────

#[test]
fn block_size_from_content_clamped_by_max_height() {
    let mut doc = Document::new();
    let root = doc.root();

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.max_height = Length::px(80.0);
    doc.append_child(root, parent);

    // Children total 150px
    let child_boxes = [lu(60), lu(90)];
    let result = compute_block_size_from_content(&doc, parent, &child_boxes);
    // Clamped by max-height: min(150, 80) = 80
    assert_eq!(result, lu(80));
}
