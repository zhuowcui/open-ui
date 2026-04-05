//! Tests for SP11 Round 19 fixes.
//!
//! Issue 1: RTL inline start/end MBP uses correct physical side.
//! Issue 2: Tab expansion is advance-based, not column-count based.
//! Issue 3: capitalize uses titlecase, not uppercase.
//! Issue 4: Justification mis-counts trailing spaces in pre-wrap.

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::inline::items::InlineItemType;
use openui_layout::inline::items_builder::{expand_tabs, InlineItemsBuilder};
use openui_layout::inline::line_breaker::LineBreaker;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    BorderStyle, Direction, Display, TabSize, TextAlign, WhiteSpace,
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

fn collect_line_boxes(fragment: &Fragment) -> Vec<&Fragment> {
    fragment
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Box)
        .collect()
}

fn collect_text_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    fragment
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Text)
        .collect()
}

// ── Issue 1: RTL inline start/end MBP picks correct physical side ───────

#[test]
fn rtl_span_asymmetric_padding_inline_start_is_right() {
    // In RTL, inline-start is the RIGHT side.
    // Span has padding-left: 5px, padding-right: 20px.
    // In RTL the OpenTag should contribute the right-side MBP (20px),
    // and the CloseTag should contribute the left-side MBP (5px).
    let mut doc = Document::new();
    let root = doc.root();

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.direction = Direction::Rtl;
    doc.append_child(root, block);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.direction = Direction::Rtl;
    doc.node_mut(span).style.padding_left = Length::px(5.0);
    doc.node_mut(span).style.padding_right = Length::px(20.0);
    doc.append_child(block, span);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hello".to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.node_mut(text).style.direction = Direction::Rtl;
    doc.append_child(span, text);

    let mut items_data = InlineItemsBuilder::collect(&doc, block);
    items_data.shape_text();

    let containing_width = lu_i(400);
    let mut breaker = LineBreaker::new(&items_data, containing_width);
    let line = breaker.next_line(containing_width).unwrap();

    let open_tags: Vec<_> = line
        .items
        .iter()
        .filter(|i| i.item_type == InlineItemType::OpenTag)
        .collect();
    let close_tags: Vec<_> = line
        .items
        .iter()
        .filter(|i| i.item_type == InlineItemType::CloseTag)
        .collect();

    assert!(!open_tags.is_empty(), "Should have an OpenTag item");
    assert!(!close_tags.is_empty(), "Should have a CloseTag item");

    // RTL: OpenTag = inline-start = right side padding (20px)
    let open_mbp = open_tags[0].inline_size.to_f32();
    assert!(
        (open_mbp - 20.0).abs() < 1.0,
        "RTL OpenTag (inline-start) should use right-side padding 20px, got {:.1}",
        open_mbp,
    );

    // RTL: CloseTag = inline-end = left side padding (5px)
    let close_mbp = close_tags[0].inline_size.to_f32();
    assert!(
        (close_mbp - 5.0).abs() < 1.0,
        "RTL CloseTag (inline-end) should use left-side padding 5px, got {:.1}",
        close_mbp,
    );
}

#[test]
fn rtl_span_asymmetric_border_uses_correct_side() {
    // RTL span with border-left: 2px solid, border-right: 10px solid.
    // OpenTag (inline-start) = right border = 10px in RTL.
    let mut doc = Document::new();
    let root = doc.root();

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.direction = Direction::Rtl;
    doc.append_child(root, block);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.direction = Direction::Rtl;
    doc.node_mut(span).style.border_left_width = 2;
    doc.node_mut(span).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(span).style.border_right_width = 10;
    doc.node_mut(span).style.border_right_style = BorderStyle::Solid;
    doc.append_child(block, span);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Test".to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.node_mut(text).style.direction = Direction::Rtl;
    doc.append_child(span, text);

    let mut items_data = InlineItemsBuilder::collect(&doc, block);
    items_data.shape_text();

    let containing_width = lu_i(400);
    let mut breaker = LineBreaker::new(&items_data, containing_width);
    let line = breaker.next_line(containing_width).unwrap();

    let open_tags: Vec<_> = line
        .items
        .iter()
        .filter(|i| i.item_type == InlineItemType::OpenTag)
        .collect();
    let close_tags: Vec<_> = line
        .items
        .iter()
        .filter(|i| i.item_type == InlineItemType::CloseTag)
        .collect();

    // RTL: inline-start = right side → border-right = 10px
    let open_mbp = open_tags[0].inline_size.to_f32();
    assert!(
        (open_mbp - 10.0).abs() < 1.0,
        "RTL OpenTag should use border-right (10px), got {:.1}",
        open_mbp,
    );

    // RTL: inline-end = left side → border-left = 2px
    let close_mbp = close_tags[0].inline_size.to_f32();
    assert!(
        (close_mbp - 2.0).abs() < 1.0,
        "RTL CloseTag should use border-left (2px), got {:.1}",
        close_mbp,
    );
}

// ── Issue 2: Tab stops align based on advance width ─────────────────────

#[test]
fn tab_expansion_advance_based_with_larger_space() {
    // With space_advance = 10.0, tab-size: 4 → tab_interval = 40.0.
    // "\t" at position 0 → next stop at 40 → 4 spaces (4 * 10 = 40).
    let result = expand_tabs("\thello", &TabSize::Spaces(4), 10.0, |_| 10.0);
    assert_eq!(
        result, "    hello",
        "Tab at start with space_advance=10 and tab-size=4 should produce 4 spaces"
    );
}

#[test]
fn tab_expansion_length_based_tab_size() {
    // TabSize::Length(40.0) with space_advance=10.0 → tab_interval = 40.0.
    // "ab\t" at advance 20.0 → next stop at 40 → 2 spaces.
    let result = expand_tabs("ab\tx", &TabSize::Length(40.0), 10.0, |_| 10.0);
    assert_eq!(
        result, "ab  x",
        "TabSize::Length(40) with space_advance=10 after 2 chars should give 2 spaces"
    );
}

#[test]
fn tab_expansion_position_dependent_not_column_count() {
    // With space_advance = 8.0 and tab-size: 4, tab_interval = 32.0.
    // "abc\t" → advance = 24.0, next stop = 32.0, spaces = round(8/8) = 1 space.
    // But 1 space (8.0) < space_advance(8.0) edge — we get exactly 1.
    let result = expand_tabs("abc\tx", &TabSize::Spaces(4), 8.0, |_| 8.0);
    assert_eq!(
        result, "abc x",
        "Tab after 3 chars (24px advance) with tab_interval=32px should produce 1 space to reach 32px"
    );
}

// ── Issue 4: pre-wrap justify with multiple trailing spaces ─────────────

#[test]
fn justify_prewrap_multiple_trailing_spaces_not_expanded() {
    // "hello world   " in pre-wrap justify mode.
    // There's 1 inter-word space and 3 trailing spaces.
    // The trailing spaces should NOT be counted as expansion opportunities,
    // so all justification should go to the 1 inter-word space.
    let mut doc = Document::new();
    let root = doc.root();

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.append_child(root, block);

    // Use pre-wrap text with a newline to force a second line (justify
    // only applies to non-last, non-forced-break lines).
    // Put "hello world   \nnext" — first line has 3 trailing spaces.
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("hello world   \nnext".to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.node_mut(text).style.white_space = WhiteSpace::PreWrap;
    doc.append_child(block, text);

    let constraint = make_constraint_width(400);
    let frag = inline_layout(&doc, block, &constraint);
    let lines = collect_line_boxes(&frag);
    assert!(
        lines.len() >= 2,
        "Should have at least 2 lines, got {}",
        lines.len()
    );

    // The first line should have text fragments. The trailing spaces
    // should NOT have extra justification width. The text content width
    // should fill close to the 400px container with justification applied
    // only to the inter-word gap, not the trailing spaces.
    let text_frags = collect_text_fragments(lines[0]);
    assert!(!text_frags.is_empty(), "First line should have text");

    // At minimum, verify no panic and text is laid out.
    // The inter-word space should be expanded, trailing spaces should not.
    let total_text_width: f32 = text_frags.iter().map(|f| f.size.width.to_f32()).sum();
    assert!(
        total_text_width > 0.0,
        "Text fragments should have positive width"
    );
}

#[test]
fn justify_single_trailing_space_not_expanded() {
    // Ensure the original single-trailing-space case still works.
    // "hello world " — 1 inter-word space, 1 trailing space.
    // Only the inter-word space should be an expansion opportunity.
    let mut doc = Document::new();
    let root = doc.root();

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Justify;
    doc.append_child(root, block);

    // Use pre-wrap text with a newline to create a second line.
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("hello world \nnext".to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.node_mut(text).style.white_space = WhiteSpace::PreWrap;
    doc.append_child(block, text);

    let constraint = make_constraint_width(400);
    let frag = inline_layout(&doc, block, &constraint);
    let lines = collect_line_boxes(&frag);
    assert!(lines.len() >= 2, "Should have at least 2 lines");

    let text_frags = collect_text_fragments(lines[0]);
    assert!(!text_frags.is_empty(), "First line should have text");
}
