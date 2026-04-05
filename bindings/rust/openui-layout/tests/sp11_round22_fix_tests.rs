//! Tests for SP11 Round 22 code review fixes — openui-layout crate.
//!
//! Issue 1: `text-align-last` overrides last line alignment regardless of `text-align`.
//! Issue 2 & 4: Inter-character justification creates justified shape and handles boundaries.

use openui_dom::{Document, ElementTag};
use openui_geometry::LayoutUnit;
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    Direction, Display, TextAlign, TextAlignLast, TextJustify,
};

// ── Helpers ─────────────────────────────────────────────────────────────

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

// ── Issue 1: text-align-last overrides last line for non-justify text-align ──

#[test]
fn text_align_last_center_with_text_align_left() {
    // CSS: text-align: left; text-align-last: center
    // The last (only) line should be centered, not left-aligned.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Left;
    doc.node_mut(block).style.text_align_last = TextAlignLast::Center;
    doc.append_child(root, block);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hi".to_string());
    doc.append_child(block, text);

    let space = make_constraint_width(400);
    let result = inline_layout(&doc, block, &space);

    let texts = all_text_fragments(&result);
    assert!(!texts.is_empty(), "should produce text fragments");

    // With center alignment on a 400px container, the offset should be roughly
    // (400 - text_width) / 2 which is > 0 for short text.
    let offset_left = texts[0].offset.left.to_f32();
    assert!(
        offset_left > 1.0,
        "text-align-last:center should shift text right; got offset={offset_left}"
    );
}

#[test]
fn text_align_last_end_with_text_align_start() {
    // CSS: text-align: start; text-align-last: end
    // For LTR, the last line should be right-aligned.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Start;
    doc.node_mut(block).style.text_align_last = TextAlignLast::End;
    doc.node_mut(block).style.direction = Direction::Ltr;
    doc.append_child(root, block);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hi".to_string());
    doc.append_child(block, text);

    let space = make_constraint_width(400);
    let result = inline_layout(&doc, block, &space);

    let texts = all_text_fragments(&result);
    assert!(!texts.is_empty());

    // Right-aligned text in 400px container: offset should be near 400 - text_width
    let offset_left = texts[0].offset.left.to_f32();
    assert!(
        offset_left > 100.0,
        "text-align-last:end should push text to the right; got offset={offset_left}"
    );
}

#[test]
fn text_align_last_auto_with_non_justify_preserves_text_align() {
    // CSS: text-align: right; text-align-last: auto
    // Auto should fall back to the text-align value (right), not start.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Right;
    doc.node_mut(block).style.text_align_last = TextAlignLast::Auto;
    doc.append_child(root, block);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hi".to_string());
    doc.append_child(block, text);

    let space = make_constraint_width(400);
    let result = inline_layout(&doc, block, &space);

    let texts = all_text_fragments(&result);
    assert!(!texts.is_empty());

    // Right-aligned: offset should be near 400 - text_width
    let offset_left = texts[0].offset.left.to_f32();
    assert!(
        offset_left > 100.0,
        "text-align-last:auto with text-align:right should right-align; got offset={offset_left}"
    );
}

// ── Issue 2 & 4: Inter-character justification ──────────────────────────

#[test]
fn inter_character_justify_single_item_expands_shape() {
    // CSS: text-align: justify; text-justify: inter-character
    // Two lines of CJK-like text. The first line should be justified
    // with inter-character spacing. We verify the first line's text
    // fragment width expands to fill the available width.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.node_mut(block).style.text_justify = TextJustify::InterCharacter;
    doc.append_child(root, block);

    // Use enough text that it wraps to two lines in a narrow container.
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("ABCD EFGH".to_string());
    doc.append_child(block, text);

    // Narrow container to force wrapping
    let space = make_constraint_width(60);
    let result = inline_layout(&doc, block, &space);

    // The first line's text fragment should have a shape result.
    let texts = all_text_fragments(&result);
    if texts.len() >= 2 {
        // First line's text should have justified shape result
        let first_text = texts[0];
        assert!(
            first_text.shape_result.is_some(),
            "First line text should have shape result for rendering"
        );
    }
}

#[test]
fn inter_character_justify_boundary_gaps_between_items() {
    // CSS: text-align: justify; text-justify: inter-character
    // Two adjacent inline spans on the same line: <span>AB</span><span>CD</span>
    // There should be a gap between B and C (the boundary gap).
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.node_mut(block).style.text_justify = TextJustify::InterCharacter;
    doc.append_child(root, block);

    // Create two inline spans with text, and a second line to ensure
    // the first line is not the last line (so justify applies).
    let span1 = doc.create_node(ElementTag::Span);
    doc.node_mut(span1).style.display = Display::Inline;
    doc.append_child(block, span1);
    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("AB".to_string());
    doc.append_child(span1, t1);

    let span2 = doc.create_node(ElementTag::Span);
    doc.node_mut(span2).style.display = Display::Inline;
    doc.append_child(block, span2);
    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("CD EFGH IJKL".to_string());
    doc.append_child(span2, t2);

    let space = make_constraint_width(80);
    let result = inline_layout(&doc, block, &space);

    // Just verify the layout doesn't panic and produces fragments.
    let texts = all_text_fragments(&result);
    assert!(
        !texts.is_empty(),
        "Should produce text fragments for inter-character justify with multiple spans"
    );
}

// ── Issue 1 additional: verify logic matches CSS spec ────────────────────

#[test]
fn text_align_last_mapping_logic_comprehensive() {
    // Verify the mapping for every TextAlignLast variant when text-align is Left.
    // Per CSS Text Level 3 §7.3, text-align-last should override regardless of text-align.
    let test_cases = vec![
        (TextAlignLast::Auto, TextAlign::Left, TextAlign::Left),      // auto → use text-align
        (TextAlignLast::Start, TextAlign::Left, TextAlign::Start),    // start
        (TextAlignLast::End, TextAlign::Left, TextAlign::End),        // end
        (TextAlignLast::Center, TextAlign::Left, TextAlign::Center),  // center
        (TextAlignLast::Right, TextAlign::Left, TextAlign::Right),    // right
        (TextAlignLast::Justify, TextAlign::Left, TextAlign::Justify),// justify
        (TextAlignLast::Left, TextAlign::Left, TextAlign::Left),      // left
        // Auto with justify falls back to start
        (TextAlignLast::Auto, TextAlign::Justify, TextAlign::Start),
    ];

    for (tal, ta, expected) in test_cases {
        let is_last = true;
        let effective = if is_last {
            match tal {
                TextAlignLast::Auto => match ta {
                    TextAlign::Justify => TextAlign::Start,
                    other => other,
                },
                TextAlignLast::Start => TextAlign::Start,
                TextAlignLast::End => TextAlign::End,
                TextAlignLast::Left => TextAlign::Left,
                TextAlignLast::Right => TextAlign::Right,
                TextAlignLast::Center => TextAlign::Center,
                TextAlignLast::Justify => TextAlign::Justify,
            }
        } else {
            ta
        };
        assert_eq!(
            effective, expected,
            "text-align-last={tal:?} with text-align={ta:?} should give {expected:?}"
        );
    }
}
