//! Tests for inline item collection and white-space processing.
//!
//! Validates that the InlineItemsBuilder correctly flattens the DOM tree
//! into inline items, handles white-space collapsing per CSS Text Level 3,
//! and integrates text shaping.

use openui_dom::{Document, ElementTag, NodeId};
use openui_layout::inline::items::{CollapseType, InlineItemType};
use openui_layout::inline::items_builder::{
    collapse_spaces_preserve_newlines, collapse_whitespace, expand_tabs, process_white_space,
    InlineItemsBuilder,
};
use openui_style::{ComputedStyle, Direction, Display, TabSize, WhiteSpace};

// ── Helpers ─────────────────────────────────────────────────────────────

/// Create a document with a block container and add text children.
fn make_doc_with_text(texts: &[&str]) -> (Document, NodeId) {
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

/// Create a document with a span wrapping text inside a block.
fn make_doc_with_span(span_texts: &[&str]) -> (Document, NodeId) {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(block, span);

    for text in span_texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.append_child(span, t);
    }
    (doc, block)
}

// ── White-Space Collapsing Tests ─────────────────────────────────────────

#[test]
fn collapse_whitespace_basic() {
    assert_eq!(collapse_whitespace("hello world"), "hello world");
}

#[test]
fn collapse_whitespace_multiple_spaces() {
    assert_eq!(collapse_whitespace("hello    world"), "hello world");
}

#[test]
fn collapse_whitespace_leading_trailing() {
    assert_eq!(collapse_whitespace("  hello  world  "), " hello world ");
}

#[test]
fn collapse_whitespace_tabs() {
    assert_eq!(collapse_whitespace("hello\tworld"), "hello world");
}

#[test]
fn collapse_whitespace_newlines() {
    assert_eq!(collapse_whitespace("hello\nworld"), "hello world");
}

#[test]
fn collapse_whitespace_mixed() {
    assert_eq!(collapse_whitespace("  hello \t\n world  "), " hello world ");
}

#[test]
fn collapse_whitespace_empty() {
    assert_eq!(collapse_whitespace(""), "");
}

#[test]
fn collapse_whitespace_only_spaces() {
    assert_eq!(collapse_whitespace("   "), " ");
}

#[test]
fn collapse_whitespace_cr_and_ff() {
    assert_eq!(collapse_whitespace("a\r\x0Cb"), "a b");
}

#[test]
fn collapse_whitespace_single_space() {
    assert_eq!(collapse_whitespace(" "), " ");
}

#[test]
fn collapse_whitespace_no_spaces() {
    assert_eq!(collapse_whitespace("hello"), "hello");
}

#[test]
fn collapse_spaces_preserve_newlines_basic() {
    assert_eq!(collapse_spaces_preserve_newlines("hello\nworld"), "hello\nworld");
}

#[test]
fn collapse_spaces_preserve_newlines_spaces_collapse() {
    assert_eq!(
        collapse_spaces_preserve_newlines("hello   world"),
        "hello world"
    );
}

#[test]
fn collapse_spaces_preserve_newlines_space_before_newline() {
    // Space before newline should be removed (CSS Text §4.1.1)
    assert_eq!(
        collapse_spaces_preserve_newlines("hello \nworld"),
        "hello\nworld"
    );
}

#[test]
fn collapse_spaces_preserve_newlines_multiple_newlines() {
    assert_eq!(
        collapse_spaces_preserve_newlines("a\n\nb"),
        "a\n\nb"
    );
}

#[test]
fn collapse_spaces_preserve_newlines_tabs_collapse() {
    assert_eq!(
        collapse_spaces_preserve_newlines("hello\t\tworld"),
        "hello world"
    );
}

#[test]
fn collapse_spaces_preserve_newlines_empty() {
    assert_eq!(collapse_spaces_preserve_newlines(""), "");
}

// ── process_white_space Tests ───────────────────────────────────────────

#[test]
fn process_white_space_normal() {
    assert_eq!(process_white_space("  hello  world  ", WhiteSpace::Normal), " hello world ");
}

#[test]
fn process_white_space_nowrap() {
    assert_eq!(process_white_space("  hello  world  ", WhiteSpace::Nowrap), " hello world ");
}

#[test]
fn process_white_space_pre() {
    assert_eq!(process_white_space("  hello  world  ", WhiteSpace::Pre), "  hello  world  ");
}

#[test]
fn process_white_space_pre_wrap() {
    assert_eq!(
        process_white_space("  hello  world  ", WhiteSpace::PreWrap),
        "  hello  world  "
    );
}

#[test]
fn process_white_space_pre_line() {
    assert_eq!(
        process_white_space("  hello  world  ", WhiteSpace::PreLine),
        " hello world "
    );
}

#[test]
fn process_white_space_break_spaces() {
    assert_eq!(
        process_white_space("  hello  world  ", WhiteSpace::BreakSpaces),
        "  hello  world  "
    );
}

#[test]
fn process_white_space_pre_with_newlines() {
    assert_eq!(
        process_white_space("hello\n  world", WhiteSpace::Pre),
        "hello\n  world"
    );
}

#[test]
fn process_white_space_pre_line_with_newlines() {
    // CSS Text §4.1.1: In pre-line, collapsible spaces after a forced line
    // break (newline) are removed.
    assert_eq!(
        process_white_space("hello\n  world", WhiteSpace::PreLine),
        "hello\nworld"
    );
}

// ── Inline Items Builder Tests ──────────────────────────────────────────

#[test]
fn text_only_produces_text_item() {
    let (doc, block) = make_doc_with_text(&["hello"]);
    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(data.items.len(), 1);
    assert_eq!(data.items[0].item_type, InlineItemType::Text);
    assert_eq!(&data.text[data.items[0].text_range.clone()], "hello");
}

#[test]
fn multiple_text_nodes_concatenate() {
    let (doc, block) = make_doc_with_text(&["hello", " ", "world"]);
    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(data.items.len(), 3);
    assert_eq!(data.text, "hello world");
}

#[test]
fn span_produces_open_close_tags() {
    let (doc, block) = make_doc_with_span(&["hello"]);
    let data = InlineItemsBuilder::collect(&doc, block);
    // OpenTag, Text, CloseTag
    assert_eq!(data.items.len(), 3);
    assert_eq!(data.items[0].item_type, InlineItemType::OpenTag);
    assert_eq!(data.items[1].item_type, InlineItemType::Text);
    assert_eq!(data.items[2].item_type, InlineItemType::CloseTag);
}

#[test]
fn nested_spans_produce_correct_order() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let outer_span = doc.create_node(ElementTag::Span);
    doc.node_mut(outer_span).style.display = Display::Inline;
    doc.append_child(block, outer_span);

    let inner_span = doc.create_node(ElementTag::Span);
    doc.node_mut(inner_span).style.display = Display::Inline;
    doc.append_child(outer_span, inner_span);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("hello".to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.append_child(inner_span, text);

    let data = InlineItemsBuilder::collect(&doc, block);
    // outer open, inner open, text, inner close, outer close
    assert_eq!(data.items.len(), 5);
    assert_eq!(data.items[0].item_type, InlineItemType::OpenTag);
    assert_eq!(data.items[1].item_type, InlineItemType::OpenTag);
    assert_eq!(data.items[2].item_type, InlineItemType::Text);
    assert_eq!(data.items[3].item_type, InlineItemType::CloseTag);
    assert_eq!(data.items[4].item_type, InlineItemType::CloseTag);
}

#[test]
fn text_ranges_are_correct_byte_offsets() {
    let (doc, block) = make_doc_with_text(&["abc", "def"]);
    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(data.items[0].text_range, 0..3);
    assert_eq!(data.items[1].text_range, 3..6);
    assert_eq!(&data.text[0..3], "abc");
    assert_eq!(&data.text[3..6], "def");
}

#[test]
fn empty_text_node_produces_no_items() {
    let (doc, block) = make_doc_with_text(&[""]);
    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(data.items.len(), 0);
}

#[test]
fn whitespace_only_text_normal_produces_single_space() {
    let (doc, block) = make_doc_with_text(&["   "]);
    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(data.items.len(), 1);
    assert_eq!(&data.text[data.items[0].text_range.clone()], " ");
}

#[test]
fn whitespace_collapsing_in_normal_mode() {
    let (doc, block) = make_doc_with_text(&["  hello  world  "]);
    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(&data.text[data.items[0].text_range.clone()], " hello world ");
}

#[test]
fn pre_preserves_all_spaces() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("  hello  world  ".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::Pre;
    doc.append_child(block, t);

    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(&data.text[data.items[0].text_range.clone()], "  hello  world  ");
}

#[test]
fn pre_line_collapses_spaces_keeps_newlines() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("hello   \n   world".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::PreLine;
    doc.append_child(block, t);

    let data = InlineItemsBuilder::collect(&doc, block);
    // CSS Text §4.1.1: In pre-line, spaces before/after newlines are stripped.
    assert_eq!(&data.text[data.items[0].text_range.clone()], "hello\nworld");
}

#[test]
fn style_index_references_correct_style() {
    let (doc, block) = make_doc_with_text(&["hello"]);
    let data = InlineItemsBuilder::collect(&doc, block);
    assert!(data.items[0].style_index < data.styles.len());
}

#[test]
fn multiple_text_nodes_have_separate_style_indices() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("hello".to_string());
    doc.node_mut(t1).style.font_size = 16.0;
    doc.append_child(block, t1);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("world".to_string());
    doc.node_mut(t2).style.font_size = 24.0;
    doc.append_child(block, t2);

    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(data.items.len(), 2);
    // Each text node gets its own style index
    let s1 = data.items[0].style_index;
    let s2 = data.items[1].style_index;
    assert_ne!(s1, s2);
    assert!((data.styles[s1].font_size - 16.0).abs() < 0.01);
    assert!((data.styles[s2].font_size - 24.0).abs() < 0.01);
}

#[test]
fn end_collapse_type_collapsible_for_trailing_space() {
    let (doc, block) = make_doc_with_text(&["hello "]);
    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(data.items[0].end_collapse_type, CollapseType::Collapsible);
}

#[test]
fn end_collapse_type_not_collapsible_for_no_trailing_space() {
    let (doc, block) = make_doc_with_text(&["hello"]);
    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(data.items[0].end_collapse_type, CollapseType::NotCollapsible);
}

#[test]
fn bidi_level_ltr_default() {
    let (doc, block) = make_doc_with_text(&["hello"]);
    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(data.items[0].bidi_level, 0);
}

#[test]
fn bidi_level_rtl() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("hello".to_string());
    doc.node_mut(t).style.direction = Direction::Rtl;
    doc.append_child(block, t);

    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(data.items[0].bidi_level, 1);
}

#[test]
fn open_close_tag_text_ranges_are_empty() {
    let (doc, block) = make_doc_with_span(&["hello"]);
    let data = InlineItemsBuilder::collect(&doc, block);
    let open = &data.items[0];
    let close = &data.items[2];
    assert_eq!(open.text_range.len(), 0);
    assert_eq!(close.text_range.len(), 0);
}

#[test]
fn inline_block_produces_atomic_inline() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let ib = doc.create_node(ElementTag::Div);
    doc.node_mut(ib).style.display = Display::InlineBlock;
    doc.append_child(block, ib);

    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(data.items.len(), 1);
    assert_eq!(data.items[0].item_type, InlineItemType::AtomicInline);
    // Object replacement character U+FFFC
    assert_eq!(&data.text[data.items[0].text_range.clone()], "\u{FFFC}");
}

#[test]
fn text_shape_result_is_none_before_shaping() {
    let (doc, block) = make_doc_with_text(&["hello"]);
    let data = InlineItemsBuilder::collect(&doc, block);
    assert!(data.items[0].shape_result.is_none());
}

#[test]
fn shaping_populates_shape_result() {
    let (doc, block) = make_doc_with_text(&["hello"]);
    let mut data = InlineItemsBuilder::collect(&doc, block);
    data.shape_text();
    assert!(data.items[0].shape_result.is_some());
}

#[test]
fn shaped_text_has_nonzero_width() {
    let (doc, block) = make_doc_with_text(&["hello"]);
    let mut data = InlineItemsBuilder::collect(&doc, block);
    data.shape_text();
    let sr = data.items[0].shape_result.as_ref().unwrap();
    assert!(sr.width() > 0.0, "shaped 'hello' should have positive width");
}

#[test]
fn shaped_text_width_increases_with_more_text() {
    let (doc1, block1) = make_doc_with_text(&["hi"]);
    let (doc2, block2) = make_doc_with_text(&["hello world"]);
    let mut data1 = InlineItemsBuilder::collect(&doc1, block1);
    let mut data2 = InlineItemsBuilder::collect(&doc2, block2);
    data1.shape_text();
    data2.shape_text();
    let w1 = data1.items[0].shape_result.as_ref().unwrap().width();
    let w2 = data2.items[0].shape_result.as_ref().unwrap().width();
    assert!(w2 > w1, "'hello world' should be wider than 'hi'");
}

#[test]
fn open_close_tags_have_no_shape_result() {
    let (doc, block) = make_doc_with_span(&["hello"]);
    let mut data = InlineItemsBuilder::collect(&doc, block);
    data.shape_text();
    assert!(data.items[0].shape_result.is_none()); // OpenTag
    assert!(data.items[2].shape_result.is_none()); // CloseTag
}

#[test]
fn mixed_inline_elements_flatten_correctly() {
    // <div> text1 <span> text2 </span> text3 </div>
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("before ".to_string());
    doc.append_child(block, t1);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(block, span);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("inside".to_string());
    doc.append_child(span, t2);

    let t3 = doc.create_node(ElementTag::Text);
    doc.node_mut(t3).text = Some(" after".to_string());
    doc.append_child(block, t3);

    let data = InlineItemsBuilder::collect(&doc, block);
    // text1, OpenTag, text2, CloseTag, text3
    assert_eq!(data.items.len(), 5);
    assert_eq!(data.items[0].item_type, InlineItemType::Text);
    assert_eq!(data.items[1].item_type, InlineItemType::OpenTag);
    assert_eq!(data.items[2].item_type, InlineItemType::Text);
    assert_eq!(data.items[3].item_type, InlineItemType::CloseTag);
    assert_eq!(data.items[4].item_type, InlineItemType::Text);
    assert_eq!(data.text, "before inside after");
}

#[test]
fn no_children_produces_empty_items() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(data.items.len(), 0);
    assert_eq!(data.text.len(), 0);
}

#[test]
fn unicode_text_byte_offsets_correct() {
    // 'é' is 2 bytes, '中' is 3 bytes in UTF-8
    let (doc, block) = make_doc_with_text(&["café"]);
    let data = InlineItemsBuilder::collect(&doc, block);
    let range = &data.items[0].text_range;
    assert_eq!(&data.text[range.clone()], "café");
    // "café" = 'c'(1) + 'a'(1) + 'f'(1) + 'é'(2) = 5 bytes
    assert_eq!(range.len(), 5);
}

#[test]
fn newline_in_normal_mode_becomes_space() {
    let (doc, block) = make_doc_with_text(&["hello\nworld"]);
    let data = InlineItemsBuilder::collect(&doc, block);
    assert_eq!(&data.text[data.items[0].text_range.clone()], "hello world");
}

#[test]
fn multiple_spans_with_text() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    // <span>a</span><span>b</span>
    for text in &["a", "b"] {
        let span = doc.create_node(ElementTag::Span);
        doc.node_mut(span).style.display = Display::Inline;
        doc.append_child(block, span);

        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.append_child(span, t);
    }

    let data = InlineItemsBuilder::collect(&doc, block);
    // open, text, close, open, text, close
    assert_eq!(data.items.len(), 6);
    assert_eq!(data.text, "ab");
}

#[test]
fn style_to_font_description_preserves_size() {
    use openui_layout::inline::items_builder::style_to_font_description;
    let mut style = ComputedStyle::initial();
    style.font_size = 24.0;
    let desc = style_to_font_description(&style);
    assert!((desc.size - 24.0).abs() < 0.01);
}

#[test]
fn style_to_font_description_preserves_weight() {
    use openui_layout::inline::items_builder::style_to_font_description;
    use openui_style::FontWeight;
    let mut style = ComputedStyle::initial();
    style.font_weight = FontWeight::BOLD;
    let desc = style_to_font_description(&style);
    assert!((desc.weight.0 - 700.0).abs() < 0.01);
}

// ═══════════════════════════════════════════════════════════════════════
// SP11 Dual-Model Review Fixes — Regression Tests
// ═══════════════════════════════════════════════════════════════════════

// ── Issue 2: Bidi splitting ─────────────────────────────────────────────

#[test]
fn bidi_mixed_text_splits_into_runs() {
    // "abc אבג def" — should split into at least 2 items after bidi analysis
    // (LTR "abc ", RTL "אבג", LTR " def")
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("abc \u{05D0}\u{05D1}\u{05D2} def".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let mut data = InlineItemsBuilder::collect(&doc, block);
    data.apply_bidi(openui_text::TextDirection::Ltr);

    // Count text items — should be more than 1 due to bidi split
    let text_items: Vec<_> = data.items.iter()
        .filter(|item| item.item_type == InlineItemType::Text)
        .collect();
    assert!(
        text_items.len() >= 2,
        "Mixed bidi text should be split into multiple items, got {}",
        text_items.len()
    );
}

#[test]
fn bidi_split_items_have_correct_levels() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    // Hebrew characters are RTL (odd bidi level), Latin are LTR (even level)
    doc.node_mut(t).text = Some("hello \u{05D0}\u{05D1}\u{05D2} world".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let mut data = InlineItemsBuilder::collect(&doc, block);
    data.apply_bidi(openui_text::TextDirection::Ltr);

    let text_items: Vec<_> = data.items.iter()
        .filter(|item| item.item_type == InlineItemType::Text)
        .collect();

    // At least some items should have RTL level (odd number)
    let has_rtl = text_items.iter().any(|item| item.bidi_level % 2 == 1);
    assert!(has_rtl, "Mixed bidi text should have items with RTL levels");
}

#[test]
fn bidi_all_ltr_no_split() {
    // All-LTR text should not be split
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("hello world".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(block, t);

    let mut data = InlineItemsBuilder::collect(&doc, block);
    data.apply_bidi(openui_text::TextDirection::Ltr);

    let text_items: Vec<_> = data.items.iter()
        .filter(|item| item.item_type == InlineItemType::Text)
        .collect();
    assert_eq!(text_items.len(), 1, "All-LTR text should remain one item");
}

// ── Issue 8: pre-line leading spaces after newlines ─────────────────────

#[test]
fn pre_line_strips_leading_spaces_after_newline() {
    // CSS Text §4.1.1: Spaces after a newline in pre-line should be stripped.
    assert_eq!(
        collapse_spaces_preserve_newlines("hello\n  world"),
        "hello\nworld"
    );
}

#[test]
fn pre_line_strips_tabs_after_newline() {
    assert_eq!(
        collapse_spaces_preserve_newlines("hello\n\t\tworld"),
        "hello\nworld"
    );
}

#[test]
fn pre_line_multiple_newlines_strip_intermediate_spaces() {
    // Each newline should strip following spaces
    assert_eq!(
        collapse_spaces_preserve_newlines("a\n  b\n  c"),
        "a\nb\nc"
    );
}

// ── SP11 Round 14 Issue 1: OpenTag/CloseTag bidi level ──────────────────

#[test]
fn open_close_tag_bidi_level_rtl_with_span() {
    // RTL text wrapped in a span: the OpenTag and CloseTag should get the
    // same bidi level as their content, not remain at 0.
    use openui_text::TextDirection;

    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(block, span);

    let text = doc.create_node(ElementTag::Text);
    // Arabic text — should be RTL (odd bidi level).
    doc.node_mut(text).text = Some("\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}".to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.append_child(span, text);

    let mut data = InlineItemsBuilder::collect(&doc, block);
    data.apply_bidi(TextDirection::Rtl);

    let open_tag = data.items.iter().find(|i| i.item_type == InlineItemType::OpenTag);
    let close_tag = data.items.iter().find(|i| i.item_type == InlineItemType::CloseTag);
    let text_item = data.items.iter().find(|i| i.item_type == InlineItemType::Text);

    assert!(open_tag.is_some(), "Should have an OpenTag item");
    assert!(close_tag.is_some(), "Should have a CloseTag item");
    assert!(text_item.is_some(), "Should have a Text item");

    let text_level = text_item.unwrap().bidi_level;
    assert!(
        text_level % 2 == 1,
        "Arabic text should have odd bidi level, got {}",
        text_level,
    );
    assert_eq!(
        open_tag.unwrap().bidi_level, text_level,
        "OpenTag should have the same bidi level as its content text"
    );
    assert_eq!(
        close_tag.unwrap().bidi_level, text_level,
        "CloseTag should have the same bidi level as its content text"
    );
}

// ── SP11 Round 14 Issue 5: Tab expansion ────────────────────────────────

#[test]
fn expand_tabs_basic_tab_size_8() {
    // Tab at column 0 should expand to 8 spaces.
    let result = expand_tabs("\thello", &TabSize::Spaces(8));
    assert_eq!(result, "        hello");
}

#[test]
fn expand_tabs_mid_line() {
    // "ab\t" — column 2, tab-size 8 → need 6 spaces to reach column 8.
    let result = expand_tabs("ab\tx", &TabSize::Spaces(8));
    assert_eq!(result, "ab      x");
}

#[test]
fn expand_tabs_tab_size_4() {
    // Tab at column 0, tab-size 4 → 4 spaces.
    let result = expand_tabs("\tx", &TabSize::Spaces(4));
    assert_eq!(result, "    x");
    // Tab at column 2, tab-size 4 → 2 spaces.
    let result2 = expand_tabs("ab\tx", &TabSize::Spaces(4));
    assert_eq!(result2, "ab  x");
}

#[test]
fn expand_tabs_resets_at_newline() {
    // After a newline, column resets to 0.
    let result = expand_tabs("ab\n\tx", &TabSize::Spaces(4));
    assert_eq!(result, "ab\n    x");
}

#[test]
fn expand_tabs_no_tabs_returns_same() {
    let input = "hello world";
    let result = expand_tabs(input, &TabSize::Spaces(8));
    assert_eq!(result, input);
}
