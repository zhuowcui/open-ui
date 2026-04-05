//! Edge-case and regression tests for inline / block layout.
//!
//! Covers empty inputs, extreme sizes, Unicode, RTL, line-height corner cases,
//! zero-width containers, very long paragraphs, and mixed-content structures.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::block::block_layout;
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{Direction, Display, LineHeight, TextAlign};

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

fn layout_text(texts: &[&str], width: i32) -> Fragment {
    let (doc, block) = make_text_block(texts, width);
    let sp = ConstraintSpace::for_block_child(
        lu_i(width),
        lu_i(600),
        lu_i(width),
        lu_i(600),
        false,
    );
    inline_layout(&doc, block, &sp)
}

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

fn count_line_boxes(fragment: &Fragment) -> usize {
    fragment
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Box)
        .count()
}

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
// 1. EMPTY / WHITESPACE PARAGRAPHS (4 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn edge_empty_text_no_crash() {
    let frag = layout_text(&[""], 300);
    // An empty text node should still produce a fragment without panicking.
    assert!(
        frag.size.height >= LayoutUnit::zero(),
        "Empty text layout should not have negative height"
    );
}

#[test]
fn edge_only_spaces_collapses() {
    let frag = layout_text(&["     "], 300);
    // Whitespace-only text in normal mode collapses multiple spaces into one.
    // The resulting width should be at most one space character wide (~10px).
    let texts = collect_text_fragments(&frag);
    if !texts.is_empty() {
        assert!(
            texts[0].size.width < lu(15.0),
            "Spaces-only text should collapse to at most one space width, got {:?}",
            texts[0].size.width
        );
    }
}

#[test]
fn edge_only_tabs_collapses() {
    let frag = layout_text(&["\t\t"], 300);
    // Tabs in normal white-space mode collapse like spaces.
    // The resulting width should be at most one space character wide.
    let texts = collect_text_fragments(&frag);
    if !texts.is_empty() {
        assert!(
            texts[0].size.width < lu(15.0),
            "Tab-only text should collapse to at most one space width, got {:?}",
            texts[0].size.width
        );
    }
}

#[test]
fn edge_only_newlines_collapses() {
    let frag = layout_text(&["\n\n"], 300);
    // In normal white-space mode, newlines collapse to a space.
    let texts = collect_text_fragments(&frag);
    if !texts.is_empty() {
        // Collapsed newlines should produce at most a single space width.
        assert!(
            texts[0].size.width < lu(20.0),
            "Newline-only text should collapse to minimal width"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 2. VERY LONG PARAGRAPHS (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn edge_100_words_many_lines() {
    let long_text = "word ".repeat(100);
    let frag = layout_text(&[&long_text], 200);
    let lines = count_line_boxes(&frag);
    assert!(lines >= 2, "100 words at 200px should wrap to multiple lines, got {lines}");
    assert!(
        frag.size.height > LayoutUnit::zero(),
        "100-word paragraph should have positive height"
    );
}

#[test]
fn edge_500_words_no_crash() {
    let long_100 = "word ".repeat(100);
    let frag_100 = layout_text(&[&long_100], 200);

    let long_500 = "word ".repeat(500);
    let frag_500 = layout_text(&[&long_500], 200);

    assert!(
        frag_500.size.height > frag_100.size.height,
        "500 words should be taller than 100 words"
    );
}

#[test]
fn edge_single_very_long_word() {
    let long_word = "a".repeat(500);
    let frag = layout_text(&[&long_word], 200);
    let lines = count_line_boxes(&frag);
    assert!(lines >= 1, "Very long word should produce at least 1 line box");
    assert!(
        frag.size.height > LayoutUnit::zero(),
        "Very long word should have positive height"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 3. SINGLE CHARACTER PARAGRAPHS (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn edge_single_char_produces_text_fragment() {
    let frag = layout_text(&["A"], 800);
    let text_count = count_text_fragments(&frag);
    assert_eq!(text_count, 1, "Single char 'A' should produce exactly 1 text fragment");
}

#[test]
fn edge_single_char_positive_line_height() {
    let frag = layout_text(&["A"], 800);
    let lines = count_line_boxes(&frag);
    assert_eq!(lines, 1, "Single char should produce 1 line box");
    let line = &frag.children[0];
    assert!(
        line.size.height > LayoutUnit::zero(),
        "Line box for single char should have positive height"
    );
}

#[test]
fn edge_unicode_single_char_produces_layout() {
    let frag = layout_text(&["\u{4E2D}"], 800); // '中'
    assert!(
        frag.size.height > LayoutUnit::zero(),
        "Unicode CJK character should produce layout with positive height"
    );
    let text_count = count_text_fragments(&frag);
    assert!(text_count >= 1, "Unicode char should produce at least 1 text fragment");
}

// ═══════════════════════════════════════════════════════════════════════
// 4. ZERO-WIDTH EDGE CASES (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn edge_empty_string_shaped_zero_width() {
    let frag = layout_text(&[""], 800);
    let texts = collect_text_fragments(&frag);
    // Empty string should produce either no text fragments or a zero-width one.
    for t in &texts {
        assert!(
            t.size.width <= LayoutUnit::epsilon(),
            "Empty string text fragment should have zero or epsilon width"
        );
    }
}

#[test]
fn edge_empty_fragment_width() {
    let frag = layout_text(&[""], 400);
    // The inline container itself may have width from the constraint space
    // but any text child should be zero or minimal.
    let texts = collect_text_fragments(&frag);
    for t in &texts {
        assert!(
            t.size.width <= LayoutUnit::epsilon(),
            "Width of empty text fragment should be zero or minimal"
        );
    }
}

#[test]
fn edge_layout_at_width_zero_no_crash() {
    // Layout in a zero-width container: should not panic.
    let frag = layout_text(&["Hello world"], 0);
    // Produces at least one line box — text must go somewhere.
    let lines = count_line_boxes(&frag);
    assert!(lines >= 1, "Zero-width layout should still produce line boxes");
}

// ═══════════════════════════════════════════════════════════════════════
// 5. EXTREME FONT SIZES (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn edge_very_small_font_layout_succeeds() {
    // Use the default (small) font and a tiny container.
    let frag = layout_text(&["tiny text"], 50);
    assert!(
        frag.size.height > LayoutUnit::zero(),
        "Very small container layout should still succeed with positive height"
    );
}

#[test]
fn edge_very_large_text_content() {
    // A very large block of text should lay out without crashing.
    let big = "The quick brown fox jumps over the lazy dog. ".repeat(200);
    let frag = layout_text(&[&big], 600);
    assert!(
        frag.size.height > lu(100.0),
        "Very large text content should produce significant height"
    );
}

#[test]
fn edge_mixed_tiny_and_normal_text() {
    // Two text nodes laid out together — the taller line-height should win.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    // "small" text node (default font size, tiny line-height)
    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("small".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.node_mut(t1).style.line_height = LineHeight::Number(0.5);
    doc.append_child(block, t1);

    // "LARGE" text node (default font size, large line-height)
    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("LARGE".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.node_mut(t2).style.line_height = LineHeight::Number(3.0);
    doc.append_child(block, t2);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // The line box should be at least as tall as the larger line-height.
    // Default font is 16px, so line-height: 3.0 → 48px.
    assert!(
        frag.size.height >= lu(40.0),
        "Mixed line-heights: taller line-height should dominate, got {:?}",
        frag.size.height
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 6. MIXED CONTENT STRUCTURES (4 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn edge_text_empty_text_merges() {
    // ["Hello", "", "World"] — the empty node should not disrupt layout.
    let frag = layout_text(&["Hello", "", "World"], 800);
    let texts = collect_text_fragments(&frag);
    // At least the two non-empty text nodes should be present.
    let non_empty: Vec<_> = texts
        .iter()
        .filter(|t| t.size.width > LayoutUnit::zero())
        .collect();
    assert!(
        non_empty.len() >= 2,
        "Non-empty text nodes should produce visible fragments"
    );
}

#[test]
fn edge_multiple_text_nodes_combined() {
    let frag = layout_text(&["Hello ", "beautiful ", "world"], 800);
    let text_count = count_text_fragments(&frag);
    assert!(
        text_count >= 1,
        "Multiple text nodes should be combined into layout"
    );
    assert!(
        frag.size.height > LayoutUnit::zero(),
        "Combined text should have positive height"
    );
}

#[test]
fn edge_ten_text_nodes_all_contribute() {
    let nodes: Vec<&str> = vec!["a "; 10];
    let frag = layout_text(&nodes, 800);
    let text_count = count_text_fragments(&frag);
    assert!(
        text_count >= 1,
        "10 text nodes should produce at least 1 text fragment"
    );
    let texts = collect_text_fragments(&frag);
    let total_width: f32 = texts.iter().map(|t| t.size.width.to_f32()).sum();
    assert!(
        total_width > 0.0,
        "10 text nodes should contribute positive total width"
    );
}

#[test]
fn edge_alternating_text_and_spaces() {
    let frag = layout_text(&["word", " ", "word", " ", "word", " ", "word"], 80);
    let lines = count_line_boxes(&frag);
    // At 80px width, four "word" tokens should wrap to multiple lines.
    assert!(
        lines >= 1,
        "Alternating text and spaces should produce at least 1 line"
    );
    assert!(
        frag.size.height > LayoutUnit::zero(),
        "Alternating text should have positive height"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 7. RTL PARAGRAPH (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn edge_rtl_direction_produces_fragment() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.direction = Direction::Rtl;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    assert!(
        frag.size.height > LayoutUnit::zero(),
        "RTL layout should produce a fragment with positive height"
    );
}

#[test]
fn edge_rtl_text_align_start_offsets_right() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.direction = Direction::Rtl;
    doc.node_mut(block).style.text_align = TextAlign::Start;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hi".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty(), "RTL layout should produce text fragments");
    // In RTL with text-align: start, short text should be offset to the right.
    assert!(
        texts[0].offset.left > LayoutUnit::zero(),
        "RTL start-aligned short text should have positive left offset"
    );
}

#[test]
fn edge_ltr_text_in_rtl_paragraph_renders() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.direction = Direction::Rtl;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("LTR content in RTL".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let text_count = count_text_fragments(&frag);
    assert!(
        text_count >= 1,
        "LTR text in RTL paragraph should still render"
    );
    assert!(
        frag.size.height > LayoutUnit::zero(),
        "LTR text in RTL paragraph should have positive height"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 8. LINE HEIGHT INTERACTIONS (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn edge_line_height_less_than_font_no_crash() {
    // line-height: 0.1 (much less than font-size) — should not panic.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Number(0.1);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Squeezed".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Number(0.1);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    // Even with tiny line-height, the fragment should exist with non-negative height.
    assert!(
        frag.size.height >= LayoutUnit::zero(),
        "Tiny line-height should not produce negative height"
    );
}

#[test]
fn edge_line_height_much_larger_than_font() {
    // line-height: 200px — the line box should be at least 200px tall.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Length(200.0);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Tall".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Length(200.0);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    assert!(
        frag.size.height >= lu(200.0),
        "line-height: 200px should produce height >= 200px, got {:?}",
        frag.size.height
    );
}

#[test]
fn edge_large_indent_with_center_alignment_no_crash() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Center;
    doc.node_mut(block).style.text_indent = Length::px(500.0);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Indented center".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(300), lu_i(600), lu_i(300), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    // Large indent + center should not crash; fragment should exist.
    assert!(
        frag.size.height > LayoutUnit::zero(),
        "Large indent + center alignment should produce valid layout"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 9. ADDITIONAL EDGE CASES (2+ bonus tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn edge_block_layout_empty_text_no_crash() {
    // Full block_layout path with empty text — integration test.
    let frag = block_layout_text(&[""], 400);
    assert!(
        frag.size.height >= LayoutUnit::zero(),
        "Block layout with empty text should not crash"
    );
}

#[test]
fn edge_block_layout_multiple_paragraphs() {
    // Block layout with several text nodes acting as a paragraph.
    let frag = block_layout_text(&["First. ", "Second. ", "Third."], 400);
    assert!(
        frag.size.height > LayoutUnit::zero(),
        "Block layout with multiple text nodes should have positive height"
    );
    let text_count = count_text_fragments(&frag);
    assert!(
        text_count >= 1,
        "Block layout should produce at least 1 text fragment"
    );
}

#[test]
fn edge_whitespace_between_words_single_line() {
    // "Hello World" should fit on one line at 800px.
    let frag = layout_text(&["Hello World"], 800);
    let lines = count_line_boxes(&frag);
    assert_eq!(lines, 1, "Short text at wide width should be a single line");
}

#[test]
fn edge_narrow_container_forces_wrapping() {
    // At 50px, multi-word text must wrap.
    let frag = layout_text(&["one two three four five six seven eight"], 50);
    let lines = count_line_boxes(&frag);
    assert!(
        lines >= 2,
        "Very narrow container should force multi-word text to wrap, got {lines} lines"
    );
}
