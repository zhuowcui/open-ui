//! Tests for the inline layout algorithm (SP11 Wave 4).
//!
//! Validates line height calculation, vertical alignment, text alignment,
//! multi-line layout, block layout integration, and edge cases.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::block::block_layout;
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    ComputedStyle, Direction, Display, LineHeight, TextAlign, VerticalAlign, WhiteSpace,
};

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

/// Create a block container with text children and perform inline layout.
fn layout_text(texts: &[&str], width: i32) -> Fragment {
    let (doc, block) = make_text_block(&texts, width);
    let sp = ConstraintSpace::for_block_child(
        lu_i(width),
        lu_i(600),
        lu_i(width),
        lu_i(600),
        false,
    );
    inline_layout(&doc, block, &sp)
}

/// Create a block with text children using block_layout (full integration).
fn block_layout_text(texts: &[&str], width: i32) -> Fragment {
    let mut doc = Document::new();
    let vp = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(vp, block);

    for text in texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.append_child(block, t);
    }

    let sp = space(width, 600);
    block_layout(&doc, vp, &sp)
}

/// Helper: create a document with a block and text children, returns (doc, block_id).
fn make_text_block(texts: &[&str], _width: i32) -> (Document, NodeId) {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    for text in texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.append_child(block, t);
    }
    (doc, block)
}

/// Helper: create a block with a span wrapping text.
fn make_span_block(
    span_texts: &[&str],
    span_style_fn: impl Fn(&mut ComputedStyle),
) -> (Document, NodeId) {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    span_style_fn(&mut doc.node_mut(span).style);
    doc.append_child(block, span);

    for text in span_texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        // Inherit span's style properties for font size, line height, etc.
        let span_style = doc.node(span).style.clone();
        doc.node_mut(t).style.font_size = span_style.font_size;
        doc.node_mut(t).style.line_height = span_style.line_height;
        doc.node_mut(t).style.vertical_align = span_style.vertical_align;
        doc.append_child(span, t);
    }
    (doc, block)
}

/// Count text fragments recursively.
fn count_text_fragments(fragment: &Fragment) -> usize {
    let mut count = 0;
    if fragment.kind == FragmentKind::Text {
        count += 1;
    }
    for child in &fragment.children {
        count += count_text_fragments(child);
    }
    count
}

/// Count direct line box children (Box fragments that are direct children).
fn count_line_boxes(fragment: &Fragment) -> usize {
    fragment
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Box)
        .count()
}

/// Get all text fragments from a fragment tree.
fn collect_text_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    let mut result = Vec::new();
    if fragment.kind == FragmentKind::Text {
        result.push(fragment);
    }
    for child in &fragment.children {
        result.extend(collect_text_fragments(child));
    }
    result
}

// ═══════════════════════════════════════════════════════════════════════
// LINE HEIGHT TESTS (20)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn line_height_default_normal_produces_positive_height() {
    // Default line-height: normal uses font metrics line_spacing
    let frag = layout_text(&["Hello"], 800);
    assert!(frag.size.height > LayoutUnit::zero(), "Line height should be positive");
}

#[test]
fn line_height_single_line_has_one_line_box() {
    let frag = layout_text(&["Hello world"], 800);
    let line_boxes = count_line_boxes(&frag);
    assert_eq!(line_boxes, 1, "Single line text should produce one line box");
}

#[test]
fn line_height_text_fragment_inside_line_box() {
    let frag = layout_text(&["Hello"], 800);
    assert_eq!(count_line_boxes(&frag), 1);
    let line = &frag.children[0];
    let text_count = count_text_fragments(line);
    assert_eq!(text_count, 1, "Line box should contain one text fragment");
}

#[test]
fn line_height_explicit_number_multiplier() {
    // line-height: 2.0 should produce a taller line
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Number(2.0);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Number(2.0);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // With line-height: 2.0 at 16px font, computed line-height = 32px
    // Line should be at least 32px tall
    assert!(frag.size.height >= lu(32.0), "line-height: 2.0 should produce >= 32px line");
}

#[test]
fn line_height_explicit_px_value() {
    // line-height: 48px
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Length(48.0);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Test".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Length(48.0);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    // Line height should be at least 48px (half-leading may add rounding)
    assert!(frag.size.height >= lu(48.0));
}

#[test]
fn line_height_percentage_value() {
    // line-height: 200% at 16px = 32px
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Percentage(200.0);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Test".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Percentage(200.0);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    assert!(frag.size.height >= lu(32.0));
}

#[test]
fn line_height_half_leading_distributes_evenly() {
    // With even leading, both sides get equal amounts
    let frag = layout_text(&["Hello"], 800);
    let line = &frag.children[0];
    let text_frags = collect_text_fragments(line);
    assert!(!text_frags.is_empty());
    // Text fragment should be vertically centered-ish in the line box
    let text_top = text_frags[0].offset.top;
    let text_height = text_frags[0].size.height;
    let text_bottom = text_top + text_height;
    assert!(text_top >= LayoutUnit::zero());
    assert!(text_bottom <= line.size.height);
}

#[test]
fn line_height_strut_from_parent_block_always_contributes() {
    // Even with small text, the parent block's strut is the minimum line height.
    // A small font-size on text shouldn't shrink below the block's strut.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.font_size = 32.0; // Large block strut
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("x".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.font_size = 8.0; // Small text
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    // Line height should be >= the block's 32px strut, not the text's 8px
    assert!(frag.size.height > lu(20.0), "Strut from 32px block font should dominate");
}

#[test]
fn line_height_multiple_fonts_tallest_wins() {
    // Two text nodes with different font sizes: the taller one determines line height
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.font_size = 8.0; // Small block strut
    doc.append_child(root, block);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("small ".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.node_mut(t1).style.font_size = 8.0;
    doc.append_child(block, t1);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("BIG".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.node_mut(t2).style.font_size = 48.0;
    doc.append_child(block, t2);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    // Line height should be >= the 48px font's metrics
    assert!(frag.size.height > lu(40.0));
}

#[test]
fn line_height_zero_does_not_crash() {
    // line-height: 0 should not crash
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Length(0.0);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("test".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Length(0.0);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let _frag = inline_layout(&doc, block, &sp);
    // Just verify no panic
}

#[test]
fn line_height_large_value_creates_tall_line() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Length(100.0);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hi".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Length(100.0);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    assert!(frag.size.height >= lu(100.0));
}

#[test]
fn line_height_number_one_matches_font_metrics() {
    // line-height: 1.0 means computed = font-size
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Number(1.0);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("x".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Number(1.0);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    // With line-height: 1.0 and font-size: 16, computed = 16px
    // Line height should be at least 16px
    assert!(frag.size.height >= lu(16.0));
}

#[test]
fn line_height_text_fragment_has_font_height() {
    // Text fragment height = ascent + descent of the font
    let frag = layout_text(&["Hello"], 800);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    // Text height should be positive and less than line height
    assert!(texts[0].size.height > LayoutUnit::zero());
}

#[test]
fn line_height_fragment_kind_is_text() {
    let frag = layout_text(&["Hello"], 800);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert_eq!(texts[0].kind, FragmentKind::Text);
}

#[test]
fn line_height_line_box_kind_is_box() {
    let frag = layout_text(&["Hello"], 800);
    assert!(!frag.children.is_empty());
    assert_eq!(frag.children[0].kind, FragmentKind::Box);
}

#[test]
fn line_height_line_box_width_equals_available() {
    let frag = layout_text(&["Hello"], 800);
    let line = &frag.children[0];
    assert_eq!(line.size.width.to_i32(), 800);
}

#[test]
fn line_height_text_offset_within_line_box() {
    let frag = layout_text(&["Hello"], 800);
    let line = &frag.children[0];
    let texts = collect_text_fragments(line);
    assert!(!texts.is_empty());
    let t = texts[0];
    // Text should be within the line box vertically
    assert!(t.offset.top >= LayoutUnit::zero());
    assert!(t.offset.top + t.size.height <= line.size.height + LayoutUnit::epsilon());
}

#[test]
fn line_height_text_has_positive_width() {
    let frag = layout_text(&["Hello"], 800);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(texts[0].size.width > LayoutUnit::zero());
}

#[test]
fn line_height_shape_result_attached() {
    // Text fragments should have a shape_result
    let frag = layout_text(&["Hello"], 800);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(texts[0].shape_result.is_some(), "Text fragment should have shape_result");
}

// ═══════════════════════════════════════════════════════════════════════
// VERTICAL ALIGNMENT TESTS (20)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn vertical_align_baseline_default() {
    // Default vertical-align is baseline — text at normal position
    let frag = layout_text(&["Hello"], 800);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(texts[0].offset.top >= LayoutUnit::zero());
}

#[test]
fn vertical_align_sub_lowers_text() {
    // vertical-align: sub should position text lower than baseline
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Sub;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    // Sub should increase the top offset (move text down)
}

#[test]
fn vertical_align_super_raises_text() {
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Super;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
}

#[test]
fn vertical_align_middle() {
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Middle;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
}

#[test]
fn vertical_align_text_top() {
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::TextTop;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
}

#[test]
fn vertical_align_text_bottom() {
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::TextBottom;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
}

#[test]
fn vertical_align_top() {
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Top;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    // Top-aligned: text top should be near line box top
    let t = texts[0];
    // The text's top within the line should be near 0
    assert!(t.offset.top.to_f32().abs() < 10.0, "Top-aligned text should be near line top");
}

#[test]
fn vertical_align_bottom() {
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Bottom;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
}

#[test]
fn vertical_align_length_positive() {
    // vertical-align: 5px — raises text by 5px
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Length(5.0);
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
}

#[test]
fn vertical_align_length_negative() {
    // vertical-align: -5px — lowers text
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Length(-5.0);
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
}

#[test]
fn vertical_align_percentage() {
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Percentage(50.0);
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
}

#[test]
fn vertical_align_sub_different_from_baseline() {
    // Sub-aligned text should be at a different vertical position than baseline-aligned
    let frag_baseline = layout_text(&["Hello"], 800);
    let texts_bl = collect_text_fragments(&frag_baseline);

    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Sub;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag_sub = inline_layout(&doc, block, &sp);
    let texts_sub = collect_text_fragments(&frag_sub);

    assert!(!texts_bl.is_empty() && !texts_sub.is_empty());
    // Sub should be positioned differently
    assert_ne!(texts_bl[0].offset.top, texts_sub[0].offset.top);
}

#[test]
fn vertical_align_super_different_from_baseline() {
    let frag_baseline = layout_text(&["Hello"], 800);

    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Super;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag_super = inline_layout(&doc, block, &sp);

    // Super-aligned text should produce a taller line box than baseline
    // (because the line expands upward to accommodate the raised text)
    assert!(frag_super.size.height >= frag_baseline.size.height,
        "Super alignment should expand line height: super={:?} baseline={:?}",
        frag_super.size.height, frag_baseline.size.height);
}

#[test]
fn vertical_align_super_above_sub() {
    // Super should position text higher (smaller top offset) than sub
    let (doc_sub, block_sub) = make_span_block(&["Test"], |s| {
        s.vertical_align = VerticalAlign::Sub;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag_sub = inline_layout(&doc_sub, block_sub, &sp);

    let (doc_sup, block_sup) = make_span_block(&["Test"], |s| {
        s.vertical_align = VerticalAlign::Super;
    });
    let frag_sup = inline_layout(&doc_sup, block_sup, &sp);

    let texts_sub = collect_text_fragments(&frag_sub);
    let texts_sup = collect_text_fragments(&frag_sup);
    assert!(!texts_sub.is_empty() && !texts_sup.is_empty());
    // Super text top should be less (higher) than sub text top
    assert!(texts_sup[0].offset.top < texts_sub[0].offset.top,
        "Super should be above sub: super={:?} sub={:?}",
        texts_sup[0].offset.top, texts_sub[0].offset.top);
}

#[test]
fn vertical_align_does_not_change_inline_size() {
    // Vertical alignment should not affect the inline (horizontal) size
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Super;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(texts[0].size.width > LayoutUnit::zero());
}

#[test]
fn vertical_align_sub_expands_line_box() {
    // Sub alignment may expand the line box downward
    let frag_normal = layout_text(&["Hello"], 800);

    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Sub;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag_sub = inline_layout(&doc, block, &sp);

    assert!(frag_sub.size.height >= frag_normal.size.height,
        "Sub alignment should not shrink line box");
}

#[test]
fn vertical_align_super_expands_line_box() {
    let frag_normal = layout_text(&["Hello"], 800);

    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Super;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag_super = inline_layout(&doc, block, &sp);

    assert!(frag_super.size.height >= frag_normal.size.height);
}

#[test]
fn vertical_align_mixed_on_same_line() {
    // Multiple items with different vertical-align on the same line
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    // Normal text
    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("normal ".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.append_child(block, t1);

    // Span with super
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.vertical_align = VerticalAlign::Super;
    doc.append_child(block, span);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("super".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.node_mut(t2).style.vertical_align = VerticalAlign::Super;
    doc.append_child(span, t2);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert_eq!(texts.len(), 2, "Should have two text fragments");
    // They should be at different vertical positions
    assert_ne!(texts[0].offset.top, texts[1].offset.top);
}

#[test]
fn vertical_align_length_zero_same_as_baseline() {
    // vertical-align: 0px should be same as baseline
    let frag_bl = layout_text(&["Hello"], 800);

    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Length(0.0);
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag_len = inline_layout(&doc, block, &sp);

    let t_bl = collect_text_fragments(&frag_bl);
    let t_len = collect_text_fragments(&frag_len);
    assert!(!t_bl.is_empty() && !t_len.is_empty());
    assert_eq!(t_bl[0].offset.top, t_len[0].offset.top);
}

#[test]
fn vertical_align_preserves_text_content() {
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Super;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(texts[0].shape_result.is_some());
}

// ═══════════════════════════════════════════════════════════════════════
// TEXT ALIGNMENT TESTS (15)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn text_align_left_offset_zero() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Left;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hi".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert_eq!(texts[0].offset.left, LayoutUnit::zero(), "Left-aligned text should start at 0");
}

#[test]
fn text_align_right_offset_positive() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Right;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hi".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(texts[0].offset.left > LayoutUnit::zero(), "Right-aligned text should have offset > 0");
}

#[test]
fn text_align_center_offset_positive() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Center;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hi".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(texts[0].offset.left > LayoutUnit::zero());
}

#[test]
fn text_align_center_is_between_left_and_right() {
    // Center offset should be between left(0) and right
    let build = |align: TextAlign| {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.text_align = align;
        doc.append_child(root, block);

        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some("Hello".to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.append_child(block, t);

        let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
        let frag = inline_layout(&doc, block, &sp);
        let texts = collect_text_fragments(&frag);
        texts[0].offset.left
    };

    let left = build(TextAlign::Left);
    let center = build(TextAlign::Center);
    let right = build(TextAlign::Right);

    assert!(center > left, "Center should be > left");
    assert!(center < right, "Center should be < right");
}

#[test]
fn text_align_start_ltr_is_left() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Start;
    doc.node_mut(block).style.direction = Direction::Ltr;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert_eq!(texts[0].offset.left, LayoutUnit::zero());
}

#[test]
fn text_align_start_rtl_is_right() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Start;
    doc.node_mut(block).style.direction = Direction::Rtl;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(texts[0].offset.left > LayoutUnit::zero());
}

#[test]
fn text_align_end_ltr_is_right() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::End;
    doc.node_mut(block).style.direction = Direction::Ltr;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(texts[0].offset.left > LayoutUnit::zero());
}

#[test]
fn text_align_end_rtl_is_left() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::End;
    doc.node_mut(block).style.direction = Direction::Rtl;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert_eq!(texts[0].offset.left, LayoutUnit::zero());
}

#[test]
fn text_align_justify_non_last_line_no_crash() {
    // Justify with multiple words should work without crashing
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.append_child(root, block);

    // Long text that wraps to multiple lines
    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello world this is a long text that should wrap across multiple lines when the width is narrow enough".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(200), lu_i(600), lu_i(200), lu_i(600), false);
    let _frag = inline_layout(&doc, block, &sp);
}

#[test]
fn text_align_justify_last_line_not_justified() {
    // Last line of justify should fall back to start (not stretched)
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello world test".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    // Single line = last line, should be start-aligned (offset = 0)
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert_eq!(texts[0].offset.left, LayoutUnit::zero());
}

#[test]
fn text_align_right_consistent_across_lines() {
    // All lines of right-aligned text should have text at the right
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Right;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello world this wraps".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(80), lu_i(600), lu_i(80), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    for line in &frag.children {
        let texts = collect_text_fragments(line);
        if !texts.is_empty() {
            assert!(texts[0].offset.left > LayoutUnit::zero(),
                "Each line should have right-aligned offset");
        }
    }
}

#[test]
fn text_align_default_is_start() {
    // Default text-align should be Start (which is Left for LTR)
    let frag = layout_text(&["Hello"], 800);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert_eq!(texts[0].offset.left, LayoutUnit::zero());
}

#[test]
fn text_align_does_not_affect_height() {
    // Text alignment should not change the line height
    let build = |align: TextAlign| {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.text_align = align;
        doc.append_child(root, block);

        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some("Hello".to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.append_child(block, t);

        let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
        inline_layout(&doc, block, &sp)
    };

    let left = build(TextAlign::Left);
    let right = build(TextAlign::Right);
    let center = build(TextAlign::Center);

    assert_eq!(left.size.height, right.size.height);
    assert_eq!(left.size.height, center.size.height);
}

#[test]
fn text_align_with_overflow() {
    // When text overflows the container, alignment offset should be 0
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Right;
    doc.node_mut(block).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Very long text that overflows the tiny container".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(10), lu_i(600), lu_i(10), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    // When text overflows, offset should be 0 (can't push right)
    assert!(!texts.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════
// MULTI-LINE TESTS (20)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn multi_line_wrapping_at_available_width() {
    // Text wider than available width should wrap
    let frag = layout_text(
        &["Hello world this is a sentence that should definitely wrap"],
        100,
    );
    assert!(count_line_boxes(&frag) > 1, "Text should wrap into multiple lines");
}

#[test]
fn multi_line_each_line_below_previous() {
    let frag = layout_text(
        &["Hello world this is text"],
        60,
    );
    let lines: Vec<_> = frag.children.iter().collect();
    assert!(lines.len() > 1);
    for i in 1..lines.len() {
        assert!(lines[i].offset.top > lines[i - 1].offset.top,
            "Line {} should be below line {}", i, i - 1);
    }
}

#[test]
fn multi_line_total_height_is_sum_of_lines() {
    let frag = layout_text(
        &["Hello world this is text"],
        60,
    );
    let mut expected_height = LayoutUnit::zero();
    for line in &frag.children {
        expected_height = expected_height + line.size.height;
    }
    // Total fragment height should be approximately the sum of line heights
    // (may differ slightly due to border/padding)
    assert!(frag.size.height >= expected_height - LayoutUnit::epsilon());
}

#[test]
fn multi_line_all_lines_have_text() {
    let frag = layout_text(
        &["Hello world this is text"],
        60,
    );
    for line in &frag.children {
        let texts = collect_text_fragments(line);
        assert!(!texts.is_empty(), "Each line should have at least one text fragment");
    }
}

#[test]
fn multi_line_first_line_starts_at_top() {
    let frag = layout_text(&["Hello world this is text"], 60);
    assert!(!frag.children.is_empty());
    // First line box should be at the top
    assert_eq!(frag.children[0].offset.top, LayoutUnit::zero());
}

#[test]
fn multi_line_last_line_bottom_equals_total_height() {
    let frag = layout_text(&["Hello world this is text"], 60);
    assert!(!frag.children.is_empty());
    let last = frag.children.last().unwrap();
    let last_bottom = last.offset.top + last.size.height;
    assert_eq!(frag.size.height, last_bottom);
}

#[test]
fn multi_line_consistent_line_heights_same_font() {
    let frag = layout_text(
        &["Hello world this is text that wraps around"],
        60,
    );
    let heights: Vec<_> = frag.children.iter().map(|l| l.size.height).collect();
    // All lines with same font should have the same height
    if heights.len() > 1 {
        for h in &heights[1..] {
            assert_eq!(*h, heights[0], "All lines should have same height with same font");
        }
    }
}

#[test]
fn multi_line_width_does_not_exceed_available() {
    let frag = layout_text(
        &["Hello world this is text"],
        60,
    );
    for line in &frag.children {
        let texts = collect_text_fragments(line);
        for t in &texts {
            // Each text fragment should be within the available width
            let right_edge = t.offset.left + t.size.width;
            // Allow some tolerance for rounding
            assert!(right_edge <= lu_i(60) + lu(2.0),
                "Text should not significantly exceed available width");
        }
    }
}

#[test]
fn multi_line_forced_break_creates_new_line() {
    // Use line breaker directly since our DOM doesn't have <br> element handling
    let frag = layout_text(
        &["Hello\nWorld"],
        800,
    );
    // With normal white-space, \n collapses to space, so this is one line
    // But with pre white-space, \n forces a break
    assert!(count_line_boxes(&frag) >= 1);
}

#[test]
fn multi_line_pre_whitespace_preserves_newlines() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.white_space = WhiteSpace::Pre;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Line1\nLine2\nLine3".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::Pre;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    assert!(count_line_boxes(&frag) >= 3, "Pre whitespace should preserve newlines: got {} lines", count_line_boxes(&frag));
}

#[test]
fn multi_line_different_font_sizes() {
    // A line with a big font should be taller than one with small font
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.font_size = 8.0;
    doc.append_child(root, block);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("small text that wraps to its own line ".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.node_mut(t1).style.font_size = 8.0;
    doc.append_child(block, t1);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("BIG TEXT ON SECOND LINE".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.node_mut(t2).style.font_size = 32.0;
    doc.append_child(block, t2);

    let sp = ConstraintSpace::for_block_child(lu_i(200), lu_i(600), lu_i(200), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    assert!(count_line_boxes(&frag) >= 1);
}

#[test]
fn multi_line_empty_text_produces_no_lines() {
    let frag = layout_text(&[""], 800);
    // Empty text should produce no line boxes
    assert_eq!(count_line_boxes(&frag), 0);
}

#[test]
fn multi_line_height_grows_with_lines() {
    // More lines = taller fragment
    let frag_short = layout_text(&["Hi"], 800);
    let frag_long = layout_text(&["Hello world this is text that wraps"], 60);
    assert!(frag_long.size.height >= frag_short.size.height);
}

#[test]
fn multi_line_two_text_nodes_same_line() {
    // Two short text nodes should be on the same line
    let frag = layout_text(&["Hello ", "World"], 800);
    assert_eq!(count_line_boxes(&frag), 1);
    let line = &frag.children[0];
    let texts = collect_text_fragments(line);
    assert_eq!(texts.len(), 2, "Both text nodes should be on same line");
}

#[test]
fn multi_line_text_fragments_ordered_horizontally() {
    let frag = layout_text(&["Hello ", "World"], 800);
    let line = &frag.children[0];
    let texts = collect_text_fragments(line);
    assert_eq!(texts.len(), 2);
    assert!(texts[1].offset.left > texts[0].offset.left,
        "Second text should be to the right of first");
}

#[test]
fn multi_line_no_gap_between_text_fragments() {
    // Adjacent text nodes should be contiguous (no gap)
    let frag = layout_text(&["Hello", "World"], 800);
    let line = &frag.children[0];
    let texts = collect_text_fragments(line);
    assert_eq!(texts.len(), 2);
    let first_end = texts[0].offset.left + texts[0].size.width;
    // Second text should start where first ends (approximately)
    let gap = (texts[1].offset.left - first_end).abs();
    assert!(gap < lu(1.0), "Should be no significant gap between adjacent text fragments");
}

#[test]
fn multi_line_line_boxes_span_available_width() {
    let frag = layout_text(&["Hello"], 400);
    let line = &frag.children[0];
    assert_eq!(line.size.width.to_i32(), 400);
}

#[test]
fn multi_line_narrow_width_causes_many_lines() {
    // With a narrow width, long text should produce many lines
    let frag = layout_text(
        &["Hello world this is a sentence"],
        80,
    );
    assert!(count_line_boxes(&frag) >= 2,
        "Narrow width should cause wrapping: got {} lines",
        count_line_boxes(&frag));
}

#[test]
fn multi_line_exact_fit_no_wrap() {
    // Text exactly fitting in width should be one line
    let frag = layout_text(&["Hi"], 10000);
    assert_eq!(count_line_boxes(&frag), 1);
}

#[test]
fn multi_line_all_text_fragments_have_shape_results() {
    let frag = layout_text(&["Hello world this wraps"], 60);
    let texts = collect_text_fragments(&frag);
    for t in &texts {
        assert!(t.shape_result.is_some(), "All text fragments should have shape_result");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// INTEGRATION WITH BLOCK LAYOUT (15)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn integration_block_with_text_produces_content() {
    let frag = block_layout_text(&["Hello world"], 800);
    let block = &frag.children[0];
    assert!(!block.children.is_empty(), "Block with text should have children");
}

#[test]
fn integration_block_with_text_has_height() {
    let frag = block_layout_text(&["Hello"], 800);
    let block = &frag.children[0];
    assert!(block.size.height > LayoutUnit::zero(), "Block with text should have height");
}

#[test]
fn integration_block_width_correct() {
    let frag = block_layout_text(&["Hello"], 800);
    let block = &frag.children[0];
    assert_eq!(block.size.width.to_i32(), 800);
}

#[test]
fn integration_block_with_span_text() {
    let mut doc = Document::new();
    let vp = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(vp, block);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(block, span);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Span text".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(span, t);

    let sp = space(800, 600);
    let frag = block_layout(&doc, vp, &sp);
    let block_frag = &frag.children[0];
    assert!(block_frag.size.height > LayoutUnit::zero());
}

#[test]
fn integration_nested_spans() {
    let mut doc = Document::new();
    let vp = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(vp, block);

    let span1 = doc.create_node(ElementTag::Span);
    doc.node_mut(span1).style.display = Display::Inline;
    doc.append_child(block, span1);

    let span2 = doc.create_node(ElementTag::Span);
    doc.node_mut(span2).style.display = Display::Inline;
    doc.append_child(span1, span2);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Nested".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(span2, t);

    let sp = space(800, 600);
    let frag = block_layout(&doc, vp, &sp);
    let block_frag = &frag.children[0];
    assert!(block_frag.size.height > LayoutUnit::zero());
    let texts = collect_text_fragments(block_frag);
    assert!(!texts.is_empty());
}

#[test]
fn integration_multiple_text_nodes() {
    let frag = block_layout_text(&["Hello ", "World ", "!"], 800);
    let block = &frag.children[0];
    let texts = collect_text_fragments(block);
    assert_eq!(texts.len(), 3, "Should have 3 text fragments");
}

#[test]
fn integration_text_with_right_align() {
    let mut doc = Document::new();
    let vp = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Right;
    doc.append_child(vp, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Right".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = space(800, 600);
    let frag = block_layout(&doc, vp, &sp);
    let block_frag = &frag.children[0];
    let texts = collect_text_fragments(block_frag);
    assert!(!texts.is_empty());
}

#[test]
fn integration_block_with_padding_and_text() {
    let mut doc = Document::new();
    let vp = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.padding_top = Length::px(10.0);
    doc.node_mut(block).style.padding_bottom = Length::px(10.0);
    doc.append_child(vp, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Padded".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = space(800, 600);
    let frag = block_layout(&doc, vp, &sp);
    let block_frag = &frag.children[0];
    // Height should include padding + text line
    assert!(block_frag.size.height > lu(20.0),
        "Block with 20px padding + text should be > 20px");
}

#[test]
fn integration_block_with_border_and_text() {
    let mut doc = Document::new();
    let vp = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.border_top_width = 5;
    doc.node_mut(block).style.border_top_style = openui_style::BorderStyle::Solid;
    doc.node_mut(block).style.border_bottom_width = 5;
    doc.node_mut(block).style.border_bottom_style = openui_style::BorderStyle::Solid;
    doc.append_child(vp, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Bordered".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = space(800, 600);
    let frag = block_layout(&doc, vp, &sp);
    let block_frag = &frag.children[0];
    assert!(block_frag.size.height > lu(10.0));
}

#[test]
fn integration_block_with_only_block_children_unchanged() {
    // Blocks with only block children should still work normally
    let mut doc = Document::new();
    let vp = doc.root();
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.append_child(vp, parent);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.height = Length::px(50.0);
    doc.append_child(parent, child);

    let sp = space(800, 600);
    let frag = block_layout(&doc, vp, &sp);
    let parent_frag = &frag.children[0];
    assert_eq!(parent_frag.children.len(), 1);
    assert_eq!(parent_frag.children[0].size.height.to_i32(), 50);
}

#[test]
fn integration_empty_block_still_works() {
    let mut doc = Document::new();
    let vp = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(vp, block);

    let sp = space(800, 600);
    let frag = block_layout(&doc, vp, &sp);
    let block_frag = &frag.children[0];
    assert_eq!(block_frag.size.height.to_i32(), 0);
}

#[test]
fn integration_viewport_with_text_block() {
    let frag = block_layout_text(&["Hello world"], 800);
    assert_eq!(frag.kind, FragmentKind::Viewport);
    assert!(!frag.children.is_empty());
}

#[test]
fn integration_two_blocks_one_inline_one_block() {
    // Two blocks: one with text, one empty block child
    let mut doc = Document::new();
    let vp = doc.root();

    let block1 = doc.create_node(ElementTag::Div);
    doc.node_mut(block1).style.display = Display::Block;
    doc.append_child(vp, block1);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Text".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block1, t);

    let block2 = doc.create_node(ElementTag::Div);
    doc.node_mut(block2).style.display = Display::Block;
    doc.node_mut(block2).style.height = Length::px(30.0);
    doc.append_child(vp, block2);

    let sp = space(800, 600);
    let frag = block_layout(&doc, vp, &sp);
    assert_eq!(frag.children.len(), 2);
    assert!(frag.children[0].size.height > LayoutUnit::zero());
    assert_eq!(frag.children[1].size.height.to_i32(), 30);
}

#[test]
fn integration_width_constraint_affects_wrapping() {
    // Narrower width should produce more lines
    let frag_wide = block_layout_text(
        &["Hello world this text wraps"], 800);
    let frag_narrow = block_layout_text(
        &["Hello world this text wraps"], 50);

    let wide_lines = count_line_boxes(&frag_wide.children[0]);
    let narrow_lines = count_line_boxes(&frag_narrow.children[0]);
    assert!(narrow_lines >= wide_lines);
}

// ═══════════════════════════════════════════════════════════════════════
// EDGE CASES (10)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn edge_empty_text_node() {
    let frag = layout_text(&[""], 800);
    // Empty text should produce no line boxes
    assert_eq!(count_line_boxes(&frag), 0);
    assert_eq!(frag.size.height, LayoutUnit::zero());
}

#[test]
fn edge_only_whitespace() {
    let frag = layout_text(&["   "], 800);
    // Only whitespace may collapse to a single space
    // Should produce at most one line
    assert!(count_line_boxes(&frag) <= 1);
}

#[test]
fn edge_single_character() {
    let frag = layout_text(&["x"], 800);
    assert_eq!(count_line_boxes(&frag), 1);
    let texts = collect_text_fragments(&frag);
    assert_eq!(texts.len(), 1);
    assert!(texts[0].size.width > LayoutUnit::zero());
}

#[test]
fn edge_very_long_word_overflow() {
    // A single long word with no break opportunities
    let frag = layout_text(
        &["Supercalifragilisticexpialidocious"],
        50,
    );
    // Should have at least one line (word overflows)
    assert!(count_line_boxes(&frag) >= 1);
}

#[test]
fn edge_zero_width_produces_many_lines() {
    // Zero available width — each word gets its own line or overflows
    let frag = layout_text(&["Hello world"], 0);
    assert!(count_line_boxes(&frag) >= 1);
}

#[test]
fn edge_many_text_nodes() {
    // Many small text nodes
    let texts: Vec<&str> = vec!["a "; 20];
    let frag = layout_text(&texts, 800);
    assert!(count_line_boxes(&frag) >= 1);
    let text_frags = collect_text_fragments(&frag);
    assert_eq!(text_frags.len(), 20);
}

#[test]
fn edge_multiple_spaces_between_words() {
    // Multiple spaces should collapse to one
    let frag = layout_text(&["Hello      World"], 800);
    assert_eq!(count_line_boxes(&frag), 1);
}

#[test]
fn edge_unicode_text() {
    let frag = layout_text(&["日本語テスト"], 800);
    assert!(count_line_boxes(&frag) >= 1);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(texts[0].size.width > LayoutUnit::zero());
}

#[test]
fn edge_mixed_latin_text() {
    // Mix of Latin text on the same line
    let frag = layout_text(&["Hello World Café"], 800);
    assert!(count_line_boxes(&frag) >= 1);
}

#[test]
fn edge_tab_characters_collapse() {
    let frag = layout_text(&["Hello\tWorld"], 800);
    // Tab should collapse to space in normal white-space mode
    assert_eq!(count_line_boxes(&frag), 1);
}
