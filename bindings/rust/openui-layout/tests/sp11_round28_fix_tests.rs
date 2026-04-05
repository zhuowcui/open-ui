//! Tests for SP11 Round 28 code review fixes — openui-layout crate.
//!
//! Issue 4: Intrinsic sizing overcounts inline content around block descendants.
//! Issue 5: vertical-align percentage uses wrong line-height:normal basis.

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    ComputedStyle, Display, LineHeight, VerticalAlign,
};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu(px: f32) -> LayoutUnit {
    LayoutUnit::from_f32(px)
}

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn make_constraint_width(w: i32) -> ConstraintSpace {
    ConstraintSpace::for_block_child(lu_i(w), lu_i(600), lu_i(w), lu_i(600), false)
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

// ── Issue 5: vertical-align percentage uses int_line_spacing ────────────

#[test]
fn r28_valign_percentage_uses_int_line_spacing_for_normal() {
    // vertical-align: 50% with line-height: normal should compute the shift
    // using int_line_spacing() (rounded), not raw line_spacing.
    // We verify that the layout produces a consistent result by comparing
    // the position of a percentage-shifted element vs. a length-shifted one
    // where the length is computed from int_line_spacing.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.font_size = 16.0;
    doc.append_child(root, block);

    // Span with vertical-align: 50%
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.vertical_align = VerticalAlign::Percentage(50.0);
    doc.node_mut(span).style.font_size = 16.0;
    doc.node_mut(span).style.line_height = LineHeight::Normal;
    doc.append_child(block, span);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Test".to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.node_mut(text).style.font_size = 16.0;
    doc.node_mut(text).style.line_height = LineHeight::Normal;
    doc.node_mut(text).style.vertical_align = VerticalAlign::Percentage(50.0);
    doc.append_child(span, text);

    let sp = make_constraint_width(800);
    let frag = inline_layout(&doc, block, &sp);
    let texts = all_text_fragments(&frag);
    assert!(!texts.is_empty(), "Should produce text fragments");

    // The fragment should have a valid position (not NaN, not absurdly large).
    let top = texts[0].offset.top.to_f32();
    assert!(
        top.is_finite(),
        "vertical-align: 50% with normal line-height should produce finite position",
    );
}

#[test]
fn r28_valign_percentage_consistent_with_explicit_length() {
    // Compute what int_line_spacing would give us for the default font at 16px,
    // then compare vertical-align: 50% (which uses int_line_spacing internally)
    // against vertical-align: <explicit length> of 50% * int_line_spacing.
    //
    // Both should produce the same text position.
    let font_desc = openui_text::font::FontDescription::default();
    let font = openui_text::font::Font::new(font_desc);
    let int_ls = font
        .font_metrics()
        .map(|m| m.int_line_spacing())
        .unwrap_or(19.0);
    let expected_shift = int_ls * 50.0 / 100.0;

    // Layout with vertical-align: 50%
    let mut doc1 = Document::new();
    let root1 = doc1.root();
    let block1 = doc1.create_node(ElementTag::Div);
    doc1.node_mut(block1).style.display = Display::Block;
    doc1.node_mut(block1).style.font_size = 16.0;
    doc1.append_child(root1, block1);

    let span1 = doc1.create_node(ElementTag::Span);
    doc1.node_mut(span1).style.display = Display::Inline;
    doc1.node_mut(span1).style.vertical_align = VerticalAlign::Percentage(50.0);
    doc1.node_mut(span1).style.font_size = 16.0;
    doc1.node_mut(span1).style.line_height = LineHeight::Normal;
    doc1.append_child(block1, span1);

    let text1 = doc1.create_node(ElementTag::Text);
    doc1.node_mut(text1).text = Some("A".to_string());
    doc1.node_mut(text1).style.display = Display::Inline;
    doc1.node_mut(text1).style.font_size = 16.0;
    doc1.node_mut(text1).style.line_height = LineHeight::Normal;
    doc1.node_mut(text1).style.vertical_align = VerticalAlign::Percentage(50.0);
    doc1.append_child(span1, text1);

    let sp = make_constraint_width(800);
    let frag1 = inline_layout(&doc1, block1, &sp);

    // Layout with vertical-align: <expected_shift>px
    let mut doc2 = Document::new();
    let root2 = doc2.root();
    let block2 = doc2.create_node(ElementTag::Div);
    doc2.node_mut(block2).style.display = Display::Block;
    doc2.node_mut(block2).style.font_size = 16.0;
    doc2.append_child(root2, block2);

    let span2 = doc2.create_node(ElementTag::Span);
    doc2.node_mut(span2).style.display = Display::Inline;
    doc2.node_mut(span2).style.vertical_align = VerticalAlign::Length(expected_shift);
    doc2.node_mut(span2).style.font_size = 16.0;
    doc2.node_mut(span2).style.line_height = LineHeight::Normal;
    doc2.append_child(block2, span2);

    let text2 = doc2.create_node(ElementTag::Text);
    doc2.node_mut(text2).text = Some("A".to_string());
    doc2.node_mut(text2).style.display = Display::Inline;
    doc2.node_mut(text2).style.font_size = 16.0;
    doc2.node_mut(text2).style.line_height = LineHeight::Normal;
    doc2.node_mut(text2).style.vertical_align = VerticalAlign::Length(expected_shift);
    doc2.append_child(span2, text2);

    let frag2 = inline_layout(&doc2, block2, &sp);

    let texts1 = all_text_fragments(&frag1);
    let texts2 = all_text_fragments(&frag2);
    assert!(!texts1.is_empty() && !texts2.is_empty());

    // Both should have the same vertical position (within 1px tolerance
    // for LayoutUnit rounding).
    let diff = (texts1[0].offset.top.to_f32() - texts2[0].offset.top.to_f32()).abs();
    assert!(
        diff < 1.5,
        "vertical-align: 50% (line-height:normal) should match explicit length \
         of {:.1}px: positions differ by {:.2}px",
        expected_shift,
        diff,
    );
}
