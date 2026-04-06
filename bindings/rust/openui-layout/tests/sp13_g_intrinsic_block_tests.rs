// SP13 Phase G: Intrinsic block-size from inline content tests
//
// CSS 2.1 §10.6.3 / CSS Sizing 3 §5: Block containers with inline content
// must compute their intrinsic block-size by running inline layout at the
// min-content and max-content widths to determine line wrapping height.
//
// This was deferred from SP12 — inline children previously contributed
// zero block-size to intrinsic sizing.

use openui_dom::{Document, ElementTag};
use openui_geometry::LayoutUnit;
use openui_layout::intrinsic_sizing::{compute_intrinsic_block_sizes, IntrinsicSizes};

fn lu(v: f32) -> LayoutUnit {
    LayoutUnit::from_f32(v)
}

#[test]
fn text_contributes_nonzero_block_size() {
    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = openui_style::Display::Block;
    doc.node_mut(div).style.font_size = 16.0;
    doc.append_child(vp, div);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hello world".to_string());
    doc.append_child(div, text);

    let sizes = compute_intrinsic_block_sizes(&doc, div);

    assert!(
        sizes.min_content_block_size > lu(0.0),
        "min_content_block_size should be > 0 for text content, got {}",
        sizes.min_content_block_size.to_f32()
    );
    assert!(
        sizes.max_content_block_size > lu(0.0),
        "max_content_block_size should be > 0 for text content, got {}",
        sizes.max_content_block_size.to_f32()
    );
}

#[test]
fn wrapping_text_min_block_size_greater_than_max() {
    // At min-content width, text wraps more → taller.
    // At max-content width, text fits on fewer lines → shorter.
    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = openui_style::Display::Block;
    doc.node_mut(div).style.font_size = 16.0;
    doc.append_child(vp, div);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some(
        "This is a longer sentence that should wrap differently at different widths".to_string()
    );
    doc.append_child(div, text);

    let sizes = compute_intrinsic_block_sizes(&doc, div);

    assert!(
        sizes.min_content_block_size >= sizes.max_content_block_size,
        "min-content block-size ({}) should be >= max-content block-size ({}) because text wraps more at narrow widths",
        sizes.min_content_block_size.to_f32(),
        sizes.max_content_block_size.to_f32()
    );
}

#[test]
fn single_word_min_equals_max_block_size() {
    // Single word: no wrapping possible → same height at both widths
    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = openui_style::Display::Block;
    doc.node_mut(div).style.font_size = 16.0;
    doc.append_child(vp, div);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Word".to_string());
    doc.append_child(div, text);

    let sizes = compute_intrinsic_block_sizes(&doc, div);

    assert_eq!(
        sizes.min_content_block_size, sizes.max_content_block_size,
        "Single word should have same min/max block-size (one line)"
    );
}

#[test]
fn empty_block_zero_block_size() {
    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = openui_style::Display::Block;
    doc.append_child(vp, div);

    let sizes = compute_intrinsic_block_sizes(&doc, div);

    assert_eq!(
        sizes.min_content_block_size,
        lu(0.0),
        "Empty block should have zero intrinsic block-size"
    );
}

#[test]
fn block_size_includes_border_padding() {
    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(div);
        node.style.display = openui_style::Display::Block;
        node.style.font_size = 16.0;
        node.style.padding_top = openui_geometry::Length::px(10.0);
        node.style.padding_bottom = openui_geometry::Length::px(10.0);
        node.style.border_top_width = 2;
        node.style.border_bottom_width = 2;
    }
    doc.append_child(vp, div);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Content".to_string());
    doc.append_child(div, text);

    let sizes = compute_intrinsic_block_sizes(&doc, div);

    // Block-size should be at least 24px (10+10 padding + 2+2 border) + text height
    assert!(
        sizes.max_content_block_size.to_f32() >= 24.0,
        "Block-size should include border+padding. Got {}",
        sizes.max_content_block_size.to_f32()
    );
}

#[test]
fn nested_block_with_text_has_nonzero_block_size() {
    // Outer block → inner block with text → outer should have nonzero block-size
    let mut doc = Document::new();
    let vp = doc.root();

    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = openui_style::Display::Block;
    doc.append_child(vp, outer);

    let inner = doc.create_node(ElementTag::Div);
    doc.node_mut(inner).style.display = openui_style::Display::Block;
    doc.node_mut(inner).style.font_size = 16.0;
    doc.append_child(outer, inner);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Nested text".to_string());
    doc.append_child(inner, text);

    let sizes = compute_intrinsic_block_sizes(&doc, outer);

    assert!(
        sizes.min_content_block_size > lu(0.0),
        "Nested block with text should have nonzero min_content_block_size, got {}",
        sizes.min_content_block_size.to_f32()
    );
}

#[test]
fn larger_font_produces_taller_block_size() {
    let mut doc = Document::new();
    let vp = doc.root();

    // Small font
    let div_small = doc.create_node(ElementTag::Div);
    doc.node_mut(div_small).style.display = openui_style::Display::Block;
    doc.node_mut(div_small).style.font_size = 10.0;
    doc.append_child(vp, div_small);
    let text_small = doc.create_node(ElementTag::Text);
    doc.node_mut(text_small).text = Some("Text".to_string());
    doc.append_child(div_small, text_small);

    // Large font
    let div_large = doc.create_node(ElementTag::Div);
    doc.node_mut(div_large).style.display = openui_style::Display::Block;
    doc.node_mut(div_large).style.font_size = 40.0;
    doc.append_child(vp, div_large);
    let text_large = doc.create_node(ElementTag::Text);
    doc.node_mut(text_large).text = Some("Text".to_string());
    doc.append_child(div_large, text_large);

    let sizes_small = compute_intrinsic_block_sizes(&doc, div_small);
    let sizes_large = compute_intrinsic_block_sizes(&doc, div_large);

    assert!(
        sizes_large.max_content_block_size > sizes_small.max_content_block_size,
        "Larger font ({}) should produce taller block-size than smaller font ({})",
        sizes_large.max_content_block_size.to_f32(),
        sizes_small.max_content_block_size.to_f32()
    );
}
